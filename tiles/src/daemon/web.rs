//! Serves the web UI and the Tilekit API on port 1730

use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Result, bail};

use axum::{
    Router,
    extract::State,
    http::{
        HeaderValue,
        header::{CACHE_CONTROL, CONTENT_SECURITY_POLICY, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS},
    },
    middleware,
    response::IntoResponse,
    routing::any,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::TcpListener;
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};

use crate::{
    daemon::{
        ApiResponse, AppState, daemon_client, daemon_url, no_route, require_json,
        require_local_host, tilekit_router, wrong_method,
    },
    utils::{
        config::{ConfigProvider, DefaultProvider},
        open_in_browser,
    },
};

pub const UI_PORT: u32 = 1730;

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Overrides where the UI is stored, for development
const UI_DIR_ENV: &str = "TILES_UI_DIR";

// only what the daemon serves, so a model reply can't leak chat text through an image url
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
    img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; \
    object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'";

/// `TILES_UI_DIR`, else `ui` in the lib dir
pub fn ui_dir() -> Result<PathBuf> {
    match std::env::var_os(UI_DIR_ENV) {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => Ok(DefaultProvider.get_lib_dir()?.join("ui")),
    }
}

/// Whether the web UI's port came up
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum WebUiState {
    Serving { url: String },
    PortInUse { reason: String },
}

/// Binds the web UI's port. If it's taken, the daemon runs without the web UI
pub async fn bind() -> (WebUiState, Option<TcpListener>) {
    bind_on(UI_PORT).await
}

async fn bind_on(port: u32) -> (WebUiState, Option<TcpListener>) {
    match TcpListener::bind(format!("127.0.0.1:{port}")).await {
        Ok(listener) => {
            let url = format!("http://127.0.0.1:{port}");
            log::info!("Web UI at {url}");
            (WebUiState::Serving { url }, Some(listener))
        }
        Err(err) => {
            let reason = format!("Port {port} is in use by another program ({err})");
            log::warn!("Web UI not started: {reason}");
            (WebUiState::PortInUse { reason }, None)
        }
    }
}

/// Where the web UI is, or why it isn't running
pub async fn ui_state(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ApiResponse::success(state.web_ui.clone())
}

/// Asks the daemon what became of the web UI. `None` when it can't say
pub async fn fetch_state(client: &Client) -> Option<WebUiState> {
    let res = client
        .get(daemon_url("/v1/tilekit/ui"))
        .timeout(PROBE_TIMEOUT)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?;
    let mut body: Value = res.json().await.ok()?;
    serde_json::from_value(body["data"].take()).ok()
}

/// Opens the web UI once the daemon is up and on this version
pub async fn open_cli() -> Result<()> {
    let client = daemon_client().await?;
    match fetch_state(client).await {
        Some(WebUiState::Serving { url }) => {
            println!("Opening {url}");
            open_in_browser(&url);
            Ok(())
        }
        Some(WebUiState::PortInUse { reason }) => bail!(
            "The Tiles web UI couldn't start: {reason}. Free the port, then restart Tiles with `tiles daemon stop` and `tiles ui`"
        ),
        None => bail!("Couldn't ask the Tiles daemon about the web UI"),
    }
}

