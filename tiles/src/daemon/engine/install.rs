//! Downloading, checking and installing the engine, one at a time

use std::{
    convert::Infallible,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use axum::response::sse::Event;
use futures_util::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tiles_engine::{BUILDS, Builds, Choice, Engine, Hardware, Installed, Store, check, choices};
use tokio_util::sync::CancellationToken;

use crate::{
    daemon::{
        AppError,
        job::{Job, JobState, Speed, is_live},
        model::{free_space, gb},
    },
    utils::{
        config::{ConfigProvider, DefaultProvider, get_engine_pick},
        hf_model_downloader::{DownloadEvent, Remote, fetch},
    },
};

const RELEASES: &str = "https://github.com/ggml-org/llama.cpp/releases/download";
const USER_AGENT: &str = concat!("tiles/", env!("CARGO_PKG_VERSION"));
// fail instead of hanging on a stalled connection
const STALL_TIMEOUT: Duration = Duration::from_secs(30);
// left free for models, logs and the rest of the computer
const SPARE_ROOM: u64 = 1_000_000_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum InstallState {
    Idle,
    Downloading {
        engine: Engine,
        done: u64,
        total: u64,
        bytes_per_sec: u64,
    },
    Unpacking {
        engine: Engine,
    },
    Checking {
        engine: Engine,
    },
    Paused {
        engine: Engine,
        done: u64,
        total: u64,
    },
    Done {
        engine: Engine,
    },
    Failed {
        engine: Engine,
        reason: String,
    },
}

impl JobState for InstallState {
    fn is_running(&self) -> bool {
        matches!(
            self,
            Self::Downloading { .. } | Self::Unpacking { .. } | Self::Checking { .. }
        )
    }
}

// why one engine's install stopped
enum Stop {
    Paused,
    // the build doesn't run here, the next engine is tried
    Check(String),
    Failed(String),
}

pub struct Engines {
    builds: &'static Builds,
    store: Store,
    root: PathBuf,
    client: Client,
    releases: String,
    pub(super) job: Arc<Job<InstallState>>,
    /// held while the pick is saved, and while a download reads it and starts
    pub(super) picking: tokio::sync::Mutex<()>,
}

impl Engines {
    pub fn new(root: PathBuf, releases: &str, builds: &'static Builds) -> anyhow::Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(STALL_TIMEOUT)
            .read_timeout(STALL_TIMEOUT)
            .build()?;
        Ok(Self {
            builds,
            store: Store::new(root.clone()),
            root,
            client,
            releases: releases.trim_end_matches('/').to_owned(),
            job: Job::new(InstallState::Idle),
            picking: tokio::sync::Mutex::new(()),
        })
    }

    pub fn for_daemon() -> anyhow::Result<Self> {
        Self::new(
            DefaultProvider.get_data_dir()?.join("engines"),
            RELEASES,
            &BUILDS,
        )
    }

    pub fn installed(&self) -> Option<Installed> {
        self.store.installed()
    }

    pub fn needs_download(&self, engine: Engine) -> bool {
        !engine.is_bundled()
            && !self.store.installed().is_some_and(|installed| {
                installed.engine == engine && installed.tag == self.builds.tag
            })
    }

    /// The llama-server replies run on, `None` for the one that comes with Tiles
    pub fn server(&self) -> Option<PathBuf> {
        if get_engine_pick() == Some(Engine::Cpu) {
            return None;
        }
        self.store.installed().map(|installed| installed.server())
    }

    /// Every engine for this computer, with the reason for those that failed their check here
    pub fn choices(&self, hardware: &Hardware) -> Vec<Choice> {
        let mut choices = choices(hardware, self.builds);
        self.store.mark_failures(&mut choices, &self.builds.tag);
        choices
    }

    pub fn state(&self) -> InstallState {
        self.job.state()
    }

    pub fn clear_failure(&self, engine: Engine) -> Result<(), AppError> {
        self.store.clear_failure(engine).map_err(|err| {
            AppError::InternalServerError(format!("Couldn't save the engine: {err}"))
        })
    }

    pub async fn is_installing(&self) -> bool {
        is_live(&*self.job.lock().await)
    }

    fn set(&self, state: InstallState) {
        self.job.set(state);
    }

    /// Installs `wanted`. When its check fails, the engines after it are tried in turn
    pub async fn start(
        self: &Arc<Self>,
        wanted: Engine,
        choices: &[Choice],
    ) -> Result<InstallState, AppError> {
        let mut running = self.job.lock().await;
        if is_live(&running) {
            return Ok(self.state());
        }
        let current = self.store.installed().is_some_and(|installed| {
            installed.engine == wanted && installed.tag == self.builds.tag
        });
        if wanted.is_bundled() || current {
            self.set(InstallState::Done { engine: wanted });
            return Ok(self.state());
        }
        let Some(build) = self.builds.find(wanted) else {
            return Err(AppError::CannotProcess(
                "llama.cpp has no build of it for this processor".to_owned(),
            ));
        };

        let downloads = self.store.downloads();
        tokio::fs::create_dir_all(&downloads).await.map_err(|err| {
            AppError::InternalServerError(format!("Couldn't make {}: {err}", downloads.display()))
        })?;
        let total: u64 = build.files.iter().map(|file| file.size).sum();
        let mut on_disk = 0;
        for file in &build.files {
            // a part longer than its file counts as the whole file, past it the sum below underflows
            on_disk += file_len(&downloads.join(&file.name))
                .await
                .max(file_len(&downloads.join(format!("{}.part", file.name))).await)
                .min(file.size);
        }
        let unpacked: u64 = build.files.iter().map(|file| file.unpacked).sum();
        let needed = total - on_disk + unpacked + SPARE_ROOM;
        if let Ok(free) = free_space(&self.root)
            && free < needed
        {
            return Err(AppError::InsufficientStorage(format!(
                "The engine needs {} GB of free disk space, and {} GB is free",
                gb(needed),
                gb(free)
            )));
        }

        let order: Vec<Engine> = choices
            .iter()
            .skip_while(|choice| choice.engine != wanted)
            .filter(|choice| choice.unavailable.is_none())
            .map(|choice| choice.engine)
            .collect();
        log::info!("Installing the {} engine", wanted.name());
        let cancel = CancellationToken::new();
        self.set(InstallState::Downloading {
            engine: wanted,
            done: on_disk,
            total,
            bytes_per_sec: 0,
        });
        let task = tokio::spawn(Arc::clone(self).run(order, cancel.clone()));
        *running = Some((cancel, task));
        Ok(self.state())
    }

    async fn run(self: Arc<Self>, order: Vec<Engine>, cancel: CancellationToken) {
        for engine in order {
            if engine.is_bundled() {
                self.set(InstallState::Done { engine });
                return;
            }
            match self.install(engine, &cancel).await {
                Ok(installed) => {
                    log::info!("Installed {}", installed.dir.display());
                    self.set(InstallState::Done { engine });
                    return;
                }
                Err(Stop::Paused) => return,
                Err(Stop::Failed(reason)) => {
                    log::warn!("Installing the {} engine failed: {reason}", engine.name());
                    self.set(InstallState::Failed { engine, reason });
                    return;
                }
                Err(Stop::Check(reason)) => {
                    log::warn!("The {} engine failed its check: {reason}", engine.name());
                    if let Err(err) = self.store.record_failure(engine, &self.builds.tag, &reason) {
                        log::warn!(
                            "Couldn't save why the {} engine failed: {err}",
                            engine.name()
                        );
                    }
                }
            }
        }
    }

    async fn install(&self, engine: Engine, cancel: &CancellationToken) -> Result<Installed, Stop> {
        let build = self.builds.find(engine).ok_or_else(|| {
            Stop::Failed("llama.cpp has no build of it for this processor".to_owned())
        })?;
        let downloads = self.store.downloads();
        tokio::fs::create_dir_all(&downloads)
            .await
            .map_err(|err| Stop::Failed(err.to_string()))?;
        let total = build.files.iter().map(|file| file.size).sum();
        let mut speed = Speed::new();
        let mut finished = 0;
        let mut archives = Vec::new();
        for file in &build.files {
            let path = downloads.join(&file.name);
            archives.push(path.clone());
            // a file is only moved here once its hash matched
            if file_len(&path).await == file.size {
                finished += file.size;
                continue;
            }
            let part = downloads.join(format!("{}.part", file.name));
            let url = format!("{}/{}/{}", self.releases, self.builds.tag, file.name);
            let remote = Remote {
                name: &file.name,
                size: file.size,
                request: &|| self.client.get(&url),
                check: &|res| Ok(res.error_for_status()?),
            };
            let fetched = fetch(&remote, &part, cancel, &mut |event| {
                if let DownloadEvent::Progress { done, .. } = event
                    && let Some(bytes_per_sec) = speed.update(finished + done)
                {
                    self.set(InstallState::Downloading {
                        engine,
                        done: finished + done,
                        total,
                        bytes_per_sec,
                    });
                }
            })
            .await
            .map_err(|err| Stop::Failed(format!("{err:#}")))?;
            let Some(sha256) = fetched else {
                self.set(InstallState::Paused {
                    engine,
                    done: finished + file_len(&part).await,
                    total,
                });
                return Err(Stop::Paused);
            };
            if sha256 != file.sha256 {
                let _ = tokio::fs::remove_file(&part).await;
                return Err(Stop::Failed(format!(
                    "{} didn't match its SHA-256, so it was deleted. Download it again",
                    file.name
                )));
            }
            tokio::fs::rename(&part, &path)
                .await
                .map_err(|err| Stop::Failed(err.to_string()))?;
            finished += file.size;
        }

        self.set(InstallState::Unpacking { engine });
        let store = self.store.clone();
        let unpacked = archives.clone();
        let staging = tokio::task::spawn_blocking(move || store.unpack(&unpacked))
            .await
            .map_err(|err| Stop::Failed(err.to_string()))?
            .map_err(|err| Stop::Failed(format!("Couldn't unpack the engine: {err}")))?;

        self.set(InstallState::Checking { engine });
        let checked = tokio::select! {
            () = cancel.cancelled() => None,
            checked = check(&staging, engine) => Some(checked),
        };
        let store = self.store.clone();
        match checked {
            None => {
                let _ = tokio::task::spawn_blocking(move || store.discard_staging()).await;
                self.set(InstallState::Paused {
                    engine,
                    done: total,
                    total,
                });
                Err(Stop::Paused)
            }
            Some(Err(reason)) => {
                let _ = tokio::task::spawn_blocking(move || store.discard_staging()).await;
                for archive in &archives {
                    let _ = tokio::fs::remove_file(archive).await;
                }
                Err(Stop::Check(reason))
            }
            Some(Ok(())) => {
                let tag = self.builds.tag.clone();
                tokio::task::spawn_blocking(move || store.finish(engine, &tag))
                    .await
                    .map_err(|err| Stop::Failed(err.to_string()))?
                    .map_err(|err| Stop::Failed(format!("Couldn't install the engine: {err}")))
            }
        }
    }

    /// Deletes the engine on disk, replies go back to the one that comes with Tiles
    pub async fn delete(&self) -> Result<u64, AppError> {
        // held so nothing starts installing while it's deleted
        let running = self.job.lock().await;
        if is_live(&running) {
            return Err(AppError::Conflict(
                "An engine is downloading, pause it before deleting".to_owned(),
            ));
        }
        if self.installed().is_none() && !self.store.downloads().exists() {
            return Err(AppError::NotFound("No engine is downloaded".to_owned()));
        }
        let store = self.store.clone();
        let freed = tokio::task::spawn_blocking(move || store.delete())
            .await
            .map_err(|err| AppError::InternalServerError(err.to_string()))?
            .map_err(|err| {
                AppError::InternalServerError(format!("Couldn't delete the engine: {err}"))
            })?;
        self.set(InstallState::Idle);
        Ok(freed)
    }

    pub async fn pause(&self) -> Result<InstallState, AppError> {
        self.job
            .pause()
            .await
            .ok_or_else(|| AppError::NotFound("No engine is downloading".to_owned()))
    }

    // with `pause_on_close`, closing the stream early pauses the install
    pub fn events(
        &self,
        pause_on_close: bool,
    ) -> impl Stream<Item = Result<Event, Infallible>> + use<> {
        self.job.events(pause_on_close)
    }
}

async fn file_len(path: &Path) -> u64 {
    tokio::fs::metadata(path).await.map_or(0, |meta| meta.len())
}

#[cfg(test)]
mod tests {
    use flate2::{Compression, write::GzEncoder};
    use sha2::{Digest, Sha256};
    use tiles_engine::{Build, BuildFile, Gpu, Vendor, auto};
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};

    use super::*;

    // laid out like upstream's, with everything in one top folder
    fn archive(script: &str) -> Vec<u8> {
        let mut tar = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
        let body = format!("#!/bin/sh\n{script}\n");
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(0o755);
        tar.append_data(&mut header, "llama-b9/llama-server", body.as_bytes())
            .unwrap();
        tar.into_inner().unwrap().finish().unwrap()
    }

    const LISTS_VULKAN: &str =
        "echo 'Available devices:'; echo '  Vulkan0: GPU (1 MiB, 1 MiB free)'";
    const NO_LIBCUDA: &str = "echo 'libcuda.so.1: cannot open shared object file' >&2; exit 127";

    fn file(name: &str, bytes: &[u8]) -> BuildFile {
        BuildFile {
            name: name.to_owned(),
            size: bytes.len() as u64,
            unpacked: 0,
            sha256: format!("{:x}", Sha256::digest(bytes)),
        }
    }

    fn builds(files: Vec<(Engine, BuildFile)>) -> &'static Builds {
        let engines = files
            .into_iter()
            .map(|(engine, file)| Build {
                engine,
                arch: std::env::consts::ARCH.to_owned(),
                version: None,
                files: vec![file],
            })
            .collect();
        Box::leak(Box::new(Builds {
            tag: "b9".to_owned(),
            engines,
        }))
    }

    fn order(engines: &[Engine]) -> Vec<Choice> {
        engines
            .iter()
            .map(|&engine| Choice {
                engine,
                label: engine.name().to_owned(),
                download: 0,
                unavailable: None,
            })
            .collect()
    }

    async fn serve(server: &MockServer, name: &str, bytes: Vec<u8>, delay: Duration) {
        Mock::given(path(format!("/b9/{name}")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(bytes)
                    .set_delay(delay),
            )
            .mount(server)
            .await;
    }

    async fn finished(engines: &Engines) -> InstallState {
        let mut changes = engines.job.subscribe();
        loop {
            let state = changes.borrow_and_update().clone();
            if !state.is_running() {
                return state;
            }
            changes.changed().await.unwrap();
        }
    }

    // CUDA 13 that can't start, and a Vulkan build that lists its GPU
    async fn setup(
        cuda: &str,
        vulkan: &str,
        delay: Duration,
    ) -> (MockServer, tempfile::TempDir, Arc<Engines>) {
        let server = MockServer::start().await;
        let (cuda, vulkan) = (archive(cuda), archive(vulkan));
        let builds = builds(vec![
            (Engine::Cuda13, file("cuda.tar.gz", &cuda)),
            (Engine::Vulkan, file("vulkan.tar.gz", &vulkan)),
        ]);
        serve(&server, "cuda.tar.gz", cuda, delay).await;
        serve(&server, "vulkan.tar.gz", vulkan, delay).await;
        let dir = tempfile::tempdir().unwrap();
        let engines = Engines::new(dir.path().join("engines"), &server.uri(), builds).unwrap();
        (server, dir, Arc::new(engines))
    }

    #[tokio::test]
    async fn a_build_that_fails_its_check_falls_back_to_the_next() {
        let (_server, _dir, engines) = setup(NO_LIBCUDA, LISTS_VULKAN, Duration::ZERO).await;
        let choices = order(&[Engine::Cuda13, Engine::Vulkan, Engine::Cpu]);
        engines.start(Engine::Cuda13, &choices).await.unwrap();

        assert_eq!(
            finished(&engines).await,
            InstallState::Done {
                engine: Engine::Vulkan
            }
        );
        let installed = engines.installed().unwrap();
        assert_eq!(
            (installed.engine, installed.tag.as_str()),
            (Engine::Vulkan, "b9")
        );
        assert!(installed.server().is_file());
        let failures = engines.store.failures("b9");
        assert!(failures[&Engine::Cuda13].contains("libcuda.so.1"));
        assert!(!engines.store.downloads().exists());

        // installed now, nothing is downloaded again
        let again = engines.start(Engine::Vulkan, &choices).await.unwrap();
        assert_eq!(
            again,
            InstallState::Done {
                engine: Engine::Vulkan
            }
        );
    }

    #[tokio::test]
    async fn a_part_longer_than_its_file_counts_as_the_whole_file() {
        let (_server, _dir, engines) = setup(NO_LIBCUDA, LISTS_VULKAN, Duration::ZERO).await;
        let downloads = engines.store.downloads();
        std::fs::create_dir_all(&downloads).unwrap();
        std::fs::write(downloads.join("vulkan.tar.gz.part"), vec![0; 1 << 20]).unwrap();
        let choices = order(&[Engine::Vulkan, Engine::Cpu]);

        // the free space check underflowed on it
        assert!(engines.start(Engine::Vulkan, &choices).await.is_ok());
        let _ = tokio::time::timeout(Duration::from_secs(10), finished(&engines)).await;
    }

    #[tokio::test]
    async fn when_every_build_fails_the_cpu_is_left() {
        let (_server, _dir, engines) = setup(NO_LIBCUDA, "echo '  (none)'", Duration::ZERO).await;
        let choices = order(&[Engine::Cuda13, Engine::Vulkan, Engine::Cpu]);
        engines.start(Engine::Cuda13, &choices).await.unwrap();

        assert_eq!(
            finished(&engines).await,
            InstallState::Done {
                engine: Engine::Cpu
            }
        );
        assert_eq!(engines.installed(), None);
        assert!(engines.store.failures("b9")[&Engine::Vulkan].contains("no GPU"));
    }

    #[tokio::test]
    async fn a_download_with_the_wrong_hash_fails_without_trying_the_next() {
        let server = MockServer::start().await;
        let builds = builds(vec![(Engine::Vulkan, file("vulkan.tar.gz", b"expected"))]);
        serve(
            &server,
            "vulkan.tar.gz",
            b"tampered".to_vec(),
            Duration::ZERO,
        )
        .await;
        let dir = tempfile::tempdir().unwrap();
        let engines = Arc::new(Engines::new(dir.path().to_owned(), &server.uri(), builds).unwrap());
        engines
            .start(Engine::Vulkan, &order(&[Engine::Vulkan, Engine::Cpu]))
            .await
            .unwrap();

        let InstallState::Failed { engine, reason } = finished(&engines).await else {
            panic!("the install didn't fail");
        };
        assert_eq!(engine, Engine::Vulkan);
        assert!(reason.contains("SHA-256"));
        assert!(engines.store.failures("b9").is_empty());
    }

    #[tokio::test]
    async fn a_paused_install_continues_when_started_again() {
        let (_server, _dir, engines) =
            setup(NO_LIBCUDA, LISTS_VULKAN, Duration::from_millis(500)).await;
        let choices = order(&[Engine::Vulkan, Engine::Cpu]);
        engines.start(Engine::Vulkan, &choices).await.unwrap();
        let paused = engines.pause().await.unwrap();
        assert!(matches!(
            paused,
            InstallState::Paused {
                engine: Engine::Vulkan,
                ..
            }
        ));
        assert_eq!(engines.installed(), None);

        engines.start(Engine::Vulkan, &choices).await.unwrap();
        assert_eq!(
            finished(&engines).await,
            InstallState::Done {
                engine: Engine::Vulkan
            }
        );
    }

    #[tokio::test]
    async fn an_installed_engine_needs_no_download() {
        let (_server, _dir, engines) = setup(NO_LIBCUDA, LISTS_VULKAN, Duration::ZERO).await;
        // auto picks CUDA 13 for it
        let hardware = Hardware {
            os: "linux".to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            gpus: vec![Gpu {
                vendor: Vendor::Nvidia,
                name: Some("NVIDIA GPU".to_owned()),
                compute_capability: Some((12, 0)),
                vram: None,
            }],
            nvidia_driver: Some("999.0".to_owned()),
            nvidia_cuda: None,
            vulkan_loader: true,
        };

        let choices = engines.choices(&hardware);
        let picked = auto(&hardware, &choices);
        assert_eq!(picked, Engine::Cuda13);
        assert!(engines.needs_download(picked));

        engines.start(picked, &choices).await.unwrap();
        // CUDA 13 fails its check here, Vulkan is next
        assert_eq!(
            finished(&engines).await,
            InstallState::Done {
                engine: Engine::Vulkan
            }
        );

        // Vulkan is the auto pick now, and it's installed
        let picked = auto(&hardware, &engines.choices(&hardware));
        assert_eq!(picked, Engine::Vulkan);
        assert!(!engines.needs_download(picked));
    }
}
