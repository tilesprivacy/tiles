//! The Demon that runs the core with his spear

use std::{
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, OnceLock},
    time::Duration,
};

use crate::{
    core::agent::pi::PiAgent,
    daemon::{
        account::account_router,
        agent::agent_router,
        atproto::atproto_router,
        authz::authz_router,
        model::model_router,
        net::{NetworkState, network_router, start_network_cli},
        server::server_router,
        session::session_router,
    },
};
use anyhow::{Result, anyhow};
use axum::{
    Json, Router,
    extract::{Query, Request, State, rejection::JsonRejection},
    http::{
        HeaderMap, HeaderValue, Method, StatusCode, Uri,
        header::{CONTENT_TYPE, HOST},
        uri::Authority,
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_macros::{FromRequest, debug_handler};
use iroh_tickets::endpoint::EndpointTicket;
use log::info;
use nix::unistd::setsid;
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::OpenOptions;
use std::sync::Mutex;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::OnceCell;
use tokio::sync::oneshot::{self, Receiver, Sender};
use tokio::sync::watch;
use tokio::{
    signal::unix::{SignalKind, signal},
    sync::RwLock as AsyncRwLock,
};
use tower_http::cors::CorsLayer;

pub mod account;
pub mod agent;
pub mod atproto;
pub mod authz;
pub mod model;
pub mod net;
pub mod server;
pub mod session;
mod sse;
pub mod web;
use crate::{
    core::{
        account::{atproto::AtCallbackParams, local::get_current_user},
        network::{self, create_endpoint, share},
        service,
        storage::db::get_db_conn,
        ui::{self, Ui},
    },
    utils::config::{
        ConfigProvider, DefaultProvider, ModelCacheError, get_config_json, get_model_cache,
    },
};

pub struct AppState {
    /// A watch rather than a one-shot: quit, `tiles daemon stop` and a SIGTERM
    /// from launchd all land here, and a one-shot panics on the second of them
    pub shutdown_sender: watch::Sender<bool>,
    pub vsn: String,
    //TODO: refactor the remote infy related fields
    pub remote_ticket: Mutex<Option<String>>,
    pub remote_running: Mutex<bool>,
    pub remote_shutdown_sender: Mutex<Option<oneshot::Sender<bool>>>,
    pub agent: AsyncMutex<Option<PiAgent>>,
    /// The model the running agent was started with. Kept outside the agent lock, so
    /// saving a chat never waits behind a streaming reply
    pub agent_model: Mutex<Option<String>>,
    /// The ATmosphere sign in waiting for the browser, if any
    pub login: AsyncMutex<Option<atproto::LoginTask>>,
    pub ui: Arc<Ui>,
    pub web_ui: web::WebUiState,
    pub models: model::Models,
    pub network_state: AsyncRwLock<Option<NetworkState>>,
}

#[derive(Deserialize, Serialize)]
pub struct SimpleResponse {
    pub message: String,
}
#[cfg(test)]
impl AppState {
    /// the routes under test never shut anything down, so the handles are inert
    pub fn for_tests() -> Self {
        Self {
            shutdown_sender: watch::channel(false).0,
            vsn: env!("CARGO_PKG_VERSION").to_owned(),
            remote_ticket: Mutex::new(None),
            remote_shutdown_sender: Mutex::new(None),
            remote_running: Mutex::new(false),
            agent: None.into(),
            agent_model: Mutex::new(None),
            login: None.into(),
            ui: Ui::new(),
            web_ui: web::WebUiState::Serving {
                url: format!("http://127.0.0.1:{}", web::UI_PORT),
            },
            models: model::Models::new(
                crate::utils::hf_model_downloader::Hub::new("http://127.0.0.1:9", None)
                    .expect("a test hub"),
                std::env::temp_dir(),
                std::env::temp_dir(),
                None,
            ),
            network_state: None.into(),
        }
    }
}

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    InternalServerError(String),
    RequestTimeout,
    BadRequest(String),
    AlreadyExists(String),
    CannotProcess(String),
    Forbidden(String),
    MethodNotAllowed(String),
    UnsupportedMediaType(String),
    ServiceUnavailable(String),
    InsufficientStorage(String),
}
impl AppError {
    fn parts(self) -> (StatusCode, String) {
        match self {
            Self::NotFound(e) => (StatusCode::NOT_FOUND, e),
            Self::InternalServerError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            Self::RequestTimeout => (StatusCode::REQUEST_TIMEOUT, "request timedout".to_string()),
            Self::BadRequest(e) => (StatusCode::BAD_REQUEST, e),
            Self::AlreadyExists(e) => (StatusCode::CONFLICT, e),
            Self::CannotProcess(e) => (StatusCode::UNPROCESSABLE_ENTITY, e),
            Self::Forbidden(e) => (StatusCode::FORBIDDEN, e),
            Self::MethodNotAllowed(e) => (StatusCode::METHOD_NOT_ALLOWED, e),
            Self::UnsupportedMediaType(e) => (StatusCode::UNSUPPORTED_MEDIA_TYPE, e),
            Self::ServiceUnavailable(e) => (StatusCode::SERVICE_UNAVAILABLE, e),
            Self::InsufficientStorage(e) => (StatusCode::INSUFFICIENT_STORAGE, e),
        }
    }

    pub fn reason(self) -> String {
        self.parts().1
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, reason) = self.parts();
        let body = Json(json!({
            "status": "failed",
            "reason": reason
        }));

        (status, body).into_response()
    }
}

