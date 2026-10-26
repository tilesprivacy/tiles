//! The model cache on disk, in the same layout hf-hub uses

use std::{
    io::SeekFrom,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Result, anyhow};
use nix::{
    errno::Errno,
    fcntl::{Flock, FlockArg},
};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncSeekExt},
};
use tokio_util::sync::CancellationToken;

use super::{DownloadPlan, ModelFile};

// how often a download waiting on another one's lock tries again
const LOCK_RETRY: Duration = Duration::from_millis(500);

/// The repo's folder in the cache is laid out the same way as in hf
pub fn repo_dir(root: &Path, repo: &str) -> PathBuf {
    root.join(format!("models--{}", repo.replace('/', "--")))
}

/// Bytes of `plan` already in the cache at `root`, finished and partial files both
pub async fn on_disk(plan: &DownloadPlan, root: &Path) -> u64 {
    let blobs = repo_dir(root, &plan.repo).join("blobs");
    let mut bytes = 0;
    for file in &plan.files {
        bytes += if is_complete(&blobs.join(&file.blob), file.size).await {
            file.size
        } else {
            let part = blobs.join(format!("{}.part", file.blob));
            file_len(&part).await.min(file.size)
        };
    }
    bytes
}

pub(super) async fn is_complete(blob: &Path, size: u64) -> bool {
    fs::metadata(blob)
        .await
        .is_ok_and(|meta| meta.len() == size)
}

pub(super) async fn file_len(path: &Path) -> u64 {
    fs::metadata(path).await.map_or(0, |meta| meta.len())
}

/// Holds an exclusive lock on `path` until dropped, waiting while another download
/// holds it. `None` when cancelled while waiting
pub(super) async fn lock(
    path: &Path,
    cancel: &CancellationToken,
) -> Result<Option<Flock<std::fs::File>>> {
    loop {
        if let Some(lock) = try_lock(path).await? {
            return Ok(Some(lock));
        }
        tokio::select! {
            () = cancel.cancelled() => return Ok(None),
            () = tokio::time::sleep(LOCK_RETRY) => {}
        }
    }
}

// `flock` locks belong to the open file, so even another download in this process
// that holds it shows up here as taken
async fn try_lock(path: &Path) -> Result<Option<Flock<std::fs::File>>> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)?;
        match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
            Ok(lock) => Ok(Some(lock)),
            Err((_, Errno::EWOULDBLOCK)) => Ok(None),
            Err((_, errno)) => Err(anyhow!("Failed to lock {}: {errno}", path.display())),
        }
    })
    .await?
}

/// An interrupted hf download leaves the `<blob>.sync.part`  file at full size
/// plus an 8 byte count of the bytes written without gaps. Keep those bytes
pub(super) async fn reuse_sync_part(blobs: &Path, file: &ModelFile, part: &Path) -> Result<()> {
    let old = blobs.join(format!("{}.sync.part", file.blob));
    let Ok(meta) = fs::metadata(&old).await else {
        return Ok(());
    };
    if meta.len() == file.size + 8 && fs::metadata(part).await.is_err() {
        let mut handle = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&old)
            .await?;
        handle.seek(SeekFrom::Start(file.size)).await?;
        let mut footer = [0u8; 8];
        handle.read_exact(&mut footer).await?;
        let committed = u64::from_le_bytes(footer).min(file.size);
        handle.set_len(committed).await?;
        drop(handle);
        fs::rename(&old, part).await?;
    } else {
        fs::remove_file(&old).await?;
    }
    Ok(())
}

/// Points `snapshots/<commit>/<name>` at its blob with a relative symlink
pub(super) async fn link(snapshot: &Path, name: &str, blob: &str) -> Result<()> {
    let path = snapshot.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let depth = Path::new(name).components().count() + 1;
    let target = PathBuf::from("../".repeat(depth)).join("blobs").join(blob);
    if fs::symlink_metadata(&path).await.is_ok() {
        fs::remove_file(&path).await?;
    }
    fs::symlink(target, &path).await?;
    Ok(())
}
