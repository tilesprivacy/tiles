//! watching the daemon, which is the thing that started us

use std::sync::Mutex;
use std::time::Duration;

use crate::{account, inference, remote, sessions};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

const PORT: u16 = 1729;

const PING_TIMEOUT: Duration = Duration::from_secs(1);
const POLL_UP: Duration = Duration::from_secs(5);
const POLL_STARTING: Duration = Duration::from_millis(500);

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// the lifeline normally takes us down long before this, it only covers a
/// daemon that never answered
const QUIT_FALLBACK: Duration = Duration::from_secs(5);

pub const HEALTH_EVENT: &str = "daemon://health";

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Health {
    Down { reason: String },
    Starting,
    Up { version: String },
}

pub struct Daemon {
    health: Mutex<Health>,
}

pub fn init(app: &AppHandle) {
    app.manage(Daemon {
        health: Mutex::new(Health::Starting),
    });

    let app = app.clone();
    tauri::async_runtime::spawn(watch(app));
}

/// everything the daemon serves hangs off one loopback port
pub fn url(path: &str) -> String {
    format!("http://127.0.0.1:{PORT}{path}")
}

/// `GET /` answers with the daemon's version, so one request covers both questions
async fn ping(client: &reqwest::Client) -> Option<String> {
    let res = client.get(url("/")).send().await.ok()?;
    // reqwest only errors on transport, so an error page would pass for a version
    if !res.status().is_success() {
        return None;
    }
    Some(res.text().await.ok()?.trim().to_owned())
}

fn current(app: &AppHandle) -> Health {
    app.state::<Daemon>().health.lock().unwrap().clone()
}

/// emits on change only, the watcher polls far more often than state moves
fn set(app: &AppHandle, next: Health) {
    let daemon = app.state::<Daemon>();
    let mut health = daemon.health.lock().unwrap();
    if *health == next {
        return;
    }
    let came_up = matches!(next, Health::Up { .. });
    *health = next.clone();
    drop(health);

    let _ = app.emit(HEALTH_EVENT, next);

    // sessions are read on demand, and this edge is the one time the list can
    // go stale without the panel being open to ask
    if came_up {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { sessions::refresh(&app).await });
    }
}

/// the daemon owns its own lifecycle now, so this only ever reports
async fn watch(app: AppHandle) {
    let client = reqwest::Client::builder()
        .timeout(PING_TIMEOUT)
        .build()
        .expect("a client with only a timeout set always builds");

    loop {
        match ping(&client).await {
            Some(version) => {
                set(&app, Health::Up { version });
                inference::poll(&app, &client).await;
                account::poll(&app, &client).await;
                remote::poll(&app, &client).await;
            }
            None => {
                set(
                    &app,
                    Health::Down {
                        reason: "not running".into(),
                    },
                );
                inference::unknown(&app);
                account::unknown(&app);
                sessions::unknown(&app);
                remote::unknown(&app);
            }
        }

        let settling = matches!(current(&app), Health::Starting) || inference::is_settling(&app);
        tokio::time::sleep(if settling { POLL_STARTING } else { POLL_UP }).await;
    }
}

#[tauri::command]
pub fn daemon_health(app: AppHandle) -> Health {
    current(&app)
}

/// quitting belongs to the daemon, it holds the inference server and us. the
/// lifeline closing is what brings us down, this is only the ask
pub fn quit(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(SHUTDOWN_TIMEOUT)
            .build()
            .expect("a client with only a timeout set always builds");

        // a dropped connection here is the graceful shutdown, not a failure
        let _ = client.get(url("/shutdown")).send().await;

        tokio::time::sleep(QUIT_FALLBACK).await;
        app.exit(0);
    });
}
