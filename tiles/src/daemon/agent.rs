//! APIs for communication with Agent harness (Pi)

use crate::{
    core::{
        account::local::{User, get_current_user},
        agent::{
            pi::{self, PiAgent, handle_graceful_exit},
            types::{
                CommandType, PiAgentEndEvent, PiMsgEvent, PiResponse, PiResponseMessage,
                ReasoningEffort,
            },
        },
        chats::{self, OpenTurn},
        storage::db::{DBTYPE, call_error, get_async_db_conn},
    },
    daemon::{
        ApiJson, ApiResponse, AppError, AppState,
        model::{get_default_modelfile, model_spec, picked_modelfile},
        session,
    },
    utils::config::{ConfigProvider, DefaultProvider, PY_PORT, get_current_model},
};

// use async_stream::stream;
use anyhow::anyhow;
use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Sse, sse::Event},
    routing::{get, post},
};
use axum_macros::debug_handler;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{
    MutexGuard,
    mpsc::{self, Sender},
};
use tokio_rusqlite::rusqlite;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
#[derive(Deserialize)]
struct PromptRequest {
    message: String,
}

#[derive(Deserialize)]
struct ThinkingLevelRequest {
    level: ReasoningEffort,
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
        .route("/v1/tilekit/agent/thinking-level", post(set_thinking_level))
        .route("/v1/tilekit/agent/skills", get(list_skills))
}

