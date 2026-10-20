use std::{convert::Infallible, sync::Arc};

use anyhow::{Context, Result, anyhow};
use axum::{
    Router,
    extract::State,
    response::{
        IntoResponse, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post},
};
use axum_macros::debug_handler;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;

use crate::{
    core::{
        account::atproto::{self, AtprotoAccount, HandleNotFound, LOGIN_TIMEOUT},
        storage::db::get_db_conn,
    },
    daemon::{ApiJson, ApiResponse, AppError, AppState, DEFAULT_PORT},
};

/// A sign in waiting for the browser. Cancelling it frees the callback port
pub struct LoginTask {
    cancel: CancellationToken,
    task: JoinHandle<()>,
}

#[derive(Serialize, Deserialize)]
pub struct AtLoginReq {
    user_handle: String,
}
// atproto apis needed
// login, logout
pub fn atproto_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/atproto/login", post(login))
        .route("/v1/tilekit/atproto/logout", post(logout))
        .route("/v1/tilekit/atproto/status", get(status))
    // .route(
    //     "/v1/tilekit/atproto/share-session/{session-id}",
    //     get(share_session),
    // )
}

#[debug_handler]
pub async fn login(
    State(state): State<Arc<AppState>>,
    ApiJson(login_request): ApiJson<AtLoginReq>,
) -> Result<impl IntoResponse, AppError> {
    // the handle is resolved before the stream, so a bad one is a plain error
    let pending = atproto::begin_login(&login_request.user_handle)
        .await
        .map_err(|e| {
            if e.downcast_ref::<HandleNotFound>().is_some() {
                AppError::NotFound(e.to_string())
            } else {
                AppError::CannotProcess(e.to_string())
            }
        })?;

    // a new sign in replaces one still waiting, which has to let go of the port first
    let mut current = state.login.lock().await;
    if let Some(previous) = current.take() {
        previous.cancel.cancel();
        let _ = previous.task.await;
    }

    let (tx, rx) = mpsc::channel(4);
    let cancel = CancellationToken::new();
    let task_cancel = cancel.clone();
    let task = tokio::spawn(async move {
        let _ = tx
            .send(event("authorize", json!({ "url": pending.url })))
            .await;

        let outcome = tokio::select! {
            _ = task_cancel.cancelled() => return,
            outcome = pending.finish(LOGIN_TIMEOUT) => outcome,
        };
        let last = match outcome {
            Ok(account) => event("signed_in", json!(account)),
            Err(err) => event("error", json!({ "reason": err.to_string() })),
        };
        let _ = tx.send(last).await;
    });
    *current = Some(LoginTask {
        cancel: cancel.clone(),
        task,
    });
    drop(current);

    // closing the stream cancels the sign in
    let stream = async_stream::stream! {
        let _cancel_on_close = cancel.drop_guard();
        let mut events = ReceiverStream::new(rx);
        while let Some(event) = events.next().await {
            yield Ok::<_, Infallible>(event);
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

fn event(name: &str, data: Value) -> Event {
    Event::default().event(name).data(data.to_string())
}

/// Signs in through the daemon, the same flow every client uses: prints and
/// opens the url, then waits for the browser to come back
pub async fn sign_in(handle: &str) -> Result<AtprotoAccount> {
    let res = reqwest::Client::new()
        .post(format!(
            "http://127.0.0.1:{DEFAULT_PORT}/v1/tilekit/atproto/login"
        ))
        .json(&json!({ "user_handle": handle }))
        .send()
        .await
        .context("Could not reach the Tiles daemon")?;

    if !res.status().is_success() {
        let body: Value = res.json().await.unwrap_or_default();
        let reason = body["reason"].as_str().unwrap_or("The sign in failed");
        return Err(anyhow!(reason.to_owned()));
    }

    let mut res = res;
    let mut pending = String::new();
    while let Some(chunk) = res.chunk().await? {
        pending.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(end) = pending.find("\n\n") {
            let block: String = pending.drain(..end + 2).collect();
            match sse_event(&block) {
                Some((name, data)) if name == "authorize" => {
                    let url = data["url"]
                        .as_str()
                        .context("The sign in came without a url")?;
                    println!(
                        "\nIf the browser doesn't open automatically, open this URL:\n\n{url}\n"
                    );
                    atproto::open_in_browser(url);
                }
                Some((name, data)) if name == "signed_in" => {
                    return Ok(serde_json::from_value(data)?);
                }
                Some((name, data)) if name == "error" => {
                    let reason = data["reason"].as_str().unwrap_or("The sign in failed");
                    return Err(anyhow!(reason.to_owned()));
                }
                _ => {}
            }
        }
    }

    Err(anyhow!("The daemon closed the sign in before it finished"))
}

// one event block as its name and json data, keep-alive comments come back as none
fn sse_event(block: &str) -> Option<(String, Value)> {
    let mut name = None;
    let mut data = None;
    for line in block.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            name = Some(value.trim().to_owned());
        } else if let Some(value) = line.strip_prefix("data:") {
            data = serde_json::from_str(value.trim()).ok();
        }
    }
    Some((name?, data?))
}

pub async fn logout() -> Result<impl IntoResponse, AppError> {
    let user_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    // core::logout errors with cli wording when nobody is signed in, so check first and answer 404
    let Some(signed_in) = atproto::fetch_logged_in_data(&user_db_conn)
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
    else {
        return Err(AppError::NotFound(
            "No ATmosphere account is signed in".to_owned(),
        ));
    };

    atproto::logout(&user_db_conn).map_err(|e| AppError::CannotProcess(e.to_string()))?;

    Ok(ApiResponse::success(json!({
        "handle": signed_in.handle,
        "did": signed_in.key
    })))
}

pub async fn status() -> Result<impl IntoResponse, AppError> {
    let user_db_conn = get_db_conn(&crate::core::storage::db::DBTYPE::COMMON)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?;

    if let Some(atproto_user) = atproto::fetch_logged_in_data(&user_db_conn)
        .map_err(|e| AppError::CannotProcess(e.to_string()))?
    {
        Ok(ApiResponse::success(json!({
            "handle": atproto_user.handle,
            "did": atproto_user.key
        })))
    } else {
        Err(AppError::NotFound("Not logged-in".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::sse_event;

    #[test]
    fn sign_in_events_are_read_and_keep_alives_skipped() {
        let (name, data) =
            sse_event("event: authorize\ndata: {\"url\":\"https://bsky.social/x\"}\n\n").unwrap();
        assert_eq!(name, "authorize");
        assert_eq!(data["url"], "https://bsky.social/x");

        assert!(sse_event(":\n\n").is_none());
    }
}
