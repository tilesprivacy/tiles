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
        account::local::{Capabilities, fetch_linked_tokens, get_current_user},
        storage::db::get_db_conn,
    },
    daemon::{ApiResponse, AppError, AppState},
};

/// Request format for `/create-token`
/// - aud_did: receiver DID,
/// - capabilities: Define the capabilities the token should be eligible for in a list
/// - aud_nickname (Optional): Nickname, which can be used to identify the peer who you want to delegate the token, easily
#[derive(Deserialize)]
pub struct TokenCreateRequest {
    aud_did: String,
    capabilities: Vec<Capabilities>,
    aud_nickname: Option<String>,
}

/// Request format for `/add-token`
/// - delegated_token: The delegated token string,
/// - issuer_nickname(Optional): Nickname, which can be used to identify the peer who delegated the token, easily
#[derive(Deserialize)]
pub struct TokenAddRequest {
    delegated_token: String,
    issuer_nickname: Option<String>,
}

pub fn authz_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/authz/create-token", post(create_token))
        .route("/v1/tilekit/authz/add-token", post(add_token))
        .route("/v1/tilekit/authz/token-info/{cid}", get(fetch_token_info))
        .route(
            "/v1/tilekit/peers/list-linked-devices",
            get(fetch_linked_devices),
        )
        .route("/v1/tilekit/peers/revoke/{cid}", post(revoke_token))
}

#[debug_handler]
pub async fn create_token(
    Json(create_token_req): Json<TokenCreateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let aud_nickname = create_token_req.aud_nickname.unwrap_or(String::from(""));

    let token = crate::core::account::local::create_token(
        &create_token_req.aud_did,
        Some(&aud_nickname),
        create_token_req.capabilities,
    )
    .await
    .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    Ok(ApiResponse::success(json!({
        "token": token.token.clone(),
        "cid": token.cid,
        "did": token.did,
        "aud": token.aud_did,
        "nickname": token.nickname,
        "type": token.r#type
    })))
}

#[debug_handler]
pub async fn add_token(
    Json(add_token_req): Json<TokenAddRequest>,
) -> Result<impl IntoResponse, AppError> {
    let issuer_nickname = add_token_req.issuer_nickname.unwrap_or(String::from(""));
    let common_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    let token = crate::core::account::local::add_token(
        &add_token_req.delegated_token,
        &common_db_conn,
        Some(&issuer_nickname),
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

#[debug_handler]
pub async fn fetch_linked_devices() -> Result<impl IntoResponse, AppError> {
    let common_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let user = get_current_user(&common_db_conn).map_err(|e| AppError::NotFound(e.to_string()))?;
    let tokens = fetch_linked_tokens(&common_db_conn, &user.user_id)
        .map_err(|e| AppError::NotFound(e.to_string()))?;

    Ok(ApiResponse::success(tokens))
}

// Later we will see into UCAN revocation
#[debug_handler]
pub async fn revoke_token(Path(cid): Path<String>) -> Result<impl IntoResponse, AppError> {
    let common_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    crate::core::account::local::delete_token_by_cid(&common_db_conn, &cid)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    Ok(ApiResponse::success(json!("Successfully revoked")))
}
