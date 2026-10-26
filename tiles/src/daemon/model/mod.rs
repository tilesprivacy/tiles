//! Downloads models with progress, pause and resume, for the UI and the CLI alike
//!
//! - `download`: one model's download, its state and its progress stream
//! - `bundled`: the models Tiles ships a Modelfile for
//! - `pull`: the CLI's download, as a client of these routes

mod bundled;
mod download;
mod pull;
#[cfg(test)]
mod tests;

use std::{
    collections::HashMap,
    convert::Infallible,
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, bail};
use axum::{
    Router,
    extract::{Query, State},
    http::HeaderMap,
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use serde::Deserialize;
use serde_json::json;
use tilekit::modelfile::split_model_spec;
use tokio::sync::broadcast::{self, error::RecvError};

pub use download::{DownloadState, ModelChange, ModelDownload};
pub use pull::pull;

use crate::{
    daemon::{ApiJson, ApiResponse, AppError, AppState},
    repl::get_default_modelfile,
    utils::{
        config::{
            ConfigProvider, DefaultProvider, get_config_json, get_or_create_model_download_path,
        },
        hf_model_downloader::{DEFAULT_QUANT, Hub},
    },
};

// left free after a download, so it never fills the disk to the last byte
const SPARE_ROOM: u64 = 1_000_000_000;
// changes a slow events client can fall behind by before it's sent everything again
const CHANGES_KEPT: usize = 64;

/// Sent by the CLI with `pause_on_close`. Other websites can't send a custom header
/// without a CORS preflight, which the daemon refuses, so a page can't pause downloads
pub(super) const CLIENT_HEADER: &str = "x-tiles-client";

pub fn model_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/model/status", get(status))
        .route("/v1/tilekit/model/downloads", get(downloads))
        .route("/v1/tilekit/model/events", get(events))
        .route(
            "/v1/tilekit/model/download",
            get(progress).post(start).delete(pause),
        )
}

/// A model as `org/name:quant`, the quant `Q4_K_M` when not given
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spec {
    repo: String,
    quant: String,
}

impl Spec {
    fn new(repo: &str, quant: Option<&str>) -> anyhow::Result<Self> {
        let mut parts = repo.split('/');
        if !matches!((parts.next(), parts.next(), parts.next()), (Some(org), Some(name), None) if !org.is_empty() && !name.is_empty())
        {
            bail!("Name a model as org/name:quant, like unsloth/gemma-4-12b-it-GGUF:Q4_K_M");
        }
        Ok(Self {
            repo: repo.to_owned(),
            // file names are matched ignoring case, so one model has one entry
            quant: quant.unwrap_or(DEFAULT_QUANT).to_uppercase(),
        })
    }

    fn parse(model: &str) -> anyhow::Result<Self> {
        let (repo, quant) = split_model_spec(model.trim());
        Self::new(repo, quant)
    }
}

impl fmt::Display for Spec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.repo, self.quant)
    }
}

/// Every model the daemon has been asked about, each with its own download
pub struct Models {
    hub: Arc<Hub>,
    /// The models folder
    root: PathBuf,
    /// The Modelfiles folder, with one for each bundled model
    modelfiles: PathBuf,
    spare_room: u64,
    /// The default Modelfile's model, for requests that name none
    default: Option<Spec>,
    downloads: Mutex<HashMap<Spec, Arc<ModelDownload>>>,
    /// Every download's changes, for the events stream
    changes: broadcast::Sender<ModelChange>,
    /// The model `tiles run` last ran. The daemon reads it from config.toml
    current_model: fn() -> Option<String>,
}

impl Models {
    pub fn new(hub: Hub, root: PathBuf, modelfiles: PathBuf, default: Option<Spec>) -> Self {
        Self {
            hub: Arc::new(hub),
            root,
            modelfiles,
            spare_room: SPARE_ROOM,
            default,
            downloads: Mutex::new(HashMap::new()),
            changes: broadcast::Sender::new(CHANGES_KEPT),
            current_model: || None,
        }
    }

    /// Hugging Face, the models folder and the Modelfiles in the lib dir
    pub fn for_daemon() -> anyhow::Result<Self> {
        let default = get_default_modelfile(DefaultProvider)
            .and_then(|path| modelfile_spec(&path))
            .inspect_err(|err| log::warn!("No default model to download: {err:#}"))
            .ok();
        Ok(Self {
            current_model: || {
                let config = get_config_json().ok()?;
                config["model"]["current"].as_str().map(str::to_owned)
            },
            ..Self::new(
                Hub::from_env()?,
                get_or_create_model_download_path()?,
                DefaultProvider.get_lib_dir()?.join("modelfiles"),
                default,
            )
        })
    }

