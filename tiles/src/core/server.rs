//! Starts, stops and checks the Python inference server on `PY_PORT`.

use anyhow::{Context, Result, anyhow};
use nix::{
    fcntl::{Flock, FlockArg},
    sys::{
        signal::{Signal, kill, killpg},
        wait::{WaitPidFlag, waitpid},
    },
    unistd::Pid,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    env, fmt,
    fs::{File, OpenOptions},
    path::PathBuf,
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{process::Command, time::sleep};

use crate::utils::config::{ConfigProvider, DefaultProvider, PY_PORT};

// it starts in about half a second, the rest is room for a slow first launch
const READY_TIMEOUT: Duration = Duration::from_secs(15);
const READY_POLL: Duration = Duration::from_millis(250);
// how long stop waits before force killing, keep it under the daemon's 10s shutdown limit
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const EXIT_POLL: Duration = Duration::from_millis(100);
const PING_TIMEOUT: Duration = Duration::from_secs(2);

/// What a start or stop did, sent to API clients as `state`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerState {
    Started,
    AlreadyRunning,
    Stopped,
    NotRunning,
}

impl fmt::Display for ServerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Started => "Inference server started",
            Self::AlreadyRunning => "Inference server is already running",
            Self::Stopped => "Inference server stopped",
            Self::NotRunning => "Inference server is not running",
        })
    }
}

/// The model state the Python server reports on `/ping`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadedModel {
    pub state: ModelState,
    /// The GGUF file llama-server is running, `None` when no model is loaded
    pub path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelState {
    None,
    Loading,
    Loaded,
}

#[derive(Deserialize)]
struct PingBody {
    model: Option<LoadedModel>,
}

/// Errors the API answers with 409 or 503 instead of 500
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerError {
    PortInUse,
    NotReady,
    NotRunning,
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PortInUse => write!(f, "Port {PY_PORT} is in use by another process"),
            Self::NotReady => write!(
                f,
                "Inference server has not answered after {}s, it may still be starting. See logs/server.err.log",
                READY_TIMEOUT.as_secs()
            ),
            Self::NotRunning => f.write_str("Inference server is not running"),
        }
    }
}

impl std::error::Error for ServerError {}

pub async fn start_server_daemon() -> Result<ServerState> {
    let _lock = lock_server().await?;
    if ping().await.is_ok() {
        return match running_server_pid()? {
            Some(_) => {
                log::info!("{}", ServerState::AlreadyRunning);
                Ok(ServerState::AlreadyRunning)
            }
            None => Err(ServerError::PortInUse.into()),
        };
    }

    let data_dir = DefaultProvider.get_data_dir()?;
    let mut server_dir = DefaultProvider.get_lib_dir()?;
    server_dir = server_dir.join("server");
    let stdout_log = OpenOptions::new()
        .append(true)
        .open(data_dir.join("logs/server.out.log"))?;
    let stderr_log = OpenOptions::new()
        .append(true)
        .open(data_dir.join("logs/server.err.log"))?;
    let server_path = server_dir.join("stack_export_prod/app-server/bin/python");
    server_dir.pop();
    // its own process group keeps terminal ctrl-c away from it, and lets stop
    // kill llama-server with it through killpg
    let mut child = Command::new(server_path)
        .args(["-m", "server.main"])
        .current_dir(server_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_log))
        .stderr(Stdio::from(stderr_log))
        .process_group(0)
        .spawn()
        .map_err(|e| anyhow!("Error starting server due to {:?}", e))?;
    let pid = child
        .id()
        .map(|id| Pid::from_raw(id as i32))
        .ok_or_else(|| {
            anyhow!("Inference server exited while starting. See logs/server.err.log")
        })?;
    log::info!("Server started with PID {pid}");

    // keep hold of the child so we notice the moment it exits, instead of leaving a zombie
    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) if status.success() => log::info!("Inference server exited"),
            Ok(status) => {
                log::warn!("Inference server exited with {status}. See logs/server.err.log")
            }
            Err(err) => log::debug!("Lost track of the inference server: {err}"),
        }
    });

    // the server writes its own pid file once it has the port
    wait_until_ready(pid).await?;

    log::info!("{}", ServerState::Started);
    Ok(ServerState::Started)
}

pub async fn stop_server_daemon() -> Result<ServerState> {
    let _lock = lock_server().await?;
    let Some(pid) = running_server_pid()? else {
        if ping().await.is_ok() {
            return Err(ServerError::PortInUse.into());
        }
        log::info!("{}", ServerState::NotRunning);
        return Ok(ServerState::NotRunning);
    };

    stop_pid(pid).await;
    // a normal exit deletes the file itself, a force killed server can't
    remove_pid_file(pid);

    log::info!("{}", ServerState::Stopped);
    Ok(ServerState::Stopped)
}

