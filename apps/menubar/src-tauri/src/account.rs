//! the local identity, a did and a nickname held in the daemon's config

use std::sync::Mutex;

use crate::daemon;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

pub const STATE_EVENT: &str = "account://state";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum State {
    /// no daemon to ask through
    Unknown,
    /// the daemon answered, there is no identity yet
    None,
    Local {
        did: String,
        nickname: String,
    },
}

struct Account {
    state: Mutex<State>,
}

pub fn init(app: &AppHandle) {
    app.manage(Account {
        state: Mutex::new(State::Unknown),
    });
}

fn current(app: &AppHandle) -> State {
    app.state::<Account>().state.lock().unwrap().clone()
}

/// emits on change only, same as the daemon's health
fn set(app: &AppHandle, next: State) {
    let account = app.state::<Account>();
    let mut state = account.state.lock().unwrap();
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

/// one supervisor tick, only while the daemon answers
pub async fn poll(app: &AppHandle, client: &reqwest::Client) {
    // a did is written once and there is no route that removes it, so the only
    // move is none to local. asking again after that is a request per tick for
    // an answer that cannot change
    if matches!(current(app), State::Local { .. }) {
        return;
    }

    set(app, fetch(client).await);
}

async fn fetch(client: &reqwest::Client) -> State {
    let Ok(res) = client
        .get(daemon::url("/v1/tilekit/account/status"))
        .send()
        .await
    else {
        return State::Unknown;
    };

    // 404 is the answer for an empty identity, every other failure is the
    // daemon saying nothing, which is not the same as saying there is no account
    if res.status() == reqwest::StatusCode::NOT_FOUND {
        return State::None;
    }
    if !res.status().is_success() {
        return State::Unknown;
    }

    match res.text().await {
        Ok(body) => parse(&body),
        Err(_) => State::Unknown,
    }
}

/// the success body only, a non-2xx never reaches here
fn parse(body: &str) -> State {
    // reqwest is built without its json feature, serde_json is already here
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
        return State::Unknown;
    };

    let data = payload.get("data");
    let did = data.and_then(|d| d.get("id")).and_then(|v| v.as_str());
    let nickname = data
        .and_then(|d| d.get("nickname"))
        .and_then(|v| v.as_str());

    // only the empty-id path is guarded daemon side, a blank did still reaches
    // here as a success
    match (did, nickname) {
        (Some(did), Some(nickname)) if !did.is_empty() => State::Local {
            did: did.to_owned(),
            nickname: nickname.to_owned(),
        },
        _ => State::None,
    }
}

#[cfg(test)]
mod tests {
    use super::{State, parse};

    /// captured from the daemon at 0.4.18
    const SUCCESS: &str = r#"{"status":"success","data":{"id":"did:key:z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH","nickname":"menubar-dev"}}"#;

    #[test]
    fn reads_the_daemons_success_body() {
        assert_eq!(
            parse(SUCCESS),
            State::Local {
                did: "did:key:z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH".to_owned(),
                nickname: "menubar-dev".to_owned(),
            }
        );
    }

    /// get_account_status only guards the empty id on its own read, a 200
    /// carrying a blank one is still possible
    #[test]
    fn a_blank_did_is_not_an_account() {
        assert_eq!(
            parse(r#"{"status":"success","data":{"id":"","nickname":""}}"#),
            State::None
        );
    }

    #[test]
    fn a_body_without_data_is_not_an_account() {
        assert_eq!(parse(r#"{"status":"success"}"#), State::None);
    }

    /// a proxy or a captive portal answering 200 with html
    #[test]
    fn unparseable_is_not_an_answer() {
        assert_eq!(parse("<html>"), State::Unknown);
    }
}

#[tauri::command]
pub fn account_state(app: AppHandle) -> State {
    current(&app)
}
