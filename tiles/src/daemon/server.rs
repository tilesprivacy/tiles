//! APIs for managing and communicating with Inference server

use std::sync::Arc;

use axum::{Router, response::IntoResponse, routing::get};
use futures_util::TryFutureExt;
use serde_json::json;

use crate::{
    core::server::{ping, start_server_daemon, stop_server_daemon},
    daemon::{ApiResponse, AppError, AppState},
};

/// Routers for server apis
///
/// These are to be merged with the main router in daemon/mod.rs
pub fn server_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/server/start", get(start_server))
        .route("/v1/tilekit/server/stop", get(stop_server))
        .route("/v1/tilekit/server/ping", get(ping_server))
}

/// Api handler for starting inference server
async fn start_server() -> Result<impl IntoResponse, AppError> {
    let response = start_server_daemon()
        .map_err(|e| AppError::InternalServerError(e.to_string()))
        .await?;

    Ok(ApiResponse::success(json!({"message": response})))
}

/// Api handler for stoping inference server
async fn stop_server() -> Result<impl IntoResponse, AppError> {
    let response = stop_server_daemon()
        .map_err(|e| AppError::InternalServerError(e.to_string()))
        .await?;

    Ok(ApiResponse::success(json!({"message": response})))
}

/// Api handler for inference server health check
async fn ping_server() -> Result<impl IntoResponse, AppError> {
    let response = ping()
        .map_err(|e| AppError::InternalServerError(e.to_string()))
        .await?;
    Ok(ApiResponse::success(json!({"message": response})))
}