/// Checks the server answers `/ping`, and returns the model state it reports.
/// The model is `None` from older servers that don't report it
pub async fn ping() -> Result<Option<LoadedModel>> {
    let url = format!("http://127.0.0.1:{}/ping", PY_PORT);
    let res = Client::new()
        .get(url)
        .timeout(PING_TIMEOUT)
        .send()
        .await
        .and_then(|res| res.error_for_status());

    match res {
        Ok(res) => Ok(res
            .json::<PingBody>()
            .await
            .ok()
            .and_then(|body| body.model)),
        Err(err) => {
            log::debug!("Inference server ping failed: {err:?}");
            Err(ServerError::NotRunning.into())
        }
    }
}

/// Takes the start/stop lock, released when the returned value is dropped. The lock
/// file is in the shared state folder, so a dev daemon and a normal install wait for
/// each other too
async fn lock_server() -> Result<Flock<File>> {
    let dir = state_dir()?;
    let path = dir.join(format!("inference-{PY_PORT}.lock"));
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&dir)?;
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)?;
        Flock::lock(file, FlockArg::LockExclusive)
            .map_err(|(_, e)| anyhow!("Failed to lock {}: {e}", path.display()))
    })
    .await?
}

/// A per-user folder shared by every install, since they all use the same port
fn state_dir() -> Result<PathBuf> {
    let state = match env::var_os("XDG_STATE_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => env::home_dir()
            .context("Failed to fetch $HOME")?
            .join(".local/state"),
    };
    Ok(state.join("tiles"))
}

/// The server writes and deletes this file itself (`pid_path` in server/config.py).
/// We only read it and clean up leftovers
fn server_pid_path() -> Result<PathBuf> {
    Ok(state_dir()?.join(format!("inference-{PY_PORT}.pid")))
}

/// The server's pid, if the file points at a running `server.main`. A leftover file
/// is deleted
fn running_server_pid() -> Result<Option<Pid>> {
    let raw = match std::fs::read_to_string(server_pid_path()?) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).context("Failed to read the server pid file"),
    };
    // kill() treats 0 and negative numbers as whole process groups, so skip them
    let pid = match raw.trim().parse::<i32>() {
        Ok(pid) if pid > 0 => Pid::from_raw(pid),
        _ => {
            log::warn!("Ignoring a server pid file that holds {raw:?}");
            return Ok(None);
        }
    };

    if is_alive(pid) && is_inference_server(pid) {
        return Ok(Some(pid));
    }
    // leftover, the server died without deleting it
    remove_pid_file(pid);
    Ok(None)
}

/// Deletes the pid file, unless a newer server has written its own pid since
fn remove_pid_file(pid: Pid) {
    if let Ok(path) = server_pid_path()
        && std::fs::read_to_string(&path).is_ok_and(|raw| raw.trim() == pid.to_string())
    {
        let _ = std::fs::remove_file(path);
    }
}

fn is_alive(pid: Pid) -> bool {
    // a child of ours that died stays a "zombie" until we collect it, and kill(pid, 0)
    // still finds zombies, so collect it first. this returns at once if it's still running
    let _ = waitpid(pid, Some(WaitPidFlag::WNOHANG));
    // signal 0 sends nothing, it only checks the pid exists
    kill(pid, None).is_ok()
}

/// Pids get reused, so a leftover pid file can point at a different process by now.
/// This checks it really is our server. Reads /proc on Linux, uses `ps` on macOS
fn is_inference_server(pid: Pid) -> bool {
    let cmdline = std::fs::read(format!("/proc/{pid}/cmdline"))
        .map(|raw| String::from_utf8_lossy(&raw).replace('\0', " "))
        .ok()
        .or_else(|| {
            std::process::Command::new("ps")
                .args(["-o", "command=", "-p", &pid.to_string()])
                .output()
                .ok()
                .filter(|out| out.status.success())
                .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        });
    cmdline.is_some_and(|line| line.contains("server.main"))
}

async fn wait_until_ready(pid: Pid) -> Result<()> {
    let started = Instant::now();
    while started.elapsed() < READY_TIMEOUT {
        if ping().await.is_ok() {
            // make sure our own child is answering, if it died someone else has the port
            return if is_alive(pid) {
                Ok(())
            } else {
                Err(ServerError::PortInUse.into())
            };
        }
        if !is_alive(pid) {
            return Err(anyhow!(
                "Inference server exited while starting. See logs/server.err.log"
            ));
        }
        sleep(READY_POLL).await;
    }

    Err(ServerError::NotReady.into())
}

/// Asks the server to stop, waits up to `EXIT_TIMEOUT`, then force kills it
async fn stop_pid(pid: Pid) {
    if let Err(err) = kill(pid, Signal::SIGTERM) {
        log::warn!("Failed to signal the inference server: {err}");
    }

    let asked = Instant::now();
    while asked.elapsed() < EXIT_TIMEOUT {
        if !is_alive(pid) {
            return;
        }
        sleep(EXIT_POLL).await;
    }

    log::warn!(
        "Inference server did not exit within {}s, killing it",
        EXIT_TIMEOUT.as_secs()
    );
    // a server we started has its own process group, so killpg also stops
    // llama-server. one started by hand doesn't, so kill it directly too
    let _ = killpg(pid, Signal::SIGKILL);
    let _ = kill(pid, Signal::SIGKILL);
    let _ = waitpid(pid, None);
}
