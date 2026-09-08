//! the local inference server, published to peers over iroh

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::daemon;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

/// binding an endpoint and reaching a relay, not a loopback read
const SHARE_TIMEOUT: Duration = Duration::from_secs(20);
const UNSHARE_TIMEOUT: Duration = Duration::from_secs(10);

pub const STATE_EVENT: &str = "remote://state";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum State {
    /// no daemon to ask through
    Unknown,
    Off,
    Sharing {
        ticket: String,
    },
}

struct Remote {
    state: Mutex<State>,
    /// a second share would 409, and a second unshare would panic the daemon
    in_flight: AtomicBool,
}

pub fn init(app: &AppHandle) {
    app.manage(Remote {
        state: Mutex::new(State::Unknown),
        in_flight: AtomicBool::new(false),
    });
}

fn current(app: &AppHandle) -> State {
    app.state::<Remote>().state.lock().unwrap().clone()
}

/// emits on change only, same as the daemon's health
fn set(app: &AppHandle, next: State) {
    let remote = app.state::<Remote>();
    let mut state = remote.state.lock().unwrap();
    if *state == next {
        return;
    }
    *state = next.clone();
    drop(state);

    let _ = app.emit(STATE_EVENT, next);
}

/// the daemon stopped answering, and it is the only way in
pub fn unknown(app: &AppHandle) {
    set(app, State::Unknown);
}

/// one supervisor tick, only while the daemon answers. the status route reads
/// two in-memory mutexes, so unlike sessions this is cheap enough to poll
pub async fn poll(app: &AppHandle, client: &reqwest::Client) {
    // a share or unshare owns the state until it lands
    if app.state::<Remote>().in_flight.load(Ordering::SeqCst) {
        return;
    }

    let Ok(res) = client.get(daemon::url("/remote-status")).send().await else {
        set(app, State::Unknown);
        return;
    };

    if !res.status().is_success() {
        set(app, State::Unknown);
        return;
    }

    let next = match res.text().await {
        Ok(body) => parse(&body),
        Err(_) => State::Unknown,
    };
    set(app, next);
}

/// the status body, which is raw json rather than an ApiResponse
fn parse(body: &str) -> State {
    // reqwest is built without its json feature, serde_json is already here
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
        return State::Unknown;
    };

    let Some(running) = payload.get("running").and_then(|v| v.as_bool()) else {
        return State::Unknown;
    };

    if !running {
        return State::Off;
    }

    // running with no ticket leaves nothing to show and nothing to copy, which
    // is not a state the panel can draw honestly
    match payload.get("ticket").and_then(|v| v.as_str()) {
        Some(ticket) if !ticket.is_empty() => State::Sharing {
            ticket: ticket.to_owned(),
        },
        _ => State::Unknown,
    }
}

#[tauri::command]
pub fn remote_state(app: AppHandle) -> State {
    current(&app)
}

#[tauri::command]
pub async fn remote_set(app: AppHandle, on: bool) -> Result<(), String> {
    // unshare unwraps its sender, so asking to stop something that is not
    // running poisons the lock and brings share down with it until a restart
    if !on && !matches!(current(&app), State::Sharing { .. }) {
        return Ok(());
    }

    if app.state::<Remote>().in_flight.swap(true, Ordering::SeqCst) {
        return Err("a share or unshare is already in flight".into());
    }

    let outcome = request(&app, on).await;

    app.state::<Remote>()
        .in_flight
        .store(false, Ordering::SeqCst);

    outcome
}

async fn request(app: &AppHandle, on: bool) -> Result<(), String> {
    let client = reqwest::Client::new();
    let (path, timeout) = if on {
        ("/remote-share", SHARE_TIMEOUT)
    } else {
        ("/remote-unshare", UNSHARE_TIMEOUT)
    };

    let res = client
        .get(daemon::url(path))
        .timeout(timeout)
        .send()
        .await
        .map_err(|err| err.to_string())?;

    // already sharing, so the ticket we want is the one the next poll reports
    if res.status() == reqwest::StatusCode::CONFLICT {
        return Ok(());
    }

    if !res.status().is_success() {
        return Err(format!("{path} answered {}", res.status()));
    }

    if !on {
        set(app, State::Off);
        return Ok(());
    }

    // share answers with the ticket as its whole body, no json around it
    let ticket = res.text().await.map_err(|err| err.to_string())?;
    let ticket = ticket.trim();
    if ticket.is_empty() {
        return Err("the daemon shared without returning a ticket".into());
    }

    set(
        app,
        State::Sharing {
            ticket: ticket.to_owned(),
        },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{State, parse};

    /// captured from the daemon at 0.4.18
    const OFF: &str = r#"{"running":false,"ticket":null}"#;

    #[test]
    fn reads_the_daemons_idle_body() {
        assert_eq!(parse(OFF), State::Off);
    }

    #[test]
    fn a_ticket_is_the_sharing_state() {
        assert_eq!(
            parse(r#"{"running":true,"ticket":"n0abc123"}"#),
            State::Sharing {
                ticket: "n0abc123".to_owned(),
            }
        );
    }

    /// the two fields are set under separate locks, so they can disagree
    #[test]
    fn running_without_a_ticket_is_not_shareable() {
        assert_eq!(parse(r#"{"running":true,"ticket":null}"#), State::Unknown);
    }

    #[test]
    fn a_body_without_running_is_not_an_answer() {
        assert_eq!(parse(r#"{"ticket":"n0abc123"}"#), State::Unknown);
    }

    /// a proxy or a captive portal answering 200 with html
    #[test]
    fn unparseable_is_not_an_answer() {
        assert_eq!(parse("<html>"), State::Unknown);
    }
}
