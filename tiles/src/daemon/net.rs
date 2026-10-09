//! Tilekit apis related to session sync

use std::sync::Arc;

use crate::core::account::local::get_current_user;
use crate::core::network::{self as core_network, create_endpoint};
use crate::core::storage::db::get_db_conn;
use crate::daemon::{ApiResponse, AppError, AppState, SimpleResponse, get_or_set_daemon_port};
use anyhow::Result;
// use axum::extract::Path;
use axum::{
    Router,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
};
use axum_macros::debug_handler;
use iroh::Endpoint;
use reqwest::Client;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct NetworkState {
    pub is_endpoint_running: bool,
    pub main_network_sender: tokio::sync::mpsc::Sender<NetworkApiEvent>,
    pub endpoint: Endpoint,
}

/// Events emitted as part of other tile's processes communicating with the main network
///
/// For ex: A tilekit api communicating with network to stop it
pub enum NetworkApiEvent {
    NetworkStarted,
    // command event to stop the network
    StopNetwork,
    // confirmation event after stopping a network
    NetworkStopped,
    SyncListenerStarted,
    DirectSyncDone,
}

#[derive(Serialize, Deserialize)]
pub struct NetworkStatusResponse {
    is_network_running: bool,
}

pub fn network_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/net/start-network", post(start_network))
        .route("/v1/tilekit/net/stop-network", post(stop_network))
        .route("/v1/tilekit/net/status", get(get_network_status))
    // TODO: We can use the same api to link device too.
    // .route(
    //     "/v1/tilekit/sync/sync-with-device/{did}",
    //     get(sync_with_device),
    // )
}

/// Starts a p2p iroh endpoint and creates a gossip network for the device over which all the message based communication btw peers happen
#[debug_handler]
async fn start_network(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let user_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::NotFound(e.to_string()))?;
    do_start_network(state, user_db_conn).await
}

async fn do_start_network(
    state: Arc<AppState>,
    db_conn: Connection,
) -> Result<impl IntoResponse, AppError> {
    let mut network_state = state.network_state.lock().await;
    if let Some(net_state) = network_state.as_mut()
        && net_state.is_endpoint_running
    {
        log::info!("network already running");
        return Ok(ApiResponse::success(SimpleResponse {
            message: "Tiles Network already running".to_string(),
        }));
    }

    let user = get_current_user(&db_conn).map_err(|e| AppError::NotFound(e.to_string()))?;
    let endpoint = create_endpoint(&user)
        .await
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    // sender/recv for cross process communication with the tiles-network
    let (main_network_sender, main_network_recv) = mpsc::channel::<NetworkApiEvent>(10);

    let (api_resp_sendx, mut api_resp_recvx) = mpsc::channel::<NetworkApiEvent>(10);

    let endpoint_clone = endpoint.clone();
    let main_network_sender_clone = main_network_sender.clone();

    tokio::spawn(async move {
        core_network::start_network(
            user,
            endpoint,
            main_network_sender_clone,
            main_network_recv,
            api_resp_sendx.clone(),
        )
        .await?;
        anyhow::Result::<()>::Ok(())
    });

    // waiting for the confrimation from the above process that network has
    // been started
    api_resp_recvx.recv().await;

    *network_state = Some(NetworkState {
        is_endpoint_running: true,
        main_network_sender: main_network_sender.clone(),
        endpoint: endpoint_clone.clone(),
    });

    Ok(ApiResponse::success(SimpleResponse {
        message: "Started Tiles Network successfully".to_string(),
    }))
}

/// Stops the tiles network that is running in background
#[debug_handler]
async fn stop_network(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let mut network_state = state.network_state.lock().await;

    if let Some(net_state) = network_state.as_mut()
        && net_state.is_endpoint_running
    {
        let _ = net_state
            .main_network_sender
            .send(NetworkApiEvent::StopNetwork)
            .await;
        net_state.is_endpoint_running = false;
        log::info!("Stopped the network ");
        return Ok(ApiResponse::success(json!({
            "message": "Stopped Tiles network successfully"
        })));
    }

    Ok(ApiResponse::success(json!({
        "message": "Tiles Network not running"
    })))
}

pub async fn get_network_status(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let mut network_state = state.network_state.lock().await;

    let is_running = if let Some(net_state) = network_state.as_mut() {
        net_state.is_endpoint_running
    } else {
        false
    };
    Ok(ApiResponse::success(NetworkStatusResponse {
        is_network_running: is_running,
    }))
}

// CLI command functions

pub async fn start_network_cli() {
    let client = Client::new();
    let port = *get_or_set_daemon_port();
    let addr = format!("http://127.0.0.1:{}/v1/tilekit/net/start-network", port);
    let res = client.post(addr).send().await;
    match res {
        Err(err) => println!("Starting Tiles network failed due to {:?}", err),
        Ok(response) => {
            if let Ok(resp_struct) = response.json::<ApiResponse<SimpleResponse>>().await {
                println!("{}", resp_struct.data.message)
            } else {
                eprintln!("Failed to process the result")
            }
        }
    }
}

pub async fn stop_network_cli() {
    let client = Client::new();
    let port = *get_or_set_daemon_port();
    let addr = format!("http://127.0.0.1:{}/v1/tilekit/net/stop-network", port);
    let res = client.post(addr).send().await;
    match res {
        Err(err) => println!("Stopping Tiles network failed due to {:?}", err),
        Ok(response) => {
            if let Ok(resp_struct) = response.json::<ApiResponse<SimpleResponse>>().await {
                println!("{}", resp_struct.data.message)
            } else {
                eprintln!("Failed to process the result")
            }
        }
    }
}

