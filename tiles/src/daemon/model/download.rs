//! One model's download and its progress

use std::{
    convert::Infallible,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::response::sse::Event;
use futures_util::Stream;
use nix::sys::statvfs::statvfs;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as AsyncMutex, MutexGuard, broadcast};
use tokio_util::sync::CancellationToken;

use super::Spec;
use crate::{
    daemon::{
        AppError,
        job::{Job, JobState, Running, Speed, is_live},
    },
    repl::resolve_gguf_path,
    utils::{
        config::get_model_cache,
        hf_model_downloader::{self as hf, DownloadEvent, DownloadOutcome, DownloadPlan, Hub},
    },
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DownloadState {
    Idle,
    Downloading {
        done: u64,
        total: u64,
        bytes_per_sec: u64,
    },
    Verifying {
        done: u64,
        total: u64,
    },
    Paused {
        done: u64,
        total: u64,
    },
    Done,
    Failed {
        reason: String,
    },
}

impl JobState for DownloadState {
    fn is_running(&self) -> bool {
        matches!(self, Self::Downloading { .. } | Self::Verifying { .. })
    }
}

impl DownloadState {
    pub(super) fn is_unfinished(&self) -> bool {
        !matches!(self, Self::Idle | Self::Done)
    }

    fn from_disk(done: u64, total: u64) -> Self {
        match done {
            0 => Self::Idle,
            done if done >= total => Self::Done,
            done => Self::Paused { done, total },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelChange {
    model: String,
    #[serde(flatten)]
    pub(super) state: DownloadState,
}

#[derive(Debug, Serialize)]
pub struct ModelStatus {
    model: String,
    // left out when the model is already there
    total: Option<u64>,
    on_disk: Option<u64>,
    free_disk: Option<u64>,
    download: DownloadState,
}

impl ModelStatus {
    pub(super) fn is_done(&self) -> bool {
        matches!(self.download, DownloadState::Done)
    }
}

pub struct ModelDownload {
    hub: Arc<Hub>,
    spec: Spec,
    root: PathBuf,
    spare_room: u64,
    pub(super) job: Arc<Job<DownloadState>>,
    changes: broadcast::Sender<ModelChange>,
    plan: AsyncMutex<Option<DownloadPlan>>,
}

/// A repo's downloads, kept from starting while its files go
pub struct Held<'q> {
    quants: &'q [Arc<ModelDownload>],
    _running: Vec<MutexGuard<'q, Running>>,
}

impl ModelDownload {
    pub(super) fn spec(&self) -> &Spec {
        &self.spec
    }

    pub(super) fn new(
        hub: Arc<Hub>,
        spec: Spec,
        root: PathBuf,
        spare_room: u64,
        changes: broadcast::Sender<ModelChange>,
    ) -> Self {
        Self {
            hub,
            spec,
            root,
            spare_room,
            job: Job::new(DownloadState::Idle),
            changes,
            plan: AsyncMutex::new(None),
        }
    }

    pub(super) fn change(&self) -> ModelChange {
        ModelChange {
            model: self.spec.to_string(),
            state: self.job.state(),
        }
    }

    // only sends real changes, as status re-reads the same state on every call
    fn set(&self, state: DownloadState) {
        if self.job.set(state) {
            let _ = self.changes.send(self.change());
        }
    }

    // the same check `tiles run` makes before loading
    fn is_ready(&self) -> bool {
        get_model_cache(&self.spec.repo)
            .and_then(|dir| resolve_gguf_path(&dir, Some(&self.spec.quant)))
            .is_ok()
    }

    /// When its GGUF landed on disk, `None` until it's downloaded
    pub(super) fn downloaded_at(&self) -> Option<std::time::SystemTime> {
        let gguf = get_model_cache(&self.spec.repo)
            .and_then(|dir| resolve_gguf_path(&dir, Some(&self.spec.quant)))
            .ok()?;
        std::fs::metadata(gguf)
            .and_then(|meta| meta.modified())
            .ok()
    }

    async fn plan(&self) -> Result<DownloadPlan, AppError> {
        let mut cached = self.plan.lock().await;
        if let Some(plan) = cached.as_ref() {
            return Ok(plan.clone());
        }
        let plan = hf::plan(&self.hub, &self.spec.repo, Some(&self.spec.quant))
            .await
            .map_err(|err| {
                // not reaching the Hub is worth a retry
                let reason = format!("{err:#}");
                if err.downcast_ref::<reqwest::Error>().is_some() {
                    AppError::ServiceUnavailable(reason)
                } else {
                    AppError::CannotProcess(reason)
                }
            })?;
        Ok(cached.insert(plan).clone())
    }

    pub async fn status(&self) -> Result<ModelStatus, AppError> {
        // held so a starting download isn't overwritten
        let running = self.job.lock().await;
        let free_disk = free_space(&self.root).ok();
        if !is_live(&running) && self.is_ready() {
            self.set(DownloadState::Done);
            return Ok(ModelStatus {
                model: self.spec.to_string(),
                total: None,
                on_disk: None,
                free_disk,
                download: DownloadState::Done,
            });
        }

        let plan = self.plan().await?;
        let (total, on_disk) = (plan.total(), hf::on_disk(&plan, &self.root).await);
        // a failure stays until the next start
        let failed = matches!(self.job.state(), DownloadState::Failed { .. });
        if !is_live(&running) && !failed {
            self.set(DownloadState::from_disk(on_disk, total));
        }
        Ok(ModelStatus {
            model: self.spec.to_string(),
            total: Some(total),
            on_disk: Some(on_disk),
            free_disk,
            download: self.job.state(),
        })
    }

    pub async fn start(self: &Arc<Self>) -> Result<DownloadState, AppError> {
        let mut running = self.job.lock().await;
        if is_live(&running) {
            return Ok(self.job.state());
        }

        let plan = self.plan().await?;
        let (total, done) = (plan.total(), hf::on_disk(&plan, &self.root).await);
        let needed = total - done + self.spare_room;
        if done < total
            && let Ok(free) = free_space(&self.root)
            && free < needed
        {
            return Err(AppError::InsufficientStorage(format!(
                "The model needs {} GB more free disk space, and {} GB is free",
                gb(needed),
                gb(free)
            )));
        }

        log::info!("Downloading {}", self.spec);
        let cancel = CancellationToken::new();
        self.set(DownloadState::Downloading {
            done,
            total,
            bytes_per_sec: 0,
        });
        let task = tokio::spawn(Arc::clone(self).run(plan, done, cancel.clone()));
        *running = Some((cancel, task));
        Ok(self.job.state())
    }

    async fn run(self: Arc<Self>, plan: DownloadPlan, done: u64, cancel: CancellationToken) {
        let total = plan.total();
        let mut progress = Progress::new(done, total);
        let outcome = hf::download(&self.hub, &plan, &self.root, &cancel, |event| {
            if let Some(state) = progress.update(event) {
                self.set(state);
            }
        })
        .await;

        let state = match outcome {
            Ok(DownloadOutcome::Done) => DownloadState::Done,
            Ok(DownloadOutcome::Paused) => DownloadState::Paused {
                done: hf::on_disk(&plan, &self.root).await,
                total,
            },
            Err(err) => DownloadState::Failed {
                reason: format!("{err:#}"),
            },
        };
        log::info!("Model download for {}: {state:?}", self.spec);
        self.set(state);
    }

    /// Deletes the repo's folder, shared by all its quants, and returns the bytes freed.
    /// `quants` lists every quant, this one included
    /// Blocks the downloads of `quants` until the files are deleted, refused
    /// while one of them runs
    pub async fn hold<'q>(&self, quants: &'q [Arc<ModelDownload>]) -> Result<Held<'q>, AppError> {
        let mut running = Vec::with_capacity(quants.len());
        for quant in quants {
            let job = quant.job.lock().await;
            if is_live(&job) {
                return Err(AppError::Conflict(format!(
                    "{} is downloading, pause it before deleting {}",
                    quant.spec, self.spec
                )));
            }
            running.push(job);
        }
        Ok(Held {
            quants,
            _running: running,
        })
    }

    pub async fn delete(&self, held: Held<'_>) -> Result<u64, AppError> {
        let dir = hf::repo_dir(&self.root, &self.spec.repo);
        let freed = tokio::task::spawn_blocking(move || {
            let freed = size_of(&dir);
            std::fs::remove_dir_all(&dir).map(|()| freed)
        })
        .await
        .map_err(|err| AppError::InternalServerError(err.to_string()))?
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => {
                AppError::NotFound(format!("{} isn't on this computer", self.spec))
            }
            _ => AppError::InternalServerError(format!("Couldn't delete {}: {err}", self.spec)),
        })?;
        for quant in held.quants {
            quant.set(DownloadState::Idle);
        }
        Ok(freed)
    }

    pub async fn pause(&self) -> Result<DownloadState, AppError> {
        self.job
            .pause()
            .await
            .ok_or_else(|| AppError::NotFound(format!("{} isn't downloading", self.spec)))
    }

    // with `pause_on_close`, closing the stream early pauses the download
    pub fn events(
        &self,
        pause_on_close: bool,
    ) -> impl Stream<Item = Result<Event, Infallible>> + use<> {
        self.job.events(pause_on_close)
    }
}

