//! APIs to deal with authorization tokens (UCAN)

use std::sync::Arc;

use anyhow::Result;
use axum::{
    Router,
    extract::Path,
    response::IntoResponse,
    routing::{get, post},
};
use axum_macros::debug_handler;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    core::{
        account::local::{Capabilities, Token, fetch_linked_tokens, get_current_user},
        storage::db::get_db_conn,
    },
    daemon::{ApiJson, ApiResponse, AppError, AppState, SimpleResponse, get_or_set_daemon_port},
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
#[derive(Deserialize, Serialize)]
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
    ApiJson(create_token_req): ApiJson<TokenCreateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let aud_nickname = create_token_req.aud_nickname.unwrap_or(String::from(""));

    let token = crate::core::account::local::create_token(
        &create_token_req.aud_did,
        Some(&aud_nickname),
        create_token_req.capabilities,
    )
    .await
    .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    Ok(ApiResponse::success(token))
}

#[debug_handler]
pub async fn add_token(
    ApiJson(add_token_req): ApiJson<TokenAddRequest>,
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

    Ok(ApiResponse::success(SimpleResponse {
        message: "Successfully revoked".to_string(),
    }))
}

/// CLI command handler for create-link
pub async fn create_link_cli(aud_did: Option<String>) -> Result<()> {
    if let Some(audience_did) = aud_did {
        let client = Client::new();
        let port = *get_or_set_daemon_port();
        let addr = format!("http://127.0.0.1:{}/v1/tilekit/authz/create-token", port);
        let res = client
            .post(addr)
            .header("content-type", "application/json")
            .body(
                json!({
                    "aud_did": audience_did,
                    "capabilities": [{"type": "all"}]
                })
                .to_string(),
            )
            .send()
            .await?;

        if res.status().is_success() {
            let resp_struct = res.json::<ApiResponse<Token>>().await?;
            println!(
                "Use the below token to link your other device `{}`\n using `tiles link join <token> <nickname>` on that device\n{}",
                audience_did, resp_struct.data.token
            );
        } else {
            eprintln!(
                "Creating link token failed due to {}",
                res.text()
                    .await
                    .unwrap_or_else(|_| "Couldn't read the body".to_string())
            );
        }
    } else {
        // TODO: will implement a syncronized linking over offline network
        eprintln!("Expected peer DID")
    }
    Ok(())
}

/// CLI command handler for joining request to a to-be linked device
pub async fn join_request_cli(token: String, nickname: Option<String>) -> Result<()> {
    let client = Client::new();
    let port = *get_or_set_daemon_port();
    let addr = format!("http://127.0.0.1:{}/v1/tilekit/authz/add-token", port);
    let res = client
        .post(addr)
        .header("content-type", "application/json")
        .body(
            json!(TokenAddRequest {
                delegated_token: token.to_string(),
                issuer_nickname: nickname
            })
            .to_string(),
        )
        .send()
        .await?;

    if res.status().is_success() {
        process_device_linking(&client, port, &token).await?
    } else {
        eprintln!(
            "Failed to process the command due to {}",
            res.text()
                .await
                .unwrap_or_else(|_| "Couldn't read the body".to_string())
        )
    }

    Ok(())
}

pub async fn list_linked_devices_cli() -> Result<()> {
    let client = Client::new();
    let port = *get_or_set_daemon_port();
    let addr = format!(
        "http://127.0.0.1:{}/v1/tilekit/peers/list-linked-devices",
        port
    );
    let res = client
        .get(addr)
        .header("content-type", "application/json")
        .send()
        .await?;

    if res.status().is_success() {
        let resp_struct = res.json::<ApiResponse<Vec<Token>>>().await?;
        println!();
        resp_struct.data.iter().for_each(|t| {
            println!(
                "Nickname: {}\nDID: {}\nCID: {}\n",
                t.nickname.clone().unwrap_or("".to_string()),
                t.did,
                t.cid
            );

            println!();
        });
    } else {
        eprintln!(
            "Failed to list-linked-devices failed due to {}",
            res.text()
                .await
                .unwrap_or_else(|_| "Couldn't read the body".to_string())
        );
    }
    Ok(())
}

pub async fn revoke_peers_cli(cid: &str) -> Result<()> {
    let client = Client::new();
    let port = *get_or_set_daemon_port();
    let addr = format!("http://127.0.0.1:{}/v1/tilekit/peers/revoke/{}", port, cid);
    let res = client
        .post(addr)
        .header("content-type", "application/json")
        .send()
        .await?;

    if res.status().is_success() {
        let resp_struct = res.json::<ApiResponse<SimpleResponse>>().await?;
        println!("{}", resp_struct.data.message)
    } else {
        eprintln!(
            "Failed to revoke peer due to {}",
            res.text()
                .await
                .unwrap_or_else(|_| "Couldn't read the body".to_string())
        );
    }
    Ok(())
}
async fn process_device_linking(client: &Client, port: u32, token: &str) -> Result<()> {
    let addr = format!(
        "http://127.0.0.1:{}/v1/tilekit/net/link-device?token={}",
        port, &token
    );
    let res = client
        .get(addr)
        .header("content-type", "application/json")
        .send()
        .await?;

    if res.status().is_success() {
        let resp_struct = res.json::<ApiResponse<SimpleResponse>>().await?;
        println!("{}", resp_struct.data.message);
    } else {
        eprintln!(
            "Failed to process device linking due to {}",
            res.text()
                .await
                .unwrap_or_else(|_| "Couldn't read the body".to_string())
        );
    }
    Ok(())
}
