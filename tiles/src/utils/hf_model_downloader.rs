/// Manages model snapshot downloading from HuggingFace
use std::{
    fmt,
    io::SeekFrom,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
use indicatif::{ProgressBar, ProgressStyle};
use nix::{
    errno::Errno,
    fcntl::{Flock, FlockArg},
};
use reqwest::{Client, RequestBuilder, Response, StatusCode, header::RANGE};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufWriter},
};
use tokio_util::sync::CancellationToken;

use crate::utils::config::get_or_create_model_download_path;

/// Default quantization pulled when a modelfile doesn't specify one
pub const DEFAULT_QUANT: &str = "Q4_K_M";

/// Downloads the model `model_name` at `quant` (`Q4_K_M` when not given) with a
/// progress bar, continuing an earlier partial download
pub async fn pull_model(model_name: &str, quant: Option<&str>) -> Result<()> {
    let plan = plan(model_name, quant).await?;
    let root = get_or_create_model_download_path()?;
    let (total, done) = (plan.total(), on_disk(&plan, &root).await);
    // nothing to download, only the snapshot links to check
    let bar = if done == total {
        ProgressBar::hidden()
    } else {
        download_bar(total, done)
    };
    bar.set_message(model_name.to_owned());
    let mut verifying = false;
    let downloaded = download(
        &plan,
        &root,
        &CancellationToken::new(),
        |event| match event {
            DownloadEvent::Progress { done, .. } => {
                if std::mem::take(&mut verifying) {
                    bar.set_message(model_name.to_owned());
                }
                bar.set_position(done);
            }
            DownloadEvent::Verifying { .. } => {
                verifying = true;
                bar.set_message("Verifying the download…");
            }
        },
    )
    .await;
    bar.finish();
    downloaded.map(|_| ())
}

// starts at what's already on disk, so a resume doesn't count it as speed
fn download_bar(total: u64, done: u64) -> ProgressBar {
    let style = ProgressStyle::with_template(
        "{msg:.bold}  {bar:32.226/238} {percent:>3}%  \
         {decimal_bytes:.dim}/{decimal_total_bytes:.dim}  {decimal_bytes_per_sec:.cyan}  ETA {eta}",
    )
    .expect("download bar template is invalid")
    .progress_chars("▰▰▱");
    ProgressBar::new(total)
        .with_position(done)
        .with_style(style)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFile {
    pub name: String,
    pub size: u64,
    /// The name under `blobs/`, the same one hf-hub uses
    pub blob: String,
    pub sha256: Option<String>,
}

/// When downloading a model, we take its repo, the commit and the files
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
    Progress {
        done: u64,
        total: u64,
    },
    /// Hashing the part of `file` already on disk, before continuing it
    Verifying {
        file: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum DownloadOutcome {
    Done,
    Paused,
}

/// Another process holds the lock on a file this download needs
#[derive(Debug)]
pub struct AlreadyDownloading;

impl fmt::Display for AlreadyDownloading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("The model is already downloading in another Tiles process")
    }
}

impl std::error::Error for AlreadyDownloading {}

/// The connection closed before the whole file came
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

const HF_ENDPOINT: &str = "https://huggingface.co";
const USER_AGENT: &str = concat!("tiles/", env!("CARGO_PKG_VERSION"));
const METADATA_SUFFIXES: [&str; 5] = [".json", ".txt", ".md", ".gitattributes", "LICENSE"];
// a connection that sends nothing for this long fails instead of hanging
const STALL_TIMEOUT: Duration = Duration::from_secs(30);
// a failed download is retried after 1, 2, 4, 8 and 16 seconds
const RETRIES: u32 = 5;

/// The Hugging Face Hub, at `HF_ENDPOINT` if set, sending `HF_TOKEN` for private
/// and gated models
struct Hub {
    client: Client,
    endpoint: String,
    token: Option<String>,
}

