//! APIs for communication with Agent harness (Pi)

use crate::{
    core::agent::{
        pi::{self, PiAgent, handle_graceful_exit},
        types::{PiAgentEndEvent, PiMsgContent, PiResponse},
    },
    core::chats::append_turn_to_snapshot,
    core::storage::db::{DBTYPE, get_db_conn},
    daemon::{ApiResponse, AppError, AppState},
    repl::{get_default_modelfile, model_spec},
    utils::config::{ConfigProvider, DefaultProvider, PY_PORT},
    utils::lexicons::Turn,
};

// use async_stream::stream;
use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Sse, sse::Event},
    routing::{get, post},
};
use axum_macros::debug_handler;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Sender};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
#[derive(Deserialize)]
struct PromptRequest {
    message: String,
    /// Which session the turn belongs to. Without it the turn still runs, it
    /// just leaves no snapshot behind, which is what sharing publishes.
    #[serde(default)]
    session_id: Option<String>,
}

struct SseEvent {
    event: String,
    data: String,
}

struct SseGuard {
    pub token: CancellationToken,
}

impl Drop for SseGuard {
    fn drop(&mut self) {
        log::info!("Stream dropped, cancelling cancel_token");
        self.token.cancel();
    }
}
pub fn agent_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/agent/start", get(start_agent))
        .route("/v1/tilekit/agent/end_session", get(end_current_session))
        .route("/v1/tilekit/agent/state", get(agent_state))
        .route("/v1/tilekit/agent/prompt", post(process_chat_prompt))
        .route("/v1/tilekit/agent/reload", get(reload_agent))
}

async fn start_agent(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let (modelname, system_prompt) = get_agent_start_params(DefaultProvider)?;

    let mut agent = state.agent.lock().await;
    if agent.is_some() {
        Ok(ApiResponse::success(
            json!({"message": "Agent already started"}),
        ))
    } else {
        let pi_agent = pi::new(&modelname, &system_prompt, PY_PORT)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;
        *agent = Some(pi_agent);
        Ok(ApiResponse::success(json!({"message": "started agent"})))
    }
}

pub fn get_agent_start_params(provider: impl ConfigProvider) -> Result<(String, String), AppError> {
    let modelfile_path =
        get_default_modelfile(provider).map_err(|e| AppError::NotFound(e.to_string()))?;

    let default_modelfile = tilekit::modelfile::parse_from_file(&modelfile_path.to_string_lossy())
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let modelname =
        model_spec(&default_modelfile).map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let system_prompt = default_modelfile.system.clone().unwrap_or("".to_owned());

    Ok((modelname, system_prompt))
}

/// Drop the running agent and start a fresh one. The modelfile is only read at
/// start, so this is what applies an edited one without restarting the daemon.
async fn reload_agent(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    // before taking the old one down, so a bad modelfile leaves it running
    let (modelname, system_prompt) = get_agent_start_params(DefaultProvider)?;

    let mut agent = state.agent.lock().await;

    // pi is setsid into its own session and PiAgent has no Drop, so letting the
    // handle go would leave the process behind
    if let Some(current) = agent.take() {
        let (mut process, _, _) = current.split();
        let _ = process.kill().await;
    }

    let pi_agent = pi::new(&modelname, &system_prompt, PY_PORT)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    *agent = Some(pi_agent);

    Ok(ApiResponse::success(json!({"message": "reloaded agent"})))
}

#[debug_handler]
async fn end_current_session(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let mut agent = state.agent.lock().await;
    let agent = agent.as_mut().ok_or(AppError::InternalServerError(
        "Failed to get a mutable agent instance".to_string(),
    ))?;
    pi::handle_graceful_exit(&mut agent.writer)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(ApiResponse::success(
        json!({"message": "Successfully ended current session"}),
    ))
}

