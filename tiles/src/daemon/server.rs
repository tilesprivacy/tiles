//! APIs for managing and communicating with Inference server

use std::sync::Arc;

use axum::{Router, response::IntoResponse, routing::get};
use futures_util::TryFutureExt;
use serde_json::json;

use crate::{
    core::server::{ping, start_server_daemon, stop_server_daemon, warm_up},
    daemon::{ApiResponse, AppError, AppState, agent::get_agent_start_params},
    utils::config::DefaultProvider,
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

/// Load the current model and prefill the agent's prompt in the background,
/// so the first message after starting inference does not wait on either.
pub fn warm_up_current_model() {
    let model = match get_agent_start_params(DefaultProvider) {
        Ok((model, _)) => model,
        Err(err) => {
            log::warn!("Not warming up, no model to warm: {}", err.reason());
            return;
        }
    };
    tokio::spawn(async move {
        let started = std::time::Instant::now();
        match warm_up(&model).await {
            Ok(prefilled) => log::info!(
                "Warmed up {model} in {:.1}s{}",
                started.elapsed().as_secs_f32(),
                if prefilled {
                    ", agent prompt prefilled"
                } else {
                    ""
                }
            ),
            Err(err) => log::warn!("Warm-up of {model} failed: {err:#}"),
        }
    });
}

/// Api handler for starting inference server
async fn start_server() -> Result<impl IntoResponse, AppError> {
    let response = start_server_daemon()
        .map_err(|e| AppError::InternalServerError(e.to_string()))
        .await?;
    warm_up_current_model();

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
