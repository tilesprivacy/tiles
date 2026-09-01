//! The Demon that runs the core with his spear

use std::{
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::Arc,
    time::Duration,
};

use crate::{
    core::agent::pi::PiAgent,
    daemon::{
        account::account_router, agent::agent_router, server::server_router,
        session::session_router,
    },
};
use anyhow::{Result, anyhow};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use axum_macros::debug_handler;
use iroh_tickets::endpoint::EndpointTicket;
use log::info;
use nix::unistd::setsid;
use reqwest::Client;
use semver::Version;
use serde::Serialize;
use serde_json::json;
use std::fs::OpenOptions;
use std::sync::Mutex;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::oneshot::{self, Receiver, Sender};

pub mod account;
pub mod agent;
pub mod server;
pub mod session;

use crate::{
    core::{
        account::{atproto::AtCallbackParams, local::get_current_user},
        network::{self, create_endpoint, share},
        storage::db::get_db_conn,
    },
    utils::config::{ConfigProvider, DefaultProvider, get_config_json, get_model_cache},
};

pub struct AppState {
    pub shutdown_sender: Mutex<Option<oneshot::Sender<bool>>>,
    pub vsn: String,
    //TODO: refactor the remote infy related fields
    pub remote_ticket: Mutex<Option<String>>,
    pub remote_running: Mutex<bool>,
    pub remote_shutdown_sender: Mutex<Option<oneshot::Sender<bool>>>,
    pub agent: AsyncMutex<Option<PiAgent>>,
}

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    InternalServerError(String),
    RequestTimeout,
    BadRequest(String),
    AlreadyExists(String),
    CannotProcess(String),
}
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, reason) = match self {
            Self::NotFound(e) => (StatusCode::NOT_FOUND, e),
            Self::InternalServerError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            Self::RequestTimeout => (StatusCode::REQUEST_TIMEOUT, "request timedout".to_string()),
            Self::BadRequest(e) => (StatusCode::BAD_REQUEST, e),
            Self::AlreadyExists(e) => (StatusCode::CONFLICT, e),
            Self::CannotProcess(e) => (StatusCode::UNPROCESSABLE_ENTITY, e),
        };

        let body = Json(json!({
            "status": "failed",
            "reason": reason
        }));

        (status, body).into_response()
    }
}

#[derive(Serialize, Debug)]
pub struct ApiResponse<T> {
    status: String,
    data: T,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> Json<Self> {
        Json(Self {
            status: "success".to_string(),
            data,
        })
    }
}

pub struct ApiCleanupGuard;

impl Drop for ApiCleanupGuard {
    fn drop(&mut self) {
        log::info!("Dropping the request")
    }
}

struct InternalAppState {
    pub callback_sender: Mutex<Option<oneshot::Sender<AtCallbackParams>>>,
    pub shutdown_sender: Mutex<Option<oneshot::Sender<bool>>>,
}

#[derive(serde::Deserialize)]
pub struct SendParams {
    model_name: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct RemoteConnectParams {
    ticket: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct RemoteStatus {
    running: bool,
    ticket: Option<String>,
}
const ALPN: &[u8] = b"remote-link/v1";

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
        // "Its me" check is just there for backward compatibility, prolly we will remove in future versions
        if daemon_current_vsn.contains("Its me")
            || app_vsn
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
        PathBuf::from("target/debug/tiles")
    } else {
        std::env::current_exe().unwrap_or_else(|_| PathBuf::from("tiles"))
    };
    let _process = unsafe {
        Command::new(base_command)
            .env("RUST_LOG", "info,iroh=error,tracing=off")
            .arg("daemon")
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
            .expect("Failed to start daemon")
    };

    wait_until_server_is_up(port).await
}