// TODO: Could we have explicity tell in return type we are sending
// GetStateData - derive serialize for GetStateData?
async fn agent_state(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let mut agent = state.agent.lock().await;
    let agent = agent.as_mut().ok_or(AppError::InternalServerError(
        "Failed to get a mutable agent instance".to_string(),
    ))?;

    let state = agent
        .reader
        .get_pi_state(&mut agent.writer)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(ApiResponse::success(serde_json::to_value(state).unwrap()))
}

#[debug_handler]
async fn process_chat_prompt(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PromptRequest>,
) -> Result<impl IntoResponse, AppError> {
    let t_state = state.clone();
    let session_id = payload.session_id.clone();
    let (tx, rx) = mpsc::channel::<SseEvent>(32);
    let cancel_token = CancellationToken::new();
    let t_cancel = cancel_token.clone();
    let _handle = tokio::spawn(async move {
        let mut agent = t_state.agent.lock().await;
        let agent = if let Some(agent) = agent.as_mut() {
            agent
        } else {
            let err_str = "Failed to get a mutable agent instance";
            handle_pi_errors(err_str.to_owned(), &tx).await;
            return;
        };

        let payload = json!({
            "type": "prompt",
            "message": payload.message
        });
        if let Err(err) = agent.writer.send_to_pi(payload).await {
            let err_str = format!("Failed to send the payload to Pi due to {:?}", err);
            handle_pi_errors(err_str.to_owned(), &tx).await;
            return;
        }
        let ended = tokio::select! {
                _ = t_cancel.cancelled() => {
                    log::info!("Will cancel the agent process");
                    let _ = handle_graceful_exit(&mut agent.writer).await;
                    // To read the rest of stdout after aborting the current request
                    read_from_pi(agent, &tx).await
                 },
                ended = read_from_pi(agent, &tx) => ended
        };

        // pi has reported the whole turn by now, so this is the one moment the
        // thinking and the tool calls exist together in one place
        if let (Some(session_id), Some(ended)) = (session_id, ended) {
            record_turn(&session_id, ended);
        }
    });

    let mut sse_stream = ReceiverStream::new(rx)
        .map(|msg| Ok::<_, Infallible>(Event::default().event(msg.event).data(msg.data)));

    let guarded_stream = async_stream::stream! {
        let _guard = SseGuard{
            token: cancel_token.clone()
        };
        while let Some(event) = sse_stream.next().await {
            yield event;
        }
    };

    Ok(Sse::new(guarded_stream))
}

async fn handle_pi_errors(err_str: String, tx: &Sender<SseEvent>) {
    log::error!("{err_str}");
    let event = SseEvent {
        event: "error".to_owned(),
        data: err_str,
    };
    let _ = tx.send(event).await.map_err(|e| log::error!("{:?}", e));
}

/// Returns the turn Pi reported, which is what a snapshot is built from.
async fn read_from_pi(agent: &mut PiAgent, tx: &Sender<SseEvent>) -> Option<PiAgentEndEvent> {
    let mut last_event = String::from("");
    let mut ended = None;

    while let Ok(Some(line)) = agent.reader.next_line().await {
        let response = if let Ok(response) = serde_json::from_str::<PiResponse>(&line) {
            response
        } else {
            let err_str = format!("Failed to parse pi response, response {:?}", &line);

            handle_pi_errors(err_str.to_owned(), tx).await;
            return ended;
        };

        let sse_event = SseEvent {
            event: response.get_type().to_owned(),
            data: line,
        };
        last_event = response.get_type().to_owned();
        let _ = tx.send(sse_event).await.map_err(|e| log::error!("{:?}", e));

        match response {
            PiResponse::AgentSettled => break,
            PiResponse::AgentEnd(event) => ended = Some(event),
            _ => continue,
        }
    }
    log::info!("reading ended with last event {}", last_event);

    ended
}

