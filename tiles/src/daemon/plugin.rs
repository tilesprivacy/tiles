//! Listing, installing and switching plugins
//!
//! Pi reads plugins only when it starts, so nothing here touches a running
//! agent. Every change answers with `reload_required`, and the client decides
//! when to call `/v1/tilekit/agent/reload`, which would cut off a chat in flight.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::Path,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    core::plugin::{self, EnabledChange, Installed, PluginError, PluginSummary},
    daemon::{ApiResponse, AppError, AppState},
};

#[derive(Deserialize)]
struct InstallRequest {
    /// an http(s) url, a plugin folder, or an archive on this machine
    source: String,
}

#[derive(Serialize)]
struct Change<T> {
    #[serde(flatten)]
    change: T,
    message: String,
    reload_required: bool,
}

impl<T: std::fmt::Display> Change<T> {
    fn new(change: T, reload_required: bool) -> Self {
        Self {
            message: change.to_string(),
            change,
            reload_required,
        }
    }
}

#[derive(Serialize)]
struct Uninstalled {
    name: String,
}

impl From<PluginError> for AppError {
    fn from(err: PluginError) -> Self {
        match err {
            PluginError::NotFound(reason) => Self::NotFound(reason),
            PluginError::Bundled(reason) => Self::AlreadyExists(reason),
            PluginError::Invalid(reason) => Self::CannotProcess(reason),
            PluginError::Download(reason) => Self::BadGateway(reason),
            PluginError::Other(err) => Self::InternalServerError(format!("{err:#}")),
        }
    }
}

pub fn plugin_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/plugin/list", get(list_plugins))
        .route("/v1/tilekit/plugin/install", post(install_plugin))
        .route("/v1/tilekit/plugin/{name}", delete(uninstall_plugin))
        .route("/v1/tilekit/plugin/{name}/enable", post(enable_plugin))
        .route("/v1/tilekit/plugin/{name}/disable", post(disable_plugin))
}

/// the plugin code is blocking fs work and subprocesses
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, PluginError> + Send + 'static,
) -> Result<T, AppError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|err| AppError::InternalServerError(err.to_string()))?
        .map_err(AppError::from)
}

async fn list_plugins() -> Result<Json<ApiResponse<Vec<PluginSummary>>>, AppError> {
    let plugins = blocking(|| Ok(plugin::summaries())).await?;
    Ok(ApiResponse::success(plugins))
}

async fn install_plugin(
    Json(payload): Json<InstallRequest>,
) -> Result<Json<ApiResponse<Change<Installed>>>, AppError> {
    let source = payload.source.trim().to_owned();
    if source.is_empty() {
        return Err(AppError::BadRequest("source is empty".to_owned()));
    }

    // install mixes an async download with blocking unpacking, so it gets a
    // thread of its own rather than stalling the runtime
    let installed = tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(plugin::install(&source))
    })
    .await
    .map_err(|err| AppError::InternalServerError(err.to_string()))??;

    let reload_required = !installed.unchanged;
    Ok(ApiResponse::success(Change::new(
        installed,
        reload_required,
    )))
}

