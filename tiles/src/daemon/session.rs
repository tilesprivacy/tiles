//! High level apis for sessions

use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tilekit::modelfile::Role;
use tokio_rusqlite::rusqlite::{self, Connection};

use crate::{
    core::{
        account::local::get_current_user,
        agent::pi::{self, PiAgent},
        chats::{self as sessionChats, Chats},
        storage::db::get_db_conn,
    },
    repl::ChatResponse,
};

use crate::utils::config::{
    ConfigProvider, DEFAULT_CONTEXT_LENGTH, DefaultProvider, get_current_model,
    get_pi_context_window,
};
use crate::{
    daemon::{ApiJson, ApiResponse, AppError, AppState, agent::get_agent_start_params},
    utils::config::PY_PORT,
};

#[derive(Serialize)]
struct Session {
    id: String,
}

/// An old chat Pi goes on with, in a Pi session of its own
pub struct ResumedChat {
    pub pi_session_id: String,
    pub chat_id: String,
    /// The chat so far, sent ahead of the next prompt and then dropped
    pub pending_history: Option<String>,
}

#[derive(Deserialize)]
struct ResumeRequest {
    session_id: String,
}

// the account and the model come from the daemon, so clients that still send
// `user_id` or `model_used` have them ignored
#[derive(Deserialize)]
struct SaveChatRequest {
    text: String,
    session_id: String,
    role: Role,
    parent_chat_id: Option<String>,
}

const TURN_BREAK: &str = "\n\n";

// factory type for Pi::new() fn, so that we can write some test
type CreateNewAgentFn = fn(&str, &str, u32) -> anyhow::Result<PiAgent>;

pub fn session_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/session/new", post(create_session))
        .route("/v1/tilekit/session/resume", post(resume_session))
        .route("/v1/tilekit/session/list", get(fetch_sessions))
        .route("/v1/tilekit/session/chat", post(save_chat))
        .route(
            "/v1/tilekit/session/{session_id}/chats",
            get(fetch_chats_by_session),
        )
}

// Creates a new session or starts the agent
async fn create_session(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    do_create_session(state, pi::new, DefaultProvider, get_current_model).await
}

async fn do_create_session(
    state: Arc<AppState>,
    create_agent_fn: CreateNewAgentFn,
    provider: impl ConfigProvider,
    picked: impl FnOnce() -> Option<String>,
) -> Result<impl IntoResponse, AppError> {
    let mut agent = state.agent.lock().await;
    let id = start_pi_session(&state, &mut agent, create_agent_fn, provider, picked()).await?;
    // a new chat ends the old one Pi went on with
    *state.resumed_chat.lock().expect("resumed chat lock") = None;
    Ok(ApiResponse::success(Session { id }))
}

/// A fresh Pi session, starting Pi when it isn't running. Returns Pi's id for it.
/// `picked` is read with the agent locked, so a model being deleted isn't used
async fn start_pi_session(
    state: &AppState,
    agent: &mut Option<PiAgent>,
    create_agent_fn: CreateNewAgentFn,
    provider: impl ConfigProvider,
    picked: Option<String>,
) -> Result<String, AppError> {
    let running = state
        .agent_model
        .lock()
        .expect("Failed to get agent_model lock")
        .clone();
    // keep a running agent whose model isn't known
    let params = if agent.is_none() || running.is_some() {
        Some(get_agent_start_params(provider, picked)?)
    } else {
        None
    };
    // a model picked since Pi started takes over with the next chat
    if let (Some(running), Some((wanted, _))) = (&running, &params)
        && running != wanted
        && let Some(mut old) = agent.take()
    {
        log::info!("Restarting Pi for {wanted}");
        let _ = old.process.kill().await;
    }
    if agent.is_some() {
        log::info!("Pi session already there, create a new session..");
        // agent already there, so lets create a new session
        let agent = agent.as_mut().ok_or(AppError::InternalServerError(
            "Failed to get a mutable agent instance".to_string(),
        ))?;

        let agent_state = agent
            .reader
            .create_new_session(&mut agent.writer)
            .await
            .map_err(|e| AppError::CannotProcess(e.to_string()))?;

        Ok(agent_state.session_id)
    } else {
        log::info!("Pi agent not started, so starting..");
        let (modelname, system_prompt) = params.ok_or_else(|| {
            AppError::InternalServerError("No model to start the agent with".to_owned())
        })?;
        let pi_agent = create_agent_fn(&modelname, &system_prompt, PY_PORT)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;
        *agent = Some(pi_agent);
        *state
            .agent_model
            .lock()
            .expect("Failed to get agent_model lock") = Some(modelname);
        let agent = agent.as_mut().ok_or(AppError::InternalServerError(
            "No agent instance available, start one first".to_string(),
        ))?;

        let state = agent
            .reader
            .get_pi_state(&mut agent.writer)
            .await
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;
        Ok(state.session_id)
    }
}

