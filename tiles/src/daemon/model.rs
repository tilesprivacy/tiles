//! Which models onboarding can offer, how far each is downloaded, which one
//! fits this machine, and downloading one
//!
//! Sizes and file hashes come from the hub, memory needs and free device
//! memory from the inference server, download progress straight off disk.
//! One download runs at a time; asking for the same model again joins it.

use std::{
    convert::Infallible,
    sync::{Arc, LazyLock},
};

use axum::{
    Json, Router,
    response::{
        IntoResponse, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use futures_util::{StreamExt, future::join_all};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, watch};
use tokio_stream::wrappers::WatchStream;
use tokio_util::sync::CancellationToken;

use crate::{
    core::{
        download::{self, Progress},
        models::{Candidate, Fit, LINEUP, Need, fit, recommend},
        server,
    },
    daemon::{ApiResponse, AppError, AppState},
    utils::{
        config::get_config_json,
        disk::model_volume_space,
        hf_model_downloader::{Downloaded, downloaded, repo_files},
    },
};

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum State {
    Ready,
    Partial,
    Missing,
    /// the hub could not be reached, so nothing is known about the files
    Unknown,
}

#[derive(Serialize)]
struct ModelEntry {
    id: &'static str,
    label: &'static str,
    spec: String,
    state: State,
    download_bytes: Option<u64>,
    downloaded_bytes: u64,
    vram_bytes: Option<u64>,
    fit: Fit,
    recommended: bool,
    active: bool,
}

#[derive(Serialize)]
struct Device {
    name: String,
    free_bytes: u64,
    total_bytes: u64,
}

#[derive(Serialize)]
struct Space {
    free_bytes: u64,
    total_bytes: u64,
}

#[derive(Serialize)]
struct Status {
    models: Vec<ModelEntry>,
    /// the device models are sized against, none when only the cpu can run them
    device: Option<Device>,
    disk: Option<Space>,
}

pub fn model_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/model/status", get(model_status))
        .route(
            "/v1/tilekit/model/download",
            get(download_state)
                .post(start_download)
                .delete(cancel_download),
        )
}

struct Job {
    progress: watch::Receiver<Progress>,
    cancel: CancellationToken,
}

impl Job {
    fn running(&self) -> bool {
        !self.progress.borrow().phase.is_final()
    }
}

static JOB: LazyLock<Mutex<Option<Job>>> = LazyLock::new(|| Mutex::new(None));

#[derive(Deserialize)]
struct DownloadRequest {
    /// `repo:quant`, as in a modelfile's FROM
    spec: String,
}

/// Starts downloading `spec`, or joins the download already running for it,
/// and streams its progress until it is done, cancelled or failed.
async fn start_download(
    Json(request): Json<DownloadRequest>,
) -> Result<impl IntoResponse, AppError> {
    let spec = request.spec.trim().to_owned();
    let (repo, quant) = tilekit::modelfile::split_model_spec(&spec);
    if !repo.contains('/') {
        return Err(AppError::BadRequest(format!(
            "{spec} is not a hugging face repo:quant"
        )));
    }

    let mut job = JOB.lock().await;
    let progress = match job.as_ref() {
        Some(current) if current.running() => {
            let running = current.progress.borrow().spec.clone();
            if running != spec {
                return Err(AppError::AlreadyExists(format!(
                    "{running} is downloading. Cancel it first."
                )));
            }
            current.progress.clone()
        }
        _ => {
            let (tx, rx) = watch::channel(Progress::new(&spec));
            let cancel = CancellationToken::new();
            let (repo, quant, token) = (repo.to_owned(), quant.map(str::to_owned), cancel.clone());
            tokio::spawn(async move {
                download::download(&repo, quant.as_deref(), tx, token).await;
            });
            log::info!("Downloading {spec}");
            *job = Some(Job {
                progress: rx.clone(),
                cancel,
            });
            rx
        }
    };
    drop(job);

    // the watch only holds the latest state, so a slow reader skips ahead
    // rather than falling behind; the final state is always delivered
    let events = WatchStream::new(progress)
        .scan(false, |ended, state| {
            let send = !*ended;
            *ended = state.phase.is_final();
            futures_util::future::ready(send.then_some(state))
        })
        .map(|state| {
            if state.phase.is_final() {
                log::info!("Download of {} ended: {:?}", state.spec, state.phase);
            }
            Ok::<_, Infallible>(Event::default().json_data(state).unwrap_or_default())
        });
    Ok(Sse::new(events).keep_alive(KeepAlive::default()))
}

/// Where the current or last download is, for a client that was not watching.
async fn download_state() -> Json<ApiResponse<Option<Progress>>> {
    let job = JOB.lock().await;
    ApiResponse::success(job.as_ref().map(|job| job.progress.borrow().clone()))
}