// a body that does not parse fails in the same json shape as every other error
impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        match rejection.status() {
            StatusCode::UNPROCESSABLE_ENTITY => Self::CannotProcess(rejection.body_text()),
            StatusCode::UNSUPPORTED_MEDIA_TYPE => Self::UnsupportedMediaType(rejection.body_text()),
            _ => Self::BadRequest(rejection.body_text()),
        }
    }
}

/// `Json`, with its rejection turned into an `AppError`
#[derive(FromRequest)]
#[from_request(via(Json), rejection(AppError))]
pub struct ApiJson<T>(pub T);

#[derive(Serialize, Debug, Deserialize)]
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

const DEFAULT_DAEMON_PORT: u32 = 1729;

static DAEMON_PORT: OnceLock<u32> = OnceLock::new();

pub fn get_or_set_daemon_port() -> &'static u32 {
    DAEMON_PORT.get_or_init(|| {
        if cfg!(debug_assertions) {
            std::env::var("TILES_DAEMON_DEV_PORT")
                .ok()
                .and_then(|val| val.parse().ok())
                .unwrap_or(DEFAULT_DAEMON_PORT)
        } else {
            DEFAULT_DAEMON_PORT
        }
    })
}
/// the inference server is the slowest thing in a shutdown, and the only one
/// that can hang it
const INFERENCE_STOP_TIMEOUT: Duration = Duration::from_secs(10);
const DAEMON_STOP_TIMEOUT: Duration = Duration::from_secs(20);
/// how long open connections, like an SSE stream, get to finish after a shutdown
const SHUTDOWN_GRACE: Duration = Duration::from_secs(3);
const DAEMON_STOP_POLL: Duration = Duration::from_millis(250);
pub async fn start_cmd(port: Option<u32>) -> Result<()> {
    start_daemon(port).await
}

pub async fn stop_cmd() -> Result<()> {
    stop_server(None).await
}

pub async fn status_cmd() {
    match ping(None).await {
        Ok(version) => println!(
            "Daemon running, version {version}, at http://127.0.0.1:{}",
            get_port(None)
        ),
        Err(_) => {
            println!("Daemon not running. Start it with `tiles daemon start`");
            return;
        }
    }
    // a plain client, so asking doesn't start anything
    match web::fetch_state(&Client::new()).await {
        Some(web::WebUiState::Serving { url }) => println!("Web UI at {url}"),
        Some(web::WebUiState::PortInUse { reason }) => println!("Web UI not started: {reason}"),
        None => println!("Web UI state unknown"),
    }
}

