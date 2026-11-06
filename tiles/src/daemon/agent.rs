//! APIs for communication with Agent harness (Pi)

use crate::{
    core::agent::{
        pi::{self, PiAgent, handle_graceful_exit},
        types::PiResponse,
    },
    daemon::{ApiJson, ApiResponse, AppError, AppState, model::picked_modelfile, session},
    repl::{get_default_modelfile, model_spec},
    utils::config::{ConfigProvider, DefaultProvider, PY_PORT, get_current_model},
};

// use async_stream::stream;
use axum::{
    Router,
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
        .route("/v1/tilekit/agent/start", post(start_agent))
        .route("/v1/tilekit/agent/end_session", post(end_current_session))
        .route("/v1/tilekit/agent/state", get(agent_state))
        .route("/v1/tilekit/agent/prompt", post(process_chat_prompt))
}

async fn start_agent(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let (modelname, system_prompt) = get_agent_start_params(DefaultProvider, get_current_model())?;

    let mut agent = state.agent.lock().await;
    if agent.is_some() {
        Ok(ApiResponse::success(
            json!({"message": "Agent already started"}),
        ))
    } else {
        let pi_agent = pi::new(&modelname, &system_prompt, PY_PORT)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;
        *agent = Some(pi_agent);
        *state
            .agent_model
            .lock()
            .expect("Failed to get agent_model lock") = Some(modelname);
        Ok(ApiResponse::success(json!({"message": "started agent"})))
    }
}

/// The model and system prompt from the picked model's Modelfile, else the default one
pub fn get_agent_start_params(
    provider: impl ConfigProvider,
    picked: Option<String>,
) -> Result<(String, String), AppError> {
    let modelfiles = provider
        .get_lib_dir()
        .map_err(|e| AppError::NotFound(e.to_string()))?
        .join("modelfiles");
    let modelfile_path = match picked.and_then(|picked| picked_modelfile(&modelfiles, &picked)) {
        Some(path) => path,
        None => get_default_modelfile(provider).map_err(|e| AppError::NotFound(e.to_string()))?,
    };

    let default_modelfile = tilekit::modelfile::parse_from_file(&modelfile_path.to_string_lossy())
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let modelname =
        model_spec(&default_modelfile).map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let system_prompt = default_modelfile.system.clone().unwrap_or("".to_owned());

    Ok((modelname, system_prompt))
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

    let mut pi_state = agent
        .reader
        .get_pi_state(&mut agent.writer)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    // a resumed chat keeps its own id, Pi's session for it is an inner detail
    if let Some(resumed) = state
        .resumed_chat
        .lock()
        .expect("resumed chat lock")
        .as_ref()
        && resumed.pi_session_id == pi_state.session_id
    {
        pi_state.session_id = resumed.chat_id.clone();
    }

    Ok(ApiResponse::success(
        serde_json::to_value(pi_state).unwrap(),
    ))
}

