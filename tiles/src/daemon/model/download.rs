//! One model's download: its state, the task running it, pause, and the progress stream

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
    sync::{Mutex as AsyncMutex, watch},
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

// how often progress goes out, and how far back the speed looks
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
    /// Hashing the part already downloaded, before continuing it
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

    // what the files on disk say, when nothing is running
    fn from_disk(done: u64, total: u64) -> Self {
        match done {
            0 => Self::Idle,
            done if done >= total => Self::Done,
            done => Self::Paused { done, total },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ModelStatus {
    /// Like `unsloth/gemma-4-12b-it-GGUF:Q4_K_M`
    model: String,
    /// The sizes are left out when the model is already there, as Hugging Face isn't asked
    total: Option<u64>,
    on_disk: Option<u64>,
    free_disk: Option<u64>,
    download: DownloadState,
}

type Running = Option<(CancellationToken, JoinHandle<()>)>;

/// One model's download, shared by every client that asks for that model
pub struct ModelDownload {
    hub: Arc<Hub>,
    spec: Spec,
    /// The models folder
    root: PathBuf,
    spare_room: u64,
    pub(super) state: watch::Sender<DownloadState>,
    /// The files to download, asked from Hugging Face once
    plan: AsyncMutex<Option<DownloadPlan>>,
    running: AsyncMutex<Running>,
}

impl ModelDownload {
    pub(super) fn new(hub: Arc<Hub>, spec: Spec, root: PathBuf, spare_room: u64) -> Self {
        Self {
            hub,
            spec,
            root,
            spare_room,
            state: watch::Sender::new(DownloadState::Idle),
            plan: AsyncMutex::new(None),
            running: AsyncMutex::new(None),
        }
    }

    // the check `tiles run` makes before loading, which also finds a model bundled in
    // the lib dir or left in the old huggingface cache
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
                // not reaching the Hub is worth a retry, a missing model or quant isn't
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
        // held so a download starting meanwhile can't be overwritten with what's on disk
        let running = self.running.lock().await;
        let free_disk = free_space(&self.root).ok();
        if !is_live(&running) && self.is_ready() {
            self.state.send_replace(DownloadState::Done);
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
        // a failure stays until the next start, so the UI can show why
        let failed = matches!(*self.state.borrow(), DownloadState::Failed { .. });
        if !is_live(&running) && !failed {
            self.state
                .send_replace(DownloadState::from_disk(on_disk, total));
        }
        Ok(ModelStatus {
            model: self.spec.to_string(),
            total: Some(total),
            on_disk: Some(on_disk),
            free_disk,
            download: self.state.borrow().clone(),
        })
    }

    /// Starts or resumes the download. One already running is left as it is
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
        self.state.send_replace(DownloadState::Downloading {
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
                self.state.send_replace(state);
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
        self.state.send_replace(state);
    }

    /// Stops the download and keeps what it got. Answers once it has stopped
    pub async fn pause(&self) -> Result<DownloadState, AppError> {
        let mut running = self.running.lock().await;
        let Some((cancel, task)) = running.take().filter(|(_, task)| !task.is_finished()) else {
            return Err(AppError::NotFound(format!(
                "{} isn't downloading",
                self.spec
            )));
        };
        cancel.cancel();
        // the task writes the paused state, or done if it finished first
        let _ = task.await;
        Ok(self.state.borrow().clone())
    }

    /// The current state, then each change, until the download stops. With
    /// `pause_on_close`, a client that goes away early pauses the download, which is
    /// how Ctrl-C in the CLI pauses it
    pub fn events(
        self: &Arc<Self>,
        pause_on_close: bool,
    ) -> impl Stream<Item = Result<Event, Infallible>> + use<> {
        let mut changes = self.state.subscribe();
        // made here, not in the stream, which runs nothing until it is polled
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
            // it ran to the end, so there's nothing to pause
            guard.disarm();
        }
    }
}

/// Pauses the download when dropped, unless disarmed first
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

fn is_live(running: &Running) -> bool {
    running
        .as_ref()
        .is_some_and(|(_, task)| !task.is_finished())
}

fn free_space(path: &Path) -> nix::Result<u64> {
    let stat = statvfs(path)?;
    // the field types differ between Linux and macOS
    #[allow(clippy::unnecessary_cast)]
    Ok(stat.blocks_available() as u64 * stat.fragment_size() as u64)
}

fn gb(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1e9)
}

/// Turns the downloader's events into states, at most a few a second, with the
/// speed over about the last second
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
