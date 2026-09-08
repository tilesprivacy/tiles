use std::sync::Arc;

use anyhow::Result;
use axum::{
    Json, Router,
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
        storage::db::{Dbconn, get_db_conn},
    },
    daemon::{ApiResponse, AppError, AppState},
};

#[derive(Serialize, Deserialize)]
pub struct AtLoginReq {
    user_handle: String,
}
// atproto apis needed
// login, logout
pub fn atproto_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/atproto/login", post(login))
        .route("/v1/tilekit/atproto/logout", post(logout))
        .route("/v1/tilekit/atproto/status", get(status))
    // .route(
    //     "/v1/tilekit/atproto/share-session/{session-id}",
    //     get(share_session),
    // )
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