#[debug_handler]
async fn process_chat_prompt(
    State(state): State<Arc<AppState>>,
    ApiJson(payload): ApiJson<PromptRequest>,
) -> Result<impl IntoResponse, AppError> {
    let t_state = state.clone();
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

        let history = t_state
            .resumed_chat
            .lock()
            .expect("resumed chat lock")
            .as_mut()
            .and_then(|resumed| resumed.pending_history.take());
        let message = match &history {
            Some(history) => session::prompt_with_history(history, &payload.message),
            None => payload.message,
        };
        let payload = json!({
            "type": "prompt",
            "message": message
        });
        if let Err(err) = agent.writer.send_to_pi(payload).await {
            // Pi never got it, the next prompt carries the history instead
            if let (Some(history), Some(resumed)) = (
                history,
                t_state
                    .resumed_chat
                    .lock()
                    .expect("resumed chat lock")
                    .as_mut(),
            ) {
                resumed.pending_history = Some(history);
            }
            let err_str = format!("Failed to send the payload to Pi due to {:?}", err);
            handle_pi_errors(err_str.to_owned(), &tx).await;
            return;
        }
        tokio::select! {
                _ = t_cancel.cancelled() => {
                    log::info!("Will cancel the agent process");
                    let _ = handle_graceful_exit(&mut agent.writer).await;
                    // To read the rest of stdout after aborting the current request
                    let _ = read_from_pi(agent, &tx).await;
                 },
                _ = read_from_pi(agent, &tx) => ()
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

async fn read_from_pi(agent: &mut PiAgent, tx: &Sender<SseEvent>) {
    let mut last_event = String::from("");
    while let Ok(Some(line)) = agent.reader.next_line().await {
        let response = if let Ok(response) = serde_json::from_str::<PiResponse>(&line) {
            response
        } else {
            let err_str = format!("Failed to parse pi response, response {:?}", &line);

            handle_pi_errors(err_str.to_owned(), tx).await;
            return;
        };

        let sse_event = SseEvent {
            event: response.get_type().to_owned(),
            data: line,
        };
        last_event = response.get_type().to_owned();
        let _ = tx.send(sse_event).await.map_err(|e| log::error!("{:?}", e));

        match response {
            PiResponse::AgentSettled => break,
            _ => continue,
        }
    }
    log::info!("reading ended with last event {}", last_event);
}
#[cfg(test)]
mod tests {

    use crate::core::agent::pi::from_test_command;
    use crate::daemon::{AppState, agent::agent_router, session::ResumedChat};
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use reqwest::StatusCode;
    use serde_json::json;
    use std::sync::Mutex;
    use tempfile::tempdir;
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

    #[tokio::test]
    async fn a_resumed_chat_is_reported_under_its_own_id() {
        let pi_agent = from_test_command(
            "sh",
            &[
                "-c",
                r#"read request; id=${request#*'"id":"'}; id=${id%%'"'*}
printf '{"type": "response", "id": "'"$id"'", "command": "get_state", "success": true, "data": {"sessionId": "pi-2", "model": {"id": "id", "name": "model"}, "thinkingLevel": "high", "isStreaming": false}}\n'"#,
            ],
        )
        .unwrap();
        let state = AppState {
            agent: AsyncMutex::new(Some(pi_agent)),
            resumed_chat: Mutex::new(Some(ResumedChat {
                pi_session_id: "pi-2".to_owned(),
                chat_id: "old".to_owned(),
                pending_history: None,
            })),
            ..AppState::for_tests()
        };

        let response = agent_router()
            .with_state(state.into())
            .oneshot(
                Request::builder()
                    .uri("/v1/tilekit/agent/state")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body.contains(r#""sessionId":"old""#), "{body}");
    }

    #[tokio::test]
    async fn the_first_prompt_after_a_resume_carries_the_chat_so_far() {
        let dir = tempdir().unwrap();
        let got = dir.path().join("got");
        let script = format!(
            r#"read request; printf '%s' "$request" > '{}'
printf '{{"type":"agent_settled"}}\n'"#,
            got.display()
        );
        let pi_agent = from_test_command("sh", &["-c", &script]).unwrap();
        let state = AppState {
            agent: AsyncMutex::new(Some(pi_agent)),
            resumed_chat: Mutex::new(Some(ResumedChat {
                pi_session_id: "pi-2".to_owned(),
                chat_id: "old".to_owned(),
                pending_history: Some("User: capital of India?\n\nAssistant: New Delhi".to_owned()),
            })),
            ..AppState::for_tests()
        };
        let state = std::sync::Arc::new(state);
        let response = agent_router()
            .with_state(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .header("content-type", "application/json")
                    .uri("/v1/tilekit/agent/prompt")
                    .body(Body::new(
                        json!({ "message": "and of France?" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        response.into_body().collect().await.unwrap();

        let sent: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&got).unwrap()).unwrap();
        let message = sent["message"].as_str().unwrap();
        assert!(
            message.starts_with("Here is our conversation so far"),
            "{message}"
        );
        assert!(message.contains("Assistant: New Delhi"));
        assert!(message.ends_with("and of France?"));
        // only the first prompt carries it
        assert!(
            state
                .resumed_chat
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .pending_history
                .is_none()
        );
    }
}
