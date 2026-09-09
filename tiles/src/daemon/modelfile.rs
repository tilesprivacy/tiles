//! Reading and writing the modelfile the agent starts from
//!
//! The shipped modelfile installs into the lib dir owned by root, so an edit is
//! written to the user's data dir instead and wins from there. Nothing here
//! touches a running agent: see `/v1/tilekit/agent/reload` for that.

use std::fs;
use std::sync::Arc;

use axum::{
    Json, Router,
    routing::{get, put},
};
use serde::{Deserialize, Serialize};

use crate::{
    daemon::{ApiResponse, AppError, AppState},
    repl::{get_default_modelfile, user_modelfile_path},
    utils::config::DefaultProvider,
};

#[derive(Serialize)]
struct ModelfileData {
    content: String,
    /// false while the shipped one is still in use, so the UI can say so
    edited: bool,
}

#[derive(Deserialize)]
struct ModelfileRequest {
    content: String,
}

pub fn modelfile_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/modelfile", get(read_modelfile))
        .route("/v1/tilekit/modelfile", put(write_modelfile))
}

async fn read_modelfile() -> Result<Json<ApiResponse<ModelfileData>>, AppError> {
    let path =
        get_default_modelfile(DefaultProvider).map_err(|e| AppError::NotFound(e.to_string()))?;

    let content = fs::read_to_string(&path)
        .map_err(|e| AppError::NotFound(format!("Could not read {}: {e}", path.display())))?;

    let edited = user_modelfile_path(&DefaultProvider)
        .map(|user| user == path)
        .unwrap_or(false);

    Ok(ApiResponse::success(ModelfileData { content, edited }))
}

async fn write_modelfile(
    Json(payload): Json<ModelfileRequest>,
) -> Result<Json<ApiResponse<ModelfileData>>, AppError> {
    // the parser insists on a FROM, so this also catches a modelfile that would
    // leave the agent with no model to start
    tilekit::modelfile::parse(&payload.content)
        .map_err(|e| AppError::CannotProcess(format!("Not a valid modelfile: {e}")))?;

    let path = user_modelfile_path(&DefaultProvider)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            AppError::InternalServerError(format!("Could not create {parent:?}: {e}"))
        })?;
    }

    fs::write(&path, &payload.content).map_err(|e| {
        AppError::InternalServerError(format!("Could not write {}: {e}", path.display()))
    })?;

    Ok(ApiResponse::success(ModelfileData {
        content: payload.content,
        edited: true,
    }))
}