pub fn router(state: Arc<AppState>, dir: PathBuf) -> Router {
    // a path the UI handles itself, like /chat/<id>, gets index.html
    let files = ServeDir::new(&dir).fallback(ServeFile::new(dir.join("index.html")));

    tilekit_router()
        .route_layer(middleware::from_fn(require_json))
        .route("/v1/{*rest}", any(no_route))
        .method_not_allowed_fallback(wrong_method)
        .fallback_service(files)
        // files are on local disk, so revalidating is cheap
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CSP),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(middleware::from_fn(require_local_host))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use axum::{
        body::Body,
        http::{
            Request, StatusCode,
            header::{CONTENT_TYPE, HOST},
        },
        response::Response,
    };
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tempfile::{TempDir, tempdir};
    use tower::ServiceExt;

    use super::*;

    const INDEX: &str = "<!doctype html><title>Tiles</title>";

    /// A fake UI build in a temp folder, deleted when the test ends
    struct UiFolder(TempDir);

    impl UiFolder {
        fn new() -> Self {
            let tmp = tempdir().unwrap();
            fs::create_dir_all(tmp.path().join("assets")).unwrap();
            fs::write(tmp.path().join("index.html"), INDEX).unwrap();
            fs::write(tmp.path().join("assets/app.js"), "console.log(1)").unwrap();
            Self(tmp)
        }

        fn path(&self) -> PathBuf {
            self.0.path().to_path_buf()
        }
    }

    async fn call(ui: &UiFolder, method: &str, path: &str, content_type: Option<&str>) -> Response {
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .header(HOST, "127.0.0.1:1730");
        if let Some(content_type) = content_type {
            req = req.header(CONTENT_TYPE, content_type);
        }
        router(Arc::new(AppState::for_tests()), ui.path())
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn text(res: Response) -> String {
        let body = res.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(body.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn pages_the_ui_handles_get_index_html() {
        let ui = UiFolder::new();
        for path in ["/", "/chat/abc"] {
            let res = call(&ui, "GET", path, None).await;
            assert_eq!(res.status(), StatusCode::OK, "{path}");
            assert_eq!(res.headers()[CACHE_CONTROL], "no-cache");
            assert_eq!(res.headers()[CONTENT_SECURITY_POLICY], CSP);
            assert_eq!(res.headers()[X_CONTENT_TYPE_OPTIONS], "nosniff");
            assert_eq!(text(res).await, INDEX);
        }
    }

    #[tokio::test]
    async fn files_are_served_as_they_are() {
        let ui = UiFolder::new();
        let res = call(&ui, "GET", "/assets/app.js", None).await;
        assert_eq!(res.status(), StatusCode::OK);
        assert!(
            res.headers()[CONTENT_TYPE]
                .to_str()
                .unwrap()
                .contains("javascript")
        );
        assert_eq!(text(res).await, "console.log(1)");
    }

    #[tokio::test]
    async fn api_errors_stay_json() {
        let ui = UiFolder::new();
        for (method, path, status) in [
            ("GET", "/v1/tilekit/nope", StatusCode::NOT_FOUND),
            ("POST", "/v1/tilekit/nope", StatusCode::NOT_FOUND),
            (
                "GET",
                "/v1/tilekit/agent/end_session",
                StatusCode::METHOD_NOT_ALLOWED,
            ),
        ] {
            let res = call(&ui, method, path, None).await;
            assert_eq!(res.status(), status, "{method} {path}");
            let body: Value = serde_json::from_str(&text(res).await).unwrap();
            assert_eq!(body["status"], "failed");
        }
    }

    #[tokio::test]
    async fn the_api_is_reachable_with_its_rules() {
        let ui = UiFolder::new();
        let refused = call(&ui, "POST", "/v1/tilekit/agent/end_session", None).await;
        assert_eq!(refused.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

        // with no agent running, end_session's own error proves the request got through
        let res = call(
            &ui,
            "POST",
            "/v1/tilekit/agent/end_session",
            Some("application/json"),
        )
        .await;
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(text(res).await.contains("agent"));
    }

    #[tokio::test]
    async fn root_routes_are_not_on_the_ui_port() {
        let ui = UiFolder::new();
        // a POST to /shutdown reaches the files, which only take GET and HEAD
        let res = call(&ui, "POST", "/shutdown", Some("application/json")).await;
        assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);

        // and a GET is the ui, not the daemon's version
        let res = call(&ui, "GET", "/remote-status", None).await;
        assert_eq!(text(res).await, INDEX);
    }

    #[tokio::test]
    async fn other_hosts_are_refused() {
        let ui = UiFolder::new();
        let req = Request::builder()
            .uri("/")
            .header(HOST, "evil.example:1730")
            .body(Body::empty())
            .unwrap();
        let res = router(Arc::new(AppState::for_tests()), ui.path())
            .oneshot(req)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn bind_reports_a_taken_port() {
        let taken = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = taken.local_addr().unwrap().port() as u32;
        let (state, listener) = bind_on(port).await;
        assert!(
            matches!(state, WebUiState::PortInUse { ref reason } if reason.contains(&port.to_string()))
        );
        assert!(listener.is_none());

        drop(taken);
        let (state, listener) = bind_on(port).await;
        assert_eq!(
            state,
            WebUiState::Serving {
                url: format!("http://127.0.0.1:{port}")
            }
        );
        assert!(listener.is_some());
    }

    #[tokio::test]
    async fn the_daemon_reports_the_ui_state() {
        let ui = UiFolder::new();
        let reason = "Port 1730 is in use by another program".to_owned();
        let mut state = AppState::for_tests();
        state.web_ui = WebUiState::PortInUse {
            reason: reason.clone(),
        };
        let req = Request::builder()
            .uri("/v1/tilekit/ui")
            .header(HOST, "127.0.0.1:1730")
            .body(Body::empty())
            .unwrap();
        let res = router(Arc::new(state), ui.path())
            .oneshot(req)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body: Value = serde_json::from_str(&text(res).await).unwrap();
        assert_eq!(body["data"]["state"], "port_in_use");
        assert_eq!(body["data"]["reason"], reason.as_str());
    }
}