pub async fn start_server(port: Option<u32>) -> Result<()> {
    let dyn_port: u32 = get_port(port);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<bool>();

    let state = AppState {
        shutdown_sender: Mutex::new(Some(shutdown_tx)),
        vsn: env!("CARGO_PKG_VERSION").to_owned(),
        remote_ticket: Mutex::new(None),
        remote_shutdown_sender: Mutex::new(None),
        remote_running: Mutex::new(false),
        agent: None.into(),
    };

    let shared_state = Arc::new(state);

    // let service = ServiceBuilder::new()
    //     .layer(HandleErrorLayer::new(handle_timeout_error))
    //     .layer(TimeoutLayer::new(Duration::from_secs(30)));

    let app = Router::new()
        .route("/", get(root))
        .route("/config", get(get_config))
        .route("/shutdown", get(shutdown))
        .route("/model-cache-path", get(get_model_cache_path))
        .route("/remote-share", get(share_remote_inference))
        .route("/remote-unshare", get(unshare_remote_inference))
        .route("/remote-status", get(show_remote_status))
        .route("/connect-remote", get(connect_remote_inference))
        .merge(agent_router())
        .merge(server_router())
        .merge(account_router())
        .merge(session_router())
        // .layer(service)
        .with_state(shared_state);

    let addr = format!("127.0.0.1:{}", dyn_port);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Daemon server started at {}", dyn_port);
    let _ = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_rx))
        .await;

    Ok(())
}

pub async fn start_internal_server(
    port: Option<u32>,
    callback_tx: Sender<AtCallbackParams>,
) -> Result<()> {
    let dyn_port: u32 = get_port(port);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<bool>();

    let state = InternalAppState {
        callback_sender: Mutex::new(Some(callback_tx)),
        shutdown_sender: Mutex::new(Some(shutdown_tx)),
    };
    let shared_state = Arc::new(state);
    let app = Router::new()
        .route("/callback", get(callback))
        .with_state(shared_state);

    let addr = format!("127.0.0.1:{}", dyn_port);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Internal server started at {}", dyn_port);
    let _ = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_rx))
        .await;

    Ok(())
}

async fn shutdown_signal(rx: Receiver<bool>) {
    rx.await.expect("shutdown receiver paniced");
}

async fn shutdown(State(state): State<Arc<AppState>>) {
    log::info!("Daemon server shutting down");
    let mut sender = state.shutdown_sender.lock().unwrap();
    let sender_real = sender.take().unwrap();
    let _ = sender_real.send(true);
}

#[debug_handler]
async fn callback(
    State(state): State<Arc<InternalAppState>>,
    Query(params): Query<AtCallbackParams>,
) -> &'static str {
    info!("callback reached {:?}", params);
    let mut callback_sender = state
        .callback_sender
        .lock()
        .expect("Failed to get the callback params sender lock");
    let _ = callback_sender.take().unwrap().send(params);
    let mut shutdown_sender = state
        .shutdown_sender
        .lock()
        .expect("Failed to get shutdown sender lock");
    let _ = shutdown_sender.take().unwrap().send(true);
    "Processed your authorization request, You can close this page"
}

