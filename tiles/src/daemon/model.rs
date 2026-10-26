//! Downloads the default model with progress, pause and resume, for the UI

use std::{
    convert::Infallible,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use axum::{
    Router,
    extract::State,
    response::{
        IntoResponse, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use futures_util::Stream;
use nix::sys::statvfs::statvfs;
use serde::Serialize;
use serde_json::json;
use tokio::{
    sync::{Mutex as AsyncMutex, watch},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::{
    daemon::{ApiResponse, AppError, AppState},
    repl::{get_default_modelfile, resolve_gguf_path},
    utils::{
        config::{DefaultProvider, get_model_cache, get_or_create_model_download_path},
        hf_model_downloader::{
            self as hf, DEFAULT_QUANT, DownloadEvent, DownloadOutcome, DownloadPlan, Hub,
        },
    },
};

// left free after a download, so it never fills the disk to the last byte
const SPARE_ROOM: u64 = 1_000_000_000;
// how often progress goes out, and how far back the speed looks
const UPDATE_EVERY: Duration = Duration::from_millis(250);
const SPEED_WINDOW: Duration = Duration::from_secs(1);

pub fn model_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/model/status", get(status))
        .route(
            "/v1/tilekit/model/download",
            get(progress).post(start).delete(pause),
        )
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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

/// The default model's download. One runs at a time, shared by every client
pub struct ModelDownload {
    hub: Hub,
    repo: String,
    quant: Option<String>,
    /// The models folder
    root: PathBuf,
    spare_room: u64,
    state: watch::Sender<DownloadState>,
    /// The files to download, asked from Hugging Face once
    plan: AsyncMutex<Option<DownloadPlan>>,
    running: AsyncMutex<Running>,
}

impl ModelDownload {
    pub fn new(hub: Hub, repo: String, quant: Option<String>, root: PathBuf) -> Self {
        Self {
            hub,
            repo,
            quant,
            root,
            spare_room: SPARE_ROOM,
            state: watch::Sender::new(DownloadState::Idle),
            plan: AsyncMutex::new(None),
            running: AsyncMutex::new(None),
        }
    }

    /// The model in the default Modelfile, downloaded into the models folder
    pub fn for_default_model() -> anyhow::Result<Self> {
        let path = get_default_modelfile(DefaultProvider)?;
        let modelfile = tilekit::modelfile::parse_from_file(&path.to_string_lossy())
            .map_err(anyhow::Error::msg)?;
        let repo = modelfile
            .from
            .context("The default Modelfile has no FROM")?;
        Ok(Self::new(
            Hub::from_env()?,
            repo,
            modelfile.quant,
            get_or_create_model_download_path()?,
        ))
    }

    fn spec(&self) -> String {
        format!("{}:{}", self.repo, self.quant())
    }

    fn quant(&self) -> &str {
        self.quant.as_deref().unwrap_or(DEFAULT_QUANT)
    }

    // the check `tiles run` makes before loading, which also finds a model bundled in
    // the lib dir or left in the old huggingface cache
    fn is_ready(&self) -> bool {
        get_model_cache(&self.repo)
            .and_then(|dir| resolve_gguf_path(&dir, Some(self.quant())))
            .is_ok()
    }

    async fn plan(&self) -> Result<DownloadPlan, AppError> {
        let mut cached = self.plan.lock().await;
        if let Some(plan) = cached.as_ref() {
            return Ok(plan.clone());
        }
        let plan = hf::plan(&self.hub, &self.repo, Some(self.quant()))
            .await
            .map_err(|err| AppError::ServiceUnavailable(format!("{err:#}")))?;
        Ok(cached.insert(plan).clone())
    }

    pub async fn status(&self) -> Result<ModelStatus, AppError> {
        // held so a download starting meanwhile can't be overwritten with what's on disk
        let running = self.running.lock().await;
        let free_disk = free_space(&self.root).ok();
        if !is_live(&running) && self.is_ready() {
            self.state.send_replace(DownloadState::Done);
            return Ok(ModelStatus {
                model: self.spec(),
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
            model: self.spec(),
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

        log::info!("Downloading {}", self.spec());
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
        log::info!("Model download for {}: {state:?}", self.spec());
        self.state.send_replace(state);
    }

    /// Stops the download and keeps what it got. Answers once it has stopped
    pub async fn pause(&self) -> Result<DownloadState, AppError> {
        let mut running = self.running.lock().await;
        let Some((cancel, task)) = running.take().filter(|(_, task)| !task.is_finished()) else {
            return Err(AppError::NotFound(
                "No model download is running".to_owned(),
            ));
        };
        cancel.cancel();
        // the task writes the paused state, or done if it finished first
        let _ = task.await;
        Ok(self.state.borrow().clone())
    }

    /// The current state, then each change, until the download stops
    pub fn events(&self) -> impl Stream<Item = Result<Event, Infallible>> + use<> {
        let mut changes = self.state.subscribe();
        async_stream::stream! {
            loop {
                let state = changes.borrow_and_update().clone();
                let running = state.is_running();
                yield Ok(Event::default().event("state").data(json!(state).to_string()));
                if !running || changes.changed().await.is_err() {
                    break;
                }
            }
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

fn model(state: &AppState) -> Result<&Arc<ModelDownload>, AppError> {
    state
        .model
        .as_ref()
        .ok_or_else(|| AppError::NotFound("No default model is set up".to_owned()))
}

/// `GET /v1/tilekit/model/status`
async fn status(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    Ok(ApiResponse::success(model(&state)?.status().await?))
}

/// `POST /v1/tilekit/model/download`
async fn start(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    Ok(ApiResponse::success(model(&state)?.start().await?))
}

/// `DELETE /v1/tilekit/model/download`
async fn pause(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    Ok(ApiResponse::success(model(&state)?.pause().await?))
}

/// `GET /v1/tilekit/model/download`, as server-sent events. Closing it doesn't stop
/// the download
async fn progress(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    Ok(Sse::new(model(&state)?.events()).keep_alive(KeepAlive::default()))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    use tempfile::{TempDir, tempdir};
    use tower::ServiceExt;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;

    const REPO: &str = "org/model-GGUF";
    const COMMIT: &str = "c0ffee";
    const MODEL: &[u8] = b"GGUF the model weights";
    const CONFIG: &[u8] = b"{}";

    fn model_sha256() -> String {
        format!("{:x}", Sha256::digest(MODEL))
    }

    /// A Hub with one small model, its GGUF answered after `delay`
    async fn hub_with_model(delay: Duration) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!("/api/models/{REPO}/revision/main")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "sha": COMMIT,
                "siblings": [
                    { "rfilename": "config.json", "size": CONFIG.len(), "blobId": "a-git-id" },
                    {
                        "rfilename": "model-Q4_K_M.gguf",
                        "size": MODEL.len(),
                        "blobId": "another-git-id",
                        "lfs": { "sha256": model_sha256(), "size": MODEL.len() },
                    },
                ],
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/{REPO}/resolve/{COMMIT}/config.json")))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(CONFIG))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/{REPO}/resolve/{COMMIT}/model-Q4_K_M.gguf")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(MODEL)
                    .set_delay(delay),
            )
            .mount(&server)
            .await;
        server
    }

    struct Daemon {
        state: Arc<AppState>,
        root: TempDir,
    }

    impl Daemon {
        fn new(server: &MockServer) -> Self {
            Self::with_root(server, tempdir().unwrap())
        }

        fn with_root(server: &MockServer, root: TempDir) -> Self {
            let hub = Hub::new(&server.uri(), None).unwrap();
            let download = ModelDownload::new(
                hub,
                REPO.to_owned(),
                Some("Q4_K_M".to_owned()),
                root.path().to_path_buf(),
            );
            let mut state = AppState::for_tests();
            state.model = Some(Arc::new(download));
            Self {
                state: Arc::new(state),
                root,
            }
        }

        fn download(&self) -> &Arc<ModelDownload> {
            self.state.model.as_ref().unwrap()
        }

        async fn call(&self, method: &str, uri: &str) -> (StatusCode, String) {
            let req = Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap();
            let res = model_router()
                .with_state(self.state.clone())
                .oneshot(req)
                .await
                .unwrap();
            let status = res.status();
            let body = res.into_body().collect().await.unwrap().to_bytes();
            (status, String::from_utf8(body.to_vec()).unwrap())
        }

        async fn json(&self, method: &str, uri: &str) -> (StatusCode, Value) {
            let (status, body) = self.call(method, uri).await;
            (status, serde_json::from_str(&body).unwrap())
        }

        async fn until(&self, done: impl FnMut(&DownloadState) -> bool) {
            let mut changes = self.download().state.subscribe();
            tokio::time::timeout(Duration::from_secs(5), changes.wait_for(done))
                .await
                .expect("the download never got there")
                .unwrap();
        }
    }

    #[tokio::test]
    async fn a_second_start_joins_the_running_download() {
        let server = hub_with_model(Duration::from_millis(300)).await;
        let daemon = Daemon::new(&server);

        let (status, first) = daemon.json("POST", "/v1/tilekit/model/download").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first["data"]["state"], "downloading");
        let (_, second) = daemon.json("POST", "/v1/tilekit/model/download").await;
        assert_eq!(second["data"]["state"], "downloading");

        daemon.until(|state| *state == DownloadState::Done).await;
        let gguf_requests = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|req| req.url.path().ends_with(".gguf"))
            .count();
        assert_eq!(gguf_requests, 1);
    }

    #[tokio::test]
    async fn pausing_keeps_what_was_downloaded() {
        let server = hub_with_model(Duration::from_secs(10)).await;
        let daemon = Daemon::new(&server);
        daemon.json("POST", "/v1/tilekit/model/download").await;
        // the config comes at once, the model never does
        tokio::time::sleep(Duration::from_millis(200)).await;

        let (status, paused) = daemon.json("DELETE", "/v1/tilekit/model/download").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(paused["data"]["state"], "paused");
        assert_eq!(paused["data"]["done"], CONFIG.len());
        assert_eq!(paused["data"]["total"], CONFIG.len() + MODEL.len());

        let (status, _) = daemon.json("DELETE", "/v1/tilekit/model/download").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn status_after_a_restart_shows_a_part_download_as_paused() {
        let server = hub_with_model(Duration::ZERO).await;
        let root = tempdir().unwrap();
        let blobs = hf::repo_dir(root.path(), REPO).join("blobs");
        std::fs::create_dir_all(&blobs).unwrap();
        std::fs::write(blobs.join(format!("{}.part", model_sha256())), &MODEL[..5]).unwrap();
        let daemon = Daemon::with_root(&server, root);

        let (status, body) = daemon.json("GET", "/v1/tilekit/model/status").await;

        assert_eq!(status, StatusCode::OK);
        let data = &body["data"];
        assert_eq!(data["model"], "org/model-GGUF:Q4_K_M");
        assert_eq!(data["on_disk"], 5);
        assert_eq!(
            data["download"],
            json!({ "state": "paused", "done": 5, "total": CONFIG.len() + MODEL.len() })
        );
        assert!(data["free_disk"].as_u64().is_some_and(|free| free > 0));
    }

    #[tokio::test]
    async fn the_progress_stream_follows_the_download_to_the_end() {
        let server = hub_with_model(Duration::from_millis(300)).await;
        let daemon = Daemon::new(&server);
        daemon.json("POST", "/v1/tilekit/model/download").await;

        let (status, events) = daemon.call("GET", "/v1/tilekit/model/download").await;

        assert_eq!(status, StatusCode::OK);
        let states: Vec<Value> = events
            .lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .map(|data| serde_json::from_str(data).unwrap())
            .collect();
        assert_eq!(states.first().unwrap()["state"], "downloading");
        assert_eq!(states.last().unwrap(), &json!({ "state": "done" }));

        // a client that comes later gets where it ended, and the stream closes
        let (_, late) = daemon.call("GET", "/v1/tilekit/model/download").await;
        assert_eq!(late.matches("data: ").count(), 1);
        assert!(late.contains(r#"{"state":"done"}"#));
    }

    #[tokio::test]
    async fn a_download_too_big_for_the_disk_is_refused() {
        let server = hub_with_model(Duration::ZERO).await;
        let root = tempdir().unwrap();
        let hub = Hub::new(&server.uri(), None).unwrap();
        let mut download = ModelDownload::new(
            hub,
            REPO.to_owned(),
            Some("Q4_K_M".to_owned()),
            root.path().to_path_buf(),
        );
        download.spare_room = u64::MAX / 2;
        let mut state = AppState::for_tests();
        state.model = Some(Arc::new(download));
        let daemon = Daemon {
            state: Arc::new(state),
            root,
        };

        let (status, body) = daemon.json("POST", "/v1/tilekit/model/download").await;

        assert_eq!(status, StatusCode::INSUFFICIENT_STORAGE);
        assert!(body["reason"].as_str().unwrap().contains("free disk space"));
        assert_eq!(*daemon.download().state.borrow(), DownloadState::Idle);
        assert!(!hf::repo_dir(daemon.root.path(), REPO).exists());
    }

    #[tokio::test]
    async fn no_default_model_is_a_404() {
        let state = Arc::new(AppState::for_tests());
        let req = Request::builder()
            .uri("/v1/tilekit/model/status")
            .body(Body::empty())
            .unwrap();
        let res = model_router().with_state(state).oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}
