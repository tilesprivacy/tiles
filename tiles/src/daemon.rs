//! The Demon that runs the core with his spear

use std::{
    process::{Command, Stdio},
    sync::Arc,
    time::Duration,
};

use anyhow::{Result, anyhow};
use axum::{Router, extract::State, routing::get};
use reqwest::Client;
use std::fs::OpenOptions;
use std::sync::Mutex;
use tokio::sync::oneshot::{self, Receiver};

use crate::utils::config::{ConfigProvider, DefaultProvider};

struct AppState {
    pub shutdown_sender: Mutex<Option<oneshot::Sender<bool>>>,
}

pub async fn start_cmd() -> Result<()> {
    if cfg!(debug_assertions) {
        start_server().await
    } else {
        start_daemon().await
    }
}

pub async fn stop_cmd() -> Result<()> {
    stop_server().await
}
async fn root() -> &'static str {
    "Its me luttappi"
}

// allow zombie, since this process is expected to be
// running in background and have commands to stop if needed
#[allow(clippy::zombie_processes)]
async fn start_daemon() -> Result<()> {
    tokio::time::sleep(Duration::from_secs(5)).await;
    if (ping().await).is_ok() {
        return Ok(());
    }
    let data_dir = DefaultProvider.get_data_dir()?;
    let stdout_log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_dir.join("logs/daemon.out.log"))?;
    let stderr_log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_dir.join("logs/daemon.err.log"))?;
    let _process = Command::new("tiles")
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_log))
        .stderr(Stdio::from(stderr_log))
        .spawn()
        .expect("Failed to start daemon");

    wait_until_server_is_up().await
}

pub async fn start_server() -> Result<()> {
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<bool>();

    let state = AppState {
        shutdown_sender: Mutex::new(Some(shutdown_tx)),
    };
    let shared_state = Arc::new(state);
    let app = Router::new()
        .route("/", get(root))
        .route("/shutdown", get(shutdown))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:1729").await?;

    println!("Daemon server started at 1729");
    let _ = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_rx))
        .await;

    Ok(())
}

async fn shutdown_signal(rx: Receiver<bool>) {
    rx.await.expect("shutdown receiver paniced");
}

async fn shutdown(State(state): State<Arc<AppState>>) {
    println!("Daemon server shutting down");
    let mut sender = state.shutdown_sender.lock().unwrap();
    let sender_real = sender.take().unwrap();
    let _ = sender_real.send(true);
}

async fn stop_server() -> Result<()> {
    let client = Client::new();
    let res = client.get("http://127.0.0.1:1729/shutdown").send().await;

    match res {
        Err(err) => Err(anyhow!("Daemon shutdown failed due to {:?}", err)),
        _ => Ok(()),
    }
}
pub async fn ping() -> Result<(), String> {
    let client = Client::new();
    let res = client.get("http://127.0.0.1:1729").send().await;

    match res {
        Err(err) => Err(format!("Pong failed:  {:?}", err)),
        _ => Ok(()),
    }
}

async fn wait_until_server_is_up() -> Result<()> {
    let mut retry_count = 5;
    let mut error: String = String::new();
    loop {
        if retry_count < 1 {
            if !cfg!(debug_assertions) {
                println!("{:?}", error);
            }
            return Err(anyhow!(error));
        }
        match ping().await {
            Ok(()) => return Ok(()),
            Err(err) => {
                retry_count -= 1;
                error = err;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
