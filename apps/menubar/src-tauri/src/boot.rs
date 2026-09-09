//! starting the daemon when nobody else has
//!
//! the daemon owns everything, including us, so a hand launched app has one job
//! before it gets out of the way: make sure the daemon is running. it will spawn
//! its own copy of us, and the single instance plugin hands over

use std::process::{Command, Stdio};
use std::time::Duration;

use tauri::AppHandle;

use crate::{daemon, lifeline};

const CLI: &str = "/usr/local/bin/tiles";
const POLL: Duration = Duration::from_millis(300);
const GIVE_UP: Duration = Duration::from_secs(20);

fn cli() -> String {
    std::env::var("TILES_CLI_BIN").unwrap_or_else(|_| CLI.to_owned())
}

pub fn init(app: &AppHandle) {
    if lifeline::is_supervised() {
        return;
    }

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::new();
        if ping(&client).await {
            return;
        }

        // detached, so it outlives us when the handover happens
        let spawned = Command::new(cli())
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        if let Err(err) = spawned {
            eprintln!("[boot] could not start the daemon: {err}");
            return;
        }

        let deadline = tokio::time::Instant::now() + GIVE_UP;
        while tokio::time::Instant::now() < deadline {
            if ping(&client).await {
                return;
            }
            tokio::time::sleep(POLL).await;
        }

        eprintln!("[boot] daemon did not come up");
        let _ = &app;
    });
}

async fn ping(client: &reqwest::Client) -> bool {
    client
        .get(daemon::url("/"))
        .timeout(Duration::from_secs(1))
        .send()
        .await
        .map(|res| res.status().is_success())
        .unwrap_or(false)
}
