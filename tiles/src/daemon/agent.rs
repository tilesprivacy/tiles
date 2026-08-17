//! APIs for communication with Agent harness (Pi)

use std::sync::Arc;

use axum::{Router, extract::State, response::IntoResponse, routing::get};
use axum_macros::debug_handler;
use serde_json::json;

use crate::{
    core::agent::pi::{self},
    daemon::{ApiResponse, AppError, AppState},
    repl::{get_default_modelfile, model_spec},
    utils::config::PY_PORT,
};

pub fn agent_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/agent/start", get(start_agent))
        .route("/v1/tilekit/agent/end_session", get(end_current_session))
        .route("/v1/tilekit/agent/status", get(agent_status))
}

async fn start_agent(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let modelfile_path =
        get_default_modelfile().map_err(|e| AppError::ModelFileNotFound(e.to_string()))?;
    let default_modelfile = tilekit::modelfile::parse_from_file(&modelfile_path.to_string_lossy())
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let modelname =
        model_spec(&default_modelfile).map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let system_prompt = default_modelfile.system.clone().unwrap_or("".to_owned());

    let mut agent = state.agent.lock().await;
    if agent.is_some() {
        Ok(ApiResponse::success(
            json!({"message": "Agent already started"}),
        ))
    } else {
        let pi_agent = pi::new(&modelname, &system_prompt, PY_PORT)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;
        *agent = Some(pi_agent);
        Ok(ApiResponse::success(json!({"message": "started agent"})))
    }
}

#[debug_handler]
async fn end_current_session(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let mut agent = state.agent.lock().await;
    let agent = agent.as_mut().ok_or(AppError::InternalServerError(
        "Failed to get a mutable agent instance".to_string(),
    ))?;
    pi::handle_graceful_exit(&mut agent.writer)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(ApiResponse::success(
        json!({"message": "Successfully ended current session"}),
    ))
}

// TODO: add timeout to apis
async fn agent_status(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let mut agent = state.agent.lock().await;
    let agent = agent.as_mut().ok_or(AppError::InternalServerError(
        "Failed to get a mutable agent instance".to_string(),
    ))?;

    let state = agent
        .reader
        .get_pi_state(&mut agent.writer)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(ApiResponse::success(serde_json::to_value(state).unwrap()))
}
