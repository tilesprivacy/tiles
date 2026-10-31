//! APIs for managing and communicating with Inference server

use std::sync::Arc;

use axum::{
    Router,
    response::IntoResponse,
    routing::{get, post},
};
use futures_util::TryFutureExt;
use serde_json::json;

use crate::{
    core::server::{ServerError, ping, start_server_daemon, stop_server_daemon},
    daemon::{ApiResponse, AppError, AppState},
};

/// Routers for server apis
///
/// These are to be merged with the main router in daemon/mod.rs
pub fn server_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/server/start", post(start_server))
        .route("/v1/tilekit/server/stop", post(stop_server))
        .route("/v1/tilekit/server/ping", get(ping_server))
}

/// Api handler for starting inference server
///
/// Answers once the server is reachable, with `started` or `already_running`
async fn start_server() -> Result<impl IntoResponse, AppError> {
    let state = start_server_daemon().map_err(to_app_error).await?;

    Ok(ApiResponse::success(json!({"state": state})))
}

/// Api handler for stoping inference server
///
/// Answers once the process has exited, with `stopped` or `not_running`
async fn stop_server() -> Result<impl IntoResponse, AppError> {
    let state = stop_server_daemon().map_err(to_app_error).await?;

    Ok(ApiResponse::success(json!({"state": state})))
}

/// Api handler for inference server health check
///
/// `model` is the server's model state, null from older servers that don't report it
async fn ping_server() -> Result<impl IntoResponse, AppError> {
    let model = ping().map_err(to_app_error).await?;

    Ok(ApiResponse::success(
        json!({"state": "running", "model": model}),
    ))
}

fn to_app_error(err: anyhow::Error) -> AppError {
    match err.downcast_ref::<ServerError>() {
        Some(ServerError::PortInUse) => AppError::AlreadyExists(err.to_string()),
        Some(ServerError::NotReady | ServerError::NotRunning) => {
            AppError::ServiceUnavailable(err.to_string())
        }
        None => AppError::InternalServerError(err.to_string()),
    }
}
