//! Downloads against a mock Hub, and two against the real one, run by hand

use std::{path::Path, time::Duration};

use anyhow::Result;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

use super::{
    DownloadEvent, DownloadOutcome, DownloadPlan, Hub, ModelFile, cache, download, on_disk, plan,
    repo_dir,
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
            // a git id, 40 hex digits
            hash[..40].to_owned()
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
    download(hub, plan, root, &CancellationToken::new(), |_| {}).await
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
        "blobId": if file.sha256.is_some() { "0123456789abcdef0123456789abcdef01234567" } else { file.blob.as_str() },
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
    let found = plan(&hub, REPO, Some("Q4_K_M")).await.unwrap();
    assert_eq!(found.commit, COMMIT);
    assert_eq!(
        found.files,
        [&files[0], &files[1], &files[2], &files[4]].map(Clone::clone)
    );

    let missing = plan(&hub, REPO, Some("Q2_K")).await.unwrap_err();
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

    let err = plan(&hub_for(&server), REPO, None).await.unwrap_err();
    assert!(err.to_string().contains("No model org/model-GGUF"));
    assert!(err.to_string().contains("HF_TOKEN"));
}

#[tokio::test]
async fn a_download_lands_in_the_hf_hub_layout() {
    let server = MockServer::start().await;
    serve_all(&server).await;
    let root = tempdir().unwrap();
    let mut events = vec![];

    let outcome = download(
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

    let outcome = download(
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
        download(
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
    let repo = repo_dir(root.path(), REPO);
    assert!(!repo.join("blobs").join(sha256_hex(MODEL)).exists());
    // the config finished, but the snapshot waits for every file
    assert!(repo.join("blobs").join(&test_plan().files[0].blob).exists());
    assert!(!repo.join("snapshots").exists());
    assert!(!repo.join("refs/main").exists());
}

#[tokio::test]
async fn a_plan_with_an_unsafe_file_name_is_refused() {
    let server = MockServer::start().await;
    let mut escaping = model_file("model-Q4_K_M.gguf", MODEL, true);
    escaping.name = "../../escape-Q4_K_M.gguf".to_owned();
    Mock::given(method("GET"))
        .and(path(format!("/api/models/{REPO}/revision/main")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sha": COMMIT,
            "siblings": [sibling(&model_file("model-Q8_0.gguf", MODEL, true)), sibling(&escaping)],
        })))
        .mount(&server)
        .await;

    let err = plan(&hub_for(&server), REPO, Some("Q4_K_M"))
        .await
        .unwrap_err();

    assert!(err.to_string().contains("isn't safe to save"));
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
async fn a_file_another_download_holds_is_waited_for() {
    let server = MockServer::start().await;
    serve_all(&server).await;
    let root = tempdir().unwrap();
    let blobs = repo_dir(root.path(), REPO).join("blobs");
    std::fs::create_dir_all(&blobs).unwrap();
    let lock_path = blobs.join(format!("{}.lock", sha256_hex(MODEL)));
    let held = cache::lock(&lock_path, &CancellationToken::new())
        .await
        .unwrap()
        .unwrap();

    let hub = hub_for(&server);
    let plan = test_plan();
    let downloading = run(&hub, &plan, root.path());
    tokio::pin!(downloading);
    assert!(
        tokio::time::timeout(Duration::from_millis(300), &mut downloading)
            .await
            .is_err(),
        "didn't wait for the lock"
    );
    drop(held);

    let outcome = tokio::time::timeout(Duration::from_secs(5), downloading)
        .await
        .expect("never got the lock")
        .unwrap();
    assert_eq!(outcome, DownloadOutcome::Done);
    assert_eq!(snapshot_file(root.path(), "model-Q4_K_M.gguf"), MODEL);
}

#[tokio::test]
async fn cancelling_while_waiting_for_a_lock_pauses() {
    let server = MockServer::start().await;
    serve_all(&server).await;
    let root = tempdir().unwrap();
    let blobs = repo_dir(root.path(), REPO).join("blobs");
    std::fs::create_dir_all(&blobs).unwrap();
    let _held = cache::lock(
        &blobs.join(format!("{}.lock", sha256_hex(MODEL))),
        &CancellationToken::new(),
    )
    .await
    .unwrap()
    .unwrap();
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        stop.cancel();
    });

    let outcome = download(
        &hub_for(&server),
        &test_plan(),
        root.path(),
        &cancel,
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(outcome, DownloadOutcome::Paused);
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
    let hub = Hub::from_env().unwrap();
    let plan = plan(&hub, "unsloth/gemma-4-12b-it-GGUF", Some("Q4_K_M"))
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
    let hub = Hub::from_env().unwrap();
    let mut plan = plan(&hub, "unsloth/gemma-4-12b-it-GGUF", Some("Q4_K_M"))
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
    let outcome = download(&hub, &plan, root.path(), &cancel, |_| {})
        .await
        .unwrap();
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
    let outcome = download(
        &hub,
        &plan,
        root.path(),
        &CancellationToken::new(),
        |event| {
            if matches!(event, DownloadEvent::Verifying { .. }) {
                verified = true;
            }
        },
    )
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
