//! The menu bar app, which the daemon owns for the length of its own life
//!
//! The child is spawned with a piped stdin that nothing ever writes to. The app
//! blocks a thread on reading it, so the pipe closing is what tells the app the
//! daemon is gone. That holds even when the daemon is killed outright, which no
//! amount of polling would.

use std::{
    path::PathBuf,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use tokio::{process::Command, sync::Notify};

use crate::utils::config::{ConfigProvider, DefaultProvider, get_ui_config};

const BUNDLE_EXEC: &str = "Tiles.app/Contents/MacOS/tiles-menubar";

/// Set on the child so it knows to watch the lifeline. A hand-launched app has
/// no parent to outlive and leaves the watcher dormant
const SUPERVISED: &str = "TILES_MENUBAR_SUPERVISED";
const SUPERVISED_ARG: &str = "--tiles-daemon-supervised";

const MIN_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(30);
/// Past this the child counts as healthy and the next crash starts over
const SETTLED: Duration = Duration::from_secs(60);
/// How long the app gets to notice the closed pipe before it is killed
const GRACE: Duration = Duration::from_secs(3);
/// A shutdown must not wait on a wedged app
const STOP_DEADLINE: Duration = Duration::from_secs(5);

pub struct Ui {
    /// False whenever there is no supervisor, so a headless shutdown does not
    /// sit out the stop deadline waiting for a task that was never spawned
    running: AtomicBool,
    stopping: AtomicBool,
    stop: Notify,
    done: Notify,
}

impl Ui {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            running: AtomicBool::new(false),
            stopping: AtomicBool::new(false),
            stop: Notify::new(),
            done: Notify::new(),
        })
    }
}

/// Release runs the installed bundle, debug the one `tauri build` leaves in the
/// workspace target. Neither is required to exist
fn resolve() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("TILES_MENUBAR_BIN") {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }

    let installed = PathBuf::from("/Applications").join(BUNDLE_EXEC);
    if installed.is_file() {
        return Some(installed);
    }

    if cfg!(debug_assertions) {
        let built = std::env::current_dir()
            .ok()?
            .join("target/debug/bundle/macos")
            .join(BUNDLE_EXEC);
        return built.is_file().then_some(built);
    }

    None
}

/// Off by default in debug, where a stale bundle would win the app's
/// single-instance race against `pnpm tauri dev`
fn enabled_by_config() -> bool {
    get_ui_config()
        .ok()
        .and_then(|config| config.menubar)
        .unwrap_or(!cfg!(debug_assertions))
}

fn spawn(bin: &PathBuf) -> Result<tokio::process::Child> {
    let data_dir = DefaultProvider.get_or_create_data_dir()?;
    let out = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_dir.join("logs/menubar.out.log"))?;
    let err = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_dir.join("logs/menubar.err.log"))?;

    // deliberately no setsid, unlike the inference server and the agent. this
    // child is meant to be reachable, not detached
    Command::new(bin)
        .arg(SUPERVISED_ARG)
        .env(SUPERVISED, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("Failed to start {}", bin.display()))
}

/// Nothing here is fatal to the daemon, a headless daemon is a working daemon
pub fn start(ui: Arc<Ui>) {
    if !cfg!(target_os = "macos") {
        return;
    }
    if !enabled_by_config() {
        log::info!("Menu bar app disabled by config");
        return;
    }
    let Some(bin) = resolve() else {
        log::info!("No menu bar app found, running headless");
        return;
    };

    ui.running.store(true, Ordering::SeqCst);
    tokio::spawn(async move {
        let mut backoff = MIN_BACKOFF;

        loop {
            if ui.stopping.load(Ordering::SeqCst) {
                break;
            }

            let mut child = match spawn(&bin) {
                Ok(child) => child,
                Err(err) => {
                    log::error!("Menu bar app failed to start: {err:?}");
                    break;
                }
            };
            log::info!("Menu bar app started with PID {:?}", child.id());
            let started = Instant::now();

            // tokio's `wait` closes the child's stdin before it waits, so the
            // lifeline has to be held out here or the app reads EOF at once
            let lifeline = child.stdin.take();

            let status = tokio::select! {
                status = child.wait() => status,
                _ = ui.stop.notified() => {
                    // closing the lifeline is the ask, the kill is the threat
                    drop(lifeline);
                    match tokio::time::timeout(GRACE, child.wait()).await {
                        Ok(status) => status,
                        Err(_) => {
                            let _ = child.kill().await;
                            child.wait().await
                        }
                    }
                }
            };

            if ui.stopping.load(Ordering::SeqCst) {
                log::info!("Menu bar app stopped");
                break;
            }

            match status {
                Ok(status) if status.success() => {
                    log::info!("Menu bar app exited cleanly, relaunching")
                }
                Ok(status) => log::warn!("Menu bar app died with {status}, relaunching"),
                Err(err) => log::warn!("Lost track of the menu bar app: {err:?}, relaunching"),
            }

            if started.elapsed() >= SETTLED {
                backoff = MIN_BACKOFF;
            }
            tokio::select! {
                _ = tokio::time::sleep(backoff) => {}
                _ = ui.stop.notified() => break,
            }
            backoff = (backoff * 2).min(MAX_BACKOFF);
        }

        ui.running.store(false, Ordering::SeqCst);
        ui.done.notify_one();
    });
}

/// Mark shutdown before slower daemon teardown begins so no exited child is
/// relaunched in the meantime.
pub fn begin_stop(ui: &Arc<Ui>) {
    if !ui.stopping.swap(true, Ordering::SeqCst) && ui.running.load(Ordering::SeqCst) {
        ui.stop.notify_one();
    }
}

/// Returns once the app is gone, or once it has had long enough to be
pub async fn stop(ui: &Arc<Ui>) {
    begin_stop(ui);
    if !ui.running.load(Ordering::SeqCst) {
        return;
    }
    if tokio::time::timeout(STOP_DEADLINE, ui.done.notified())
        .await
        .is_err()
    {
        log::warn!("Menu bar app did not confirm its exit in time");
    }
}
