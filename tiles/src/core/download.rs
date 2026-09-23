//! downloading a model into the hugging face cache, resumable and cancellable
//!
//! Files land where hf-hub puts them and in its format, so either can finish
//! what the other started: a file in flight is `blobs/<blob>.sync.part`, sized
//! to the file plus 8 bytes that record how much of it is written. The
//! snapshot's links are only made once every file is whole, since the newest
//! snapshot is the one that gets loaded.

use std::{
    io::SeekFrom,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use futures_util::StreamExt;
use reqwest::{Client, StatusCode, header::RANGE};
use serde::Serialize;
use tokio::{
    fs::{self, OpenOptions},
    io::{AsyncSeekExt, AsyncWriteExt},
    sync::watch,
};
use tokio_util::sync::CancellationToken;

use crate::utils::hf_model_downloader::{RepoFile, committed, repo_dir, repo_snapshot};

const HUB: &str = "https://huggingface.co";
/// how much may be lost to a crash or a cancel, and how often progress moves
const COMMIT_EVERY: u64 = 16 << 20;
const REPORT_EVERY: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Starting,
    Downloading,
    Done,
    Cancelled,
    Failed,
}

impl Phase {
    pub fn is_final(self) -> bool {
        matches!(self, Self::Done | Self::Cancelled | Self::Failed)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub spec: String,
    pub phase: Phase,
    /// the file being fetched right now
    pub file: Option<String>,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub bytes_per_sec: u64,
    pub error: Option<String>,
}

impl Progress {
    pub fn new(spec: &str) -> Self {
        Self {
            spec: spec.to_owned(),
            phase: Phase::Starting,
            file: None,
            done_bytes: 0,
            total_bytes: 0,
            bytes_per_sec: 0,
            error: None,
        }
    }
}

/// Downloads `repo` at `quant`, reporting on `progress` until it is final.
pub async fn download(
    repo: &str,
    quant: Option<&str>,
    progress: watch::Sender<Progress>,
    cancel: CancellationToken,
) {
    let result = download_inner(repo, quant, &progress, &cancel).await;
    progress.send_modify(|state| match result {
        Ok(()) if cancel.is_cancelled() => state.phase = Phase::Cancelled,
        Ok(()) => {
            state.phase = Phase::Done;
            state.file = None;
            state.bytes_per_sec = 0;
        }
        Err(err) => {
            state.phase = Phase::Failed;
            state.error = Some(format!("{err:#}"));
        }
    });
}

async fn download_inner(
    repo: &str,
    quant: Option<&str>,
    progress: &watch::Sender<Progress>,
    cancel: &CancellationToken,
) -> Result<()> {
    let (commit, files) = repo_snapshot(repo, quant)
        .await
        .context("Could not list the model's files")?;
    if !files.iter().any(RepoFile::is_main_gguf) {
        return Err(anyhow!("{repo} has no {} file", quant.unwrap_or("default")));
    }

    let dir = repo_dir(repo)?;
    let blobs = dir.join("blobs");
    fs::create_dir_all(&blobs).await?;

    let total = files.iter().map(|file| file.size).sum();
    let mut done: u64 = files.iter().map(|file| on_disk(&blobs, file)).sum();
    progress.send_modify(|state| {
        state.phase = Phase::Downloading;
        state.total_bytes = total;
        state.done_bytes = done;
    });

    let client = Client::new();
    for file in &files {
        if is_whole(&blobs, file) {
            continue;
        }
        let url = format!("{HUB}/{repo}/resolve/{commit}/{}", file.name);
        progress.send_modify(|state| state.file = Some(file.name.clone()));
        let before = on_disk(&blobs, file);
        let finished = fetch(&client, &url, &blobs, file, cancel, |bytes, rate| {
            progress.send_modify(|state| {
                state.done_bytes = done - before + bytes;
                state.bytes_per_sec = rate;
            });
        })
        .await
        .with_context(|| format!("Downloading {} failed", file.name))?;
        if !finished {
            return Ok(());
        }
        done += file.size - before;
    }

    link_snapshot(&dir, &commit, &files).await
}

fn part_path(blobs: &Path, file: &RepoFile) -> PathBuf {
    blobs.join(format!("{}.sync.part", file.blob))
}

fn is_whole(blobs: &Path, file: &RepoFile) -> bool {
    std::fs::metadata(blobs.join(&file.blob)).is_ok_and(|meta| meta.len() == file.size)
}

fn on_disk(blobs: &Path, file: &RepoFile) -> u64 {
    if is_whole(blobs, file) {
        return file.size;
    }
    committed(&part_path(blobs, file), file.size).unwrap_or(0)
}

/// Fetches one file from where its part file left off. False when cancelled,
/// with what arrived kept for next time.
async fn fetch(
    client: &Client,
    url: &str,
    blobs: &Path,
    file: &RepoFile,
    cancel: &CancellationToken,
    mut report: impl FnMut(u64, u64),
) -> Result<bool> {
    let part = part_path(blobs, file);
    let mut start = committed(&part, file.size).unwrap_or(0);
    let mut out = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&part)
        .await?;
    if start == 0 {
        out.set_len(file.size + 8).await?;
    }

    let response = client
        .get(url)
        .header(RANGE, format!("bytes={start}-"))
        .send()
        .await?
        .error_for_status()?;
    // a server that ignores the range sends the whole file again
    if response.status() != StatusCode::PARTIAL_CONTENT {
        start = 0;
    }
    out.seek(SeekFrom::Start(start)).await?;

