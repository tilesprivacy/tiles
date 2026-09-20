//! APIs to deal with authorization tokens (UCAN)

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::Path,
    response::IntoResponse,
    routing::{get, post},
};
use axum_macros::debug_handler;
use serde::Deserialize;
use serde_json::json;

use crate::{
    core::{
        account::local::{TokenType, fetch_token_by_ucan},
        storage::db::get_db_conn,
    },
    daemon::{ApiResponse, AppError, AppState},
};

#[derive(Deserialize)]
pub struct TokenCreateRequest {
    aud_did: String,
    token_type: TokenType,
    nickname: Option<String>,
}

#[derive(Deserialize)]
pub struct TokenAddRequest {
    delegated_token: String,
    token_type: TokenType,
    nickname: Option<String>,
}

pub fn authz_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/authz/create-token", post(create_token))
        .route("/v1/tilekit/authz/add-token", post(add_token))
        .route("/v1/tilekit/authz/token-info/{cid}", get(fetch_token_info))
}

#[debug_handler]
pub async fn create_token(
    Json(create_token_req): Json<TokenCreateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let aud_nickname = create_token_req.nickname.unwrap_or(String::from(""));

    let token_str = crate::core::account::local::create_token(
        &create_token_req.aud_did,
        Some(&aud_nickname),
        create_token_req.token_type,
    )
    .await
    .map_err(|e| AppError::CannotProcess(e.to_string()))?;
    let common_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    let token_struct = fetch_token_by_ucan(&token_str, &common_db_conn)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    if let Some(token) = token_struct {
        Ok(ApiResponse::success(json!({
            "token": token_str,
            "cid": token.cid,
            "did": token.did,
            "aud": token.aud_did,
            "nickname": token.aud_nickname,
            "type": token.r#type
        })))
    } else {
        Err(AppError::NotFound("Token not found".to_owned()))
    }
}

#[debug_handler]
pub async fn add_token(
    Json(add_token_req): Json<TokenAddRequest>,
) -> Result<impl IntoResponse, AppError> {
    let aud_nickname = add_token_req.nickname.unwrap_or(String::from(""));
    let common_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    let token = crate::core::account::local::add_token(
        &add_token_req.delegated_token,
        &common_db_conn,
        Some(&aud_nickname),
        add_token_req.token_type,
    )
    .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    Ok(ApiResponse::success(token))
}

#[debug_handler]
pub async fn fetch_token_info(Path(cid): Path<String>) -> Result<impl IntoResponse, AppError> {
    let common_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    let token = crate::core::account::local::fetch_token_by_cid(&cid, &common_db_conn)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    if let Some(token_struct) = token {
        Ok(ApiResponse::success(token_struct))
    } else {
        Err(AppError::NotFound("Token not found".to_owned()))
    }
}
