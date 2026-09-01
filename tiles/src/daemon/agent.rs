//! APIs for communication with Agent harness (Pi)

use crate::{
    core::agent::{
        pi::{self, PiAgent, handle_graceful_exit},
        types::PiResponse,
    },
    daemon::{ApiResponse, AppError, AppState},
    repl::{get_default_modelfile, model_spec},
    utils::config::{ConfigProvider, DefaultProvider, PY_PORT},
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
    use std::sync::Mutex;

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
        let state = AppState {
            shutdown_sender: Mutex::new(None),
            vsn: env!("CARGO_PKG_VERSION").to_owned(),
            remote_ticket: Mutex::new(None),
            remote_shutdown_sender: Mutex::new(None),
            remote_running: Mutex::new(false),
            agent: None.into(),
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
            shutdown_sender: Mutex::new(None),
            vsn: env!("CARGO_PKG_VERSION").to_owned(),
            remote_ticket: Mutex::new(None),
            remote_shutdown_sender: Mutex::new(None),
            remote_running: Mutex::new(false),
            agent: AsyncMutex::new(Some(pi_agent)),
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
            shutdown_sender: Mutex::new(None),
            vsn: env!("CARGO_PKG_VERSION").to_owned(),
            remote_ticket: Mutex::new(None),
            remote_shutdown_sender: Mutex::new(None),
            remote_running: Mutex::new(false),
            agent: AsyncMutex::new(Some(pi_agent)),
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
