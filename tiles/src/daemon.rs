//! The Demon that runs the core with his spear

use std::{
    process::{Command, Stdio},
    sync::Arc,
    time::Duration,
};

use anyhow::{Result, anyhow};
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
};
use axum_macros::debug_handler;
use log::info;
use reqwest::Client;
use semver::Version;
use std::fs::OpenOptions;
use std::sync::Mutex;
use tokio::sync::oneshot::{self, Receiver};

use crate::utils::config::{ConfigProvider, DefaultProvider, get_model_cache};

struct AppState {
    pub shutdown_sender: Mutex<Option<oneshot::Sender<bool>>>,
    pub vsn: String,
}

#[derive(serde::Deserialize)]
pub struct SendParams {
    model_name: String,
}

//TODO: Add a different PORT for development
// We should update that in py server too for the daemon api calls
const DEFAULT_PORT: u32 = 1729;
pub async fn start_cmd(port: Option<u32>) -> Result<()> {
    start_daemon(port).await
}

pub async fn stop_cmd() -> Result<()> {
    stop_server(None).await
}
async fn root(State(state): State<Arc<AppState>>) -> String {
    state.vsn.clone()
}

// allow zombie, since this process is expected to be
// running in background and have commands to stop if needed
#[allow(clippy::zombie_processes)]
async fn start_daemon(port: Option<u32>) -> Result<()> {
    if let Ok(daemon_current_vsn) = ping(port).await {
        let app_vsn = Version::parse(env!("CARGO_PKG_VERSION"))?;
        log::info!(
            "app version found {}, daemon version {}",
            app_vsn,
            daemon_current_vsn
        );
        if app_vsn
            .cmp_precedence(&Version::parse(&daemon_current_vsn)?)
            .is_ne()
        {
            log::info!(
                "New app version found {}, hot reload the daemon {}",
                app_vsn,
                daemon_current_vsn
            );
            stop_server(None).await?;
            log::info!("Stopped the current daemon server");
        } else {
            return Ok(());
        }
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
    let base_command = if cfg!(debug_assertions) {
        "target/debug/tiles"
    } else {
        "tiles"
    };
    let _process = Command::new(base_command)
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_log))
        .stderr(Stdio::from(stderr_log))
        .spawn()
        .expect("Failed to start daemon");

    wait_until_server_is_up(port).await
}

pub async fn start_server(port: Option<u32>) -> Result<()> {
    let dyn_port: u32 = get_port(port);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<bool>();

    let state = AppState {
        shutdown_sender: Mutex::new(Some(shutdown_tx)),
        vsn: env!("CARGO_PKG_VERSION").to_owned(),
    };

    let shared_state = Arc::new(state);
    let app = Router::new()
        .route("/", get(root))
        .route("/shutdown", get(shutdown))
        .route("/model-cache-path", get(get_model_cache_path))
        .with_state(shared_state);

    let addr = format!("127.0.0.1:{}", dyn_port);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Daemon server started at {}", dyn_port);
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

#[debug_handler]
async fn get_model_cache_path(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<SendParams>,
) -> Result<String, StatusCode> {
    println!("getting model cache path");
    if let Ok(model_path) = get_model_cache(&params.model_name) {
        Ok(model_path
            .to_str()
            .expect("Pathbuf to str failed")
            .to_owned())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// #[debug_handler]
// async fn send_ping(State(_state): State<Arc<AppState>>, Query(params): Query<SendParams>) {
//     println!("Trying to send ping");
//     let _ = network::init(Some(&params.ticket)).await;
// }

// async fn receive_ping(State(_state): State<Arc<AppState>>) {
//     println!("Trying to receive ping");
//     let _ = network::init(None).await;
// }

async fn stop_server(port: Option<u32>) -> Result<()> {
    let dyn_port = get_port(port);
    let client = Client::new();
    let addr = format!("http://127.0.0.1:{}/shutdown", dyn_port);
    let res = client.get(addr).send().await;

    match res {
        Err(err) => Err(anyhow!("Daemon shutdown failed due to {:?}", err)),
        _ => Ok(()),
    }
}
pub async fn ping(port: Option<u32>) -> anyhow::Result<String> {
    let dyn_port = get_port(port);
    let client = Client::new();
    let addr = format!("http://127.0.0.1:{}", dyn_port);
    let res = client.get(addr).send().await;

    match res {
        Err(err) => Err(anyhow!(format!("Pong failed:  {:?}", err))),
        Ok(resp) => resp.text().await.map_err(Into::into),
    }
}

async fn wait_until_server_is_up(port: Option<u32>) -> Result<()> {
    let mut retry_count = 5;
    let mut error: String = String::new();
    loop {
        if retry_count < 1 {
            if !cfg!(debug_assertions) {
                println!("{:?}", error);
            }
            return Err(anyhow!(error));
        }
        match ping(port).await {
            Ok(_) => return Ok(()),
            Err(err) => {
                retry_count -= 1;
                error = err.to_string();
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

fn get_port(port: Option<u32>) -> u32 {
    if let Some(port_number) = port {
        port_number
    } else {
        DEFAULT_PORT
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serial_test::serial;

    use crate::daemon::{ping, start_server, stop_server, wait_until_server_is_up};

    #[tokio::test]
    #[serial]
    async fn test_sever_process_started_not_server() -> Result<()> {
        tokio::spawn(async move {
            let _ = start_server(None).await;
        });
        assert!(ping(None).await.is_err());
        stop_server(None).await
    }

    #[tokio::test]
    #[serial]
    async fn test_sever_process_and_server_started() -> Result<()> {
        tokio::spawn(async move {
            let _ = start_server(None).await;
        });
        wait_until_server_is_up(None).await?;
        assert!(ping(None).await.is_ok());

        stop_server(None).await
    }

    #[tokio::test]
    #[serial]
    async fn stop_server_but_server_not_up() {
        assert!(stop_server(None).await.is_err())
    }
}
