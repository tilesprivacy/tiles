use std::sync::Mutex;
use std::time::Duration;

use crate::daemon;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

const FETCH_TIMEOUT: Duration = Duration::from_secs(3);

pub const STATE_EVENT: &str = "sessions://state";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    /// the first prompt of the conversation, the daemon has no titles
    pub name: String,
    /// unix milliseconds
    pub created_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum State {
    /// no daemon to ask through
    Unknown,
    Ready {
        sessions: Vec<Session>,
    },
}

struct Sessions {
    state: Mutex<State>,
}

pub fn init(app: &AppHandle) {
    app.manage(Sessions {
        state: Mutex::new(State::Unknown),
    });
}

fn current(app: &AppHandle) -> State {
    app.state::<Sessions>().state.lock().unwrap().clone()
}

/// emits on change only, same as the daemon's health
fn set(app: &AppHandle, next: State) {
    let sessions = app.state::<Sessions>();
    let mut state = sessions.state.lock().unwrap();
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

/// every list costs the daemon a fresh sqlcipher connection and a passkey read,
/// so this is called when the panel opens, never on the supervisor's tick
pub async fn refresh(app: &AppHandle) {
    let Ok(client) = reqwest::Client::builder().timeout(FETCH_TIMEOUT).build() else {
        return;
    };

    set(app, fetch(&client).await);
}

async fn fetch(client: &reqwest::Client) -> State {
    let Ok(res) = client
        .get(daemon::url("/v1/tilekit/session/list"))
        .send()
        .await
    else {
        return State::Unknown;
    };

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
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
        return State::Unknown;
    };

    let Some(entries) = payload.get("data").and_then(|d| d.as_array()) else {
        return State::Unknown;
    };

    // an entry missing a field is one row we cannot draw, not a failed read
    let sessions = entries
        .iter()
        .filter_map(|entry| {
            Some(Session {
                id: entry.get("id")?.as_str()?.to_owned(),
                name: entry.get("name")?.as_str()?.to_owned(),
                created_at: entry.get("created_at")?.as_u64()?,
            })
        })
        .collect();

    State::Ready { sessions }
}

#[tauri::command]
pub fn sessions_state(app: AppHandle) -> State {
    current(&app)
}

#[cfg(test)]
mod tests {
    use super::{Session, State, parse};

    /// captured from the daemon at 0.4.18, two of the five it returned
    const SUCCESS: &str = r#"{"status":"success","data":[{"id":"seed-2","name":"explain the borrow checker","created_at":1788209778867,"creator_id":"did:key:z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH","snapshot":null},{"id":"seed-1","name":"capital of India","created_at":1788209778816,"creator_id":"did:key:z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH","snapshot":null}]}"#;

    /// newest first, and creator_id and snapshot are dropped
    #[test]
    fn reads_the_daemons_success_body() {
        assert_eq!(
            parse(SUCCESS),
            State::Ready {
                sessions: vec![
                    Session {
                        id: "seed-2".to_owned(),
                        name: "explain the borrow checker".to_owned(),
                        created_at: 1_788_209_778_867,
                    },
                    Session {
                        id: "seed-1".to_owned(),
                        name: "capital of India".to_owned(),
                        created_at: 1_788_209_778_816,
                    },
                ],
            }
        );
    }

    /// the daemon builds these rows from five nullable columns
    #[test]
    fn an_entry_missing_a_field_is_skipped_not_fatal() {
        assert_eq!(
            parse(
                r#"{"status":"success","data":[{"id":"a"},{"id":"b","name":"ok","created_at":1}]}"#
            ),
            State::Ready {
                sessions: vec![Session {
                    id: "b".to_owned(),
                    name: "ok".to_owned(),
                    created_at: 1,
                }],
            }
        );
    }

    /// nobody has chatted yet, which is an answer
    #[test]
    fn an_empty_list_is_still_an_answer() {
        assert_eq!(
            parse(r#"{"status":"success","data":[]}"#),
            State::Ready { sessions: vec![] }
        );
    }

    #[test]
    fn a_body_without_data_is_not_an_answer() {
        assert_eq!(parse(r#"{"status":"success"}"#), State::Unknown);
    }

    /// a proxy or a captive portal answering 200 with html
    #[test]
    fn unparseable_is_not_an_answer() {
        assert_eq!(parse("<html>"), State::Unknown);
    }
}
