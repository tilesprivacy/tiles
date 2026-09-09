use std::sync::Arc;

use anyhow::Result;
use axum::{
    Json, Router,
    extract::Path,
    response::IntoResponse,
    routing::{get, post},
};
use axum_macros::debug_handler;
use futures_util::TryFutureExt;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    core::{
        account::atproto,
        chats::{fetch_chats_by_session_id, snapshot_from_chats},
        storage::db::{Dbconn, get_db_conn},
    },
    daemon::{ApiResponse, AppError, AppState},
    utils::lexicons::SessionSnapshotRecord,
};

#[derive(Serialize, Deserialize)]
pub struct AtLoginReq {
    user_handle: String,
}

#[derive(Deserialize)]
pub struct ShareSessionReq {
    /// A private share is encrypted before it goes to the PDS, and the key
    /// rides in the link's fragment rather than in the record
    #[serde(default)]
    is_private: bool,
}
// atproto apis needed
// login, logout
pub fn atproto_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/atproto/login", post(login))
        .route("/v1/tilekit/atproto/logout", post(logout))
        .route("/v1/tilekit/atproto/status", get(status))
        .route(
            "/v1/tilekit/atproto/share-session/{session_id}",
            post(share_session),
        )
}

/// Publishes a session to the user's PDS and hands back the link. Posting
/// rather than getting, because it writes a record every time it is called.
#[debug_handler]
pub async fn share_session(
    Path(session_id): Path<String>,
    Json(request): Json<ShareSessionReq>,
) -> Result<impl IntoResponse, AppError> {
    let chat_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::CHAT)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;
    let delta_chats = fetch_chats_by_session_id(&chat_db_conn, &session_id)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    let session = delta_chats
        .sessions
        .first()
        .ok_or_else(|| AppError::NotFound(format!("No session {session_id}")))?;

    // the REPL stores a snapshot as it goes and the daemon does not, so a
    // session started from the app arrives here with nothing stored and has one
    // built from its rows instead
    let shared_session: SessionSnapshotRecord = match session.snapshot.as_ref() {
        Some(snapshot) => {
            serde_json::from_str(snapshot).map_err(|e| AppError::CannotProcess(e.to_string()))?
        }
        None => snapshot_from_chats(session, &delta_chats.chats),
    };

    let is_private = request.is_private;

    // rusqlite's Connection is not Sync, so a share that holds one across its
    // awaits cannot be a Send future, which is what axum wants. Keeping the
    // whole thing on one blocking thread with a runtime of its own sidesteps
    // that without threading a connection pool through the atproto code
    let url = tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        runtime.block_on(async {
            let conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)?;

            atproto::share_session(&conn, &shared_session, is_private).await
        })
    })
    .await
    .map_err(|e| AppError::InternalServerError(format!("Share task failed: {e}")))?
    .map_err(|e| {
        if e.to_string() == "NOT_LOGGED_IN" {
            AppError::CannotProcess(
                "Sharing needs an ATmosphere login, the session is stored on your PDS".to_owned(),
            )
        } else {
            AppError::InternalServerError(e.to_string())
        }
    })?;

    Ok(ApiResponse::success(json!({
        "url": url,
        "is_private": is_private
    })))
}

#[debug_handler]
pub async fn login(Json(login_request): Json<AtLoginReq>) -> Result<impl IntoResponse, AppError> {
    let resp = atproto::login(&login_request.user_handle)
        .map_err(|e| AppError::CannotProcess(e.to_string()))
        .await?;

    Ok(ApiResponse::success(json!(resp)))
}

pub async fn logout() -> Result<impl IntoResponse, AppError> {
    let chat_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::CHAT)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;
    let user_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    let conn = Dbconn {
        chat: chat_db_conn,
        common: user_db_conn,
    };
    let resp = atproto::logout(&conn).map_err(|e| AppError::CannotProcess(e.to_string()))?;

    Ok(ApiResponse::success(json!(resp)))
}

pub async fn status() -> Result<impl IntoResponse, AppError> {
    let user_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    if let Some(atproto_user) = atproto::fetch_logged_in_data(&user_db_conn)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?
    {
        Ok(ApiResponse::success(json!({
            "handle": atproto_user.handle,
            "did": atproto_user.key
        })))
    } else {
        Err(AppError::NotFound("Not logged-in".to_string()))
    }
}