async fn start_agent(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let mut agent = state.agent.lock().await;
    if agent.is_some() {
        Ok(ApiResponse::success(
            json!({"message": "Agent already started"}),
        ))
    } else {
        // read with the agent locked, so a model being deleted isn't used
        let (modelname, system_prompt) =
            get_agent_start_params(DefaultProvider, get_current_model())?;
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

/// The model and system prompt of a Modelfile a client sent as text
pub fn agent_start_params_from_modelfile(modelfile: &str) -> Result<(String, String), AppError> {
    let invalid = |reason: String| AppError::BadRequest(format!("Invalid Modelfile: {reason}"));
    let modelfile = tilekit::modelfile::parse(modelfile).map_err(invalid)?;
    let modelname = model_spec(&modelfile).map_err(|e| invalid(e.to_string()))?;

    Ok((modelname, modelfile.system.unwrap_or_default()))
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
    pi_state.session_id = chat_id_of_pi_session(&state, pi_state.session_id);

    Ok(ApiResponse::success(
        serde_json::to_value(pi_state).unwrap(),
    ))
}

/// Sets the agent's thinking level and keeps it as the default for later chats.
/// Answers with the level the agent is on afterwards
async fn set_thinking_level(
    State(state): State<Arc<AppState>>,
    ApiJson(request): ApiJson<ThinkingLevelRequest>,
) -> Result<impl IntoResponse, AppError> {
    set_thinking_level_with(&state, DefaultProvider, request.level).await
}

async fn set_thinking_level_with(
    state: &AppState,
    provider: impl ConfigProvider,
    level: ReasoningEffort,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let mut agent = lock_idle_agent(state)?;
    let agent = agent.as_mut().ok_or_else(no_agent_running)?;
    let level_now = agent
        .reader
        .set_thinking_level(&mut agent.writer, level)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // Pi falls back to a level of its own when the model lacks the one asked for.
    // only a level Tiles knows is kept, the settings file can't hold another
    if let Ok(level) = level_now.parse::<ReasoningEffort>()
        && let Err(err) = pi::save_default_thinking_level(provider, level)
    {
        log::warn!("Couldn't keep the thinking level for later chats: {err:#}");
    }
    Ok(ApiResponse::success(json!({ "level": level_now })))
}

/// Lists the skills the agent has loaded, a prompt runs one with `$name`
async fn list_skills(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let mut agent = lock_idle_agent(&state)?;
    let agent = agent.as_mut().ok_or_else(no_agent_running)?;
    let skills = agent
        .reader
        .fetch_skills(&mut agent.writer)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(ApiResponse::success(json!({ "skills": skills })))
}

// resumed chats keep their own id instead of Pi's session id
fn chat_id_of_pi_session(state: &AppState, pi_session_id: String) -> String {
    match state
        .resumed_chat
        .lock()
        .expect("resumed chat lock")
        .as_ref()
    {
        Some(resumed) if resumed.pi_session_id == pi_session_id => resumed.chat_id.clone(),
        _ => pi_session_id,
    }
}

// reply holds the agent for as long as it streams
fn lock_idle_agent(state: &AppState) -> Result<MutexGuard<'_, Option<PiAgent>>, AppError> {
    state.agent.try_lock().map_err(|_| {
        AppError::Conflict("A reply is still streaming, wait for it to finish".to_owned())
    })
}

fn no_agent_running() -> AppError {
    AppError::Conflict("No agent is running, start a chat first".to_owned())
}

#[debug_handler]
async fn process_chat_prompt(
    State(state): State<Arc<AppState>>,
    ApiJson(payload): ApiJson<PromptRequest>,
) -> Result<impl IntoResponse, AppError> {
    let (tx, rx) = mpsc::channel::<SseEvent>(32);
    let cancel_token = CancellationToken::new();
    let store = if state.saves_turns {
        match open_chat_store().await {
            Ok(store) => Some(store),
            Err(err) => {
                report_failed_save(err, &tx).await;
                None
            }
        }
    } else {
        None
    };
    let _handle = tokio::spawn(run_prompt(
        state.clone(),
        payload.message,
        tx,
        cancel_token.clone(),
        store,
    ));

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

// sends the prompt to Pi and streams the reply into tx
async fn run_prompt(
    state: Arc<AppState>,
    typed: String,
    tx: Sender<SseEvent>,
    cancel: CancellationToken,
    store: Option<ChatStore>,
) {
    let mut agent = state.agent.lock().await;
    let agent = if let Some(agent) = agent.as_mut() {
        agent
    } else {
        let err_str = "Failed to get a mutable agent instance";
        handle_pi_errors(err_str.to_owned(), &tx).await;
        return;
    };

    // failed save doesn't stop the reply
    let saving = match store {
        Some(store) => match save_prompt(&state, agent, store, typed.clone()).await {
            Ok(saving) => Some(saving),
            Err(err) => {
                report_failed_save(err, &tx).await;
                None
            }
        },
        None => None,
    };

    let history = state
        .resumed_chat
        .lock()
        .expect("resumed chat lock")
        .as_mut()
        .and_then(|resumed| resumed.pending_history.take());
    let message = match &history {
        Some(history) => session::prompt_with_history(history, &typed),
        None => typed,
    };
    let payload = json!({
        "type": "prompt",
        "message": message
    });
    if let Err(err) = agent.writer.send_to_pi(payload).await {
        // Pi never got it, the next prompt carries the history instead
        keep_history(&state, history);
        let err_str = format!("Failed to send the payload to Pi due to {:?}", err);
        handle_pi_errors(err_str.to_owned(), &tx).await;
        return;
    }
    // shared by both reads below
    let mut turn_messages = vec![];
    let read = tokio::select! {
            _ = cancel.cancelled() => {
                log::info!("Will cancel the agent process");
                let _ = handle_graceful_exit(&mut agent.writer).await;
                // To read the rest of stdout after aborting the current request
                read_from_pi(agent, &tx, &mut turn_messages).await
             },
            read = read_from_pi(agent, &tx, &mut turn_messages) => read
    };
    if read == PiRead::Refused {
        keep_history(&state, history);
    }
    if let Some(saving) = saving
        && !turn_messages.is_empty()
        && let Err(err) = save_reply(saving, turn_messages).await
    {
        report_failed_save(err, &tx).await;
    }
}

// chats db and the account that owns the turn
struct ChatStore {
    chats: tokio_rusqlite::Connection,
    user: User,
}

async fn open_chat_store() -> anyhow::Result<ChatStore> {
    let user = get_async_db_conn(&DBTYPE::COMMON)?
        .call(|conn| get_current_user(conn))
        .await
        .map_err(call_error)
        .map_err(|err| match err.downcast_ref::<rusqlite::Error>() {
            Some(rusqlite::Error::QueryReturnedNoRows) => {
                anyhow!("No local account, create one first")
            }
            _ => err,
        })?;

    Ok(ChatStore {
        chats: get_async_db_conn(&DBTYPE::CHAT)?,
        user,
    })
}

struct SavingTurn {
    store: ChatStore,
    turn: OpenTurn,
}

// saves to the chat Pi is in
async fn save_prompt(
    state: &AppState,
    agent: &mut PiAgent,
    store: ChatStore,
    prompt: String,
) -> anyhow::Result<SavingTurn> {
    let pi_state = agent.reader.get_pi_state(&mut agent.writer).await?;
    let chat_id = chat_id_of_pi_session(state, pi_state.session_id);
    let model = session::agent_model(state).map_err(|err| anyhow!(err.parts().1))?;

    let user = store.user.clone();
    let turn = store
        .chats
        .call(move |conn| chats::save_prompt(conn, &user, &chat_id, &model, &prompt))
        .await
        .map_err(call_error)?;
    Ok(SavingTurn { store, turn })
}

async fn save_reply(saving: SavingTurn, messages: Vec<PiMsgEvent>) -> anyhow::Result<()> {
    let SavingTurn { store, turn } = saving;
    let ChatStore { chats, user } = store;
    chats
        .call(move |conn| chats::save_reply(conn, &user, turn, messages))
        .await
        .map_err(call_error)
}

// separate from the error event
async fn report_failed_save(err: anyhow::Error, tx: &Sender<SseEvent>) {
    log::error!("Couldn't save the chat: {err:#}");
    let event = SseEvent {
        event: "save_failed".to_owned(),
        data: json!({ "type": "save_failed", "reason": format!("{err:#}") }).to_string(),
    };
    let _ = tx.send(event).await.map_err(|e| log::error!("{:?}", e));
}

async fn handle_pi_errors(err_str: String, tx: &Sender<SseEvent>) {
    log::error!("{err_str}");
    let event = SseEvent {
        event: "error".to_owned(),
        data: err_str,
    };
    let _ = tx.send(event).await.map_err(|e| log::error!("{:?}", e));
}

/// Puts back the history a prompt Pi never ran was carrying
fn keep_history(state: &AppState, history: Option<String>) {
    if let (Some(history), Some(resumed)) = (
        history,
        state
            .resumed_chat
            .lock()
            .expect("resumed chat lock")
            .as_mut(),
    ) {
        resumed.pending_history = Some(history);
    }
}

#[derive(Debug, PartialEq)]
enum PiRead {
    Settled,
    /// Pi turned the prompt down, it sends no `agent_settled` then
    Refused,
    Stopped,
}

// streams Pi's lines into tx and collects the agent_end messages
async fn read_from_pi(
    agent: &mut PiAgent,
    tx: &Sender<SseEvent>,
    turn_messages: &mut Vec<PiMsgEvent>,
) -> PiRead {
    let mut last_event = String::from("");
    while let Ok(Some(line)) = agent.reader.next_line().await {
        let response = if let Ok(response) = serde_json::from_str::<PiResponse>(&line) {
            response
        } else {
            let err_str = format!("Failed to parse pi response, response {:?}", &line);

            handle_pi_errors(err_str.to_owned(), tx).await;
            return PiRead::Stopped;
        };

        let sse_event = SseEvent {
            event: response.get_type().to_owned(),
            data: line,
        };
        last_event = response.get_type().to_owned();
        let _ = tx.send(sse_event).await.map_err(|e| log::error!("{:?}", e));

        match response {
            // retries send agent_end more than once
            PiResponse::AgentEnd(PiAgentEndEvent { messages }) => turn_messages.extend(messages),
            PiResponse::AgentSettled => {
                log::info!("reading ended with last event {last_event}");
                return PiRead::Settled;
            }
            PiResponse::Response(PiResponseMessage {
                command: CommandType::Prompt,
                success: false,
                error,
                ..
            }) => {
                let reason = error.unwrap_or_else(|| "no reason given".to_owned());
                handle_pi_errors(format!("Pi didn't take the prompt: {reason}"), tx).await;
                return PiRead::Refused;
            }
            _ => continue,
        }
    }
    log::info!("reading ended with last event {}", last_event);
    PiRead::Stopped
}
#[cfg(test)]
mod tests {

    use super::{ChatStore, run_prompt, set_thinking_level_with};
    use crate::core::agent::{pi::from_test_command, types::ReasoningEffort};
    use crate::core::chats;
    use crate::daemon::{
        AppState, account::tests::MockProvider, agent::agent_router, session::ResumedChat,
    };
    use axum::response::IntoResponse;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use reqwest::StatusCode;
    use serde_json::{Value, json};
    use std::sync::Mutex;
    use tempfile::tempdir;
    use tokio::sync::Mutex as AsyncMutex;
    use tokio::sync::mpsc;
    use tokio_rusqlite::rusqlite;
    use tokio_util::sync::CancellationToken;
    use tower::ServiceExt;

    // a fake Pi that answers each request it reads with the next of `answers`, where $id is
    // that request's id
    fn pi_answering(answers: &[&str]) -> crate::core::agent::pi::PiAgent {
        let script: String = answers
            .iter()
            .map(|answer| {
                format!(
                    "read request; id=${{request#*'\"id\":\"'}}; id=${{id%%'\"'*}}\nprintf '%s\\n' {answer}\n"
                )
            })
            .collect();
        from_test_command("sh", &["-c", &script]).unwrap()
    }

    // Pi 0.84 changes the level at once but only answers when the next request comes in
    const LEVEL_CHANGED: &str = r#"'{"type":"thinking_level_changed"}'"#;

    fn late_answer_then_state_on_level(level: &str) -> String {
        format!(
            r#"'{{"type":"response","command":"set_thinking_level","success":true}}' '{{"type":"response","id":"'"$id"'","command":"get_state","success":true,"data":{{"sessionId":"s1","model":{{"id":"id","name":"model"}},"thinkingLevel":"{level}","isStreaming":false}}}}'"#
        )
    }

    async fn call_agent_route(state: AppState, request: Request<Body>) -> (StatusCode, Value) {
        let response = agent_router()
            .with_state(state.into())
            .oneshot(request)
            .await
            .unwrap();
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&body).unwrap())
    }

    fn post_thinking_level(level: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .header("content-type", "application/json")
            .uri("/v1/tilekit/agent/thinking-level")
            .body(Body::new(json!({ "level": level }).to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn a_thinking_level_is_set_and_kept_for_later_chats() {
        let tmp_dir = tempdir().unwrap();
        let settings = tmp_dir.path().join("pi/agent/settings.json");
        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        std::fs::write(&settings, r#"{"compaction":{"enabled":false}}"#).unwrap();
        let state = AppState {
            agent: AsyncMutex::new(Some(pi_answering(&[
                LEVEL_CHANGED,
                &late_answer_then_state_on_level("high"),
            ]))),
            ..AppState::for_tests()
        };
        let provider = MockProvider {
            tmp_path: tmp_dir.path().to_path_buf(),
        };

        let response = set_thinking_level_with(&state, provider, ReasoningEffort::High)
            .await
            .unwrap()
            .into_response();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["data"]["level"], "high");
        let saved: Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert_eq!(saved["defaultThinkingLevel"], "high");
        assert_eq!(saved["compaction"]["enabled"], false, "{saved}");
    }

    #[tokio::test]
    async fn a_level_the_model_lacks_is_reported_but_not_kept() {
        let tmp_dir = tempdir().unwrap();
        let state = AppState {
            agent: AsyncMutex::new(Some(pi_answering(&[
                LEVEL_CHANGED,
                &late_answer_then_state_on_level("off"),
            ]))),
            ..AppState::for_tests()
        };
        let provider = MockProvider {
            tmp_path: tmp_dir.path().to_path_buf(),
        };

        let response = set_thinking_level_with(&state, provider, ReasoningEffort::High)
            .await
            .unwrap()
            .into_response();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["data"]["level"], "off");
        assert!(!tmp_dir.path().join("pi/agent/settings.json").exists());
    }

    #[tokio::test]
    async fn a_thinking_level_needs_a_running_agent_and_a_known_level() {
        let (status, body) =
            call_agent_route(AppState::for_tests(), post_thinking_level("low")).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["reason"], "No agent is running, start a chat first");

        let (status, _) =
            call_agent_route(AppState::for_tests(), post_thinking_level("extreme")).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn a_modelfile_gives_the_model_and_system_prompt_to_start_with() {
        let (model, system_prompt) = super::agent_start_params_from_modelfile(
            "FROM org/custom-GGUF:Q4_K_M\nSYSTEM be brief\n",
        )
        .unwrap();
        assert_eq!(model, "org/custom-GGUF:Q4_K_M");
        assert_eq!(system_prompt, "be brief");

        let (_, system_prompt) =
            super::agent_start_params_from_modelfile("FROM org/custom-GGUF:Q4_K_M\n").unwrap();
        assert_eq!(system_prompt, "");
    }

    #[tokio::test]
    async fn skills_are_listed_by_name_without_pis_other_commands() {
        let commands = r#"'{"type":"response","id":"'"$id"'","command":"get_commands","success":true,"data":{"commands":[{"name":"session-name","source":"extension"},{"name":"fix-tests","description":"Fix failing tests","source":"prompt"},{"name":"skill:brave-search","description":"Web search","source":"skill"}]}}'"#;
        let state = AppState {
            agent: AsyncMutex::new(Some(pi_answering(&[commands]))),
            ..AppState::for_tests()
        };
        let request = Request::builder()
            .uri("/v1/tilekit/agent/skills")
            .body(Body::empty())
            .unwrap();

        let (status, body) = call_agent_route(state, request).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body["data"]["skills"],
            json!([{ "name": "brave-search", "description": "Web search" }])
        );
    }

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

    const STATE_IN_PI_SESSION_S1: &str = r#"'{"type":"response","id":"'"$id"'","command":"get_state","success":true,"data":{"sessionId":"s1","model":{"id":"id","name":"model"},"thinkingLevel":"off","isStreaming":false}}'"#;

    // user message carries the resume history
    const A_TURN_ENDING_IN_HELLO: &str = r#"'{"type":"agent_start"}' '{"type":"agent_end","messages":[{"role":"user","content":[{"type":"text","text":"the chat so far, then: hi"}],"timestamp":1},{"role":"assistant","content":[{"type":"thinking","thinking":"a greeting"},{"type":"text","text":"hello"}],"timestamp":2}]}' '{"type":"agent_settled"}'"#;

    fn state_with_pi_in_resumed_chat(chat_id: &str) -> AppState {
        AppState {
            agent: AsyncMutex::new(Some(pi_answering(&[
                STATE_IN_PI_SESSION_S1,
                A_TURN_ENDING_IN_HELLO,
            ]))),
            agent_model: Mutex::new(Some("llama3.2".to_owned())),
            resumed_chat: Mutex::new(Some(ResumedChat {
                pi_session_id: "s1".to_owned(),
                chat_id: chat_id.to_owned(),
                pending_history: Some("User: earlier".to_owned()),
            })),
            ..AppState::for_tests()
        }
    }

    async fn events_of_prompt(state: AppState, store: ChatStore) -> Vec<String> {
        let (tx, mut rx) = mpsc::channel(32);
        let cancel = CancellationToken::new();
        run_prompt(state.into(), "hi".to_owned(), tx, cancel, Some(store)).await;
        let mut events = vec![];
        while let Some(event) = rx.recv().await {
            events.push(event.event);
        }
        events
    }

    #[tokio::test]
    async fn a_turn_is_saved_to_the_chat_pi_is_in() {
        let chats = tokio_rusqlite::Connection::from(crate::core::chats::tests::setup_db_schema());
        let store = ChatStore {
            chats: chats.clone(),
            user: crate::core::chats::tests::create_user(),
        };

        let events = events_of_prompt(state_with_pi_in_resumed_chat("old"), store).await;

        assert_eq!(events, ["agent_start", "agent_end", "agent_settled"]);
        let (saved, snapshot) = chats
            .call(|conn| {
                let saved = chats::fetch_chats_by_session_id(conn, "old")?.chats;
                let snapshot = chats::session_snapshot(conn, "old")?;
                anyhow::Ok((saved, snapshot))
            })
            .await
            .unwrap();
        let saved: Vec<&str> = saved.iter().map(|chat| chat.content.as_str()).collect();
        assert_eq!(saved, ["hi", "hello"]);
        let turn = &snapshot.unwrap().turns[0];
        assert_eq!(turn.model, "llama3.2");
        assert_eq!(turn.messages[0].content[0].text.as_deref(), Some("hi"));
        assert_eq!(
            turn.messages[1].content[0].thinking.as_deref(),
            Some("a greeting")
        );
    }

    #[tokio::test]
    async fn a_turn_that_cant_be_saved_is_reported_and_still_answered() {
        // db without tables
        let chats =
            tokio_rusqlite::Connection::from(rusqlite::Connection::open_in_memory().unwrap());
        let store = ChatStore {
            chats,
            user: crate::core::chats::tests::create_user(),
        };

        let events = events_of_prompt(state_with_pi_in_resumed_chat("old"), store).await;

        assert_eq!(
            events,
            ["save_failed", "agent_start", "agent_end", "agent_settled"]
        );
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

    #[tokio::test]
    async fn a_prompt_pi_turns_down_ends_the_reply_and_keeps_the_history() {
        // stdout stays open, as a running Pi's does
        let pi_agent = from_test_command(
            "sh",
            &[
                "-c",
                r#"read request
printf '{"type":"response","command":"prompt","success":false,"error":"Agent is already processing"}\n'
sleep 5"#,
            ],
        )
        .unwrap();
        let state = std::sync::Arc::new(AppState {
            agent: AsyncMutex::new(Some(pi_agent)),
            resumed_chat: Mutex::new(Some(ResumedChat {
                pi_session_id: "pi-2".to_owned(),
                chat_id: "old".to_owned(),
                pending_history: Some("User: hi".to_owned()),
            })),
            ..AppState::for_tests()
        });
        let started = std::time::Instant::now();
        let response = agent_router()
            .with_state(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .header("content-type", "application/json")
                    .uri("/v1/tilekit/agent/prompt")
                    .body(Body::new(json!({ "message": "hello" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let events = String::from_utf8(bytes.to_vec()).unwrap();

        assert!(started.elapsed() < std::time::Duration::from_secs(4));
        assert!(events.contains("Agent is already processing"), "{events}");
        assert!(state.agent.try_lock().is_ok());
        assert_eq!(
            state
                .resumed_chat
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .pending_history
                .as_deref(),
            Some("User: hi")
        );
    }
}