/// Every route the daemon serves on its port
fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/config", get(get_config))
        .route("/shutdown", post(shutdown))
        .route("/model-cache-path", get(get_model_cache_path))
        .route("/remote-share", post(share_remote_inference))
        .route("/remote-unshare", post(unshare_remote_inference))
        .route("/remote-status", get(show_remote_status))
        .route("/connect-remote", post(connect_remote_inference))
        .merge(tilekit_router())
        .route_layer(middleware::from_fn(require_json))
        .fallback(no_route)
        .method_not_allowed_fallback(wrong_method)
        // .layer(service)
        .layer(cors_layer())
        // a request for another host is refused before anything else runs
        .layer(middleware::from_fn(require_local_host))
        .with_state(state)
}

/// Every `/v1/tilekit` route
fn tilekit_router() -> Router<Arc<AppState>> {
    Router::new()
        .merge(agent_router())
        .merge(server_router())
        .merge(account_router())
        .merge(session_router())
        .merge(atproto_router())
        .merge(model_router())
        .merge(authz_router())
        .merge(network_router())
        .route("/v1/tilekit/ui", get(web::ui_state))
}

const LOCAL_HOSTS: [&str; 3] = ["127.0.0.1", "localhost", "[::1]"];

/// Refuses requests not addressed to localhost, which stops DNS rebinding
async fn require_local_host(req: Request, next: Next) -> Result<Response, AppError> {
    if !is_local_host(&req) {
        return Err(AppError::Forbidden(
            "The Tiles daemon only answers requests made to localhost".to_owned(),
        ));
    }
    Ok(next.run(req).await)
}

fn is_local_host(req: &Request) -> bool {
    // http/2 carries it in the uri instead of a header
    let host = match req.headers().get(HOST) {
        Some(value) => value.to_str().ok(),
        None => req.uri().authority().map(Authority::as_str),
    };
    host.and_then(|host| host.parse::<Authority>().ok())
        .is_some_and(|authority| {
            LOCAL_HOSTS
                .iter()
                .any(|name| authority.host().eq_ignore_ascii_case(name))
        })
}

/// Every POST needs a JSON content type, which other websites can't send without a CORS preflight
async fn require_json(req: Request, next: Next) -> Result<Response, AppError> {
    if req.method() == Method::POST && !is_json(req.headers()) {
        return Err(AppError::UnsupportedMediaType(
            "Expected a `Content-Type: application/json` header".to_owned(),
        ));
    }
    Ok(next.run(req).await)
}

fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
}

/// The client for CLI calls to the daemon. Each call first makes sure the daemon is
/// up and on this version
pub async fn daemon_client() -> Result<&'static Client> {
    static CLIENT: OnceCell<Client> = OnceCell::const_new();
    // a running daemon on this version costs one ping
    start_cmd(None).await?;
    Ok(CLIENT.get_or_init(|| async { Client::new() }).await)
}

/// The daemon's address for `path`, to call through `daemon_client`
pub fn daemon_url(path: &str) -> String {
    let port = get_or_set_daemon_port();
    format!("http://127.0.0.1:{port}{path}")
}
async fn root(State(state): State<Arc<AppState>>) -> String {
    state.vsn.clone()
}

async fn no_route(uri: Uri) -> AppError {
    AppError::NotFound(format!("No route for {}", uri.path()))
}

async fn wrong_method(method: Method, uri: Uri) -> AppError {
    AppError::MethodNotAllowed(format!("{} is not allowed on {}", method, uri.path()))
}

