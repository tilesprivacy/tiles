//! starting the daemon when nobody else has
//!
//! the daemon owns everything, including us, so a hand launched app has one job
//! before it gets out of the way: make sure the daemon is running. it will spawn
//! its own copy of us, and the single instance plugin hands over

use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use tauri::AppHandle;

use crate::{daemon, lifeline, paths};

const CLI: &str = "/usr/local/bin/tiles";
const POLL: Duration = Duration::from_millis(300);
const GIVE_UP: Duration = Duration::from_secs(20);

fn cli() -> String {
    std::env::var("TILES_CLI_BIN").unwrap_or_else(|_| CLI.to_owned())
}

/// A daemon that dies during startup is the one that most needs its words
/// kept: it is not up to write its own logs, and health can only say "down".
/// The daemon resolves a blank `data.path` to this same default, so a moved
/// data folder strands only this file, not the daemon's own logs.
fn log_file() -> Option<(PathBuf, File)> {
    let dir = paths::default_dir()?.join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("boot.log");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()?;
    Some((path, file))
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

        // both streams share one file, appends interleave the way a terminal would
        let (log_path, out, err) = match log_file() {
            Some((path, file)) => match file.try_clone() {
                Ok(clone) => (Some(path), Stdio::from(file), Stdio::from(clone)),
                Err(_) => (None, Stdio::null(), Stdio::null()),
            },
            None => (None, Stdio::null(), Stdio::null()),
        };

        // detached, so it outlives us when the handover happens
        let spawned = Command::new(cli())
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err)
            .spawn();

        if let Err(err) = spawned {
            // the reason lands in the panel footer, which is the only place a
            // person is looking when the window never appears
            daemon::report_boot_failure(&app, format!("Could not start the daemon: {err}"));
            return;
        }

        let deadline = tokio::time::Instant::now() + GIVE_UP;
        while tokio::time::Instant::now() < deadline {
            if ping(&client).await {
                return;
            }
            tokio::time::sleep(POLL).await;
        }

        let reason = match log_path {
            Some(path) => format!("The daemon did not come up, see {}", path.display()),
            None => "The daemon did not come up, and its log could not be written".to_owned(),
        };
        daemon::report_boot_failure(&app, reason);
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
