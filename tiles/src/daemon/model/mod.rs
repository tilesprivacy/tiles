//! Model downloads with pause and resume, for the UI and the CLI

mod bundled;
mod download;
mod model_fetch;
#[cfg(test)]
mod tests;

use std::{
    collections::HashMap,
    convert::Infallible,
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
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
    routing::{get, post},
};
use futures_util::future::join_all;
use serde::Deserialize;
use serde_json::json;
use tilekit::modelfile::split_model_spec;
use tokio::sync::broadcast::{self, error::RecvError};

pub use bundled::picked_modelfile;
pub use download::{DownloadState, ModelChange, ModelDownload};
pub(crate) use download::{free_space, gb};
pub use model_fetch::pull;
pub(crate) use model_fetch::{download_bar, finish_bar, response_data};

use crate::{
    core::{
        agent::pi::PiAgent,
        server::{ServerState, start_server_daemon, stop_server_daemon},
    },
    daemon::{ApiJson, ApiResponse, AppError, AppState},
    repl::get_default_modelfile,
    utils::{
        config::{
            ConfigProvider, DefaultProvider, get_current_model, get_or_create_model_download_path,
            update_current_model,
        },
        hf_model_downloader::{DEFAULT_QUANT, Hub},
    },
};

// kept free after a download
const SPARE_ROOM: u64 = 1_000_000_000;
// how far a slow events client can fall behind
const CHANGES_KEPT: usize = 64;

// needed for `pause_on_close`, a web page can't send it without a CORS preflight
pub(crate) const CLIENT_HEADER: &str = "x-tiles-client";

pub fn model_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/model/status", get(status))
        .route("/v1/tilekit/model/downloads", get(downloads))
        .route("/v1/tilekit/model/events", get(events))
        .route("/v1/tilekit/model/download", get(progress).post(start))
        .route("/v1/tilekit/model/download/pause", post(pause))
        .route("/v1/tilekit/model/select", post(select))
        .route("/v1/tilekit/model/delete", post(delete))
}

/// A model as `org/name:quant`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spec {
    repo: String,
    quant: String,
}