async fn get_model_cache_path(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<SendParams>,
) -> Result<String, StatusCode> {
    log::info!("getting model cache path");
    if let Ok(model_path) = get_model_cache(&params.model_name) {
        Ok(model_path
            .to_str()
            .expect("Pathbuf to str failed")
            .to_owned())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Gets the contents of config.toml in json
async fn get_config(State(_state): State<Arc<AppState>>) -> Result<String, StatusCode> {
    get_config_json()
        .and_then(|config| serde_json::to_string(&config).map_err(Into::into))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

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
            log::error!("{:?}", error);
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

//TODO: handle the api responses correctly with correct msgs later..
#[debug_handler]
async fn share_remote_inference(State(state): State<Arc<AppState>>) -> Result<String, StatusCode> {
    //TODO: Handle these network stuff in network module pleje

    if *state
        .remote_running
        .lock()
        .expect("Failed to get lock on remote_running")
    {
        return Err(StatusCode::CONFLICT);
    }

    let user_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|_e| StatusCode::INTERNAL_SERVER_ERROR)?;
    let user = get_current_user(&user_db_conn).map_err(|_e| StatusCode::INTERNAL_SERVER_ERROR)?;
    let endpoint = create_endpoint(&user)
        .await
        .map_err(|_e| StatusCode::INTERNAL_SERVER_ERROR)?;
    endpoint.set_alpns(vec![ALPN.to_vec()]);
    endpoint.online().await;
    let addr = endpoint.addr();
    let ticket = EndpointTicket::from(addr).to_string();
    println!(" Here the remote connect ticket\n{}", ticket);
    println!("\nUse `tiles remote connect <ticket>` on the other peer\n");
    println!("Waiting for connections...");
    // ok we create the endpoint and ticket here, then pass the endpoint to share fn
    let (sendx, recvx) = oneshot::channel();

    //TODO: Need to handle the errors from the child processes
    tokio::spawn(async move {
        share(endpoint, recvx).await?;
        Result::<()>::Ok(())
    });

    *state
        .remote_ticket
        .lock()
        .expect("failed to get remote_ticket lock") = Some(ticket.clone());

    *state
        .remote_running
        .lock()
        .expect("failed to get remote_running lock") = true;

    *state
        .remote_shutdown_sender
        .lock()
        .expect("Failed to get remote shudown sender lock") = Some(sendx);

    Ok(ticket.clone())
}

async fn unshare_remote_inference(
    State(state): State<Arc<AppState>>,
) -> Result<String, StatusCode> {
    let mut remote_shutdown_sender = state
        .remote_shutdown_sender
        .lock()
        .expect("Failed to get shutdown sender lock");
    let _ = remote_shutdown_sender.take().unwrap().send(true);
    *state
        .remote_running
        .lock()
        .expect("failed to get remote_running lock") = false;
    *state
        .remote_ticket
        .lock()
        .expect("failed to get remote_ticket lock") = None;
    Ok(String::from("shutdown inference proxy"))
}

#[debug_handler]
async fn show_remote_status(State(state): State<Arc<AppState>>) -> Result<String, StatusCode> {
    let mut remote_ticket = state
        .remote_ticket
        .lock()
        .expect("Failed to get shutdown sender lock");
    let remote_ticket_str = remote_ticket.take();
    *remote_ticket = remote_ticket_str.clone();
    let is_running = *state
        .remote_running
        .lock()
        .expect("lock fail remote_running");

    Ok(serde_json::to_string(&RemoteStatus {
        running: is_running,
        ticket: remote_ticket_str,
    })
    .unwrap())
}

pub async fn share_remote_link() -> Result<String> {
    let client = Client::new();
    let addr = "http://127.0.0.1:1729/remote-share";
    let res = client.get(addr).send().await;
    match res {
        Err(err) => Err(anyhow!("Daemon remote share failed due to {:?}", err)),
        Ok(response) => {
            if let StatusCode::CONFLICT = response.status() {
                println!("Remote inference is already shared. Use `tiles remote status`");
                Ok(String::from(""))
            } else {
                let ticket = response.text().await?;
                Ok(ticket)
            }
        }
    }
}

pub async fn unshare_remote_link() -> Result<()> {
    let client = Client::new();
    let addr = "http://127.0.0.1:1729/remote-unshare";
    let res = client.get(addr).send().await;

    match res {
        Err(err) => Err(anyhow!("Daemon remote unshare failed due to {:?}", err)),
        _ => Ok(()),
    }
}

pub async fn remote_status() -> Result<String> {
    let client = Client::new();
    let addr = "http://127.0.0.1:1729/remote-status";
    let res = client.get(addr).send().await;

    match res {
        Err(err) => Err(anyhow!(
            "Daemon remote share ticket failed due to {:?}",
            err
        )),
        Ok(response) => {
            let ticket = response.text().await?;
            Ok(ticket)
        }
    }
}

async fn connect_remote_inference(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<RemoteConnectParams>,
) -> Result<(), StatusCode> {
    tokio::spawn(async move {
        let _ = network::connect(&params.ticket)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
        Result::<()>::Ok(())
    });
    Ok(())
}

pub async fn connect_remote(ticket: &str) -> Result<()> {
    let client = Client::new();
    let addr = format!("http://127.0.0.1:1729/connect-remote?ticket={}", ticket);
    let res = client.get(addr).send().await;

    match res {
        Err(err) => Err(anyhow!("Daemon remote connect failed due to {:?}", err)),
        Ok(_response) => Ok(()),
    }
}

// async fn handle_timeout_error(err: BoxError) -> AppError {
//     if err.is::<timeout::error::Elapsed>() {
//         AppError::RequestTimeout
//     } else {
//         AppError::InternalServerError("Something unexpected happened".to_string())
//     }
// }

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serial_test::serial;

    use crate::daemon::{ping, start_server, stop_server, wait_until_server_is_up};

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
}