impl Hub {
    fn from_env() -> Result<Self> {
        let endpoint = std::env::var("HF_ENDPOINT").unwrap_or_else(|_| HF_ENDPOINT.to_owned());
        let token = std::env::var("HF_TOKEN")
            .ok()
            .filter(|token| !token.is_empty());
        Self::new(&endpoint, token)
    }

    fn new(endpoint: &str, token: Option<String>) -> Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(STALL_TIMEOUT)
            .read_timeout(STALL_TIMEOUT)
            .build()?;
        Ok(Self {
            client,
            endpoint: endpoint.trim_end_matches('/').to_owned(),
            token,
        })
    }

    // reqwest drops the token when the Hub redirects a download to its CDN
    fn get(&self, path: &str) -> RequestBuilder {
        let request = self.client.get(format!("{}/{path}", self.endpoint));
        match &self.token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }
}

/// Turns the Hub's refusals into messages a user can act on
fn check(res: Response, repo: &str) -> Result<Response> {
    match res.status() {
        // the Hub also answers 401 for a repo that doesn't exist
        StatusCode::UNAUTHORIZED | StatusCode::NOT_FOUND => bail!(
            "No model {repo} on Hugging Face. If it's private or gated, set HF_TOKEN to a Hugging Face token that has access"
        ),
        StatusCode::FORBIDDEN => {
            bail!("{repo} needs access. Set HF_TOKEN to a Hugging Face token that has it")
        }
        _ => Ok(res.error_for_status()?),
    }
}

/// A dropped connection, a timeout or a busy server, which a retry can get past
fn is_transient(err: &anyhow::Error) -> bool {
    err.is::<Interrupted>()
        || err.downcast_ref::<reqwest::Error>().is_some_and(|err| {
            err.status().is_none_or(|status| {
                status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
            })
        })
}