/// Stops the running download. What arrived stays on disk and a later
/// download of the same model continues from there.
async fn cancel_download() -> Result<Json<ApiResponse<Progress>>, AppError> {
    let guard = JOB.lock().await;
    let Some(job) = guard.as_ref().filter(|job| job.running()) else {
        return Err(AppError::NotFound("No download is running".to_owned()));
    };
    job.cancel.cancel();
    let mut progress = job.progress.clone();
    // not held while waiting, a new download may start meanwhile
    drop(guard);
    let _ = progress.wait_for(|state| state.phase.is_final()).await;
    let state = progress.borrow().clone();
    Ok(ApiResponse::success(state))
}

/// the gpu with the most free memory, leaving out integrated ones whose "free"
/// memory is system ram; the ram itself when there is no such gpu
fn best_device(hardware: &serde_json::Value) -> (Option<Device>, u64) {
    let device = hardware["devices"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|device| !device["integrated"].as_bool().unwrap_or(false))
        .max_by_key(|device| {
            device["free_bytes"].as_u64().unwrap_or(0)
                + device["reclaimable_bytes"].as_u64().unwrap_or(0)
        });
    match device {
        Some(device) => {
            // memory our own loaded model holds comes back on a switch
            let free = device["free_bytes"].as_u64().unwrap_or(0)
                + device["reclaimable_bytes"].as_u64().unwrap_or(0);
            let device = Device {
                name: device["name"].as_str().unwrap_or_default().to_owned(),
                free_bytes: free,
                total_bytes: device["total_bytes"].as_u64().unwrap_or(0),
            };
            (Some(device), free)
        }
        None => (
            None,
            hardware["memory"]["available_bytes"].as_u64().unwrap_or(0),
        ),
    }
}

/// what the hub and the inference server say about one candidate
struct Probe {
    state: State,
    download_bytes: Option<u64>,
    downloaded_bytes: u64,
    need: Option<Need>,
}

async fn probe(candidate: &Candidate) -> Probe {
    let Ok(files) = repo_files(candidate.repo, Some(candidate.quant)).await else {
        return Probe {
            state: State::Unknown,
            download_bytes: None,
            downloaded_bytes: 0,
            need: None,
        };
    };
    let total = files.iter().map(|file| file.size).sum();
    let Downloaded { bytes, complete } = downloaded(candidate.repo, &files).unwrap_or(Downloaded {
        bytes: 0,
        complete: false,
    });
    let state = match (complete, bytes) {
        (true, _) => State::Ready,
        (false, 0) => State::Missing,
        (false, _) => State::Partial,
    };

    let need = match files.iter().find(|file| file.is_main_gguf()) {
        Some(main) => {
            let url = format!(
                "https://huggingface.co/{}/resolve/main/{}",
                candidate.repo, main.name
            );
            server::estimate(&url, main.size)
                .await
                .inspect_err(|err| log::warn!("No estimate for {}: {err:#}", candidate.id))
                .ok()
        }
        None => None,
    };

    Probe {
        state,
        download_bytes: Some(total),
        downloaded_bytes: bytes,
        need,
    }
}

async fn model_status() -> Result<Json<ApiResponse<Status>>, AppError> {
    // the estimate and the device list both live in the inference server
    server::ensure_up()
        .await
        .map_err(|err| AppError::InternalServerError(format!("{err:#}")))?;
    let hardware = server::hardware()
        .await
        .inspect_err(|err| log::warn!("No hardware report: {err:#}"))
        .unwrap_or_default();
    let (device, free) = best_device(&hardware);

    let probes = join_all(LINEUP.iter().map(probe)).await;
    let fits: Vec<Fit> = probes.iter().map(|probe| fit(probe.need, free)).collect();
    let recommended = recommend(&fits);
    let active = get_config_json()
        .ok()
        .and_then(|config| config["model"]["current"].as_str().map(str::to_owned))
        .unwrap_or_default();

    let models = LINEUP
        .iter()
        .zip(probes)
        .zip(fits)
        .enumerate()
        .map(|(index, ((candidate, probe), fit))| ModelEntry {
            id: candidate.id,
            label: candidate.label,
            active: candidate.spec() == active,
            spec: candidate.spec(),
            state: probe.state,
            download_bytes: probe.download_bytes,
            downloaded_bytes: probe.downloaded_bytes,
            vram_bytes: probe.need.map(|need| need.vram_bytes),
            fit,
            recommended: index == recommended,
        })
        .collect();

    Ok(ApiResponse::success(Status {
        models,
        device,
        disk: model_volume_space().map(|(free_bytes, total_bytes)| Space {
            free_bytes,
            total_bytes,
        }),
    }))
}
