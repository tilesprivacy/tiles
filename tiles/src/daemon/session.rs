//! High level apis for sessions

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tilekit::modelfile::Role;

use crate::{
    core::{
        account::local::get_user,
        agent::pi::{self, PiAgent},
        chats::{self as sessionChats, Chats},
        storage::db::get_db_conn,
    },
    repl::ChatResponse,
};

use crate::utils::config::{ConfigProvider, DefaultProvider};
use crate::{
    daemon::{ApiResponse, AppError, AppState, agent::get_agent_start_params},
    utils::config::PY_PORT,
};

#[derive(Serialize)]
struct Session {
    id: String,
}

#[derive(Deserialize)]
struct SaveChatRequest {
    text: String,
    session_id: String,
    role: Role,
    parent_chat_id: Option<String>,
    user_id: String,
    model_used: String,
}

// factory type for Pi::new() fn, so that we can write some test
type CreateNewAgentFn = fn(&str, &str, u32) -> anyhow::Result<PiAgent>;

pub fn session_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/session/new", post(create_session))
        .route("/v1/tilekit/session/list", get(fetch_sessions))
        .route("/v1/tilekit/session/chat", post(save_chat))
        .route(
            "/v1/tilekit/session/{session_id}/chats",
            get(fetch_chats_by_session),
        )
}

// Creates a new session or starts the agent
async fn create_session(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    do_create_session(state, pi::new, DefaultProvider).await
}

async fn do_create_session(
    state: Arc<AppState>,
    create_agent_fn: CreateNewAgentFn,
    provider: impl ConfigProvider,
) -> Result<impl IntoResponse, AppError> {
    let mut agent = state.agent.lock().await;
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

        let session_data = Session {
            id: agent_state.session_id,
        };
        Ok(ApiResponse::success(session_data))
    } else {
        log::info!("Pi agent not started, so starting..");
        let (modelname, system_prompt) = get_agent_start_params(provider)?;
        let pi_agent = create_agent_fn(&modelname, &system_prompt, PY_PORT)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;
        *agent = Some(pi_agent);
        let agent = agent.as_mut().ok_or(AppError::InternalServerError(
            "No agent instance available, start one first".to_string(),
        ))?;

        let state = agent
            .reader
            .get_pi_state(&mut agent.writer)
            .await
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;
        Ok(ApiResponse::success(Session {
            id: state.session_id,
        }))
    }
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
        .map_err(|e| AppError::NotFound(e.to_string()))?;

    Ok(ApiResponse::success(sessions))
}

async fn save_chat(
    State(state): State<Arc<AppState>>,
    Json(save_request): Json<SaveChatRequest>,
) -> Result<impl IntoResponse, AppError> {
    let chat_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::CHAT)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;
    let user_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;
    let result = do_save_chat(state, save_request, &chat_db_conn, &user_db_conn)?;

    Ok(ApiResponse::success(result))
}

fn do_save_chat(
    _state: Arc<AppState>,
    save_request: SaveChatRequest,
    chat_db_conn: &Connection,
    user_db_conn: &Connection,
) -> Result<Chats, AppError> {
    match sessionChats::fetch_session(chat_db_conn, &save_request.session_id) {
        Err(err) if err.to_string().contains("Query returned no rows") => {
            log::info!("Session doesn't exist, create it");
            // A user prompt should create a session not an agent, as of now
            if save_request.role == Role::User {
                sessionChats::create_session(
                    chat_db_conn,
                    &save_request.session_id,
                    &save_request.text,
                    &save_request.user_id,
                )
                .map_err(|e| AppError::CannotProcess(e.to_string()))?;
            } else {
                return Err(AppError::NotFound("Session doesnt exist".to_owned()));
            }
        }
        Err(err) => {
            return Err(AppError::CannotProcess(err.to_string()));
        }
        _ => (),
    }

    let chat_response = ChatResponse {
        input: save_request.text,
        session_id: save_request.session_id,
        role: save_request.role,
        parent_chat_id: save_request.parent_chat_id,
        metrics: None,
        model_used: save_request.model_used,
    };
    let current_user = get_user(user_db_conn, &save_request.user_id).map_err(|_e| {
        let err_msg = format!("User {} not found", &save_request.user_id);
        AppError::NotFound(err_msg)
    })?;
    let chat =
        sessionChats::save_chat(chat_db_conn, &current_user, chat_response).map_err(|e| {
            let err_msg = format!("Saving chat failed due to {}", e);
            AppError::CannotProcess(err_msg)
        })?;

    Ok(chat)
}