/// Goes on with an old chat. Pi can't load one, so it starts a fresh session and
/// the next prompt carries the chat so far, see [`with_history`]. Answers with the
/// chat's own id, which `agent/state` reports from then on
async fn resume_session(
    State(state): State<Arc<AppState>>,
    ApiJson(resume): ApiJson<ResumeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let chat_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::CHAT)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    let turns = load_chat_turns(&chat_db_conn, &resume.session_id)?;
    let history = newest_turns(&turns, history_budget(get_pi_context_window()));
    do_resume_session(
        state,
        pi::new,
        DefaultProvider,
        get_current_model,
        resume.session_id,
        history,
    )
    .await
}

async fn do_resume_session(
    state: Arc<AppState>,
    create_agent_fn: CreateNewAgentFn,
    provider: impl ConfigProvider,
    picked: impl FnOnce() -> Option<String>,
    chat: String,
    history: String,
) -> Result<impl IntoResponse, AppError> {
    // a reply holds the agent for as long as it streams. held until the chat is
    // set, so the next prompt gets its history
    let mut agent = state.agent.try_lock().map_err(|_| {
        AppError::Conflict("A reply is still streaming, wait for it to finish".to_owned())
    })?;
    let pi_session_id =
        start_pi_session(&state, &mut agent, create_agent_fn, provider, picked()).await?;
    *state.resumed_chat.lock().expect("resumed chat lock") = Some(ResumedChat {
        pi_session_id,
        chat_id: chat.clone(),
        pending_history: (!history.is_empty()).then_some(history),
    });
    Ok(ApiResponse::success(Session { id: chat }))
}

/// The saved chat as `User:` and `Assistant:` turns
fn load_chat_turns(chat_db_conn: &Connection, session_id: &str) -> Result<Vec<String>, AppError> {
    sessionChats::fetch_session(chat_db_conn, session_id)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("No session {session_id}")))?;
    let delta = sessionChats::fetch_chats_by_session_id(chat_db_conn, session_id)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    let turns = delta
        .chats
        .iter()
        .filter_map(|chat| {
            let speaker = match chat.role {
                Role::User => "User",
                Role::Assistant => "Assistant",
                _ => return None,
            };
            Some(format!("{speaker}: {}", chat.content))
        })
        .collect();
    Ok(turns)
}

/// About half the context window in characters, at a rough 3 per token
fn history_budget(context_window: Option<u32>) -> usize {
    context_window.unwrap_or(DEFAULT_CONTEXT_LENGTH) as usize * 3 / 2
}

/// The newest turns that fit in `budget` characters, joined. Pi's compaction is
/// off, so a long chat would overflow the model's context
fn newest_turns(turns: &[String], budget: usize) -> String {
    let mut used = 0;
    let first = turns
        .iter()
        .rposition(|turn| {
            used += turn.len() + TURN_BREAK.len();
            used > budget
        })
        .map_or(0, |over| over + 1);
    let kept = turns[first..].join(TURN_BREAK);
    if first == 0 {
        kept
    } else {
        format!("(Older messages are left out){TURN_BREAK}{kept}")
    }
}

/// The prompt Pi gets for the first message after a resume. The UI shows and
/// saves only `message`
pub fn prompt_with_history(history: &str, message: &str) -> String {
    format!(
        "Here is our conversation so far, for context:\n\n{history}\n\nNow continue it. My next message:\n\n{message}"
    )
}

async fn fetch_sessions(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let chat_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::CHAT)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    do_fetch_sessions(state, chat_db_conn)
}

fn do_fetch_sessions(
    _state: Arc<AppState>,
    chat_db_conn: Connection,
) -> Result<impl IntoResponse, AppError> {
    let sessions = sessionChats::fetch_sessions(&chat_db_conn, None)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(ApiResponse::success(sessions))
}

