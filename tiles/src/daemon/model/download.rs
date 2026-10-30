//! One model's download and its progress

use std::{
    convert::Infallible,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use axum::response::sse::Event;
use futures_util::Stream;
use nix::sys::statvfs::statvfs;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{
    runtime::Handle,
    sync::{Mutex as AsyncMutex, broadcast, watch},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use super::Spec;
use crate::{
    daemon::AppError,
    repl::resolve_gguf_path,
    utils::{
        config::get_model_cache,
        hf_model_downloader::{self as hf, DownloadEvent, DownloadOutcome, DownloadPlan, Hub},
    },
};

const UPDATE_EVERY: Duration = Duration::from_millis(250);
const SPEED_WINDOW: Duration = Duration::from_secs(1);

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

impl DownloadState {
    fn is_running(&self) -> bool {
        matches!(self, Self::Downloading { .. } | Self::Verifying { .. })
    }

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

type Running = Option<(CancellationToken, JoinHandle<()>)>;

pub struct ModelDownload {
    hub: Arc<Hub>,
    spec: Spec,
    root: PathBuf,
    spare_room: u64,
    pub(super) state: watch::Sender<DownloadState>,
    changes: broadcast::Sender<ModelChange>,
    plan: AsyncMutex<Option<DownloadPlan>>,
    running: AsyncMutex<Running>,
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
            state: watch::Sender::new(DownloadState::Idle),
            changes,
            plan: AsyncMutex::new(None),
            running: AsyncMutex::new(None),
        }
    }

    pub(super) fn change(&self) -> ModelChange {
        ModelChange {
            model: self.spec.to_string(),
            state: self.state.borrow().clone(),
        }
    }

    // only sends real changes, as status re-reads the same state on every call
    fn set(&self, state: DownloadState) {
        let changed = self.state.send_if_modified(|current| {
            let changed = *current != state;
            *current = state;
            changed
        });
        if changed {
            let _ = self.changes.send(self.change());
        }
    }

    // the same check `tiles run` makes before loading
    fn is_ready(&self) -> bool {
        get_model_cache(&self.spec.repo)
            .and_then(|dir| resolve_gguf_path(&dir, Some(&self.spec.quant)))
            .is_ok()
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
        let running = self.running.lock().await;
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
        let failed = matches!(*self.state.borrow(), DownloadState::Failed { .. });
        if !is_live(&running) && !failed {
            self.set(DownloadState::from_disk(on_disk, total));
        }
        Ok(ModelStatus {
            model: self.spec.to_string(),
            total: Some(total),
            on_disk: Some(on_disk),
            free_disk,
            download: self.state.borrow().clone(),
        })
    }

    pub async fn start(self: &Arc<Self>) -> Result<DownloadState, AppError> {
        let mut running = self.running.lock().await;
        if is_live(&running) {
            return Ok(self.state.borrow().clone());
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
        Ok(self.state.borrow().clone())
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

    /// Deletes the model's folder and returns the bytes freed
    pub async fn delete(&self) -> Result<u64, AppError> {
        // blocks a new download while the files are removed
        let running = self.running.lock().await;
        if is_live(&running) {
            return Err(AppError::Conflict(format!(
                "{} is downloading, pause it before deleting it",
                self.spec
            )));
        }
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
        self.set(DownloadState::Idle);
        Ok(freed)
    }

    pub async fn pause(&self) -> Result<DownloadState, AppError> {
        let mut running = self.running.lock().await;
        let Some((cancel, task)) = running.take().filter(|(_, task)| !task.is_finished()) else {
            return Err(AppError::NotFound(format!(
                "{} isn't downloading",
                self.spec
            )));
        };
        cancel.cancel();
        let _ = task.await;
        Ok(self.state.borrow().clone())
    }

    // with `pause_on_close`, closing the stream early pauses the download
    pub fn events(
        self: &Arc<Self>,
        pause_on_close: bool,
    ) -> impl Stream<Item = Result<Event, Infallible>> + use<> {
        let mut changes = self.state.subscribe();
        // made here, as the stream runs nothing until polled
        let mut guard = PauseOnClose(pause_on_close.then(|| Arc::clone(self)));
        async_stream::stream! {
            loop {
                let state = changes.borrow_and_update().clone();
                let running = state.is_running();
                yield Ok(Event::default().event("state").data(json!(state).to_string()));
                if !running || changes.changed().await.is_err() {
                    break;
                }
            }
            guard.disarm();
        }
    }
}

struct PauseOnClose(Option<Arc<ModelDownload>>);

impl PauseOnClose {
    fn disarm(&mut self) {
        self.0 = None;
    }
}

impl Drop for PauseOnClose {
    fn drop(&mut self) {
        if let Some(download) = self.0.take()
            && let Ok(runtime) = Handle::try_current()
        {
            runtime.spawn(async move {
                let _ = download.pause().await;
            });
        }
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

fn is_live(running: &Running) -> bool {
    running
        .as_ref()
        .is_some_and(|(_, task)| !task.is_finished())
}

fn free_space(path: &Path) -> nix::Result<u64> {
    let stat = statvfs(path)?;
    // the types differ between Linux and macOS
    #[allow(clippy::unnecessary_cast)]
    Ok(stat.blocks_available() as u64 * stat.fragment_size() as u64)
}

fn gb(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1e9)
}

// turns the downloader's events into states, a few a second
struct Progress {
    done: u64,
    total: u64,
    bytes_per_sec: u64,
    sample: (Instant, u64),
    sent: Instant,
}

impl Progress {
    fn new(done: u64, total: u64) -> Self {
        let now = Instant::now();
        Self {
            done,
            total,
            bytes_per_sec: 0,
            sample: (now, done),
            sent: now,
        }
    }

    fn update(&mut self, event: DownloadEvent) -> Option<DownloadState> {
        let now = Instant::now();
        match event {
            DownloadEvent::Verifying { .. } => {
                self.sent = now;
                Some(DownloadState::Verifying {
                    done: self.done,
                    total: self.total,
                })
            }
            DownloadEvent::Progress { done, .. } => {
                self.done = done;
                let (at, from) = self.sample;
                let elapsed = now - at;
                if elapsed >= SPEED_WINDOW {
                    self.bytes_per_sec =
                        (done.saturating_sub(from) as f64 / elapsed.as_secs_f64()) as u64;
                    self.sample = (now, done);
                }
                if now - self.sent < UPDATE_EVERY {
                    return None;
                }
                self.sent = now;
                Some(DownloadState::Downloading {
                    done,
                    total: self.total,
                    bytes_per_sec: self.bytes_per_sec,
                })
            }
        }
    }
}