// the app's webview and the vite dev server are on other origins
fn cors_layer() -> CorsLayer {
    let origins = [
        "tauri://localhost",
        "http://tauri.localhost",
        "http://localhost:5173",
        "http://127.0.0.1:5173",
    ]
    .map(HeaderValue::from_static);

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE])
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

    // a launchd-managed daemon has to come back through launchd, or the service
    // is left pointing at a process that is gone
    if service::is_installed() {
        service::start()?;
        return wait_until_server_is_up(port).await;
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

pub async fn start_server(port: Option<u32>, with_ui: bool) -> Result<()> {
    let dyn_port: u32 = get_port(port);

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let ui = Ui::new();
    let (web_ui_state, web_ui_listener) = web::bind().await;
    let models = model::Models::for_daemon()?;

    let state = AppState {
        shutdown_sender: shutdown_tx,
        vsn: env!("CARGO_PKG_VERSION").to_owned(),
        remote_ticket: Mutex::new(None),
        remote_shutdown_sender: Mutex::new(None),
        remote_running: Mutex::new(false),
        agent: None.into(),
        agent_model: Mutex::new(None),
        login: None.into(),
        ui: ui.clone(),
        web_ui: web_ui_state,
        models,
        network_state: None.into(),
    };

    let shared_state = Arc::new(state);

    let app = router(shared_state.clone());

    let addr = format!("127.0.0.1:{}", dyn_port);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Daemon server started at {}", dyn_port);

    tokio::spawn(async move {
        start_network_cli().await?;
        anyhow::Result::<()>::Ok(())
    });

    if with_ui {
        ui::start(ui);
    }
    listen_for_signals(shared_state.clone());

    let web_ui = match web_ui_listener {
        Some(listener) => Some((listener, web::router(shared_state.clone(), web::ui_dir()?))),
        None => None,
    };
    let served = serve_ports((listener, app), web_ui, shutdown_rx, SHUTDOWN_GRACE).await;
    if served.is_err() {
        shutdown_all(shared_state).await;
    }
    Ok(served?)
}

/// Serves the daemon's port, and the web UI's when it has one, until a shutdown. A
/// connection still open `grace` after it, like an SSE stream, is cut, so it can't
/// keep the daemon running.
async fn serve_ports(
    api: (tokio::net::TcpListener, Router),
    web_ui: Option<(tokio::net::TcpListener, Router)>,
    shutdown: watch::Receiver<bool>,
    grace: Duration,
) -> std::io::Result<()> {
    let serve_api =
        axum::serve(api.0, api.1).with_graceful_shutdown(watch_shutdown(shutdown.clone()));
    let web_ui_shutdown = shutdown.clone();
    let serve_web_ui = async move {
        if let Some((listener, app)) = web_ui
            && let Err(err) = axum::serve(listener, app)
                .with_graceful_shutdown(watch_shutdown(web_ui_shutdown))
                .await
        {
            log::error!("Web UI port stopped: {err}");
        }
        std::future::pending::<()>().await
    };
    let cut_off = async {
        watch_shutdown(shutdown).await;
        tokio::time::sleep(grace).await;
    };

    tokio::select! {
        served = serve_api => served,
        () = serve_web_ui => Ok(()),
        () = cut_off => {
            log::warn!("Closing connections still open {}s after shutdown", grace.as_secs());
            Ok(())
        }
    }
}

/// launchd's bootout and a logout both arrive as SIGTERM, which nothing here
/// used to listen for, so every managed stop was an abrupt kill
fn listen_for_signals(state: Arc<AppState>) {
    tokio::spawn(async move {
        let (mut term, mut int) = match (
            signal(SignalKind::terminate()),
            signal(SignalKind::interrupt()),
        ) {
            (Ok(term), Ok(int)) => (term, int),
            _ => {
                log::error!("Failed to listen for shutdown signals");
                return;
            }
        };
        tokio::select! {
            _ = term.recv() => info!("SIGTERM received"),
            _ = int.recv() => info!("SIGINT received"),
        }
        shutdown_all(state).await;
    });
}

/// The one teardown every stop goes through. The inference server is first
/// because it runs in its own process group and outlives the rest otherwise
pub async fn shutdown_all(state: Arc<AppState>) {
    info!("Daemon server shutting down");
    ui::begin_stop(&state.ui);
    // so they show as paused next time
    state.models.pause_all().await;
    // its ping has no timeout of its own, and a stuck server must not hold the
    // shutdown open forever
    match tokio::time::timeout(
        INFERENCE_STOP_TIMEOUT,
        crate::core::server::stop_server_daemon(),
    )
    .await
    {
        Ok(Err(err)) => log::warn!("Inference server did not stop cleanly: {err:?}"),
        Err(_) => log::warn!("Inference server did not answer the stop in time"),
        Ok(Ok(_)) => {}
    }
    ui::stop(&state.ui).await;
    let _ = state.shutdown_sender.send(true);
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
        .with_state(shared_state.clone());

    let addr = format!("127.0.0.1:{}", dyn_port);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Internal server started at {}", dyn_port);
    let _ = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_rx))
        .await;

    Ok(())
}