async fn save_chat(
    State(state): State<Arc<AppState>>,
    ApiJson(save_request): ApiJson<SaveChatRequest>,
) -> Result<impl IntoResponse, AppError> {
    let chat_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::CHAT)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    let user_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    let result = do_save_chat(state, save_request, &chat_db_conn, &user_db_conn)?;

    Ok(ApiResponse::success(result))
}

fn do_save_chat(
    state: Arc<AppState>,
    save_request: SaveChatRequest,
    chat_db_conn: &Connection,
    user_db_conn: &Connection,
) -> Result<Chats, AppError> {
    let current_user =
        get_current_user(user_db_conn).map_err(|e| match e.downcast_ref::<rusqlite::Error>() {
            Some(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound("No local account, create one first".to_owned())
            }
            _ => AppError::InternalServerError(e.to_string()),
        })?;

    let session = sessionChats::fetch_session(chat_db_conn, &save_request.session_id)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    // only a user's message can start a session
    if session.is_none() && save_request.role != Role::User {
        return Err(AppError::NotFound("Session doesnt exist".to_owned()));
    }

    let session_name = save_request.text.clone();
    let chat_response = ChatResponse {
        input: save_request.text,
        session_id: save_request.session_id,
        role: save_request.role,
        parent_chat_id: save_request.parent_chat_id,
        metrics: None,
        model_used: agent_model(&state)?,
    };
    let saved = if session.is_none() {
        log::info!("Session doesn't exist, create it");
        sessionChats::save_first_chat(chat_db_conn, &current_user, &session_name, chat_response)
    } else {
        sessionChats::save_chat(chat_db_conn, &current_user, chat_response)
    };

    saved.map_err(|e| AppError::InternalServerError(format!("Saving chat failed due to {}", e)))
}

/// The model the agent runs, or the one it will start with when it isn't running yet
fn agent_model(state: &AppState) -> Result<String, AppError> {
    let running = state
        .agent_model
        .lock()
        .expect("Failed to get agent_model lock")
        .clone();
    match running {
        Some(model) => Ok(model),
        None => {
            get_agent_start_params(DefaultProvider, get_current_model()).map(|(model, _)| model)
        }
    }
}

async fn fetch_chats_by_session(
    Path(session_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let chat_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::CHAT)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    sessionChats::fetch_session(&chat_db_conn, &session_id)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("No session {session_id}")))?;

    let delta = sessionChats::fetch_chats_by_session_id(&chat_db_conn, &session_id)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(ApiResponse::success(delta))
}

#[cfg(test)]
mod tests {

