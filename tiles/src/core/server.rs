//! Module for inference server

use anyhow::{Context, Result, anyhow};
use nix::{
    sys::{
        signal::{Signal, kill, killpg},
        wait::{WaitPidFlag, waitpid},
    },
    unistd::{Pid, setsid},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::OpenOptions,
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use tokio::time::sleep;

use crate::utils::config::{ConfigProvider, DefaultProvider, PY_PORT};

// the server boots in about half a second, the rest is headroom for a cold disk
const READY_TIMEOUT: Duration = Duration::from_secs(15);
const READY_POLL: Duration = Duration::from_millis(250);
// inside the daemon's 10s shutdown budget, SIGKILL after this
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const EXIT_POLL: Duration = Duration::from_millis(100);
const PING_TIMEOUT: Duration = Duration::from_secs(2);

/// What a start or stop did, for a client to switch on
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

/// What the inference server has loaded, as its `/ping` reports it
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadedModel {
    pub state: ModelState,
    /// the GGUF file llama-server was started with, `None` when the state is `None`
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

/// Why the server could not be reached, started or stopped
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerError {
    /// something that tiles did not start answers on the port
    PortInUse,
    /// the server was spawned but has not answered yet
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

#[allow(clippy::zombie_processes)]
pub async fn start_server_daemon() -> Result<ServerState> {
    if ping().await.is_ok() {
        return match own_pid()? {
            Some(_) => {
                log::info!("{}", ServerState::AlreadyRunning);
                Ok(ServerState::AlreadyRunning)
            }
            None => Err(ServerError::PortInUse.into()),
        };
    }

    let data_dir = DefaultProvider.get_data_dir()?;
    let mut server_dir = DefaultProvider.get_lib_dir()?;
    let pid_path = pid_file_path()?;
    server_dir = server_dir.join("server");
    let stdout_log = OpenOptions::new()
        .append(true)
        .open(data_dir.join("logs/server.out.log"))?;
    let stderr_log = OpenOptions::new()
        .append(true)
        .open(data_dir.join("logs/server.err.log"))?;
    let server_path = server_dir.join("stack_export_prod/app-server/bin/python");
    server_dir.pop();
    let child = unsafe {
        Command::new(server_path)
            .args(["-m", "server.main"])
            .current_dir(server_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout_log))
            .stderr(Stdio::from(stderr_log))
            .pre_exec(|| {
                if let Err(err) = setsid() {
                    Err(Into::into(err))
                } else {
                    Ok(())
                }
            })
            .spawn()
            .map_err(|e| anyhow!("Error starting server due to {:?}", e))?
    };
    let pid = Pid::from_raw(child.id() as i32);

    if std::fs::write(&pid_path, child.id().to_string()).is_err() {
        stop_pid(pid).await;
        return Err(anyhow!(
            "Server start failed due to cannot write PID to file"
        ));
    }
    log::info!("Server started with PID {}", child.id());

    if let Err(err) = wait_until_ready(pid).await {
        // a slow one is left to finish booting, a dead one is cleaned up
        if !is_alive(pid) {
            let _ = std::fs::remove_file(&pid_path);
        }
        return Err(err);
    }

    log::info!("{}", ServerState::Started);
    Ok(ServerState::Started)
}

pub async fn stop_server_daemon() -> Result<ServerState> {
    let pid = own_pid()?;
    let up = ping().await.is_ok();

    let Some(pid) = pid else {
        if up {
            return Err(ServerError::PortInUse.into());
        }
        let _ = std::fs::remove_file(pid_file_path()?);
        log::info!("{}", ServerState::NotRunning);
        return Ok(ServerState::NotRunning);
    };

    stop_pid(pid).await;
    std::fs::remove_file(pid_file_path()?)
        .map_err(|e| anyhow!("Failed to remove the server pid file due to {}", e))?;

    log::info!("{}", ServerState::Stopped);
    Ok(ServerState::Stopped)
}

/// `Ok` when the inference server answers, along with the model it reports. The model is
/// `None` when the server is an older version that does not report one
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

fn pid_file_path() -> Result<PathBuf> {
    Ok(DefaultProvider.get_config_dir()?.join("server.pid"))
}

/// The server this install started, if it is still alive
fn own_pid() -> Result<Option<Pid>> {
    let pid_path = pid_file_path()?;
    if !pid_path.exists() {
        return Ok(None);
    }

    let raw = std::fs::read_to_string(&pid_path).context("Failed to read the server pid file")?;
    // 0 and negatives would make kill() signal whole process groups
    let pid = match raw.trim().parse::<i32>() {
        Ok(pid) if pid > 0 => Pid::from_raw(pid),
        _ => {
            log::warn!("Ignoring a server pid file that holds {raw:?}");
            return Ok(None);
        }
    };

    Ok(is_alive(pid).then_some(pid))
}

fn is_alive(pid: Pid) -> bool {
    // a crashed server the daemon started lingers as a zombie, which kill() still
    // sees as alive. waitpid clears it, and returns at once if it is still running
    let _ = waitpid(pid, Some(WaitPidFlag::WNOHANG));
    // signal 0 sends nothing, it only checks that the pid exists
    kill(pid, None).is_ok()
}

async fn wait_until_ready(pid: Pid) -> Result<()> {
    let started = Instant::now();
    while started.elapsed() < READY_TIMEOUT {
        if ping().await.is_ok() {
            return Ok(());
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

/// SIGTERM, then SIGKILL to the whole group if it is still there
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
    // setsid made it a group leader, so this takes its children with it
    let _ = killpg(pid, Signal::SIGKILL);
    let _ = waitpid(pid, None);
}
