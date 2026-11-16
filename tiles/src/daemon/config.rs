//! APIs for the llama settings

use std::sync::Arc;

use anyhow::Result;
use axum::{
    Router,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::{
    daemon::{ApiJson, ApiResponse, AppError, AppState},
    utils::config::{
        InvalidSetting, LlamaSetting, get_llama_settings, llama_setting_names,
        update_llama_settings,
    },
};

const LLAMA_PATH: &str = "/v1/tilekit/config/llama";

pub fn config_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(LLAMA_PATH, get(list_llama_settings))
        .route("/v1/tilekit/config/llama/set", post(set_llama_settings))
        .route("/v1/tilekit/config/llama/reset", post(reset_llama_settings))
}

async fn list_llama_settings() -> Result<impl IntoResponse, AppError> {
    settings_response(get_llama_settings())
}

#[derive(Deserialize)]
struct SetRequest {
    values: Map<String, Value>,
}

async fn set_llama_settings(
    ApiJson(body): ApiJson<SetRequest>,
) -> Result<impl IntoResponse, AppError> {
    if body.values.is_empty() {
        return Err(AppError::BadRequest("No settings to set".to_owned()));
    }
    let changes: Vec<_> = body
        .values
        .into_iter()
        .map(|(name, value)| (name, Some(value)))
        .collect();
    settings_response(update_llama_settings(&changes))
}

#[derive(Deserialize)]
struct ResetRequest {
    #[serde(default)]
    names: Vec<String>,
    #[serde(default)]
    all: bool,
}

async fn reset_llama_settings(
    ApiJson(body): ApiJson<ResetRequest>,
) -> Result<impl IntoResponse, AppError> {
    let names: Vec<String> = match (body.all, body.names.is_empty()) {
        (true, true) => llama_setting_names().map(str::to_owned).collect(),
        (false, false) => body.names,
        _ => {
            return Err(AppError::BadRequest(
                "Name the settings to reset, or send all".to_owned(),
            ));
        }
    };
    let changes: Vec<_> = names.into_iter().map(|name| (name, None)).collect();
    settings_response(update_llama_settings(&changes))
}

fn settings_response(settings: Result<Vec<LlamaSetting>>) -> Result<impl IntoResponse, AppError> {
    match settings {
        Ok(settings) => Ok(ApiResponse::success(json!({ "settings": settings }))),
        Err(err) => Err(match err.downcast_ref::<InvalidSetting>() {
            Some(invalid) => AppError::BadRequest(invalid.to_string()),
            None => AppError::InternalServerError(format!("Couldn't use config.toml: {err}")),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn call_route(request: Request<Body>) -> (u16, Value) {
        let res = config_router()
            .with_state(Arc::new(AppState::for_tests()))
            .oneshot(request)
            .await
            .unwrap();
        let status = res.status().as_u16();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&body).unwrap())
    }

    fn post_json(path: &str, body: Value) -> Request<Body> {
        Request::post(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn a_wrong_name_or_value_is_a_bad_request() {
        let set = "/v1/tilekit/config/llama/set";
        let (status, body) =
            call_route(post_json(set, json!({ "values": { "context_size": 1 } }))).await;
        assert_eq!(status, 400);
        assert_eq!(body["reason"], "No setting named context_size");

        let (status, body) = call_route(post_json(
            set,
            json!({ "values": { "flash_attn": "maybe" } }),
        ))
        .await;
        assert_eq!(status, 400);
        assert_eq!(body["reason"], "flash_attn needs true or false, not maybe");

        let (status, _) = call_route(post_json(set, json!({ "values": {} }))).await;
        assert_eq!(status, 400);

        let reset = "/v1/tilekit/config/llama/reset";
        let (status, _) = call_route(post_json(reset, json!({}))).await;
        assert_eq!(status, 400);
        let (status, body) = call_route(post_json(reset, json!({ "names": ["nope"] }))).await;
        assert_eq!(status, 400);
        assert_eq!(body["reason"], "No setting named nope");
    }
}