// size of the files under `path`, skipping the snapshot links to blobs
fn size_of(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => size_of(&entry.path()),
            Ok(kind) if kind.is_file() => entry.metadata().map_or(0, |meta| meta.len()),
            _ => 0,
        })
        .sum()
}

pub(crate) fn free_space(path: &Path) -> nix::Result<u64> {
    let stat = statvfs(path)?;
    // the types differ between Linux and macOS
    #[allow(clippy::unnecessary_cast)]
    Ok(stat.blocks_available() as u64 * stat.fragment_size() as u64)
}

pub(crate) fn gb(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1e9)
}

// turns the downloader's events into states, a few a second
struct Progress {
    done: u64,
    total: u64,
    speed: Speed,
}

impl Progress {
    fn new(done: u64, total: u64) -> Self {
        Self {
            done,
            total,
            speed: Speed::new(),
        }
    }

    fn update(&mut self, event: DownloadEvent) -> Option<DownloadState> {
        match event {
            DownloadEvent::Verifying { .. } => {
                self.speed.hold();
                Some(DownloadState::Verifying {
                    done: self.done,
                    total: self.total,
                })
            }
            DownloadEvent::Progress { done, .. } => {
                self.done = done;
                let bytes_per_sec = self.speed.update(done)?;
                Some(DownloadState::Downloading {
                    done,
                    total: self.total,
                    bytes_per_sec,
                })
            }
        }
    }
}
