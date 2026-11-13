//! APIs to deal with authorization tokens (UCAN)

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{Router, response::IntoResponse, routing::post};
use axum_macros::debug_handler;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    core::{
        account::local::{
            self, Capability, SameDidDelegation, Token, TokenQueryOptions, TokenType,
            UserQueryOptions, fetch_tokens, fetch_users, get_current_user, update_user,
        },
        storage::db::get_db_conn,
    },
    daemon::{
        ApiJson, ApiResponse, AppError, AppState, SimpleResponse, daemon_client, daemon_url,
        net::DeviceLinkParams,
    },
};

/// Request format for `/create-token`
/// - aud_did: receiver DID,
/// - capability: Define the capability the token should be authorized for.
/// - aud_nickname (Optional): Nickname, which can be used to identify the peer who you want to delegate the token, easily
#[derive(Deserialize, Serialize)]
pub struct TokenCreateRequest {
    aud_did: String,
    capability: Capability,
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

#[derive(Deserialize, Serialize)]
pub struct RevokeParams {
    peer_did: String,
}

#[derive(Deserialize, Serialize)]
pub struct FetchTokenParams {
    cid: String,
}

#[derive(Deserialize, Serialize)]
pub struct UpdatePeerParams {
    pub user_did: String,
    pub username: Option<String>,
}

pub fn authz_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/authz/create-token", post(create_token))
        .route("/v1/tilekit/authz/add-token", post(add_token))
        .route("/v1/tilekit/authz/token-info", post(fetch_token_info))
        .route(
            "/v1/tilekit/peers/list-linked-devices",
            post(fetch_linked_devices),
        )
        .route("/v1/tilekit/peers/revoke", post(revoke_peer))
        .route("/v1/tilekit/peers/update-peer", post(update_peer))
}

#[debug_handler]
pub async fn create_token(
    ApiJson(create_token_req): ApiJson<TokenCreateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let token = crate::core::account::local::create_token(
        &create_token_req.aud_did,
        create_token_req.aud_nickname.as_deref(),
        create_token_req.capability,
    )
    .await
    .map_err(|e| {
        if e.downcast_ref::<SameDidDelegation>().is_some() {
            AppError::BadRequest("Audience DID cannot be same as user DID".to_string())
        } else {
            AppError::CannotProcess(e.to_string())
        }
    })?;

    Ok(ApiResponse::success(token))
}

#[debug_handler]
pub async fn add_token(
    ApiJson(add_token_req): ApiJson<TokenAddRequest>,
) -> Result<impl IntoResponse, AppError> {
    let common_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    let token = crate::core::account::local::add_token(
        &add_token_req.delegated_token,
        &common_db_conn,
        add_token_req.issuer_nickname.as_deref(),
    )
    .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    Ok(ApiResponse::success(token))
}

#[debug_handler]
pub async fn fetch_token_info(
    ApiJson(params): ApiJson<FetchTokenParams>,
) -> Result<impl IntoResponse, AppError> {
    let common_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    let token = crate::core::account::local::fetch_token_by_cid(&params.cid, &common_db_conn)
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

    // fetch the auth tokens delegated to this device and which are linked.
    let tokens = fetch_tokens(
        &common_db_conn,
        TokenQueryOptions {
            aud_did: Some(user.user_id.clone()),
            token_type: Some(TokenType::Linked),
            ..Default::default()
        },
    )
    .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    if tokens.is_empty() {
        return Ok(ApiResponse::success(vec![]));
    }

    let peer_ids = tokens.into_iter().map(|t| t.did).collect::<Vec<String>>();

    let peers = fetch_users(
        &common_db_conn,
        UserQueryOptions {
            user_ids: Some(peer_ids),
            ..Default::default()
        },
    )
    .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    Ok(ApiResponse::success(peers))
}

#[debug_handler]
pub async fn revoke_peer(
    ApiJson(revoke_params): ApiJson<RevokeParams>,
) -> Result<impl IntoResponse, AppError> {
    let common_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let user = get_current_user(&common_db_conn).map_err(|e| AppError::NotFound(e.to_string()))?;
    if user.user_id == revoke_params.peer_did {
        return Err(AppError::BadRequest(
            "Peer to be revoked can be same as this device".to_string(),
        ));
    }
    local::revoke_peer(&revoke_params.peer_did, &user, &common_db_conn)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    Ok(ApiResponse::success(SimpleResponse {
        message: "Successfully revoked".to_string(),
    }))
}