pub async fn fetch_network_status_cli() {
    let client = Client::new();
    let port = *get_or_set_daemon_port();
    let addr = format!("http://127.0.0.1:{}/v1/tilekit/net/status", port);
    let res = client.get(addr).send().await;
    match res {
        Err(err) => println!("Failed to get the network status due to {:?}", err),
        Ok(response) => {
            if let Ok(resp_struct) = response.json::<ApiResponse<NetworkStatusResponse>>().await {
                println!("Network Running: {}", resp_struct.data.is_network_running)
            } else {
                eprintln!("Failed to process the result")
            }
        }
    }
}
// pub async fn sync_with_device(
//     State(state): State<Arc<AppState>>,
//     Path(did): Path<String>,
// ) -> Result<impl IntoResponse, AppError> {
//     let mut network_state = state.network_state.lock().await;

//     // start the endpoint if not running
//     if let Some(sync) = network_state.as_mut()
//         && !sync.is_endpoint_running
//     {
//         let (_endpoint, _main_network_sender) = start_network(state.clone())
//             .await
//             .map_err(|e| AppError::CannotProcess(e.to_string()))?;
//     };

//     let (main_network_sender, sync_main_receiver) = mpsc::channel::<u8>(10);

//     let (api_resp_sendx, mut api_resp_recvx) = mpsc::channel::<NetworkApiResponse>(10);

//     if let Some(sync) = network_state.as_mut() {
//         let endpoint_clone = sync.endpoint.clone();
//         let sync_sender_clone = main_network_sender.clone();
//         tokio::spawn(async move {
//             core_network::sync(
//                 None,
//                 endpoint_clone,
//                 sync_sender_clone,
//                 sync_main_receiver,
//                 api_resp_sendx.clone(),
//             )
//             .await?;
//             anyhow::Result::<()>::Ok(())
//         });
//         api_resp_recvx.recv().await;

//         let resp_str = format!("Syncing completed with {}", did);
//         Ok(ApiResponse::success(json!(resp_str)))
//     } else {
//         Err(AppError::CannotProcess(
//             "Failed to get the network_state".to_owned(),
//         ))
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::account::local::{create_dummy_user, tests::setup_db_conn_v2};
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use reqwest::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_starting_network() {
        let db_conn = setup_db_conn_v2();
        let _user = create_dummy_user(&db_conn.common, Some("did:key:xyz".to_owned()));
        let shared_state = Arc::new(AppState::for_tests());
        let res = do_start_network(shared_state.clone(), db_conn.common)
            .await
            .unwrap();
        let res_into = res.into_response();
        assert_eq!(res_into.status(), StatusCode::OK);
        let body_bytes = res_into.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body.contains("Started Tiles Network successfully"));
        assert!(
            shared_state
                .network_state
                .lock()
                .await
                .as_mut()
                .unwrap()
                .is_endpoint_running
        )
    }

    #[tokio::test]
    async fn test_stopping_network() {
        let db_conn = setup_db_conn_v2();
        let _user = create_dummy_user(&db_conn.common, Some("did:key:xyz".to_owned()));
        let shared_state = Arc::new(AppState::for_tests());
        let res = do_start_network(shared_state.clone(), db_conn.common)
            .await
            .unwrap();
        let res_into = res.into_response();
        assert_eq!(res_into.status(), StatusCode::OK);
        let body_bytes = res_into.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body.contains("Started Tiles Network successfully"));
        assert!(
            shared_state
                .network_state
                .lock()
                .await
                .as_mut()
                .unwrap()
                .is_endpoint_running
        );

        let body = json!({}).to_string();
        let agent_app = network_router();
        let response = agent_app
            .with_state(shared_state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .header("content-type", "application/json")
                    .uri("/v1/tilekit/net/stop-network")
                    .body(Body::new(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let res_into = response.into_response();
        assert_eq!(res_into.status(), StatusCode::OK);
        let body_bytes = res_into.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body.contains("Stopped Tiles network successfully"));
        assert!(
            !shared_state
                .network_state
                .lock()
                .await
                .as_mut()
                .unwrap()
                .is_endpoint_running
        );
    }

    #[tokio::test]
    async fn test_stopping_network_but_network_not_up() {
        let shared_state = Arc::new(AppState::for_tests());

        let body = json!({}).to_string();
        let agent_app = network_router();
        let response = agent_app
            .with_state(shared_state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .header("content-type", "application/json")
                    .uri("/v1/tilekit/net/stop-network")
                    .body(Body::new(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let res_into = response.into_response();
        assert_eq!(res_into.status(), StatusCode::OK);
        let body_bytes = res_into.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body.contains("Network not running"));
    }

    #[tokio::test]
    async fn test_network_status_api() {
        let db_conn = setup_db_conn_v2();
        let _user = create_dummy_user(&db_conn.common, Some("did:key:xyz".to_owned()));
        let shared_state = Arc::new(AppState::for_tests());
        let res = do_start_network(shared_state.clone(), db_conn.common)
            .await
            .unwrap();
        let res_into = res.into_response();
        assert_eq!(res_into.status(), StatusCode::OK);

        let body = json!({}).to_string();
        let agent_app = network_router();
        let response = agent_app
            .with_state(shared_state.clone())
            .oneshot(
                Request::builder()
                    .method("GET")
                    .header("content-type", "application/json")
                    .uri("/v1/tilekit/net/status")
                    .body(Body::new(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let res_into = response.into_response();
        assert_eq!(res_into.status(), StatusCode::OK);
        let body_bytes = res_into.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        println!("{}", body);
        assert!(body.contains("\"is_network_running\":true"));
    }
}
