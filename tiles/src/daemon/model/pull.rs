//! `tiles run`'s download, through the daemon like any other client. Ctrl-C ends the
//! CLI, which closes its progress stream, and the daemon pauses the download

use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::{CLIENT_HEADER, DownloadState};
use crate::daemon::{daemon_client, daemon_url, sse::take_events};

/// Downloads the model `repo` at `quant` (`Q4_K_M` when not given) with a progress
/// bar, continuing an earlier partial download
pub async fn pull(repo: &str, quant: Option<&str>) -> Result<()> {
    let model = match quant {
        Some(quant) => format!("{repo}:{quant}"),
        None => repo.to_owned(),
    };
    let client = daemon_client().await?;
    let res = client
        .post(daemon_url("/v1/tilekit/model/download"))
        .json(&json!({ "model": model }))
        .send()
        .await
        .context("Could not reach the Tiles daemon")?;
    if answer::<DownloadState>(res).await? == DownloadState::Done {
        return Ok(());
    }

    let mut res = client
        .get(daemon_url("/v1/tilekit/model/download"))
        .header(CLIENT_HEADER, "cli")
        .query(&[("model", model.as_str()), ("pause_on_close", "true")])
        .send()
        .await
        .context("Could not reach the Tiles daemon")?;
    let mut bar = None;
    let mut pending = String::new();
    while let Some(chunk) = res.chunk().await? {
        pending.push_str(&String::from_utf8_lossy(&chunk));
        for (_, data) in take_events(&mut pending) {
            match serde_json::from_value(data)? {
                DownloadState::Downloading { done, total, .. } => {
                    let bar = bar.get_or_insert_with(|| download_bar(total, done));
                    bar.set_message(model.clone());
                    bar.set_position(done);
                }
                DownloadState::Verifying { done, total } => {
                    let bar = bar.get_or_insert_with(|| download_bar(total, done));
                    bar.set_message("Verifying the download…");
                }
                DownloadState::Done => {
                    if let Some(bar) = &bar {
                        finish(bar);
                    }
                    return Ok(());
                }
                DownloadState::Paused { .. } => {
                    bail!("The download was paused. Run Tiles again to continue it")
                }
                DownloadState::Failed { reason } => bail!(reason),
                DownloadState::Idle => {}
            }
        }
    }
    bail!("The daemon closed the download before it finished")
}

// the `data` of a success, or the daemon's `reason` as the error
async fn answer<T: DeserializeOwned>(res: reqwest::Response) -> Result<T> {
    let ok = res.status().is_success();
    let mut body: Value = res
        .json()
        .await
        .context("The Tiles daemon didn't answer in JSON")?;
    if !ok {
        bail!(
            "{}",
            body["reason"]
                .as_str()
                .unwrap_or("The Tiles daemon refused the download")
        );
    }
    Ok(serde_json::from_value(body["data"].take())?)
}

fn download_bar(total: u64, done: u64) -> ProgressBar {
    let bar = ProgressBar::with_draw_target(Some(total), ProgressDrawTarget::hidden())
        .with_position(done)
        .with_style(bar_style(
            "{decimal_bytes:.dim}/{decimal_total_bytes:.dim}  {decimal_bytes_per_sec:.cyan}  ETA {eta}",
        ));
    // the speed starts from what's already on disk, so a resume doesn't count it. The
    // tick that records it is hidden, as it would show that jump as the speed
    bar.tick();
    bar.reset_eta();
    bar.set_draw_target(ProgressDrawTarget::stderr());
    bar
}

// the last jump to 100% isn't a real speed, so the finished bar shows only the size
fn finish(bar: &ProgressBar) {
    bar.set_style(bar_style("{decimal_total_bytes:.dim}"));
    bar.finish();
}

fn bar_style(details: &str) -> ProgressStyle {
    ProgressStyle::with_template(&format!(
        "{{msg:.bold}}  {{bar:32.226/238}} {{percent:>3}}%  {details}"
    ))
    .expect("download bar template is invalid")
    .progress_chars("▰▰▱")
}