    let mut written = start;
    let mut last_commit = start;
    let mut window = (Instant::now(), written);
    let mut rate = 0;
    let mut stream = response.bytes_stream();
    loop {
        // biased, or a cancel can lose to a chunk that is already waiting
        let chunk = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                commit(&mut out, file.size, written).await?;
                return Ok(false);
            }
            chunk = stream.next() => chunk,
        };
        let Some(chunk) = chunk else { break };
        let chunk = chunk?;
        out.write_all(&chunk).await?;
        written += chunk.len() as u64;

        if written - last_commit >= COMMIT_EVERY {
            commit(&mut out, file.size, written).await?;
            last_commit = written;
        }
        let elapsed = window.0.elapsed();
        if elapsed >= REPORT_EVERY {
            rate = ((written - window.1) as f64 / elapsed.as_secs_f64()) as u64;
            window = (Instant::now(), written);
            report(written, rate);
        }
    }

    if written != file.size {
        commit(&mut out, file.size, written).await?;
        return Err(anyhow!(
            "the connection closed at {written} of {} bytes",
            file.size
        ));
    }
    out.set_len(file.size).await?;
    out.sync_all().await?;
    drop(out);
    fs::rename(&part, blobs.join(&file.blob)).await?;
    report(written, rate);
    Ok(true)
}

/// Records `written` in the part file's trailing marker, where the next
/// attempt resumes from.
async fn commit(out: &mut fs::File, size: u64, written: u64) -> Result<()> {
    out.flush().await?;
    out.seek(SeekFrom::Start(size)).await?;
    out.write_all(&written.to_le_bytes()).await?;
    out.flush().await?;
    out.seek(SeekFrom::Start(written)).await?;
    Ok(())
}

/// Points `snapshots/<commit>/` at the blobs, the way hf-hub lays them out.
async fn link_snapshot(dir: &Path, commit: &str, files: &[RepoFile]) -> Result<()> {
    let snapshot = dir.join("snapshots").join(commit);
    for file in files {
        let link = snapshot.join(&file.name);
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).await?;
        }
        let depth = file.name.matches('/').count() + 2;
        let target = Path::new(&"../".repeat(depth))
            .join("blobs")
            .join(&file.blob);
        let _ = fs::remove_file(&link).await;
        #[cfg(unix)]
        fs::symlink(&target, &link).await?;
        #[cfg(not(unix))]
        fs::copy(dir.join("blobs").join(&file.blob), &link).await?;
    }
    let refs = dir.join("refs");
    fs::create_dir_all(&refs).await?;
    fs::write(refs.join("main"), commit).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router, body::Body, extract::State, http::HeaderMap, response::Response, routing::get,
    };
    use std::sync::Arc;

    /// a file server that honours ranges, and can stop after `limit` bytes
    async fn serve(data: Arc<Vec<u8>>, limit: Option<usize>) -> String {
        async fn file(
            State((data, limit)): State<(Arc<Vec<u8>>, Option<usize>)>,
            headers: HeaderMap,
        ) -> Response {
            let start = headers
                .get(RANGE)
                .and_then(|range| range.to_str().ok())
                .and_then(|range| range.strip_prefix("bytes="))
                .and_then(|range| range.trim_end_matches('-').parse::<usize>().ok())
                .unwrap_or(0);
            let end = limit.map_or(data.len(), |limit| (start + limit).min(data.len()));
            Response::builder()
                .status(206)
                .body(Body::from(data[start..end].to_vec()))
                .unwrap()
        }
        let app = Router::new()
            .route("/f", get(file))
            .with_state((data, limit));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{addr}/f")
    }

    fn sample(len: usize) -> (Arc<Vec<u8>>, RepoFile) {
        let data: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        let file = RepoFile {
            name: "model.gguf".to_owned(),
            size: len as u64,
            blob: "abc".to_owned(),
        };
        (Arc::new(data), file)
    }

    #[tokio::test]
    async fn a_dropped_connection_resumes_where_it_stopped() {
        let blobs = tempfile::tempdir().unwrap();
        let (data, file) = sample(3 << 20);
        let token = CancellationToken::new();

        // the first server hangs up a third of the way in
        let url = serve(data.clone(), Some(1 << 20)).await;
        let err = fetch(&Client::new(), &url, blobs.path(), &file, &token, |_, _| {}).await;
        assert!(err.is_err());
        assert_eq!(on_disk(blobs.path(), &file), 1 << 20);

        let url = serve(data.clone(), None).await;
        let done = fetch(&Client::new(), &url, blobs.path(), &file, &token, |_, _| {})
            .await
            .unwrap();
        assert!(done);
        assert_eq!(std::fs::read(blobs.path().join("abc")).unwrap(), *data);
        assert!(!part_path(blobs.path(), &file).exists());
    }

    #[tokio::test]
    async fn a_cancel_stops_and_keeps_what_arrived() {
        let blobs = tempfile::tempdir().unwrap();
        let (data, file) = sample(1 << 20);
        let token = CancellationToken::new();
        token.cancel();

        let url = serve(data, None).await;
        let done = fetch(&Client::new(), &url, blobs.path(), &file, &token, |_, _| {})
            .await
            .unwrap();
        assert!(!done);
        assert!(!is_whole(blobs.path(), &file));
        // the part file is resumable, with its marker in place
        assert_eq!(
            committed(&part_path(blobs.path(), &file), file.size),
            Some(0)
        );
    }

    #[tokio::test]
    async fn the_snapshot_links_resolve_to_the_blobs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("blobs")).unwrap();
        std::fs::write(dir.path().join("blobs/abc"), b"weights").unwrap();
        let (_, file) = sample(7);

        link_snapshot(dir.path(), "c0ffee", &[file]).await.unwrap();
        let linked = dir.path().join("snapshots/c0ffee/model.gguf");
        assert_eq!(std::fs::read(linked).unwrap(), b"weights");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("refs/main")).unwrap(),
            "c0ffee"
        );
    }
}
