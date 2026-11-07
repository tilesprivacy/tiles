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
    model::{CLIENT_HEADER, download_bar, finish_bar, response_data},
    sse::take_events,
};

/// Asks before downloading. No keeps models on the CPU, by picking it unless the engine was picked by hand
pub async fn offer_gpu_engine() -> Result<()> {
    if !io::stdin().is_terminal() {
        return Ok(());
    }
    let client = daemon_client().await?;
    let res = client
        .get(daemon_url("/v1/tilekit/engine/status"))
        .send()
        .await
        .context("Could not reach the Tiles daemon")?;
    let status: Value = response_data(res).await?;
    if status["override"].is_string() || status["needs_download"] != true {
        return Ok(());
    }
    let engine_entry = |engine: &Value| {
        status["engines"]
            .as_array()
            .and_then(|engines| engines.iter().find(|entry| &entry["engine"] == engine))
            .cloned()
            .unwrap_or_default()
    };
    let engine_label = |engine: &Value| {
        engine_entry(engine)["label"]
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
        let size = engine_entry(engine)["download"].as_u64().unwrap_or(0);
        print!(
            "Tiles can run models on your GPU with {}, a {} download. Download it now? [y/N] ",
            engine_label(engine),
            DecimalBytes(size)
        );
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            // a pick made by hand stays, the next run offers it again
            if status["pick"] != "auto" {
                println!(
                    "Models run on the CPU until {} is downloaded",
                    engine_label(engine)
                );
                return Ok(());
            }
            let res = client
                .post(daemon_url("/v1/tilekit/engine/select"))
                .json(&json!({ "engine": "cpu" }))
                .send()
                .await?;
            response_data::<Value>(res).await?;
            println!("Models run on the CPU. Pick an engine in the Tiles app to change it");
            return Ok(());
        }
        let res = client
            .post(daemon_url("/v1/tilekit/engine/download"))
            .json(&json!({}))
            .send()
            .await?;
        if let InstallState::Done { .. } = response_data(res).await? {
            return Ok(());
        }
    }
    show_engine_download(client, &engine_label).await
}

/// Shows the install's progress until it's done, paused or failed
async fn show_engine_download(
    client: &Client,
    engine_label: &impl Fn(&Value) -> String,
) -> Result<()> {
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
                        bar.set_message(engine_label(&json!(engine)));
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
                        bar.set_message(format!(
                            "Checking {} runs here…",
                            engine_label(&json!(engine))
                        ));
                    }
                }
                InstallState::Done { engine } => {
                    if let Some(bar) = &bar {
                        bar.set_message(engine_label(&json!(engine)));
                        finish_bar(bar);
                    }
                    println!("Models run on {}", engine_label(&json!(engine)));
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
                // deleted, or the daemon restarted, before it got going
                InstallState::Idle => {
                    eprintln!(
                        "{} The engine download stopped. Models run on the CPU for now",
                        "WARNING:".yellow()
                    );
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}
