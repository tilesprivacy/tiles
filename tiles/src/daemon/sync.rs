//! Tilekit apis related to session sync

use std::sync::Arc;

use crate::core::account::local::get_current_user;
use crate::core::network::{self as core_network, create_endpoint};
use crate::core::storage::db::get_db_conn;
use crate::daemon::{ApiResponse, AppError, AppState};
use anyhow::Result;
use axum::extract::Path;
use axum::{Router, extract::State, response::IntoResponse, routing::get};
use iroh::Endpoint;
use serde_json::json;
use tokio::sync::mpsc;
pub struct SyncState {
    pub is_endpoint_running: bool,
    pub sync_main_sender: tokio::sync::mpsc::Sender<u8>,
    pub endpoint: Endpoint,
}

pub enum SyncApiResponse {
    SyncListenerStarted,
    DirectSyncDone,
}

pub fn sync_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/sync/peers", get(get_peers))
        .route(
            "/v1/tilekit/sync/toggle-sync-listener",
            get(toggle_sync_listener),
        )
        .route(
            "/v1/tilekit/sync/sync-listener-status",
            get(get_sync_listener_status),
        )
        // TODO: We can use the same api to link device too.
        .route(
            "/v1/tilekit/sync/sync-with-device/{did}",
            get(sync_with_device),
        )
}

pub async fn get_peers() -> Result<impl IntoResponse, AppError> {
    Ok(ApiResponse::success(json!("hi")))
}

//TODO: return the resp once the network starts listening
pub async fn toggle_sync_listener(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // if listener not running, call the sync function
    // else send a stop signal to stop it
    {
        let mut sync_state = state.sync_state.lock().await;

        if let Some(sync) = sync_state.as_mut()
            && sync.is_endpoint_running
        {
            let _ = sync.sync_main_sender.send(0).await;
            sync.is_endpoint_running = false;
            log::info!("Stopped the sync listener");
            return Ok(ApiResponse::success(json!("Stopped listener")));
        };
    }

    let (_endpoint, _sync_main_sender) = start_sync_listener(state)
        .await
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    Ok(ApiResponse::success(json!("started sync listener")))
}

async fn start_sync_listener(
    state: Arc<AppState>,
) -> Result<(Endpoint, tokio::sync::mpsc::Sender<u8>)> {
    let mut sync_state = state.sync_state.lock().await;
    let user_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)?;
    let user = get_current_user(&user_db_conn)?;
    let endpoint = create_endpoint(&user).await?;
    let (sync_main_sender, sync_main_receiver) = mpsc::channel::<u8>(10);

    let (api_resp_sendx, mut api_resp_recvx) = mpsc::channel::<SyncApiResponse>(10);
    let endpoint_clone = endpoint.clone();
    let sync_sender_clone = sync_main_sender.clone();
    tokio::spawn(async move {
        core_network::sync(
            None,
            endpoint,
            sync_sender_clone,
            sync_main_receiver,
            api_resp_sendx.clone(),
        )
        .await?;
        anyhow::Result::<()>::Ok(())
    });
    api_resp_recvx.recv().await;
    *sync_state = Some(SyncState {
        is_endpoint_running: true,
        sync_main_sender: sync_main_sender.clone(),
        endpoint: endpoint_clone.clone(),
    });
    Ok((endpoint_clone.clone(), sync_main_sender))
}

pub async fn get_sync_listener_status(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let mut sync_state = state.sync_state.lock().await;

    let is_running = if let Some(sync) = sync_state.as_mut() {
        sync.is_endpoint_running
    } else {
        false
    };
    Ok(ApiResponse::success(json!({
        "is_running": is_running
    })))
}

pub async fn sync_with_device(
    State(state): State<Arc<AppState>>,
    Path(did): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let mut sync_state = state.sync_state.lock().await;

    // start the endpoint if not running
    if let Some(sync) = sync_state.as_mut()
        && !sync.is_endpoint_running
    {
        let (_endpoint, _sync_main_sender) = start_sync_listener(state.clone())
            .await
            .map_err(|e| AppError::CannotProcess(e.to_string()))?;
    };

    let (sync_main_sender, sync_main_receiver) = mpsc::channel::<u8>(10);

    let (api_resp_sendx, mut api_resp_recvx) = mpsc::channel::<SyncApiResponse>(10);

    if let Some(sync) = sync_state.as_mut() {
        let endpoint_clone = sync.endpoint.clone();
        let sync_sender_clone = sync_main_sender.clone();
        tokio::spawn(async move {
            core_network::sync(
                None,
                endpoint_clone,
                sync_sender_clone,
                sync_main_receiver,
                api_resp_sendx.clone(),
            )
            .await?;
            anyhow::Result::<()>::Ok(())
        });
        api_resp_recvx.recv().await;

        let resp_str = format!("Syncing completed with {}", did);
        Ok(ApiResponse::success(json!(resp_str)))
    } else {
        Err(AppError::CannotProcess(
            "Failed to get the sync_state".to_owned(),
        ))
    }
}