async fn shutdown_signal(rx: Receiver<bool>) {
    // a dropped sender means the sign in was cancelled or timed out, which is a stop too
    let _ = rx.await;
}

async fn watch_shutdown(mut rx: watch::Receiver<bool>) {
    while rx.changed().await.is_ok() {
        if *rx.borrow() {
            return;
        }
    }
}

/// Teardown runs detached so the caller gets its response before we stop serving
async fn shutdown(State(state): State<Arc<AppState>>) {
    tokio::spawn(shutdown_all(state));
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
    if let Some(sender) = callback_sender.take() {
        let _ = sender.send(params);
    }
    let mut shutdown_sender = state
        .shutdown_sender
        .lock()
        .expect("Failed to get shutdown sender lock");
    if let Some(sender) = shutdown_sender.take() {
        let _ = sender.send(true);
    }
    "Processed your authorization request, You can close this page"
}

async fn get_model_cache_path(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<SendParams>,
) -> Result<String, AppError> {
    log::info!("getting model cache path");
    let model_path = get_model_cache(&params.model_name).map_err(|e| {
        let reason = format!("{}: {e}", params.model_name);
        match e.downcast_ref::<ModelCacheError>() {
            Some(ModelCacheError::InvalidSpec) => AppError::BadRequest(reason),
            Some(ModelCacheError::NotCached) => AppError::NotFound(reason),
            None => AppError::InternalServerError(reason),
        }
    })?;
    model_path.to_str().map(str::to_owned).ok_or_else(|| {
        AppError::InternalServerError(format!(
            "The cache path for {} is not valid UTF-8",
            params.model_name
        ))
    })
}

/// Gets the contents of config.toml in json
async fn get_config(State(_state): State<Arc<AppState>>) -> Result<String, AppError> {
    get_config_json()
        .and_then(|config| serde_json::to_string(&config).map_err(Into::into))
        .map_err(|e| AppError::InternalServerError(format!("Failed to read config.toml: {e}")))
}

async fn stop_server(port: Option<u32>) -> Result<()> {
    stop_server_with_timeout(port, DAEMON_STOP_TIMEOUT).await
}

async fn stop_server_with_timeout(port: Option<u32>, timeout: Duration) -> Result<()> {
    tokio::time::timeout(timeout, stop_server_and_wait(port))
        .await
        .map_err(|_| anyhow!("Timed out waiting for Tiles daemon to stop"))?
}

async fn stop_server_and_wait(port: Option<u32>) -> Result<()> {
    let dyn_port = get_port(port);
    let client = Client::new();
    let addr = format!("http://127.0.0.1:{}/shutdown", dyn_port);
    client
        .post(&addr)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await
        .map_err(|err| anyhow!("Daemon shutdown failed due to {err:?}"))?
        .error_for_status()
        .map_err(|err| anyhow!("Daemon shutdown failed due to {err:?}"))?;

    let addr = format!("http://127.0.0.1:{dyn_port}");
    loop {
        if client.get(&addr).send().await.is_err() {
            return Ok(());
        }
        tokio::time::sleep(DAEMON_STOP_POLL).await;
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
        *get_or_set_daemon_port()
    }
}

