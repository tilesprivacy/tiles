use std::{sync::Arc, time::Duration};

use anyhow::Result;
use axum::{
    Json, Router,
    extract::Path,
    http::header::CONTENT_TYPE,
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
struct PublicProfile {
    avatar: Option<String>,
}

/// Keep the status response small enough to render safely inside the WebView.
const AVATAR_MAX_BYTES: u64 = 2 * 1024 * 1024;

/// Formats supported by WebKit and the menubar avatar implementation.
const AVATAR_TYPES: [&str; 4] = ["image/jpeg", "image/png", "image/webp", "image/gif"];

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

fn encode_avatar(mime: &str, bytes: &[u8]) -> Option<String> {
    if !AVATAR_TYPES.contains(&mime) || bytes.len() as u64 > AVATAR_MAX_BYTES {
        return None;
    }

    Some(format!(
        "data:{mime};base64,{}",
        data_encoding::BASE64.encode(bytes)
    ))
}

/// Tauri's WebView intentionally blocks remote image URLs. Download the public
/// avatar in the daemon and return the same bounded data URI used by the
/// menubar instead of widening the app's content security policy.
async fn fetch_avatar(client: &reqwest::Client, url: &str) -> Option<String> {
    if !url.starts_with("https://") {
        return None;
    }

    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|len| len > AVATAR_MAX_BYTES)
    {
        return None;
    }

    let mime = response
        .headers()
        .get(CONTENT_TYPE)?
        .to_str()
        .ok()?
        .split(';')
        .next()?
        .trim()
        .to_ascii_lowercase();
    let bytes = response.bytes().await.ok()?;

    encode_avatar(&mime, &bytes)
}

pub async fn status() -> Result<impl IntoResponse, AppError> {
    let atproto_user = {
        let user_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
            .map_err(|e| AppError::CannotProcess(e.to_string()))?;

        atproto::fetch_logged_in_data(&user_db_conn)
            .map_err(|e| AppError::CannotProcess(e.to_string()))?
    };

    if let Some(atproto_user) = atproto_user {
        // Profile metadata is public and optional. An offline AppView must not
        // make a valid local login look disconnected.
        let avatar = match reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
        {
            Ok(client) => {
                let avatar_url = match client
                    .get("https://public.api.bsky.app/xrpc/app.bsky.actor.getProfile")
                    .query(&[("actor", atproto_user.key.as_str())])
                    .send()
                    .await
                {
                    Ok(response) if response.status().is_success() => response
                        .json::<PublicProfile>()
                        .await
                        .ok()
                        .and_then(|profile| profile.avatar),
                    _ => None,
                };

                match avatar_url {
                    Some(url) => fetch_avatar(&client, &url).await,
                    None => None,
                }
            }
            Err(_) => None,
        };

        Ok(ApiResponse::success(json!({
            "handle": atproto_user.handle,
            "did": atproto_user.key,
            "avatar": avatar
        })))
    } else {
        Err(AppError::NotFound("Not logged-in".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::encode_avatar;

    #[test]
    fn avatar_is_encoded_as_a_webview_safe_data_uri() {
        assert_eq!(
            encode_avatar("image/png", &[0, 1, 2]),
            Some("data:image/png;base64,AAEC".to_owned())
        );
    }

    #[test]
    fn non_image_avatar_is_rejected() {
        assert_eq!(encode_avatar("text/html", b"not an image"), None);
    }
}
