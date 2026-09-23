//! Which models onboarding can offer, how far each is downloaded, and which
//! one fits this machine
//!
//! Sizes and file hashes come from the hub, memory needs and free device
//! memory from the inference server, download progress straight off disk.

use std::sync::Arc;

use axum::{Json, Router, routing::get};
use futures_util::future::join_all;
use serde::Serialize;

use crate::{
    core::{
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
    Router::new().route("/v1/tilekit/model/status", get(model_status))
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