#[derive(Deserialize)]
struct RepoInfo {
    sha: String,
    siblings: Vec<Sibling>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sibling {
    rfilename: String,
    size: u64,
    blob_id: String,
    lfs: Option<Lfs>,
}

#[derive(Deserialize)]
struct Lfs {
    sha256: String,
}

impl From<Sibling> for ModelFile {
    // large files are stored by their SHA-256, small ones by their git id
    fn from(sibling: Sibling) -> Self {
        let sha256 = sibling.lfs.map(|lfs| lfs.sha256);
        Self {
            blob: sha256.clone().unwrap_or(sibling.blob_id),
            name: sibling.rfilename,
            size: sibling.size,
            sha256,
        }
    }
}

/// Lists the files of a model in huggingface API
pub async fn plan(repo: &str, quant: Option<&str>) -> Result<DownloadPlan> {
    plan_from(&Hub::from_env()?, repo, quant).await
}

async fn plan_from(hub: &Hub, repo: &str, quant: Option<&str>) -> Result<DownloadPlan> {
    let quant = quant.unwrap_or(DEFAULT_QUANT);
    let res = hub
        .get(&format!("api/models/{repo}/revision/main?blobs=true"))
        .send()
        .await?;
    let info: RepoInfo = check(res, repo)?.json().await?;

    let quant_gguf = format!("{}.gguf", quant.to_lowercase());
    let has_quant = info
        .siblings
        .iter()
        .any(|sibling| is_main_gguf(&sibling.rfilename) && wanted(&sibling.rfilename, &quant_gguf));
    if !has_quant {
        let available: Vec<&str> = info
            .siblings
            .iter()
            .map(|sibling| sibling.rfilename.as_str())
            .filter(|name| is_main_gguf(name))
            .collect();
        if available.is_empty() {
            bail!("No GGUF matching quant '{quant}' found in {repo}: the repo has no GGUF files");
        }
        bail!(
            "No GGUF matching quant '{quant}' found in {repo}. Available: {}. Pick one in the Modelfile, like `FROM {repo}:<quant>`",
            available.join(", ")
        );
    }

    Ok(DownloadPlan {
        repo: repo.to_owned(),
        commit: info.sha,
        files: info
            .siblings
            .into_iter()
            .filter(|sibling| wanted(&sibling.rfilename, &quant_gguf))
            .map(ModelFile::from)
            .collect(),
    })
}

fn wanted(name: &str, quant_gguf: &str) -> bool {
    let lower = name.to_lowercase();
    let top_level = !name.contains('/');
    METADATA_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
        || (is_main_gguf(name) && lower.ends_with(quant_gguf))
        // speculative decoding head, quant independent
        || (top_level && lower.ends_with(".gguf") && lower.contains("mtp"))
}

/// The repo's folder in the cache is laid out the same way as in hf
pub fn repo_dir(root: &Path, repo: &str) -> PathBuf {
    root.join(format!("models--{}", repo.replace('/', "--")))
}

/// Downloads what `plan` lists into the model cache at `root`, continuing any
/// partial files. Cancelling pauses it and keeps the partial files
pub async fn download(
    plan: &DownloadPlan,
    root: &Path,
    cancel: &CancellationToken,
    on_event: impl FnMut(DownloadEvent),
) -> Result<DownloadOutcome> {
    download_from(&Hub::from_env()?, plan, root, cancel, on_event).await
}

async fn download_from(
    hub: &Hub,
    plan: &DownloadPlan,
    root: &Path,
    cancel: &CancellationToken,
    mut on_event: impl FnMut(DownloadEvent),
) -> Result<DownloadOutcome> {
    let repo = repo_dir(root, &plan.repo);
    let blobs = repo.join("blobs");
    let snapshot = repo.join("snapshots").join(&plan.commit);
    fs::create_dir_all(&blobs).await?;
    fs::create_dir_all(repo.join("refs")).await?;
    fs::write(repo.join("refs/main"), &plan.commit).await?;

    let total = plan.total();
    let mut finished = 0;
    for file in &plan.files {
        let blob = blobs.join(&file.blob);
        if !is_complete(&blob, file.size).await {
            let _lock = lock(&blobs.join(format!("{}.lock", file.blob))).await?;
            let part = blobs.join(format!("{}.part", file.blob));
            reuse_sync_part(&blobs, file, &part).await?;

            // fetch counts this file's bytes, the plan's progress adds the files before it
            let mut report = |event: DownloadEvent| {
                on_event(match event {
                    DownloadEvent::Progress { done, .. } => DownloadEvent::Progress {
                        done: finished + done,
                        total,
                    },
                    event => event,
                })
            };
            let Some(sha256) = fetch(hub, plan, file, &part, cancel, &mut report)
                .await
                .with_context(|| format!("Downloading {} failed", file.name))?
            else {
                return Ok(DownloadOutcome::Paused);
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
        }
        link(&snapshot, &file.name, &file.blob).await?;
        finished += file.size;
        on_event(DownloadEvent::Progress {
            done: finished,
            total,
        });
    }
    Ok(DownloadOutcome::Done)
}

async fn is_complete(blob: &Path, size: u64) -> bool {
    fs::metadata(blob)
        .await
        .is_ok_and(|meta| meta.len() == size)
}

async fn file_len(path: &Path) -> u64 {
    fs::metadata(path).await.map_or(0, |meta| meta.len())
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

/// Holds an exclusive lock on `path` until dropped. Fails if another
/// process holds it
async fn lock(path: &Path) -> Result<Flock<std::fs::File>> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)?;
        Flock::lock(file, FlockArg::LockExclusiveNonblock).map_err(|(_, errno)| {
            if errno == Errno::EWOULDBLOCK {
                anyhow::Error::new(AlreadyDownloading)
            } else {
                anyhow!("Failed to lock {}: {errno}", path.display())
            }
        })
    })
    .await?
}