    use crate::{
        core::{
            account::local::{create_dummy_user, tests::setup_db_conn_v2},
            agent::pi::from_test_command,
            chats::tests::create_user,
        },
        daemon::account::tests::MockProvider,
    };
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use reqwest::StatusCode;
    use serde_json::json;
    use std::{
        fs::{self, File},
        io::Write,
        sync::Mutex,
    };
    use tempfile::tempdir;
    use tokio::sync::Mutex as AsyncMutex;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn test_create_new_session_when_agent_exists() {
        let pi_agent = from_test_command(
            "sh",
            &[
                "-c",
                r#"read request; id=${request#*'"id":"'}; id=${id%%'"'*}
  printf '{"type": "response", "id": "'"$id"'", "command": "new_session", "success": true, "data": {"cancelled": false}}\n'
    read request; id=${request#*'"id":"'}; id=${id%%'"'*}
 printf '{"type": "response", "id": "'"$id"'", "command": "get_state", "success": true, "data": {"sessionId": "session_id", "model": {"id": "id", "name": "model"}, "thinkingLevel": "high", "isStreaming": true}}\n'
  "#,
            ],
        )
        .unwrap();

        let state = AppState {
            agent: AsyncMutex::new(Some(pi_agent)),
            ..AppState::for_tests()
        };

        let body = json!({}).to_string();
        let agent_app = session_router();
        let response = agent_app
            .with_state(state.into())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .header("content-type", "application/json")
                    .uri("/v1/tilekit/session/new")
                    .body(Body::new(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();

        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body.contains("session_id"));
    }

    #[tokio::test]
    async fn test_create_new_session_no_agent_exists() {
        let tmp_dir = tempdir().unwrap();
        let mock_provider = MockProvider {
            tmp_path: tmp_dir.path().to_path_buf(),
        };

        let _pi_agent = from_test_command(
            "sh",
            &[
                "-c",
                r#"read request; id=${request#*'"id":"'}; id=${id%%'"'*}
  printf '{"type": "response", "id": "'"$id"'", "command": "new_session", "success": true, "data": {"cancelled": false}}\n'
    read request; id=${request#*'"id":"'}; id=${id%%'"'*}
 printf '{"type": "response", "id": "'"$id"'", "command": "get_state", "success": true, "data": {"sessionId": "session_id", "model": {"id": "id", "name": "model"}, "thinkingLevel": "high", "isStreaming": true}}\n'
  "#,
            ],
        )
        .unwrap();

        let state = AppState {
            agent: AsyncMutex::new(None),
            ..AppState::for_tests()
        };

        fs::create_dir(mock_provider.tmp_path.join("modelfiles")).unwrap();
        let mut f =
            File::create(mock_provider.tmp_path.join("modelfiles/gemma-4-12b-gguf")).unwrap();
        f.write_all("FROM llama3.2\nSYSTEM hello\n".as_bytes())
            .unwrap();
        let res =
            do_create_session(Arc::new(state), create_test_agent, mock_provider, || None).await;

        assert!(res.is_ok());
        let response = res.into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body.contains("session_id"))
    }

    #[tokio::test]
    async fn a_newly_picked_model_restarts_the_agent() {
        let tmp_dir = tempdir().unwrap();
        let mock_provider = MockProvider {
            tmp_path: tmp_dir.path().to_path_buf(),
        };
        fs::create_dir(mock_provider.tmp_path.join("modelfiles")).unwrap();
        fs::write(
            mock_provider.tmp_path.join("modelfiles/gemma-4-12b-gguf"),
            "FROM llama3.2\nSYSTEM hello\n",
        )
        .unwrap();
        // the old Pi answers a new session as "kept", the test fails if it's used
        let old = from_test_command(
            "sh",
            &[
                "-c",
                r#"read request; id=${request#*'"id":"'}; id=${id%%'"'*}
printf '%s\n' '{"type":"response","id":"'"$id"'","command":"new_session","success":true}'
read request; id=${request#*'"id":"'}; id=${id%%'"'*}
printf '%s\n' '{"type":"response","id":"'"$id"'","command":"get_state","success":true,"data":{"sessionId":"kept","model":{"id":"id","name":"old"},"thinkingLevel":"off","isStreaming":false}}'"#,
            ],
        )
        .unwrap();
        let state = Arc::new(AppState {
            agent: AsyncMutex::new(Some(old)),
            agent_model: Mutex::new(Some("org/old-GGUF:Q4_K_M".to_owned())),
            ..AppState::for_tests()
        });

        let res = do_create_session(state.clone(), create_test_agent, mock_provider, || None).await;

        let body = res
            .into_response()
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        assert!(
            String::from_utf8(body.to_vec())
                .unwrap()
                .contains("session_id")
        );
        assert_eq!(
            state.agent_model.lock().unwrap().as_deref(),
            Some("llama3.2")
        );
    }

    fn create_test_agent(_: &str, _: &str, _: u32) -> Result<PiAgent, anyhow::Error> {
        from_test_command(
            "sh",
            &[
                "-c",
                r#"
                    read request; id=${request#*'"id":"'}; id=${id%%'"'*}
 printf '{"type": "response", "id": "'"$id"'", "command": "get_state", "success": true, "data": {"sessionId": "session_id", "model": {"id": "id", "name": "model"}, "thinkingLevel": "high", "isStreaming": true}}\n'
                "#,
            ],
        )
    }

    #[tokio::test]
    async fn test_fetch_sessions_api() {
        let db_conn = setup_db_conn_v2();

        let user = create_user();
        if let Ok(None) = sessionChats::fetch_session(&db_conn.chat, "ad") {
            println!("No session created")
        }

        let _ =
            sessionChats::create_session(&db_conn.chat, "id-1", "sesh-1", &user.user_id).unwrap();

        let _ =
            sessionChats::create_session(&db_conn.chat, "id-2", "sesh-2", &user.user_id).unwrap();

        let _ =
            sessionChats::create_session(&db_conn.chat, "id-3", "sesh-3", &user.user_id).unwrap();

        let state = AppState {
            agent: AsyncMutex::new(None),
            ..AppState::for_tests()
        };

        let resp = do_fetch_sessions(state.into(), db_conn.chat)
            .unwrap()
            .into_response();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();

        println!("{:?}", String::from_utf8(body.to_vec()));
    }

    #[test]
    fn test_save_chat_api_session_dont_exist() {
        let db_conn = setup_db_conn_v2();

        let user = create_dummy_user(&db_conn.common, Some("did:key:xyz".to_owned()));

        let state = AppState {
            agent: AsyncMutex::new(None),
            agent_model: Mutex::new(Some("ada".to_owned())),
            ..AppState::for_tests()
        };

        // Test saving chat with new session

        let save_req = SaveChatRequest {
            text: "capital of India".to_string(),
            session_id: "id_non".to_owned(),
            role: Role::User,
            parent_chat_id: None,
        };

        let _res = do_save_chat(state.into(), save_req, &db_conn.chat, &db_conn.common).unwrap();

        assert!(
            sessionChats::fetch_session(&db_conn.chat, "id_non")
                .unwrap()
                .is_some()
        );

        let chats = sessionChats::fetch_chats_by_session_id(&db_conn.chat, "id_non").unwrap();

        assert_eq!(chats.chats.len(), 1);
        assert_eq!(chats.sessions.len(), 1);
        let chat = serde_json::to_value(&chats.chats[0]).unwrap();
        assert_eq!(chat["user_id"], user.user_id);
        assert_eq!(chat["model_name"], "ada");
    }

    #[test]
    fn test_save_chat_api_without_an_account() {
        let db_conn = setup_db_conn_v2();

        let state = AppState {
            agent: AsyncMutex::new(None),
            agent_model: Mutex::new(Some("ada".to_owned())),
            ..AppState::for_tests()
        };

        let save_req = SaveChatRequest {
            text: "capital of India".to_string(),
            session_id: "id_non".to_owned(),
            role: Role::User,
            parent_chat_id: None,
        };

        let err = do_save_chat(state.into(), save_req, &db_conn.chat, &db_conn.common).unwrap_err();
        println!("{:?}", err);
        assert!(matches!(err, AppError::NotFound(_)));
        assert!(
            sessionChats::fetch_session(&db_conn.chat, "id_non")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn test_save_chat_api_as_non_user_role_session_doesnt_exist() {
        let db_conn = setup_db_conn_v2();

        create_dummy_user(&db_conn.common, Some("did:key:xyz".to_owned()));

        let state = AppState {
            agent: AsyncMutex::new(None),
            agent_model: Mutex::new(Some("ada".to_owned())),
            ..AppState::for_tests()
        };

        // Test saving chat with new session

        let save_req = SaveChatRequest {
            text: "capital of India".to_string(),
            session_id: "id_non".to_owned(),
            role: Role::Assistant,
            parent_chat_id: None,
        };

        assert!(do_save_chat(state.into(), save_req, &db_conn.chat, &db_conn.common).is_err())
    }
    #[test]
    fn test_save_chat_api_session_exist_already() {
        let db_conn = setup_db_conn_v2();

        let user = create_dummy_user(&db_conn.common, Some("did:key:xyz".to_owned()));

        let _ =
            sessionChats::create_session(&db_conn.chat, "id-1", "sesh-1", &user.user_id).unwrap();

        let state = AppState {
            agent: AsyncMutex::new(None),
            agent_model: Mutex::new(Some("ada".to_owned())),
            ..AppState::for_tests()
        };

        let save_req = SaveChatRequest {
            text: "capital of India".to_string(),
            session_id: "id-1".to_owned(),
            role: Role::User,
            parent_chat_id: None,
        };

        let _res = do_save_chat(state.into(), save_req, &db_conn.chat, &db_conn.common).unwrap();

        assert!(
            sessionChats::fetch_session(&db_conn.chat, "id-1")
                .unwrap()
                .is_some()
        );

        let chats = sessionChats::fetch_chats_by_session_id(&db_conn.chat, "id-1").unwrap();

        assert_eq!(chats.chats.len(), 1);
        assert_eq!(chats.sessions.len(), 1);
    }

    // a fake Pi that opens a new session, then reports `session` as its id
    fn fake_pi_with_session(session: &str) -> PiAgent {
        let script = format!(
            r#"read request; id=${{request#*'"id":"'}}; id=${{id%%'"'*}}
printf '{{"type": "response", "id": "'"$id"'", "command": "new_session", "success": true, "data": {{"cancelled": false}}}}\n'
read request; id=${{request#*'"id":"'}}; id=${{id%%'"'*}}
printf '{{"type": "response", "id": "'"$id"'", "command": "get_state", "success": true, "data": {{"sessionId": "{session}", "model": {{"id": "id", "name": "model"}}, "thinkingLevel": "high", "isStreaming": false}}}}\n'"#
        );
        from_test_command("sh", &["-c", &script]).unwrap()
    }

    #[test]
    fn a_saved_chat_reads_back_as_turns() {
        let db_conn = setup_db_conn_v2();
        create_dummy_user(&db_conn.common, Some("did:key:xyz".to_owned()));
        let state: Arc<AppState> = AppState {
            agent_model: Mutex::new(Some("ada".to_owned())),
            ..AppState::for_tests()
        }
        .into();
        let mut parent = None;
        for (role, text) in [
            (Role::User, "capital of India?"),
            (Role::Assistant, "New Delhi"),
        ] {
            let save = SaveChatRequest {
                text: text.to_owned(),
                session_id: "old".to_owned(),
                role,
                parent_chat_id: parent,
            };
            parent = Some(
                do_save_chat(state.clone(), save, &db_conn.chat, &db_conn.common)
                    .unwrap()
                    .id,
            );
        }

        assert_eq!(
            load_chat_turns(&db_conn.chat, "old").unwrap(),
            ["User: capital of India?", "Assistant: New Delhi"]
        );
        assert!(matches!(
            load_chat_turns(&db_conn.chat, "gone"),
            Err(AppError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn resuming_goes_on_in_a_new_pi_session_under_the_chats_own_id() {
        let state = Arc::new(AppState {
            agent: AsyncMutex::new(Some(fake_pi_with_session("pi-2"))),
            ..AppState::for_tests()
        });

        let res = do_resume_session(
            state.clone(),
            create_test_agent,
            MockProvider {
                tmp_path: tempdir().unwrap().path().to_path_buf(),
            },
            || None,
            "old".to_owned(),
            "User: hi".to_owned(),
        )
        .await
        .into_response();
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        assert!(
            String::from_utf8(body.to_vec())
                .unwrap()
                .contains(r#""id":"old""#)
        );

        let resumed = state.resumed_chat.lock().unwrap();
        let resumed = resumed.as_ref().unwrap();
        assert_eq!(
            (resumed.pi_session_id.as_str(), resumed.chat_id.as_str()),
            ("pi-2", "old")
        );
        assert_eq!(resumed.pending_history.as_deref(), Some("User: hi"));
    }

    #[test]
    fn a_long_chat_resumes_with_its_newest_turns() {
        let turns =
            ["User: one", "Assistant: two\n\nin two parts", "User: three"].map(String::from);
        assert_eq!(newest_turns(&turns, 1000), turns.join(TURN_BREAK));
        assert_eq!(
            newest_turns(&turns, 45),
            "(Older messages are left out)\n\nAssistant: two\n\nin two parts\n\nUser: three"
        );
        assert_eq!(newest_turns(&[], 45), "");
    }

    #[tokio::test]
    async fn a_streaming_reply_holds_off_a_resume() {
        let state = Arc::new(AppState::for_tests());
        let _streaming = state.agent.lock().await;

        let res = do_resume_session(
            state.clone(),
            create_test_agent,
            MockProvider {
                tmp_path: tempdir().unwrap().path().to_path_buf(),
            },
            || None,
            "old".to_owned(),
            "User: hi".to_owned(),
        )
        .await;
        assert!(matches!(res, Err(AppError::Conflict(_))));
        assert!(state.resumed_chat.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn a_new_chat_ends_the_resumed_one() {
        let state = Arc::new(AppState {
            agent: AsyncMutex::new(Some(fake_pi_with_session("pi-3"))),
            resumed_chat: Mutex::new(Some(ResumedChat {
                pi_session_id: "pi-2".to_owned(),
                chat_id: "old".to_owned(),
                pending_history: Some("User: hi".to_owned()),
            })),
            ..AppState::for_tests()
        });

        let res = do_create_session(
            state.clone(),
            create_test_agent,
            MockProvider {
                tmp_path: tempdir().unwrap().path().to_path_buf(),
            },
            || None,
        )
        .await;
        assert!(res.is_ok());
        assert!(state.resumed_chat.lock().unwrap().is_none());
    }
}
