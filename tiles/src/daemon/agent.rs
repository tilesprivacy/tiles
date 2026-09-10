//! APIs for communication with Agent harness (Pi)

use crate::{
    core::agent::{
        pi::{self, PiAgent, handle_graceful_exit},
        types::{PiAgentEndEvent, PiMsgContent, PiResponse},
    },
    core::chats::{append_turn_to_snapshot, fetch_chats_by_session_id, history_for_resume},
    core::plugin::{self, Invocation},
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
use serde::{Deserialize, Serialize};
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
        .route("/v1/tilekit/agent/commands", get(agent_commands))
}

/// One thing `@name` can reach, for the UI's mention picker.
#[derive(Serialize)]
struct Mention {
    name: String,
    description: String,
    kind: &'static str,
}

/// Everything `@name` resolves against, in the order resolution tries them:
/// plugins first, then skills, then plugin commands. The REPL builds the same
/// list for its `/skills` output; this is the UI's copy of it.
async fn agent_commands(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let mut agent = state.agent.lock().await;
    let agent = agent.as_mut().ok_or(AppError::InternalServerError(
        "Failed to get a mutable agent instance".to_string(),
    ))?;

    let commands = agent
        .reader
        .get_pi_commands(&mut agent.writer)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let vendor = plugin::vendor_dir();
    let mut mentions: Vec<Mention> = plugin::enabled_summaries()
        .into_iter()
        .map(|plugin| Mention {
            name: plugin.name,
            description: plugin.description,
            kind: "plugin",
        })
        .collect();

    for command in commands
        .iter()
        .filter(|command| !plugin::is_plumbing(command, vendor.as_deref()))
    {
        let (name, kind) = match command.name.strip_prefix("skill:") {
            Some(skill) => (skill, "skill"),
            None => (command.name.as_str(), "command"),
        };
        // resolution prefers the plugin, so a same-named entry would be a lie
        if mentions.iter().any(|mention| mention.name == name) {
            continue;
        }
        mentions.push(Mention {
            name: name.to_owned(),
            description: command.description.clone(),
            kind,
        });
    }

    Ok(ApiResponse::success(json!({ "mentions": mentions })))
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
        // a fresh Pi holds a conversation no session owns yet
        *state.active_session.lock().await = None;
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
    // a fresh Pi holds a conversation no session owns yet
    *state.active_session.lock().await = None;

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

        // `@name` is resolved here so both frontends mean the same thing by it:
        // the REPL resolves before sending, the UI sends it raw and this is
        // where the daemon does the same rewrite
        let mut message = payload.message;
        let mut stop_on_ack = false;
        if let Some(invoked) = message.trim().strip_prefix('@') {
            let commands = agent
                .reader
                .get_pi_commands(&mut agent.writer)
                .await
                .unwrap_or_else(|err| {
                    // costs only skill and command resolution, plugins still work
                    log::warn!("Could not load the command list: {err}");
                    vec![]
                });
            let plugins = plugin::enabled_summaries();
            let vendor = plugin::vendor_dir();

            match plugin::resolve_invocation(&plugins, &commands, vendor.as_deref(), invoked) {
                Some(Invocation::Describe { name, description }) => {
                    // nothing goes to Pi, so no model turn is wasted on it
                    send_event(
                        &tx,
                        "mention",
                        json!({ "resolved": "describe", "name": name, "description": description }),
                    )
                    .await;
                    return;
                }
                Some(invocation) => {
                    stop_on_ack = invocation.ends_on_ack();
                    if let Some(resolved) = invocation.message() {
                        message = resolved.to_owned();
                    }
                }
                None => {
                    let name = invoked.split_whitespace().next().unwrap_or(invoked);
                    let available: Vec<String> = plugins
                        .iter()
                        .map(|plugin| format!("@{}", plugin.name))
                        .collect();
                    send_event(
                        &tx,
                        "mention",
                        json!({ "resolved": "unknown", "name": name, "available": available }),
                    )
                    .await;
                    return;
                }
            }
        }

        // Pi holds one conversation, and nothing tells it when the UI switches
        // tabs. A prompt for a session other than the one Pi is on would be
        // answered with the wrong context, so reset Pi and replay the stored
        // turns first, the way the REPL resumes a session.
        if let Some(sid) = &session_id {
            let mut active = t_state.active_session.lock().await;
            if active.as_deref() != Some(sid.as_str()) {
                match agent.reader.create_new_session(&mut agent.writer).await {
                    Ok(_) => {
                        *active = Some(sid.clone());
                        let history = get_db_conn(&DBTYPE::CHAT)
                            .ok()
                            .and_then(|conn| fetch_chats_by_session_id(&conn, sid).ok())
                            .and_then(|delta| history_for_resume(&delta.chats, &message));
                        if let Some(history) = history {
                            message = format!(
                                "user_chat_history:\n{}.\nUse the history as context.\n[Followup question] - {}",
                                history, message
                            );
                        }
                    }
                    Err(err) => {
                        // the turn still runs; wrong context beats no answer,
                        // and the next prompt will try the switch again
                        log::warn!("Could not switch Pi to session {sid}: {err}");
                    }
                }
            }
        }

        let payload = json!({
            "type": "prompt",
            "message": message
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
                    read_from_pi(agent, &tx, stop_on_ack).await
                 },
                ended = read_from_pi(agent, &tx, stop_on_ack) => ended
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

async fn send_event(tx: &Sender<SseEvent>, event: &str, data: serde_json::Value) {
    let event = SseEvent {
        event: event.to_owned(),
        data: data.to_string(),
    };
    let _ = tx.send(event).await.map_err(|e| log::error!("{:?}", e));
}

/// Returns the turn Pi reported, which is what a snapshot is built from.
///
/// `stop_on_ack` covers a plugin command: the plugin answers it and no model
/// turn runs, so Pi's ack is the last event and waiting for `agent_settled`
/// would hang the stream.
async fn read_from_pi(
    agent: &mut PiAgent,
    tx: &Sender<SseEvent>,
    stop_on_ack: bool,
) -> Option<PiAgentEndEvent> {
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
            PiResponse::Response(_) if stop_on_ack => break,
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
    async fn test_an_unknown_mention_answers_without_a_model_turn() {
        // the fake pi answers the get_commands lookup and nothing else: an
        // unresolved `@name` must never reach it as a prompt
        let pi_agent = from_test_command(
            "sh",
            &[
                "-c",
                r#"read request
  printf '{"type":"response","success":true,"data":{"commands":[]}}\n'"#,
            ],
        )
        .unwrap();

        let state = AppState {
            agent: AsyncMutex::new(Some(pi_agent)),
            ..AppState::for_tests()
        };

        let body = json!({
            "message": "@no-such-plugin do something"
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

        assert!(sse_events.contains("event: mention"), "{}", sse_events);
        assert!(sse_events.contains("\"unknown\""), "{}", sse_events);
        assert!(sse_events.contains("no-such-plugin"), "{}", sse_events);
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