/// An interrupted hf download leaves the `<blob>.sync.part`  file at full size
/// plus an 8 byte count of the bytes written without gaps. Keep those bytes
async fn reuse_sync_part(blobs: &Path, file: &ModelFile, part: &Path) -> Result<()> {
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

/// Downloads the rest of `file` into `part`, retrying when the connection fails.
/// Returns the file's SHA-256, or `None` when cancelled
async fn fetch(
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
        // any progress earns a fresh set of retries
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
    // the hash carries on from the bytes already on disk, so the end needs no second pass
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
    // a server that ignores the range sends everything again
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
    // in a block so the writer is flushed however it ends
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

/// The sha of what's in `path` so far, ready to hash more onto
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

/// Points `snapshots/<commit>/<name>` at its blob with a relative symlink
async fn link(snapshot: &Path, name: &str, blob: &str) -> Result<()> {
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

/// True for gguf files that can serve as the main model (excludes the
/// mmproj vision encoder and MTP draft heads, which are never main models).
fn is_main_gguf(filename: &str) -> bool {
    let name = filename.to_lowercase();
    name.ends_with(".gguf") && !name.contains("mmproj") && !name.contains("mtp")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use tempfile::tempdir;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path, query_param},
    };

    const REPO: &str = "org/model-GGUF";
    const COMMIT: &str = "c0ffee";
    const MODEL: &[u8] = b"GGUF the model weights";
    const MTP: &[u8] = b"GGUF the mtp head";
    const CONFIG: &[u8] = b"{}";

    fn sha256_hex(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    fn model_file(name: &str, bytes: &[u8], lfs: bool) -> ModelFile {
        let hash = sha256_hex(bytes);
        ModelFile {
            name: name.to_owned(),
            size: bytes.len() as u64,
            blob: if lfs {
                hash.clone()
            } else {
                format!("git-{name}")
            },
            sha256: lfs.then_some(hash),
        }
    }

    fn test_plan() -> DownloadPlan {
        DownloadPlan {
            repo: REPO.to_owned(),
            commit: COMMIT.to_owned(),
            files: vec![
                model_file("config.json", CONFIG, false),
                model_file("model-Q4_K_M.gguf", MODEL, true),
                model_file("MTP/mtp-head.gguf", MTP, true),
            ],
        }
    }

    fn hub_for(server: &MockServer) -> Hub {
        Hub::new(&server.uri(), None).unwrap()
    }

    fn resolve_path(name: &str) -> String {
        format!("/{REPO}/resolve/{COMMIT}/{name}")
    }

    async fn serve_file(server: &MockServer, name: &str, bytes: &[u8]) {
        Mock::given(method("GET"))
            .and(path(resolve_path(name)))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes))
            .mount(server)
            .await;
    }

    // everything but the model, which each test serves its own way
    async fn serve_the_rest(server: &MockServer) {
        serve_file(server, "config.json", CONFIG).await;
        serve_file(server, "MTP/mtp-head.gguf", MTP).await;
    }

    async fn serve_all(server: &MockServer) {
        serve_the_rest(server).await;
        serve_file(server, "model-Q4_K_M.gguf", MODEL).await;
    }

    fn write_part(root: &Path, bytes: &[u8]) {
        let blobs = repo_dir(root, REPO).join("blobs");
        std::fs::create_dir_all(&blobs).unwrap();
        std::fs::write(blobs.join(format!("{}.part", sha256_hex(MODEL))), bytes).unwrap();
    }

    async fn run(hub: &Hub, plan: &DownloadPlan, root: &Path) -> Result<DownloadOutcome> {
        download_from(hub, plan, root, &CancellationToken::new(), |_| {}).await
    }

    fn snapshot_file(root: &Path, name: &str) -> Vec<u8> {
        std::fs::read(
            repo_dir(root, REPO)
                .join("snapshots")
                .join(COMMIT)
                .join(name),
        )
        .unwrap()
    }

    // a file as the Hub's api lists it
    fn sibling(file: &ModelFile) -> Value {
        json!({
            "rfilename": file.name,
            "size": file.size,
            "blobId": if file.sha256.is_some() { "a-git-id" } else { file.blob.as_str() },
            "lfs": file.sha256.as_ref().map(|sha256| json!({ "sha256": sha256, "size": file.size })),
        })
    }

    #[tokio::test]
    async fn the_plan_takes_the_quant_metadata_and_mtp_head() {
        let server = MockServer::start().await;
        let repo: [(&str, &[u8], bool); 7] = [
            ("config.json", CONFIG, false),
            ("README.md", b"# model", false),
            ("model-Q4_K_M.gguf", MODEL, true),
            ("model-Q8_0.gguf", b"q8", true),
            ("mtp-model.gguf", MTP, true),
            ("MTP/mtp-model-Q8_0.gguf", MTP, true),
            ("mmproj-F16.gguf", b"mmproj", true),
        ];
        let files: Vec<ModelFile> = repo
            .iter()
            .map(|(name, bytes, lfs)| model_file(name, bytes, *lfs))
            .collect();
        Mock::given(method("GET"))
            .and(path(format!("/api/models/{REPO}/revision/main")))
            .and(query_param("blobs", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "sha": COMMIT,
                "siblings": files.iter().map(sibling).collect::<Vec<_>>(),
            })))
            .mount(&server)
            .await;

        let hub = hub_for(&server);
        let plan = plan_from(&hub, REPO, Some("Q4_K_M")).await.unwrap();
        assert_eq!(plan.commit, COMMIT);
        assert_eq!(
            plan.files,
            [&files[0], &files[1], &files[2], &files[4]].map(Clone::clone)
        );

        let missing = plan_from(&hub, REPO, Some("Q2_K")).await.unwrap_err();
        assert!(
            missing
                .to_string()
                .contains("No GGUF matching quant 'Q2_K'")
        );
        assert!(
            missing
                .to_string()
                .contains("Available: model-Q4_K_M.gguf, model-Q8_0.gguf")
        );
    }

    #[tokio::test]
    async fn a_model_the_hub_refuses_says_how_to_get_it() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let err = plan_from(&hub_for(&server), REPO, None).await.unwrap_err();
        assert!(err.to_string().contains("No model org/model-GGUF"));
        assert!(err.to_string().contains("HF_TOKEN"));
    }

    #[tokio::test]
    async fn a_download_lands_in_the_hf_hub_layout() {
        let server = MockServer::start().await;
        serve_all(&server).await;
        let root = tempdir().unwrap();
        let mut events = vec![];

        let outcome = download_from(
            &hub_for(&server),
            &test_plan(),
            root.path(),
            &CancellationToken::new(),
            |event| events.push(event),
        )
        .await
        .unwrap();

        assert_eq!(outcome, DownloadOutcome::Done);
        let repo = repo_dir(root.path(), REPO);
        assert_eq!(
            std::fs::read_to_string(repo.join("refs/main")).unwrap(),
            COMMIT
        );
        assert_eq!(snapshot_file(root.path(), "model-Q4_K_M.gguf"), MODEL);
        assert_eq!(snapshot_file(root.path(), "MTP/mtp-head.gguf"), MTP);
        assert_eq!(snapshot_file(root.path(), "config.json"), CONFIG);
        let link = std::fs::read_link(
            repo.join("snapshots")
                .join(COMMIT)
                .join("MTP/mtp-head.gguf"),
        )
        .unwrap();
        assert!(link.is_relative());

        let total = test_plan().total();
        assert_eq!(
            events.last(),
            Some(&DownloadEvent::Progress { done: total, total })
        );
        // nothing was on disk to check first
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, DownloadEvent::Verifying { .. }))
        );
    }

    #[tokio::test]
    async fn a_partial_file_is_continued_with_a_range() {
        let server = MockServer::start().await;
        serve_the_rest(&server).await;
        Mock::given(method("GET"))
            .and(path(resolve_path("model-Q4_K_M.gguf")))
            .and(header("range", "bytes=5-"))
            .respond_with(ResponseTemplate::new(206).set_body_bytes(&MODEL[5..]))
            .expect(1)
            .mount(&server)
            .await;
        let root = tempdir().unwrap();
        write_part(root.path(), &MODEL[..5]);
        let mut events = vec![];

        let outcome = download_from(
            &hub_for(&server),
            &test_plan(),
            root.path(),
            &CancellationToken::new(),
            |event| events.push(event),
        )
        .await
        .unwrap();

        assert_eq!(outcome, DownloadOutcome::Done);
        assert_eq!(snapshot_file(root.path(), "model-Q4_K_M.gguf"), MODEL);
        assert!(events.contains(&DownloadEvent::Verifying {
            file: "model-Q4_K_M.gguf".to_owned()
        }));
    }

    #[tokio::test]
    async fn a_redirect_to_the_cdn_keeps_the_range_and_drops_the_token() {
        let (hub_server, cdn) = (MockServer::start().await, MockServer::start().await);
        serve_the_rest(&hub_server).await;
        Mock::given(method("GET"))
            .and(path(resolve_path("model-Q4_K_M.gguf")))
            .and(header("authorization", "Bearer hf_secret"))
            .respond_with(
                ResponseTemplate::new(302)
                    .insert_header("location", format!("{}/cdn/model?signed=1", cdn.uri())),
            )
            .expect(1)
            .mount(&hub_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/cdn/model"))
            .and(header("range", "bytes=9-"))
            .respond_with(ResponseTemplate::new(206).set_body_bytes(&MODEL[9..]))
            .expect(1)
            .mount(&cdn)
            .await;
        let root = tempdir().unwrap();
        write_part(root.path(), &MODEL[..9]);
        let hub = Hub::new(&hub_server.uri(), Some("hf_secret".to_owned())).unwrap();

        run(&hub, &test_plan(), root.path()).await.unwrap();

        assert_eq!(snapshot_file(root.path(), "model-Q4_K_M.gguf"), MODEL);
        let cdn_requests = cdn.received_requests().await.unwrap();
        assert!(
            cdn_requests
                .iter()
                .all(|req| !req.headers.contains_key("authorization"))
        );
    }

    #[tokio::test]
    async fn a_server_that_ignores_the_range_starts_the_file_over() {
        let server = MockServer::start().await;
        serve_all(&server).await;
        let root = tempdir().unwrap();
        // appending to these bytes would fail the hash
        write_part(root.path(), b"XXXXX");

        run(&hub_for(&server), &test_plan(), root.path())
            .await
            .unwrap();

        assert_eq!(snapshot_file(root.path(), "model-Q4_K_M.gguf"), MODEL);
    }

    #[tokio::test]
    async fn an_old_hf_hub_partial_file_is_continued() {
        let server = MockServer::start().await;
        serve_the_rest(&server).await;
        Mock::given(method("GET"))
            .and(path(resolve_path("model-Q4_K_M.gguf")))
            .and(header("range", "bytes=7-"))
            .respond_with(ResponseTemplate::new(206).set_body_bytes(&MODEL[7..]))
            .expect(1)
            .mount(&server)
            .await;
        let root = tempdir().unwrap();
        let blobs = repo_dir(root.path(), REPO).join("blobs");
        std::fs::create_dir_all(&blobs).unwrap();
        // hf-hub's layout: the full size, the first 7 bytes real, then the byte count
        let mut sync_part = MODEL[..7].to_vec();
        sync_part.resize(MODEL.len(), 0);
        sync_part.extend_from_slice(&7u64.to_le_bytes());
        std::fs::write(
            blobs.join(format!("{}.sync.part", sha256_hex(MODEL))),
            sync_part,
        )
        .unwrap();

        run(&hub_for(&server), &test_plan(), root.path())
            .await
            .unwrap();

        assert_eq!(snapshot_file(root.path(), "model-Q4_K_M.gguf"), MODEL);
    }

    #[tokio::test]
    async fn a_failed_request_is_retried() {
        let server = MockServer::start().await;
        serve_the_rest(&server).await;
        Mock::given(method("GET"))
            .and(path(resolve_path("model-Q4_K_M.gguf")))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        serve_file(&server, "model-Q4_K_M.gguf", MODEL).await;
        let root = tempdir().unwrap();

        run(&hub_for(&server), &test_plan(), root.path())
            .await
            .unwrap();

        assert_eq!(snapshot_file(root.path(), "model-Q4_K_M.gguf"), MODEL);
    }

    #[tokio::test]
    async fn a_refusal_is_not_retried() {
        let server = MockServer::start().await;
        serve_the_rest(&server).await;
        Mock::given(method("GET"))
            .and(path(resolve_path("model-Q4_K_M.gguf")))
            .respond_with(ResponseTemplate::new(403))
            .expect(1)
            .mount(&server)
            .await;
        let root = tempdir().unwrap();

        let err = run(&hub_for(&server), &test_plan(), root.path())
            .await
            .unwrap_err();

        assert!(format!("{err:#}").contains("needs access"));
    }

    #[tokio::test]
    async fn cancelling_pauses_the_download() {
        let server = MockServer::start().await;
        serve_file(&server, "config.json", CONFIG).await;
        Mock::given(method("GET"))
            .and(path(resolve_path("model-Q4_K_M.gguf")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(MODEL)
                    .set_delay(Duration::from_secs(10)),
            )
            .mount(&server)
            .await;
        let root = tempdir().unwrap();
        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            stop.cancel();
        });

        let outcome = tokio::time::timeout(
            Duration::from_secs(5),
            download_from(
                &hub_for(&server),
                &test_plan(),
                root.path(),
                &cancel,
                |_| {},
            ),
        )
        .await
        .expect("cancel didn't stop the download")
        .unwrap();

        assert_eq!(outcome, DownloadOutcome::Paused);
        let blob = repo_dir(root.path(), REPO)
            .join("blobs")
            .join(sha256_hex(MODEL));
        assert!(!blob.exists());
    }

    #[tokio::test]
    async fn a_wrong_hash_deletes_the_file() {
        let server = MockServer::start().await;
        serve_all(&server).await;
        let root = tempdir().unwrap();
        let mut plan = test_plan();
        plan.files[1].sha256 = Some("0".repeat(64));

        let err = run(&hub_for(&server), &plan, root.path())
            .await
            .unwrap_err();

        assert!(err.to_string().contains("SHA-256"));
        let blobs = repo_dir(root.path(), REPO).join("blobs");
        assert!(!blobs.join(format!("{}.part", plan.files[1].blob)).exists());
        assert!(!blobs.join(&plan.files[1].blob).exists());
    }

    #[tokio::test]
    async fn a_file_another_process_is_downloading_is_refused() {
        let server = MockServer::start().await;
        serve_all(&server).await;
        let root = tempdir().unwrap();
        let blobs = repo_dir(root.path(), REPO).join("blobs");
        std::fs::create_dir_all(&blobs).unwrap();
        let _held = lock(&blobs.join(format!("{}.lock", sha256_hex(MODEL))))
            .await
            .unwrap();

        let err = run(&hub_for(&server), &test_plan(), root.path())
            .await
            .unwrap_err();

        assert!(err.downcast_ref::<AlreadyDownloading>().is_some());
    }

    #[tokio::test]
    async fn on_disk_counts_finished_and_partial_files() {
        let server = MockServer::start().await;
        serve_all(&server).await;
        let root = tempdir().unwrap();
        let plan = test_plan();
        assert_eq!(on_disk(&plan, root.path()).await, 0);

        write_part(root.path(), &MODEL[..5]);
        assert_eq!(on_disk(&plan, root.path()).await, 5);

        run(&hub_for(&server), &plan, root.path()).await.unwrap();
        assert_eq!(on_disk(&plan, root.path()).await, plan.total());
    }

    #[tokio::test]
    async fn finished_files_are_not_downloaded_again() {
        let server = MockServer::start().await;
        serve_all(&server).await;
        let root = tempdir().unwrap();
        let plan = test_plan();
        run(&hub_for(&server), &plan, root.path()).await.unwrap();
        server.reset().await;

        let outcome = run(&hub_for(&server), &plan, root.path()).await.unwrap();

        assert_eq!(outcome, DownloadOutcome::Done);
        assert!(server.received_requests().await.unwrap().is_empty());
        assert_eq!(snapshot_file(root.path(), "model-Q4_K_M.gguf"), MODEL);
    }

    // talks to the real Hugging Face, so only run by hand: cargo test -- --ignored
    #[tokio::test]
    #[ignore]
    async fn the_default_model_plans_against_hugging_face() {
        let plan = plan("unsloth/gemma-4-12b-it-GGUF", Some("Q4_K_M"))
            .await
            .unwrap();
        let names: Vec<&str> = plan.files.iter().map(|file| file.name.as_str()).collect();
        println!(
            "{} at {}: {names:?}, {} bytes",
            plan.repo,
            plan.commit,
            plan.total()
        );
        assert!(names.contains(&"gemma-4-12b-it-Q4_K_M.gguf"));
        assert!(names.contains(&"mtp-gemma-4-12b-it.gguf"));
        assert!(
            !names
                .iter()
                .any(|name| name.starts_with("MTP/") && name.ends_with(".gguf"))
        );
        assert!(plan.files.iter().all(|file| file.size > 0));
    }

    // downloads the 465 MB mtp head from the real Hugging Face, so only run by hand
    #[tokio::test]
    #[ignore]
    async fn a_real_download_pauses_and_resumes() {
        let mut plan = plan("unsloth/gemma-4-12b-it-GGUF", Some("Q4_K_M"))
            .await
            .unwrap();
        plan.files
            .retain(|file| file.name == "mtp-gemma-4-12b-it.gguf");
        let file = plan.files[0].clone();
        let root = tempdir().unwrap();
        let part = repo_dir(root.path(), &plan.repo)
            .join("blobs")
            .join(format!("{}.part", file.blob));

        let cancel = CancellationToken::new();
        let stop = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            stop.cancel();
        });
        let started = std::time::Instant::now();
        let outcome = download(&plan, root.path(), &cancel, |_| {}).await.unwrap();
        let paused_at = std::fs::metadata(&part).unwrap().len();
        let secs = started.elapsed().as_secs_f64();
        println!(
            "paused after {secs:.1}s at {paused_at} of {} bytes, {:.1} MB/s",
            file.size,
            paused_at as f64 / secs / 1e6
        );
        assert_eq!(outcome, DownloadOutcome::Paused);

        // bytes the process reads from anything, sockets included
        let read_so_far = || {
            std::fs::read_to_string("/proc/self/io")
                .unwrap()
                .lines()
                .find_map(|line| line.strip_prefix("rchar: "))
                .unwrap()
                .parse::<u64>()
                .unwrap()
        };
        let read_before = read_so_far();
        tokio::time::sleep(Duration::from_secs(5)).await;
        let read_while_paused = read_so_far() - read_before;
        let later = std::fs::metadata(&part).unwrap().len();
        println!("5s after pausing: {later} bytes on disk, {read_while_paused} bytes read");
        assert_eq!(later, paused_at, "kept writing after the pause");
        assert!(
            read_while_paused < 1_000_000,
            "kept downloading after the pause"
        );

        let resumed = std::time::Instant::now();
        let mut verified = false;
        let outcome = download(&plan, root.path(), &CancellationToken::new(), |event| {
            if matches!(event, DownloadEvent::Verifying { .. }) {
                verified = true;
            }
        })
        .await
        .unwrap();
        println!(
            "resumed and finished in {:.1}s",
            resumed.elapsed().as_secs_f64()
        );
        assert_eq!(outcome, DownloadOutcome::Done);
        assert!(verified);
        let snapshot = repo_dir(root.path(), &plan.repo)
            .join("snapshots")
            .join(&plan.commit)
            .join(&file.name);
        assert_eq!(std::fs::metadata(snapshot).unwrap().len(), file.size);
    }

    #[test]
    fn test_is_main_gguf() {
        assert!(is_main_gguf("gemma-4-12b-it-Q4_K_M.gguf"));
        assert!(is_main_gguf("Some-Model-Q8_0.GGUF"));
        assert!(!is_main_gguf("mmproj-F16.gguf"));
        assert!(!is_main_gguf("mtp-gemma-4-12b-it.gguf"));
        assert!(!is_main_gguf("config.json"));
    }
}