    /// The download of `model`, or of the default model when none is named
    fn get(&self, model: Option<&str>) -> Result<Arc<ModelDownload>, AppError> {
        let spec = match model {
            Some(model) => {
                Spec::parse(model).map_err(|err| AppError::BadRequest(err.to_string()))?
            }
            None => self.default.clone().ok_or_else(|| {
                AppError::NotFound("No model was named, and there's no default model".to_owned())
            })?,
        };
        Ok(self.download(spec))
    }

    fn download(&self, spec: Spec) -> Arc<ModelDownload> {
        let mut downloads = self.downloads.lock().expect("model downloads lock");
        let download = downloads.entry(spec).or_insert_with_key(|spec| {
            Arc::new(ModelDownload::new(
                self.hub.clone(),
                spec.clone(),
                self.root.clone(),
                self.spare_room,
                self.changes.clone(),
            ))
        });
        download.clone()
    }

    /// Every download that's running, or stopped before the end
    pub fn unfinished(&self) -> Vec<ModelChange> {
        self.downloads
            .lock()
            .expect("model downloads lock")
            .values()
            .map(|download| download.change())
            .filter(|change| change.state.is_unfinished())
            .collect()
    }

    /// Pauses every running download, keeping what each got
    pub async fn pause_all(&self) {
        let downloads: Vec<_> = self
            .downloads
            .lock()
            .expect("model downloads lock")
            .values()
            .cloned()
            .collect();
        for download in downloads {
            let _ = download.pause().await;
        }
    }
}

/// The model a Modelfile's `FROM` names
fn modelfile_spec(path: &Path) -> anyhow::Result<Spec> {
    let modelfile =
        tilekit::modelfile::parse_from_file(&path.to_string_lossy()).map_err(anyhow::Error::msg)?;
    let repo = modelfile
        .from
        .with_context(|| format!("{} has no FROM", path.display()))?;
    Spec::new(&repo, modelfile.quant.as_deref())
}

#[derive(Deserialize)]
struct Which {
    model: Option<String>,
}

#[derive(Deserialize)]
struct Follow {
    model: Option<String>,
    #[serde(default)]
    pause_on_close: bool,
}

/// `GET /v1/tilekit/model/downloads`: every download that's running, paused or
/// failed, wherever it was started
async fn downloads(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ApiResponse::success(json!({ "downloads": state.models.unfinished() }))
}

/// `GET /v1/tilekit/model/events`, as server-sent events that stay open: the unfinished
/// downloads, then every change to any model's download, from any client
async fn events(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // subscribed before the first list, so no change falls between the two
    let mut changes = state.models.changes.subscribe();
    let stream = async_stream::stream! {
        let mut send_all = true;
        loop {
            if std::mem::take(&mut send_all) {
                for change in state.models.unfinished() {
                    yield Ok::<_, Infallible>(model_event(&change));
                }
            }
            match changes.recv().await {
                Ok(change) => yield Ok(model_event(&change)),
                // the newest changes are still there, but older ones were dropped
                Err(RecvError::Lagged(_)) => send_all = true,
                Err(RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn model_event(change: &ModelChange) -> Event {
    Event::default()
        .event("model")
        .data(json!(change).to_string())
}

/// `GET /v1/tilekit/model/status?model=org/name:quant` for one model. With no model,
/// every bundled model, the default one marked
async fn status(
    State(state): State<Arc<AppState>>,
    Query(which): Query<Which>,
) -> Result<Response, AppError> {
    Ok(match which.model {
        Some(model) => {
            let download = state.models.get(Some(&model))?;
            ApiResponse::success(download.status().await?).into_response()
        }
        None => {
            ApiResponse::success(json!({ "models": state.models.bundled().await })).into_response()
        }
    })
}

/// `POST /v1/tilekit/model/download` with `{"model": "org/name:quant"}`, or `{}`
/// for the default model
async fn start(
    State(state): State<Arc<AppState>>,
    ApiJson(which): ApiJson<Which>,
) -> Result<impl IntoResponse, AppError> {
    let download = state.models.get(which.model.as_deref())?;
    Ok(ApiResponse::success(download.start().await?))
}

/// `DELETE /v1/tilekit/model/download?model=org/name:quant`
async fn pause(
    State(state): State<Arc<AppState>>,
    Query(which): Query<Which>,
) -> Result<impl IntoResponse, AppError> {
    let download = state.models.get(which.model.as_deref())?;
    Ok(ApiResponse::success(download.pause().await?))
}

/// `GET /v1/tilekit/model/download?model=org/name:quant`, as server-sent events.
/// Closing it doesn't stop the download, unless it asked with `pause_on_close=true`
/// and the `x-tiles-client` header
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
    let download = state.models.get(follow.model.as_deref())?;
    Ok(Sse::new(download.events(follow.pause_on_close)).keep_alive(KeepAlive::default()))
}