#[debug_handler]
pub async fn update_peer(
    ApiJson(update_params): ApiJson<UpdatePeerParams>,
) -> Result<impl IntoResponse, AppError> {
    let common_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let user = update_peer_in_db(&common_db_conn, update_params)?;

    Ok(ApiResponse::success(user))
}

fn update_peer_in_db(
    common_db_conn: &tokio_rusqlite::rusqlite::Connection,
    update_params: UpdatePeerParams,
) -> Result<local::User, AppError> {
    let users = fetch_users(
        common_db_conn,
        UserQueryOptions {
            user_id: Some(update_params.user_did.clone()),
            ..Default::default()
        },
    )
    .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    if users.is_empty() {
        return Err(AppError::NotFound("Peer not found".to_string()));
    };

    update_user(common_db_conn, update_params).map_err(|e| AppError::CannotProcess(e.to_string()))
}
/// CLI command handler for create-link
pub async fn create_link_cli(aud_did: Option<String>) -> Result<()> {
    if let Some(audience_did) = aud_did {
        let req_body = TokenCreateRequest {
            aud_did: audience_did.clone(),
            capability: Capability::All,
            aud_nickname: None,
        };
        let res = daemon_client()
            .await?
            .post(daemon_url("/v1/tilekit/authz/create-token"))
            .body(serde_json::to_value(req_body)?.to_string())
            .header("content-type", "application/json")
            .send()
            .await
            .context("Could not reach the Tiles daemon")?;

        if res.status().is_success() {
            let resp_struct = res.json::<ApiResponse<Token>>().await?;
            println!(
                "Use the below token to link your other device `{}`\n using `tiles link join <token> <nickname>` on that device\n{}",
                &audience_did, resp_struct.data.token
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
    let res = daemon_client()
        .await?
        .post(daemon_url("/v1/tilekit/authz/add-token"))
        .body(
            json!(TokenAddRequest {
                delegated_token: token.to_string(),
                issuer_nickname: nickname
            })
            .to_string(),
        )
        .header("content-type", "application/json")
        .send()
        .await
        .context("Could not reach the Tiles daemon")?;
    if res.status().is_success() {
        process_device_linking(&token).await?
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
    let res = daemon_client()
        .await?
        .post(daemon_url("/v1/tilekit/peers/list-linked-devices"))
        .header("content-type", "application/json")
        .send()
        .await
        .context("Could not reach the Tiles daemon")?;
    if res.status().is_success() {
        let resp_struct = res.json::<ApiResponse<Vec<local::User>>>().await?;
        println!();
        resp_struct.data.iter().for_each(|t| {
            println!(
                "Nickname: {}\nDID: {}\n",
                t.username.clone().unwrap_or("".to_string()),
                t.user_id,
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

pub async fn revoke_peers_cli(peer_did: &str) -> Result<()> {
    let req_body = RevokeParams {
        peer_did: peer_did.to_string(),
    };

    let res = daemon_client()
        .await?
        .post(daemon_url("/v1/tilekit/peers/revoke"))
        .body(serde_json::to_value(req_body)?.to_string())
        .header("content-type", "application/json")
        .send()
        .await
        .context("Could not reach the Tiles daemon")?;
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
async fn process_device_linking(token: &str) -> Result<()> {
    let res = daemon_client()
        .await?
        .post(daemon_url("/v1/tilekit/net/link-device"))
        .body(
            json!(DeviceLinkParams {
                token: token.to_string(),
            })
            .to_string(),
        )
        .header("content-type", "application/json")
        .send()
        .await
        .context("Could not reach the Tiles daemon")?;

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

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Bytes,
        extract::State,
        http::{StatusCode, Uri},
        response::{IntoResponse, Response},
        routing::get,
    };
    use serde_json::{Value, json};
    use serial_test::serial;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio::sync::OnceCell;

    use super::*;
    use crate::core::account::local::{save_peer_account_db, tests::setup_db_conn_v2};

    type Requests = Arc<Mutex<Vec<(String, Value)>>>;
    static TEST_DAEMON: OnceCell<Requests> = OnceCell::const_new();

    fn token() -> Value {
        json!({
            "id": "token-id",
            "did": "did:key:issuer",
            "token": "delegated-token",
            "cid": "bafy-token",
            "type": "Linked",
            "created_at": 1,
            "updated_at": 1,
            "aud_did": "did:key:peer",
            "nickname": "peer"
        })
    }

    fn user() -> Value {
        json!({
            "id": "token-id",
            "user_id": "did:key:issuer",
            "created_at": 1,
            "updated_at": 1,
            "username": "peer",
            "active_profile": false,
            "account_type": "PEER",
            "root": false
        })
    }
    async fn daemon_root() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    async fn record_request(State(requests): State<Requests>, uri: Uri, body: Bytes) -> Response {
        let body = if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&body).unwrap()
        };
        requests.lock().await.push((uri.path().to_owned(), body));

        let response = match uri.path() {
            "/v1/tilekit/authz/create-token" => {
                json!({ "status": "success", "data": token() })
            }
            "/v1/tilekit/authz/add-token" => {
                json!({ "status": "success", "data": token() })
            }
            "/v1/tilekit/net/link-device" => json!({
                "status": "success",
                "data": { "message": "Device linked" }
            }),
            "/v1/tilekit/peers/list-linked-devices" => {
                json!({ "status": "success", "data": [user()] })
            }
            "/v1/tilekit/peers/revoke" => json!({
                "status": "success",
                "data": { "message": "Successfully revoked" }
            }),
            _ => return StatusCode::NOT_FOUND.into_response(),
        };

        axum::Json(response).into_response()
    }

    async fn start_daemon() -> Requests {
        TEST_DAEMON
            .get_or_init(|| async {
                let requests = Requests::default();
                let app = Router::new()
                    .route("/", get(daemon_root))
                    .fallback(record_request)
                    .with_state(requests.clone());
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                let port = listener.local_addr().unwrap().port();
                unsafe {
                    std::env::set_var("TILES_DAEMON_DEV_PORT", port.to_string());
                }
                tokio::spawn(async move {
                    axum::serve(listener, app).await.unwrap();
                });
                requests
            })
            .await
            .clone()
    }

    #[tokio::test]
    #[serial]
    async fn cli_commands_send_expected_authz_requests() {
        let requests = start_daemon().await;
        requests.lock().await.clear();

        create_link_cli(Some("did:key:peer".to_owned()))
            .await
            .unwrap();

        let requests = requests.lock().await;
        assert_eq!(requests[0].0, "/v1/tilekit/authz/create-token");
        assert_eq!(requests[0].1["aud_did"], "did:key:peer");
        assert_eq!(requests[0].1["capability"], json!({ "type": "all" }));
        assert_eq!(requests[0].1["aud_nickname"], Value::Null);
        drop(requests);

        let requests = start_daemon().await;
        requests.lock().await.clear();
        join_request_cli("delegated-token".to_owned(), Some("laptop".to_owned()))
            .await
            .unwrap();

        let requests = requests.lock().await;
        assert_eq!(requests[0].0, "/v1/tilekit/authz/add-token");
        assert_eq!(requests[0].1["delegated_token"], "delegated-token");
        assert_eq!(requests[0].1["issuer_nickname"], "laptop");
        assert_eq!(requests[1].0, "/v1/tilekit/net/link-device");
        assert_eq!(requests[1].1["token"], "delegated-token");
        drop(requests);

        let requests = start_daemon().await;
        requests.lock().await.clear();
        list_linked_devices_cli().await.unwrap();
        let requests = requests.lock().await;
        assert_eq!(
            requests.as_slice(),
            &[(
                "/v1/tilekit/peers/list-linked-devices".to_owned(),
                Value::Null,
            )]
        );
        drop(requests);

        let requests = start_daemon().await;
        requests.lock().await.clear();
        revoke_peers_cli("did:key:peer").await.unwrap();
        let requests = requests.lock().await;
        assert_eq!(requests[0].0, "/v1/tilekit/peers/revoke");
        assert_eq!(requests[0].1["peer_did"], "did:key:peer");

        create_link_cli(None).await.unwrap();
    }

    #[test]
    fn update_peer_updates_existing_peer_username() {
        let db = setup_db_conn_v2();
        let peer_did = "did:key:peer";
        save_peer_account_db(&db.common, peer_did, "old-name").unwrap();

        let user = update_peer_in_db(
            &db.common,
            UpdatePeerParams {
                user_did: peer_did.to_owned(),
                username: Some("new-name".to_owned()),
            },
        )
        .unwrap();

        assert_eq!(user.user_id, peer_did);
        assert_eq!(user.username.as_deref(), Some("new-name"));
    }
}
