//! `tiles run` offers the GPU engine when it isn't downloaded yet. Ctrl-C pauses the download

use std::io::{self, IsTerminal, Write};

use anyhow::{Context, Result};
use indicatif::{DecimalBytes, ProgressBar};
use owo_colors::OwoColorize;
use reqwest::Client;
use serde_json::{Value, json};

use super::InstallState;
use crate::daemon::{
    daemon_client, daemon_url,
    model::{CLIENT_HEADER, answer, download_bar, finish},
    sse::take_events,
};

/// Asks before downloading. No keeps models on the CPU, by picking it
pub async fn offer() -> Result<()> {
    if !io::stdin().is_terminal() {
        return Ok(());
    }
    let client = daemon_client().await?;
    let res = client
        .get(daemon_url("/v1/tilekit/engine/status"))
        .send()
        .await
        .context("Could not reach the Tiles daemon")?;
    let status: Value = answer(res).await?;
    if status["override"].is_string() || status["needs_download"] != true {
        return Ok(());
    }
    let choice = |engine: &Value| {
        status["engines"]
            .as_array()
            .and_then(|engines| engines.iter().find(|choice| &choice["engine"] == engine))
            .cloned()
            .unwrap_or_default()
    };
    let label = |engine: &Value| {
        choice(engine)["label"]
            .as_str()
            .unwrap_or("the GPU")
            .to_owned()
    };

    // one the app started is followed instead of offered again
    let installing = matches!(
        status["download"]["state"].as_str(),
        Some("downloading" | "unpacking" | "checking")
    );
    if !installing {
        let engine = &status["engine"];
        let size = choice(engine)["download"].as_u64().unwrap_or(0);
        print!(
            "Tiles can run models on your GPU with {}, a {} download. Download it now? [y/N] ",
            label(engine),
            DecimalBytes(size)
        );
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            let res = client
                .post(daemon_url("/v1/tilekit/engine/select"))
                .json(&json!({ "engine": "cpu" }))
                .send()
                .await?;
            answer::<Value>(res).await?;
            println!("Models run on the CPU. Pick an engine in the Tiles app to change it");
            return Ok(());
        }
        let res = client
            .post(daemon_url("/v1/tilekit/engine/download"))
            .json(&json!({}))
            .send()
            .await?;
        if let InstallState::Done { .. } = answer(res).await? {
            return Ok(());
        }
    }
    follow(client, &label).await
}

/// Shows the install's progress until it's done, paused or failed
async fn follow(client: &Client, label: &impl Fn(&Value) -> String) -> Result<()> {
    let mut res = client
        .get(daemon_url("/v1/tilekit/engine/download"))
        .header(CLIENT_HEADER, "cli")
        .query(&[("pause_on_close", "true")])
        .send()
        .await
        .context("Could not reach the Tiles daemon")?;
    let mut bar: Option<ProgressBar> = None;
    let mut pending = String::new();
    while let Some(chunk) = res.chunk().await? {
        pending.push_str(&String::from_utf8_lossy(&chunk));
        for (_, data) in take_events(&mut pending) {
            match serde_json::from_value(data)? {
                InstallState::Downloading {
                    engine,
                    done,
                    total,
                    ..
                } => {
                    // a new bar when the next engine is tried
                    if bar.as_ref().is_none_or(|bar| bar.length() != Some(total)) {
                        bar = Some(download_bar(total, done));
                    }
                    if let Some(bar) = &bar {
                        bar.set_message(label(&json!(engine)));
                        bar.set_position(done);
                    }
                }
                InstallState::Unpacking { .. } => {
                    if let Some(bar) = &bar {
                        bar.set_message("Unpacking…");
                    }
                }
                InstallState::Checking { engine } => {
                    if let Some(bar) = &bar {
                        bar.set_message(format!("Checking {} runs here…", label(&json!(engine))));
                    }
                }
                InstallState::Done { engine } => {
                    if let Some(bar) = &bar {
                        bar.set_message(label(&json!(engine)));
                        finish(bar);
                    }
                    println!("Models run on {}", label(&json!(engine)));
                    return Ok(());
                }
                InstallState::Paused { .. } => {
                    println!("The engine download was paused. Models run on the CPU for now");
                    return Ok(());
                }
                InstallState::Failed { reason, .. } => {
                    if let Some(bar) = &bar {
                        bar.abandon();
                    }
                    eprintln!(
                        "{} {reason}. Models run on the CPU for now",
                        "WARNING:".yellow()
                    );
                    return Ok(());
                }
                InstallState::Idle => {}
            }
        }
    }
    Ok(())
}