/// Nothing here is worth failing a turn over, the reply already reached the
/// caller. A missing snapshot only costs the richer share.
fn record_turn(session_id: &str, ended: PiAgentEndEvent) {
    let model = match get_agent_start_params(DefaultProvider) {
        Ok((model, _)) => model,
        Err(err) => {
            log::warn!("No model name for the snapshot: {err:?}");
            String::new()
        }
    };

    let mut turn = Turn {
        api: Some(String::from("open-responses")),
        provider: Some(String::from("tiles")),
        model,
        messages: ended.messages,
    };

    // pi reports the assistant's thinking and its answer as separate messages,
    // and a snapshot reads better with the parts of one reply kept together
    fold_assistant_messages(&mut turn);

    // the connection is not Send, so it must not outlive this synchronous scope
    let recorded = get_db_conn(&DBTYPE::CHAT)
        .and_then(|conn| append_turn_to_snapshot(&conn, session_id, turn));

    if let Err(err) = recorded {
        log::warn!("Could not record the turn for session {session_id}: {err:?}");
    }
}

fn fold_assistant_messages(turn: &mut Turn) {
    let mut folded: Vec<crate::core::agent::types::PiMsgEvent> = vec![];

    for message in turn.messages.drain(..) {
        let is_assistant = matches!(message.role, tilekit::modelfile::Role::Assistant);

        match folded.last_mut() {
            Some(previous)
                if is_assistant && matches!(previous.role, tilekit::modelfile::Role::Assistant) =>
            {
                let mut content: Vec<PiMsgContent> = message.content;
                previous.content.append(&mut content);
                previous.stop_reason = message.stop_reason;
            }
            _ => folded.push(message),
        }
    }

    turn.messages = folded;
}
#[cfg(test)]
mod tests {

    use crate::core::agent::pi::from_test_command;
    use crate::daemon::{AppState, agent::agent_router};
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use reqwest::StatusCode;
    use serde_json::json;
    use tokio::sync::Mutex as AsyncMutex;
    use tower::ServiceExt;
    #[tokio::test]
    async fn test_process_chat_prompt_success_ok() {
        let state = AppState::for_tests();
        let body = json!({
            "message": "hello"
        })
        .to_string();
        let agent_app = agent_router();
        let response = agent_app
            .with_state(state.into())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .header("content-type", "application/json")
                    .uri("/v1/tilekit/agent/prompt")
                    .body(Body::new(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // curl -X POST "http://127.0.0.1:1729/v1/tilekit/agent/prompt" \
        //   -H "Content-Type: application/json" \
        //   -d '{"message":"hello"}'
    }

    #[tokio::test]
    async fn test_process_chat_prompt_success_sse_events() {
        let pi_agent = from_test_command(
            "sh",
            &[
                "-c",
                r#"read request
  printf '{"type":"agent_start"}\n{"type":"message_end"}\n{"type":"agent_settled"}\n'"#,
            ],
        )
        .unwrap();

        let state = AppState {
            agent: AsyncMutex::new(Some(pi_agent)),
            ..AppState::for_tests()
        };

        let body = json!({
            "message": "hello"
        })
        .to_string();
        let agent_app = agent_router();
        let response = agent_app
            .with_state(state.into())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .header("content-type", "application/json")
                    .uri("/v1/tilekit/agent/prompt")
                    .body(Body::new(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let sse_events = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert!(sse_events.contains("event: agent_start"));
        assert!(sse_events.contains("event: message_end"));
    }

    #[tokio::test]
    async fn test_process_chat_prompt_error_sse_events() {
        let pi_agent = from_test_command(
            "sh",
            &[
                "-c",
                r#"read request
  printf '{"watevr":"agent_start"}\n{"type":"message_end"}\n{"type":"agent_settled"}\n'"#,
            ],
        )
        .unwrap();

        let state = AppState {
            agent: AsyncMutex::new(Some(pi_agent)),
            ..AppState::for_tests()
        };

        let body = json!({
            "message": "hello"
        })
        .to_string();
        let agent_app = agent_router();
        let response = agent_app
            .with_state(state.into())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .header("content-type", "application/json")
                    .uri("/v1/tilekit/agent/prompt")
                    .body(Body::new(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let sse_events = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert!(sse_events.contains("event: error"));
    }
}