async fn fetch_chats_by_session(
    Path(session_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let chat_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::CHAT)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    let delta = sessionChats::fetch_chats_by_session_id(&chat_db_conn, &session_id)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;

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
                r#"read request
  printf '{"type": "response", "command": "new_session", "success": true, "data": {"cancelled": false}}\n'
    read request
 printf '{"type": "response", "command": "get_state", "success": true, "data": {"sessionId": "session_id", "model": {"id": "id", "name": "model"}, "thinkingLevel": "high", "isStreaming": true}}\n'
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
                r#"read request
  printf '{"type": "response", "command": "new_session", "success": true, "data": {"cancelled": false}}\n'
    read request
 printf '{"type": "response", "command": "get_state", "success": true, "data": {"sessionId": "session_id", "model": {"id": "id", "name": "model"}, "thinkingLevel": "high", "isStreaming": true}}\n'
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
        let res = do_create_session(Arc::new(state), create_test_agent, mock_provider).await;

        assert!(res.is_ok());
        let response = res.into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body.contains("session_id"))
    }

    fn create_test_agent(_: &str, _: &str, _: u32) -> Result<PiAgent, anyhow::Error> {
        from_test_command(
            "sh",
            &[
                "-c",
                r#"
                    read request
 printf '{"type": "response", "command": "get_state", "success": true, "data": {"sessionId": "session_id", "model": {"id": "id", "name": "model"}, "thinkingLevel": "high", "isStreaming": true}}\n'
                "#,
            ],
        )
    }

    #[tokio::test]
    async fn test_fetch_sessions_api() {
        let db_conn = setup_db_conn_v2();

        let user = create_user();
        if let Err(err) = sessionChats::fetch_session(&db_conn.chat, "ad")
            && err.to_string().contains("Query returned no rows")
        {
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
            ..AppState::for_tests()
        };

        // Test saving chat with new session

        let save_req = SaveChatRequest {
            text: "capital of India".to_string(),
            session_id: "id_non".to_owned(),
            role: Role::User,
            parent_chat_id: None,
            user_id: user.user_id.clone(),
            model_used: "ada".to_owned(),
        };

        let _res = do_save_chat(state.into(), save_req, &db_conn.chat, &db_conn.common).unwrap();

        assert!(sessionChats::fetch_session(&db_conn.chat, "id_non").is_ok());

        let chats = sessionChats::fetch_chats_by_session_id(&db_conn.chat, "id_non").unwrap();

        assert_eq!(chats.chats.len(), 1);
        assert_eq!(chats.sessions.len(), 1);
    }

    #[test]
    fn test_save_chat_api_as_non_user_role_session_doesnt_exist() {
        let db_conn = setup_db_conn_v2();

        let user = create_dummy_user(&db_conn.common, Some("did:key:xyz".to_owned()));

        let state = AppState {
            agent: AsyncMutex::new(None),
            ..AppState::for_tests()
        };

        // Test saving chat with new session

        let save_req = SaveChatRequest {
            text: "capital of India".to_string(),
            session_id: "id_non".to_owned(),
            role: Role::Assistant,
            parent_chat_id: None,
            user_id: user.user_id.clone(),
            model_used: "ada".to_owned(),
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
            ..AppState::for_tests()
        };

        let save_req = SaveChatRequest {
            text: "capital of India".to_string(),
            session_id: "id-1".to_owned(),
            role: Role::User,
            parent_chat_id: None,
            user_id: user.user_id.clone(),
            model_used: "ada".to_owned(),
        };

        let _res = do_save_chat(state.into(), save_req, &db_conn.chat, &db_conn.common).unwrap();

        assert!(sessionChats::fetch_session(&db_conn.chat, "id-1").is_ok());

        let chats = sessionChats::fetch_chats_by_session_id(&db_conn.chat, "id-1").unwrap();

        assert_eq!(chats.chats.len(), 1);
        assert_eq!(chats.sessions.len(), 1);
    }
}
