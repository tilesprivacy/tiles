//! Downloads models with progress, pause and resume, for the UI and the CLI alike
//!
//! - `download`: one model's download, its state and its progress stream
//! - `pull`: the CLI's download, as a client of these routes

mod download;
mod pull;
#[cfg(test)]
mod tests;

use std::{
    collections::HashMap,
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::{Context, bail};
use axum::{
    Router,
    extract::{Query, State},
    response::{IntoResponse, Sse, sse::KeepAlive},
    routing::get,
};
use serde::Deserialize;
use tilekit::modelfile::split_model_spec;

pub use download::{DownloadState, ModelDownload};
pub use pull::pull;

use crate::{
    daemon::{ApiJson, ApiResponse, AppError, AppState},
    repl::get_default_modelfile,
    utils::{
        config::{DefaultProvider, get_or_create_model_download_path},
        hf_model_downloader::{DEFAULT_QUANT, Hub},
    },
};

// left free after a download, so it never fills the disk to the last byte
const SPARE_ROOM: u64 = 1_000_000_000;

pub fn model_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/model/status", get(status))
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
    spare_room: u64,
    /// The default Modelfile's model, for requests that name none
    default: Option<Spec>,
    downloads: Mutex<HashMap<Spec, Arc<ModelDownload>>>,
}

impl Models {
    pub fn new(hub: Hub, root: PathBuf, default: Option<Spec>) -> Self {
        Self {
            hub: Arc::new(hub),
            root,
            spare_room: SPARE_ROOM,
            default,
            downloads: Mutex::new(HashMap::new()),
        }
    }

    /// Hugging Face, the models folder and the default Modelfile's model
    pub fn for_daemon() -> anyhow::Result<Self> {
        let default = default_model()
            .inspect_err(|err| log::warn!("No default model to download: {err:#}"))
            .ok();
        Ok(Self::new(
            Hub::from_env()?,
            get_or_create_model_download_path()?,
            default,
        ))
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
        let mut downloads = self.downloads.lock().expect("model downloads lock");
        let download = downloads.entry(spec).or_insert_with_key(|spec| {
            Arc::new(ModelDownload::new(
                self.hub.clone(),
                spec.clone(),
                self.root.clone(),
                self.spare_room,
            ))
        });
        Ok(download.clone())
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

fn default_model() -> anyhow::Result<Spec> {
    let path = get_default_modelfile(DefaultProvider)?;
    let modelfile =
        tilekit::modelfile::parse_from_file(&path.to_string_lossy()).map_err(anyhow::Error::msg)?;
    let repo = modelfile
        .from
        .context("The default Modelfile has no FROM")?;
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

/// `GET /v1/tilekit/model/status?model=org/name:quant`
async fn status(
    State(state): State<Arc<AppState>>,
    Query(which): Query<Which>,
) -> Result<impl IntoResponse, AppError> {
    let download = state.models.get(which.model.as_deref())?;
    Ok(ApiResponse::success(download.status().await?))
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
async fn progress(
    State(state): State<Arc<AppState>>,
    Query(follow): Query<Follow>,
) -> Result<impl IntoResponse, AppError> {
    let download = state.models.get(follow.model.as_deref())?;
    Ok(Sse::new(download.events(follow.pause_on_close)).keep_alive(KeepAlive::default()))
}
