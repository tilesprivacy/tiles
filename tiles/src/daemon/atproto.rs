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
    daemon::{ApiJson, ApiResponse, AppError, AppState, daemon_client, daemon_url},
    utils::open_in_browser,
};

/// A sign in waiting for the browser. Cancelling it frees the callback port
#[derive(Debug)]
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

    // closing the stream cancels the sign in. the guard is made here, not inside the
    // stream, which runs nothing until it is polled and would miss a drop before that
    let cancel_on_close = cancel.drop_guard();
    let stream = async_stream::stream! {
        let _cancel_on_close = cancel_on_close;
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
    let res = daemon_client()
        .await?
        .post(daemon_url("/v1/tilekit/atproto/login"))
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
        for (name, data) in take_events(&mut pending) {
            match name.as_str() {
                "authorize" => {
                    let url = data["url"]
                        .as_str()
                        .context("The sign in came without a url")?;
                    println!(
                        "\nIf the browser doesn't open automatically, open this URL:\n\n{url}\n"
                    );
                    open_in_browser(url);
                }
                "signed_in" => return Ok(serde_json::from_value(data)?),
                "error" => {
                    let reason = data["reason"].as_str().unwrap_or("The sign in failed");
                    return Err(anyhow!(reason.to_owned()));
                }
                _ => {}
            }
        }
    }

    Err(anyhow!("The daemon closed the sign in before it finished"))
}

// takes the finished event blocks off the buffer, a block cut off mid chunk waits for the next one
fn take_events(pending: &mut String) -> Vec<(String, Value)> {
    let mut events = vec![];
    while let Some(end) = pending.find("\n\n") {
        let block: String = pending.drain(..end + 2).collect();
        events.extend(sse_event(&block));
    }
    events
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
    use super::*;
    use futures_util::stream;
    use http_body_util::BodyExt;

    // the bytes axum sends for these events, so the tests read what a client really gets
    async fn wire(events: Vec<Event>) -> String {
        let body = Sse::new(stream::iter(events.into_iter().map(Ok::<_, Infallible>)))
            .into_response()
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        String::from_utf8(body.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn sign_in_reads_every_event_the_daemon_sends() {
        let account = AtprotoAccount {
            handle: "alice.bsky.social".to_owned(),
            did: "did:plc:alice".to_owned(),
        };
        let mut pending = wire(vec![
            event("authorize", json!({ "url": "https://bsky.social/oauth" })),
            event("signed_in", json!(account)),
            event("error", json!({ "reason": "The sign in timed out" })),
        ])
        .await;

        let events = take_events(&mut pending);
        assert!(pending.is_empty());
        let names: Vec<_> = events.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["authorize", "signed_in", "error"]);

        // the same fields sign_in reads
        assert_eq!(events[0].1["url"], "https://bsky.social/oauth");
        let signed_in: AtprotoAccount = serde_json::from_value(events[1].1.clone()).unwrap();
        assert_eq!(signed_in.handle, "alice.bsky.social");
        assert_eq!(signed_in.did, "did:plc:alice");
        assert_eq!(events[2].1["reason"], "The sign in timed out");
    }

    #[tokio::test]
    async fn an_event_cut_across_chunks_is_read_once_it_is_whole() {
        let sent = wire(vec![event(
            "authorize",
            json!({ "url": "https://bsky.social/oauth" }),
        )])
        .await;
        let (first, rest) = sent.split_at(sent.len() / 2);

        let mut pending = first.to_owned();
        assert!(take_events(&mut pending).is_empty());

        pending.push_str(rest);
        let events = take_events(&mut pending);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].1["url"], "https://bsky.social/oauth");
    }

    #[test]
    fn keep_alives_and_broken_events_are_skipped() {
        let mut pending =
            ":\n\nevent: error\ndata: not json\n\nevent: authorize\ndata: {\"url\":\"u\"}\n\n"
                .to_owned();

        let events = take_events(&mut pending);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "authorize");
    }
}