async fn uninstall_plugin(
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<Change<Uninstalled>>>, AppError> {
    let message = blocking({
        let name = name.clone();
        move || plugin::uninstall(&name)
    })
    .await?;

    Ok(ApiResponse::success(Change {
        change: Uninstalled { name },
        message,
        reload_required: true,
    }))
}

async fn enable_plugin(
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<Change<EnabledChange>>>, AppError> {
    set_enabled(name, true).await
}

async fn disable_plugin(
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<Change<EnabledChange>>>, AppError> {
    set_enabled(name, false).await
}

async fn set_enabled(
    name: String,
    enabled: bool,
) -> Result<Json<ApiResponse<Change<EnabledChange>>>, AppError> {
    let change = blocking(move || plugin::set_enabled(&name, enabled)).await?;
    let reload_required = change.changed;
    Ok(ApiResponse::success(Change::new(change, reload_required)))
}

#[cfg(test)]
mod tests {
    use super::plugin_router;
    use crate::daemon::AppState;
    use crate::utils::config::{ConfigProvider, DefaultProvider, get_or_create_config};
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use reqwest::StatusCode;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    async fn call(method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        // the daemon makes these at startup, a fresh checkout has neither.
        // once, since creating them is not safe to race
        static LAYOUT: std::sync::Once = std::sync::Once::new();
        LAYOUT.call_once(|| {
            DefaultProvider.get_or_create_config_dir().unwrap();
            DefaultProvider.get_or_create_data_dir().unwrap();
            get_or_create_config(DefaultProvider).unwrap();
        });

        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(body.map_or_else(Body::empty, |body| Body::new(body.to_string())))
            .unwrap();
        let response = plugin_router()
            .with_state(AppState::for_tests().into())
            .oneshot(request)
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    #[tokio::test]
    async fn test_unknown_plugin_is_not_found() {
        for (method, uri) in [
            ("DELETE", "/v1/tilekit/plugin/no-such-plugin"),
            ("POST", "/v1/tilekit/plugin/no-such-plugin/enable"),
            ("POST", "/v1/tilekit/plugin/no-such-plugin/disable"),
        ] {
            let (status, body) = call(method, uri, None).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}");
            assert_eq!(body["status"], "failed");
        }
    }

    #[tokio::test]
    async fn test_a_name_cannot_escape_the_plugins_dir() {
        let (status, _) = call("DELETE", "/v1/tilekit/plugin/..%2F..%2Fdata", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_install_rejects_what_is_not_a_plugin() {
        let (status, _) = call(
            "POST",
            "/v1/tilekit/plugin/install",
            Some(json!({"source": "  "})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let (status, _) = call(
            "POST",
            "/v1/tilekit/plugin/install",
            Some(json!({"source": "/definitely/not/here"})),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let not_a_plugin = tempfile::tempdir().unwrap();
        let (status, body) = call(
            "POST",
            "/v1/tilekit/plugin/install",
            Some(json!({"source": not_a_plugin.path()})),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(body["reason"].as_str().unwrap().contains("plugin.json"));
    }

    // shares the dev plugins dir with the mcp config tests, which expect it empty
    #[tokio::test]
    #[serial_test::serial(plugins_dir)]
    async fn test_install_list_toggle_uninstall() {
        let name = "api-roundtrip-test";
        let source = tempfile::tempdir().unwrap();
        let root = source.path().join(name);
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("plugin.json"),
            json!({
                "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
                "name": name,
                "description": "roundtrip fixture"
            })
            .to_string(),
        )
        .unwrap();

        let (status, body) = call(
            "POST",
            "/v1/tilekit/plugin/install",
            Some(json!({"source": root})),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["data"]["name"], name);
        assert_eq!(body["data"]["reload_required"], true);

        let (_, body) = call("GET", "/v1/tilekit/plugin/list", None).await;
        let listed = body["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|plugin| plugin["name"] == name)
            .cloned()
            .expect("installed plugin is listed");
        assert_eq!(listed["description"], "roundtrip fixture");
        assert_eq!(listed["enabled"], true);
        assert_eq!(listed["bundled"], false);

        let disable = format!("/v1/tilekit/plugin/{name}/disable");
        let (_, body) = call("POST", &disable, None).await;
        assert_eq!(body["data"]["enabled"], false);
        assert_eq!(body["data"]["changed"], true);
        assert_eq!(body["data"]["reload_required"], true);

        let (_, body) = call("POST", &disable, None).await;
        assert_eq!(body["data"]["changed"], false);
        assert_eq!(body["data"]["reload_required"], false);

        let (_, body) = call("POST", &format!("/v1/tilekit/plugin/{name}/enable"), None).await;
        assert_eq!(body["data"]["enabled"], true);

        let (status, body) = call("DELETE", &format!("/v1/tilekit/plugin/{name}"), None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["data"]["reload_required"], true);

        let (status, _) = call("DELETE", &format!("/v1/tilekit/plugin/{name}"), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[serial_test::serial(plugins_dir)]
    async fn test_a_reinstall_does_not_inherit_the_old_disabled_state() {
        let name = "api-reinstall-test";
        let source = tempfile::tempdir().unwrap();
        let root = source.path().join(name);
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("plugin.json"),
            json!({
                "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
                "name": name
            })
            .to_string(),
        )
        .unwrap();
        let install = || {
            call(
                "POST",
                "/v1/tilekit/plugin/install",
                Some(json!({"source": root})),
            )
        };
        let enabled = |body: Value| {
            body["data"]
                .as_array()
                .unwrap()
                .iter()
                .find(|plugin| plugin["name"] == name)
                .map(|plugin| plugin["enabled"] == true)
        };

        install().await;
        call("POST", &format!("/v1/tilekit/plugin/{name}/disable"), None).await;
        call("DELETE", &format!("/v1/tilekit/plugin/{name}"), None).await;
        assert!(!crate::utils::config::get_disabled_plugins().contains(&name.to_owned()));

        install().await;
        let (_, body) = call("GET", "/v1/tilekit/plugin/list", None).await;
        assert_eq!(enabled(body), Some(true));

        call("DELETE", &format!("/v1/tilekit/plugin/{name}"), None).await;
    }
}