//TODO: handle the api responses correctly with correct msgs later..
#[debug_handler]
async fn share_remote_inference(State(state): State<Arc<AppState>>) -> Result<String, AppError> {
    //TODO: Handle these network stuff in network module pleje

    if *state
        .remote_running
        .lock()
        .expect("Failed to get lock on remote_running")
    {
        return Err(AppError::AlreadyExists(
            "Remote inference is already being shared".to_owned(),
        ));
    }

    let user_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    let user = get_current_user(&user_db_conn)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;
    let endpoint = create_endpoint(&user).await.map_err(|e| {
        AppError::InternalServerError(format!("Failed to create the endpoint: {e}"))
    })?;
    endpoint.set_alpns(vec![ALPN.to_vec()]);
    endpoint.online().await;
    let addr = endpoint.addr();
    let ticket = EndpointTicket::from(addr).to_string();
    info!("Sharing remote inference, waiting for connections");
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
    // nothing shared is fine, unsharing just leaves it that way
    if let Some(sender) = remote_shutdown_sender.take() {
        let _ = sender.send(true);
    }
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
    let res = daemon_client()
        .await?
        .post(daemon_url("/remote-share"))
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await;
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
    let res = daemon_client()
        .await?
        .post(daemon_url("/remote-unshare"))
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await;

    match res {
        Err(err) => Err(anyhow!("Daemon remote unshare failed due to {:?}", err)),
        _ => Ok(()),
    }
}