impl Spec {
    fn new(repo: &str, quant: Option<&str>) -> anyhow::Result<Self> {
        // the name goes into a Hub url and a folder name, so only what the Hub allows
        let part = |p: &str| {
            !matches!(p, "" | "." | "..")
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        };
        let mut parts = repo.split('/');
        if !matches!((parts.next(), parts.next(), parts.next()), (Some(org), Some(name), None) if part(org) && part(name))
        {
            bail!("Name a model as org/name:quant, like unsloth/gemma-4-12b-it-GGUF:Q4_K_M");
        }
        Ok(Self {
            repo: repo.to_owned(),
            // uppercased so one model has one entry
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

pub struct Models {
    hub: Arc<Hub>,
    root: PathBuf,
    modelfiles: PathBuf,
    spare_room: u64,
    default: Option<Spec>,
    downloads: Mutex<HashMap<Spec, Arc<ModelDownload>>>,
    changes: broadcast::Sender<ModelChange>,
    current_model: CurrentModel,
}

// the model chats use, picked in the UI or last run by `tiles run`
enum CurrentModel {
    // saved in config.toml, same as `tiles run`
    Config,
    Memory(Mutex<Option<String>>),
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
            current_model: CurrentModel::Memory(Mutex::default()),
        }
    }

    pub fn for_daemon() -> anyhow::Result<Self> {
        let default = get_default_modelfile(DefaultProvider)
            .and_then(|path| modelfile_spec(&path))
            .inspect_err(|err| log::warn!("No default model to download: {err:#}"))
            .ok();
        Ok(Self {
            current_model: CurrentModel::Config,
            ..Self::new(
                Hub::from_env()?,
                get_or_create_model_download_path()?,
                DefaultProvider.get_lib_dir()?.join("modelfiles"),
                default,
            )
        })
    }

    // the default model when none is named
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

    // every quant of `repo` known so far, sorted for a fixed lock order
    fn quants(&self, repo: &str) -> Vec<Arc<ModelDownload>> {
        let downloads = self.downloads.lock().expect("model downloads lock");
        let mut quants: Vec<_> = downloads
            .iter()
            .filter(|(spec, _)| spec.repo == repo)
            .map(|(_, download)| download.clone())
            .collect();
        quants.sort_by(|a, b| a.spec().quant.cmp(&b.spec().quant));
        quants
    }

    fn current(&self) -> Option<Spec> {
        let current = match &self.current_model {
            CurrentModel::Config => get_current_model(),
            CurrentModel::Memory(current) => current.lock().expect("current model lock").clone(),
        };
        // only Tiles' own models, like agent startup
        Spec::parse(&current?)
            .ok()
            .filter(|spec| bundled::modelfile_for(&self.modelfiles, spec).is_some())
    }

    fn set_current_model(&self, spec: &Spec) -> anyhow::Result<()> {
        match &self.current_model {
            CurrentModel::Config => update_current_model(&spec.to_string()),
            CurrentModel::Memory(current) => {
                *current.lock().expect("current model lock") = Some(spec.to_string());
                Ok(())
            }
        }
    }

    // an empty one reads as none, see `current`
    fn clear_current_model(&self) -> anyhow::Result<()> {
        match &self.current_model {
            CurrentModel::Config => update_current_model(""),
            CurrentModel::Memory(current) => {
                *current.lock().expect("current model lock") = None;
                Ok(())
            }
        }
    }

    /// What chats move to once `deleted` goes, see [`default_or_newest`]
    fn next_current_model(&self, deleted: &Spec) -> Option<Spec> {
        let downloaded = bundled::BUNDLED
            .iter()
            .filter_map(|(id, _)| modelfile_spec(&self.modelfiles.join(id)).ok())
            .filter(|spec| spec.repo != deleted.repo)
            .filter_map(|spec| {
                let at = self.download(spec.clone()).downloaded_at()?;
                Some((spec, at))
            })
            .collect();
        default_or_newest(self.default.as_ref(), downloaded)
    }

    /// Downloads that are running, paused or failed
    pub fn unfinished(&self) -> Vec<ModelChange> {
        self.downloads
            .lock()
            .expect("model downloads lock")
            .values()
            .map(|download| download.change())
            .filter(|change| change.state.is_unfinished())
            .collect()
    }

    pub async fn pause_all(&self) {
        let downloads: Vec<_> = self
            .downloads
            .lock()
            .expect("model downloads lock")
            .values()
            .cloned()
            .collect();
        join_all(downloads.iter().map(|download| download.pause())).await;
    }
}

/// The default model when it's downloaded, else the one downloaded last
fn default_or_newest(default: Option<&Spec>, downloaded: Vec<(Spec, SystemTime)>) -> Option<Spec> {
    if let Some(default) =
        default.filter(|default| downloaded.iter().any(|(spec, _)| spec == *default))
    {
        return Some(default.clone());
    }
    downloaded
        .into_iter()
        .max_by_key(|(_, at)| *at)
        .map(|(spec, _)| spec)
}

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

async fn downloads(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ApiResponse::success(json!({ "downloads": state.models.unfinished() }))
}

// the unfinished downloads, then every change from any client
async fn events(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // subscribe first so no change is missed, and list now, as the stream waits to be read
    let mut changes = state.models.changes.subscribe();
    let mut pending = state.models.unfinished();
    let stream = async_stream::stream! {
        loop {
            for change in std::mem::take(&mut pending) {
                yield Ok::<_, Infallible>(model_event(&change));
            }
            match changes.recv().await {
                Ok(change) => yield Ok(model_event(&change)),
                // some changes were dropped, so send everything again
                Err(RecvError::Lagged(_)) => pending = state.models.unfinished(),
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

// with no model, every bundled model
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

async fn start(
    State(state): State<Arc<AppState>>,
    ApiJson(which): ApiJson<Which>,
) -> Result<impl IntoResponse, AppError> {
    let download = state.models.get(which.model.as_deref())?;
    Ok(ApiResponse::success(download.start().await?))
}

async fn pause(
    State(state): State<Arc<AppState>>,
    ApiJson(which): ApiJson<Which>,
) -> Result<impl IntoResponse, AppError> {
    let download = state.models.get(which.model.as_deref())?;
    Ok(ApiResponse::success(download.pause().await?))
}

// only Tiles' own models, they come with a Modelfile
async fn select(
    State(state): State<Arc<AppState>>,
    ApiJson(which): ApiJson<Which>,
) -> Result<impl IntoResponse, AppError> {
    let download = state.models.get(which.model.as_deref())?;
    let spec = download.spec().clone();
    if bundled::modelfile_for(&state.models.modelfiles, &spec).is_none() {
        return Err(AppError::NotFound(format!(
            "Tiles has no Modelfile for {spec}"
        )));
    }
    if !download.status().await?.is_done() {
        return Err(AppError::Conflict(format!(
            "Download {spec} before using it"
        )));
    }
    state
        .models
        .set_current_model(&spec)
        .map_err(|err| AppError::InternalServerError(format!("{err:#}")))?;
    Ok(ApiResponse::success(json!({ "model": spec.to_string() })))
}

#[derive(Deserialize)]
struct DeleteRequest {
    model: Option<String>,
    // the model chats use too, after unloading it
    #[serde(default)]
    unload: bool,
}

// the model chats use or Pi runs only with `unload`, and chats move to
// `next_current_model` when the picked one goes
async fn delete(
    State(state): State<Arc<AppState>>,
    ApiJson(which): ApiJson<DeleteRequest>,
) -> Result<impl IntoResponse, AppError> {
    let download = state.models.get(which.model.as_deref())?;
    let spec = download.spec().clone();
    let quants = state.models.quants(&spec.repo);
    // before anything stops, as a running download refuses the delete
    let held = download.hold(&quants).await?;
    // quants of one repo share its folder
    let picked = state
        .models
        .current()
        .or_else(|| state.models.default.clone())
        .filter(|picked| picked.repo == spec.repo);
    let running = state
        .agent_model
        .lock()
        .expect("agent model lock")
        .as_deref()
        .and_then(|model| Spec::parse(model).ok())
        .filter(|running| running.repo == spec.repo);
    let Some(in_use) = picked.clone().or(running) else {
        let freed = download.delete(held).await?;
        return Ok(deleted(&state, &spec, freed));
    };
    if !which.unload {
        let hint = match picked {
            Some(_) => "pick another model or unload it",
            None => "unload it",
        };
        return Err(AppError::Conflict(format!(
            "Chats use {in_use}, {hint} before deleting {spec}"
        )));
    }

    // a reply holds the agent for as long as it streams. kept until the files
    // are gone, so no chat starts on this model
    let mut agent = state.agent.try_lock().map_err(|_| {
        AppError::Conflict("A reply is still streaming, wait for it to finish".to_owned())
    })?;
    let server = stop_pi_and_model_server(&state, &mut agent).await?;
    if picked.is_some() {
        match state.models.next_current_model(&spec) {
            Some(next) => state.models.set_current_model(&next),
            None => state.models.clear_current_model(),
        }
        .map_err(|err| {
            AppError::InternalServerError(format!("Couldn't pick another model: {err:#}"))
        })?;
    }
    let freed = download.delete(held).await;
    if server == Some(ServerState::Stopped)
        && let Err(err) = start_server_daemon().await
    {
        log::warn!("Couldn't start the model server again: {err:#}");
    }
    drop(agent);
    Ok(deleted(&state, &spec, freed?))
}

fn deleted(state: &AppState, spec: &Spec, freed: u64) -> impl IntoResponse + use<> {
    ApiResponse::success(json!({
        "model": spec.to_string(),
        "freed": freed,
        "current": state.models.current().map(|spec| spec.to_string()),
    }))
}

/// Stops Pi and the model server, both are set up for the model about to go.
/// The model server's state is `None` when this daemon doesn't run it
async fn stop_pi_and_model_server(
    state: &AppState,
    agent: &mut Option<PiAgent>,
) -> Result<Option<ServerState>, AppError> {
    if let Some(mut pi) = agent.take() {
        let _ = pi.process.kill().await;
    }
    *state.agent_model.lock().expect("agent model lock") = None;
    *state.resumed_chat.lock().expect("resumed chat lock") = None;
    if !state.can_stop_model_server {
        return Ok(None);
    }
    stop_server_daemon().await.map(Some).map_err(|err| {
        AppError::InternalServerError(format!("Couldn't stop the model server: {err:#}"))
    })
}

// closing it only pauses the download with `pause_on_close`
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
