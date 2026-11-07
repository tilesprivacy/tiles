//! APIs for the llama.cpp build Tiles runs models with

mod engine_fetch;
mod install;

use std::{env, sync::Arc};

use axum::{
    Router,
    extract::{Query, State},
    http::HeaderMap,
    response::{
        IntoResponse,
        sse::{KeepAlive, Sse},
    },
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use tiles_engine::{BUILDS, Engine, Hardware, auto, choices, detect, engine_to_run};

pub use engine_fetch::offer_gpu_engine;
pub use install::{Engines, InstallState};

use crate::{
    daemon::{ApiJson, ApiResponse, AppError, AppState, model::CLIENT_HEADER},
    utils::config::{get_engine_pick, set_engine_pick},
};

pub fn engine_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/engine/status", get(status))
        .route("/v1/tilekit/engine/select", post(select))
        .route("/v1/tilekit/engine/download", get(progress).post(start))
        .route("/v1/tilekit/engine/download/pause", post(pause))
        .route("/v1/tilekit/engine/delete", post(delete))
}

/// What this computer has, the engine Tiles runs, and every engine with why it can't run
async fn status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ApiResponse::success(describe(&state.engines, &detect().await))
}

fn describe(engines: &Engines, hardware: &Hardware) -> Value {
    let choices = engines.choices(hardware);
    let pick = get_engine_pick();
    let engine = engine_to_run(pick, hardware, &choices);
    let installed = engines.installed();
    let needs_download = !engine.is_bundled()
        && !installed
            .as_ref()
            .is_some_and(|installed| installed.engine == engine && installed.tag == BUILDS.tag);
    json!({
        "llama_cpp": BUILDS.tag,
        "pick": pick.map_or("auto", Engine::name),
        "auto": auto(hardware, &choices),
        "engine": engine,
        "installed": installed.map(|installed| json!({
            "engine": installed.engine,
            "llama_cpp": installed.tag,
        })),
        "needs_download": needs_download,
        // set by hand, replies run on it whatever is picked
        "override": env::var("TILES_LLAMA_SERVER_BIN").ok().filter(|path| !path.is_empty()),
        "download": engines.state(),
        "gpus": hardware.gpus,
        "nvidia_driver": hardware.nvidia_driver,
        "engines": choices,
    })
}

#[derive(Deserialize)]
struct Pick {
    engine: String,
}

/// Saves the engine to use, or "auto". Picking one that failed its check tries it again
async fn select(
    State(state): State<Arc<AppState>>,
    ApiJson(pick): ApiJson<Pick>,
) -> Result<impl IntoResponse, AppError> {
    let pick = match pick.engine.as_str() {
        "auto" => None,
        name => Some(
            Engine::from_name(name)
                .ok_or_else(|| AppError::BadRequest(format!("No engine named {name}")))?,
        ),
    };
    let hardware = detect().await;
    // a download that starts meanwhile reads this pick, or is running and turns it away
    let _picking = state.engines.picking.lock().await;
    if state.engines.is_installing().await {
        return Err(AppError::Conflict(
            "An engine is downloading, pause it before picking another".to_owned(),
        ));
    }
    if let Some(engine) = pick {
        // the reasons from the hardware alone, a failed check can be tried again
        let choice = choices(&hardware, &BUILDS)
            .into_iter()
            .find(|choice| choice.engine == engine)
            .ok_or_else(|| {
                AppError::CannotProcess(format!("{} doesn't run on this computer", engine.name()))
            })?;
        if let Some(reason) = choice.unavailable {
            return Err(AppError::CannotProcess(reason));
        }
        state.engines.clear_failure(engine)?;
    }
    set_engine_pick(pick)
        .map_err(|err| AppError::InternalServerError(format!("Couldn't save the engine: {err}")))?;
    Ok(ApiResponse::success(describe(&state.engines, &hardware)))
}

/// Downloads and installs the engine Tiles runs, unless it's there already
async fn start(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let hardware = detect().await;
    // the pick and the failure marks as select left them, see `Engines::picking`
    let _picking = state.engines.picking.lock().await;
    let choices = state.engines.choices(&hardware);
    let engine = engine_to_run(get_engine_pick(), &hardware, &choices);
    Ok(ApiResponse::success(
        state.engines.start(engine, &choices).await?,
    ))
}

/// Deletes the engine on disk and returns the bytes freed. Replies go back to the CPU
async fn delete(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let freed = state.engines.delete().await?;
    Ok(ApiResponse::success(json!({ "freed": freed })))
}

async fn pause(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    Ok(ApiResponse::success(state.engines.pause().await?))
}

#[derive(Deserialize)]
struct Follow {
    #[serde(default)]
    pause_on_close: bool,
}

/// The install's state as server-sent events, ending once it stops.
/// Closing it only pauses the install with `pause_on_close`
async fn progress(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(follow): Query<Follow>,
) -> Result<impl IntoResponse, AppError> {
    if follow.pause_on_close && !headers.contains_key(CLIENT_HEADER) {
        return Err(AppError::BadRequest(format!(
            "pause_on_close needs the {CLIENT_HEADER} header"
        )));
    }
    Ok(Sse::new(state.engines.events(follow.pause_on_close)).keep_alive(KeepAlive::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn send(request: Request<Body>) -> (u16, Value) {
        let res = engine_router()
            .with_state(Arc::new(AppState::for_tests()))
            .oneshot(request)
            .await
            .unwrap();
        let status = res.status().as_u16();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&body).unwrap())
    }

    fn pick(engine: &str) -> Request<Body> {
        Request::post("/v1/tilekit/engine/select")
            .header("content-type", "application/json")
            .body(Body::from(json!({ "engine": engine }).to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn an_unknown_or_foreign_engine_is_refused() {
        let (status, body) = send(pick("rocm")).await;
        assert_eq!(status, 400);
        assert_eq!(body["reason"], "No engine named rocm");

        let foreign = if cfg!(target_os = "macos") {
            "vulkan"
        } else {
            "metal"
        };
        let (status, _) = send(pick(foreign)).await;
        assert_eq!(status, 422);
    }

    #[tokio::test]
    async fn deleting_with_no_engine_downloaded_is_not_found() {
        let (status, body) = send(
            Request::post("/v1/tilekit/engine/delete")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await;
        assert_eq!(status, 404);
        assert_eq!(body["reason"], "No engine is downloaded");
    }
}