pub async fn remote_status() -> Result<String> {
    let res = daemon_client()
        .await?
        .get(daemon_url("/remote-status"))
        .send()
        .await;

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
    let res = daemon_client()
        .await?
        .post(daemon_url("/connect-remote"))
        .query(&[("ticket", ticket)])
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await;

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
    use std::time::Duration;

    use anyhow::Result;
    use axum::{
        Router,
        http::StatusCode,
        routing::{get, post},
    };
    use serial_test::serial;

    use crate::daemon::{
        AppState, ping, router, start_server, stop_server, stop_server_with_timeout,
        wait_until_server_is_up,
    };
    use axum::{
        body::Body,
        http::{
            Request,
            header::{CONTENT_TYPE, HOST},
        },
    };
    use http_body_util::BodyExt;
    use serde_json::Value;
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    #[serial]
    async fn test_sever_process_and_server_started() -> Result<()> {
        tokio::spawn(async move {
            let _ = start_server(None, false).await;
        });
        wait_until_server_is_up(None).await?;
        assert!(ping(None).await.is_ok());

        stop_server(None).await?;
        assert!(ping(None).await.is_err());
        Ok(())
    }

    async fn serve(app: Router) -> Result<(u32, tokio::task::JoinHandle<()>)> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port() as u32;
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Ok((port, task))
    }

    #[tokio::test]
    async fn stop_rejects_an_unsuccessful_response() -> Result<()> {
        let app = Router::new().route(
            "/shutdown",
            post(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        );
        let (port, task) = serve(app).await?;

        let result = stop_server_with_timeout(Some(port), Duration::from_secs(1)).await;
        task.abort();

        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn stop_times_out_while_the_server_remains_reachable() -> Result<()> {
        let app = Router::new()
            .route("/", get(|| async { "up" }))
            .route("/shutdown", post(|| async {}));
        let (port, task) = serve(app).await?;

        let result = stop_server_with_timeout(Some(port), Duration::from_millis(100)).await;
        task.abort();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Timed out"));
        Ok(())
    }

    #[tokio::test]
    async fn state_changing_root_routes_refuse_get() {
        for path in [
            "/shutdown",
            "/remote-share",
            "/remote-unshare",
            "/connect-remote",
        ] {
            let (status, _) = call("GET", path, None).await;
            assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{path}");
        }
    }

    async fn call(method: &str, path: &str, content_type: Option<&str>) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .header(HOST, "127.0.0.1:1729");
        if let Some(content_type) = content_type {
            req = req.header(CONTENT_TYPE, content_type);
        }
        let res = router(Arc::new(AppState::for_tests()))
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&body).unwrap())
    }

    #[tokio::test]
    async fn a_post_without_json_content_type_is_refused_before_the_handler() {
        // server/stop would stop the inference server if it got through
        for content_type in [
            None,
            Some("text/plain"),
            Some("application/x-www-form-urlencoded"),
        ] {
            let (status, body) = call("POST", "/v1/tilekit/server/stop", content_type).await;
            assert_eq!(
                status,
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "{content_type:?}"
            );
            assert_eq!(body["status"], "failed");
        }
    }

    #[tokio::test]
    async fn a_post_with_json_content_type_reaches_the_handler() {
        // with no agent running, end_session's own error proves the request got through
        for content_type in ["application/json", "Application/JSON; charset=utf-8"] {
            let (status, body) =
                call("POST", "/v1/tilekit/agent/end_session", Some(content_type)).await;
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{content_type}");
            assert!(body["reason"].as_str().unwrap().contains("agent"));
        }
    }

    #[tokio::test]
    async fn state_changing_agent_routes_refuse_get() {
        for path in ["/v1/tilekit/agent/start", "/v1/tilekit/agent/end_session"] {
            let (status, body) = call("GET", path, None).await;
            assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{path}");
            assert_eq!(body["status"], "failed");
        }
    }

    #[tokio::test]
    async fn an_unknown_path_is_a_404_whatever_its_content_type() {
        let (status, _) = call("POST", "/v1/tilekit/nope", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    async fn status_for_host(host: Option<&str>) -> StatusCode {
        let mut req = Request::builder().uri("/");
        if let Some(host) = host {
            req = req.header(HOST, host);
        }
        router(Arc::new(AppState::for_tests()))
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status()
    }

    #[tokio::test]
    async fn local_hosts_are_answered_on_any_port() {
        for host in [
            "127.0.0.1:1729",
            "localhost:1729",
            "[::1]:1729",
            "LocalHost:1730",
            "127.0.0.1",
        ] {
            assert_eq!(status_for_host(Some(host)).await, StatusCode::OK, "{host}");
        }
    }

    #[tokio::test]
    async fn other_hosts_are_refused() {
        // a rebound domain keeps its own name, whatever address it points at
        for host in [
            Some("evil.example:1729"),
            Some("127.0.0.1.evil.example:1729"),
            Some("localhost.evil.example"),
            Some("127.0.0.2:1729"),
            Some("not a host"),
            None,
        ] {
            assert_eq!(
                status_for_host(host).await,
                StatusCode::FORBIDDEN,
                "{host:?}"
            );
        }
    }

    #[tokio::test]
    async fn an_unknown_path_for_another_host_is_refused_too() {
        let req = Request::builder()
            .method("POST")
            .uri("/v1/tilekit/nope")
            .header(HOST, "evil.example:1729")
            .body(Body::empty())
            .unwrap();
        let res = router(Arc::new(AppState::for_tests()))
            .oneshot(req)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["status"], "failed");
    }

    async fn open_ports(app: Router) -> (tokio::net::TcpListener, Router, String) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        (listener, app, url)
    }

    #[tokio::test]
    async fn an_open_stream_cannot_keep_the_daemon_running() {
        let never_answers = Router::new().route("/", get(std::future::pending::<()>));
        let (listener, app, url) = open_ports(never_answers).await;
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let grace = Duration::from_millis(300);
        let served = tokio::spawn(super::serve_ports(
            (listener, app),
            None,
            shutdown_rx,
            grace,
        ));

        let held = tokio::spawn(async move { reqwest::get(url).await });
        tokio::time::sleep(Duration::from_millis(200)).await;
        shutdown_tx.send(true).unwrap();

        let result = tokio::time::timeout(grace * 5, served).await;
        held.abort();
        assert!(
            result
                .expect("still serving after the grace")
                .unwrap()
                .is_ok()
        );
    }

    #[tokio::test]
    async fn a_shutdown_with_nothing_open_ends_at_once() {
        let app = Router::new().route("/", get(|| async { "up" }));
        let (listener, app, _) = open_ports(app).await;
        let (web_listener, web_app, _) = open_ports(Router::new()).await;
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let grace = Duration::from_secs(30);
        let served = tokio::spawn(super::serve_ports(
            (listener, app),
            Some((web_listener, web_app)),
            shutdown_rx,
            grace,
        ));

        shutdown_tx.send(true).unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), served).await;
        assert!(result.expect("waited for the grace").unwrap().is_ok());
    }
}
