//! Downloads models from Hugging Face, with resume, retries and a SHA-256 check

mod cache;
mod fetch;
mod hub;
mod plan;
#[cfg(test)]
mod tests;

use std::path::Path;

use anyhow::{Context, Result, bail};
use tokio::fs;
use tokio_util::sync::CancellationToken;

pub use cache::{on_disk, repo_dir};
pub use hub::Hub;
pub use plan::{DEFAULT_QUANT, plan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFile {
    pub name: String,
    pub size: u64,
    pub blob: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadPlan {
    pub repo: String,
    pub commit: String,
    pub files: Vec<ModelFile>,
}

impl DownloadPlan {
    pub fn total(&self) -> u64 {
        self.files.iter().map(|file| file.size).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadEvent {
    Progress { done: u64, total: u64 },
    Verifying { file: String },
}

#[derive(Debug, PartialEq, Eq)]
pub enum DownloadOutcome {
    Done,
    Paused,
}

/// Downloads `plan` into the cache at `root`. Cancelling keeps the partial files
pub async fn download(
    hub: &Hub,
    plan: &DownloadPlan,
    root: &Path,
    cancel: &CancellationToken,
    mut on_event: impl FnMut(DownloadEvent),
) -> Result<DownloadOutcome> {
    let repo = repo_dir(root, &plan.repo);
    let blobs = repo.join("blobs");
    fs::create_dir_all(&blobs).await?;

    let total = plan.total();
    let mut finished = 0;
    for file in &plan.files {
        // adds the files before this one to its progress
        let mut report = |event: DownloadEvent| {
            on_event(match event {
                DownloadEvent::Progress { done, .. } => DownloadEvent::Progress {
                    done: finished + done,
                    total,
                },
                event => event,
            })
        };
        if !fetch_blob(hub, plan, file, &blobs, cancel, &mut report).await? {
            return Ok(DownloadOutcome::Paused);
        }
        finished += file.size;
        on_event(DownloadEvent::Progress {
            done: finished,
            total,
        });
    }

    // linked last, so `tiles run` never loads a half done snapshot
    let snapshot = repo.join("snapshots").join(&plan.commit);
    for file in &plan.files {
        cache::link(&snapshot, &file.name, &file.blob).await?;
    }
    fs::create_dir_all(repo.join("refs")).await?;
    fs::write(repo.join("refs/main"), &plan.commit).await?;
    Ok(DownloadOutcome::Done)
}

// false when cancelled
async fn fetch_blob(
    hub: &Hub,
    plan: &DownloadPlan,
    file: &ModelFile,
    blobs: &Path,
    cancel: &CancellationToken,
    on_event: &mut impl FnMut(DownloadEvent),
) -> Result<bool> {
    let blob = blobs.join(&file.blob);
    if cache::is_complete(&blob, file.size).await {
        return Ok(true);
    }
    let Some(_lock) = cache::lock(&blobs.join(format!("{}.lock", file.blob)), cancel).await? else {
        return Ok(false);
    };
    // another download may have finished it while this one waited
    if cache::is_complete(&blob, file.size).await {
        return Ok(true);
    }

    let part = blobs.join(format!("{}.part", file.blob));
    cache::reuse_sync_part(blobs, file, &part).await?;
    let Some(sha256) = fetch::fetch(hub, plan, file, &part, cancel, on_event)
        .await
        .with_context(|| format!("Downloading {} failed", file.name))?
    else {
        return Ok(false);
    };
    if file
        .sha256
        .as_deref()
        .is_some_and(|expected| expected != sha256)
    {
        fs::remove_file(&part).await?;
        bail!(
            "{} didn't match its SHA-256, so it was deleted. Download it again",
            file.name
        );
    }
    fs::rename(&part, &blob).await?;
    Ok(true)
}
