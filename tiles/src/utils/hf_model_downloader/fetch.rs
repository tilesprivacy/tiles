//! Downloads one file, with resume and retries

use std::{fmt, path::Path, time::Duration};

use anyhow::Result;
use reqwest::{StatusCode, header::RANGE};
use sha2::{Digest, Sha256};
use tokio::{
    fs,
    io::{AsyncWriteExt, BufWriter},
};
use tokio_util::sync::CancellationToken;

use super::{
    DownloadEvent, DownloadPlan, ModelFile,
    cache::file_len,
    hub::{Hub, check},
};

// retried after 1, 2, 4, 8 and 16 seconds
const RETRIES: u32 = 5;

#[derive(Debug)]
struct Interrupted {
    done: u64,
    size: u64,
}

impl fmt::Display for Interrupted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "The download stopped at {} of {} bytes",
            self.done, self.size
        )
    }
}

impl std::error::Error for Interrupted {}

fn is_transient(err: &anyhow::Error) -> bool {
    err.is::<Interrupted>()
        || err.downcast_ref::<reqwest::Error>().is_some_and(|err| {
            err.status().is_none_or(|status| {
                status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
            })
        })
}

// the file's SHA-256, or `None` when cancelled
pub(super) async fn fetch(
    hub: &Hub,
    plan: &DownloadPlan,
    file: &ModelFile,
    part: &Path,
    cancel: &CancellationToken,
    on_event: &mut impl FnMut(DownloadEvent),
) -> Result<Option<String>> {
    let mut retries = 0;
    loop {
        let before = file_len(part).await;
        let err = match fetch_once(hub, plan, file, part, cancel, on_event).await {
            Ok(fetched) => return Ok(fetched),
            Err(err) => err,
        };
        // any progress resets the retries
        if file_len(part).await > before {
            retries = 0;
        }
        if retries == RETRIES || !is_transient(&err) {
            return Err(err);
        }
        log::warn!("Retrying {}: {err:#}", file.name);
        tokio::select! {
            () = cancel.cancelled() => return Ok(None),
            () = tokio::time::sleep(Duration::from_secs(1 << retries)) => {}
        }
        retries += 1;
    }
}

async fn fetch_once(
    hub: &Hub,
    plan: &DownloadPlan,
    file: &ModelFile,
    part: &Path,
    cancel: &CancellationToken,
    on_event: &mut impl FnMut(DownloadEvent),
) -> Result<Option<String>> {
    let size = file.size;
    let mut offset = file_len(part).await;
    if offset > size {
        fs::remove_file(part).await?;
        offset = 0;
    }
    on_event(DownloadEvent::Progress {
        done: offset,
        total: size,
    });
    // hash what's on disk first, so the end needs no second pass
    let mut hasher = if offset > 0 {
        on_event(DownloadEvent::Verifying {
            file: file.name.clone(),
        });
        hash_file(part).await?
    } else {
        Sha256::new()
    };
    if offset == size {
        return Ok(Some(hex(hasher)));
    }

    let mut request = hub.get(&format!(
        "{}/resolve/{}/{}",
        plan.repo, plan.commit, file.name
    ));
    if offset > 0 {
        request = request.header(RANGE, format!("bytes={offset}-"));
    }
    let mut res = tokio::select! {
        () = cancel.cancelled() => return Ok(None),
        res = request.send() => check(res?, &plan.repo)?,
    };
    // the server ignored the range
    if offset > 0 && res.status() != StatusCode::PARTIAL_CONTENT {
        offset = 0;
        hasher = Sha256::new();
    }

    let handle = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(offset > 0)
        .truncate(offset == 0)
        .open(part)
        .await?;
    let mut writer = BufWriter::new(handle);
    let mut done = offset;
    // so the writer is flushed however it ends
    let copied: Result<bool> = async {
        loop {
            let chunk = tokio::select! {
                () = cancel.cancelled() => return Ok(false),
                chunk = res.chunk() => chunk?,
            };
            let Some(chunk) = chunk else {
                return Ok(true);
            };
            hasher.update(&chunk);
            writer.write_all(&chunk).await?;
            done += chunk.len() as u64;
            on_event(DownloadEvent::Progress { done, total: size });
        }
    }
    .await;
    writer.flush().await?;

    if !copied? {
        return Ok(None);
    }
    if done != size {
        return Err(Interrupted { done, size }.into());
    }
    Ok(Some(hex(hasher)))
}

async fn hash_file(path: &Path) -> Result<Sha256> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut hasher = Sha256::new();
        std::io::copy(&mut std::fs::File::open(path)?, &mut hasher)?;
        Ok(hasher)
    })
    .await?
}

fn hex(hasher: Sha256) -> String {
    format!("{:x}", hasher.finalize())
}
