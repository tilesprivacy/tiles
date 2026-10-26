//! The model routes against a mock Hub, with a temp folder as the models folder

use std::{sync::Arc, time::Duration};

use axum::{
    body::Body,
    http::{Request, StatusCode, header::CONTENT_TYPE},
    response::Response,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::{TempDir, tempdir};
use tower::ServiceExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

use super::{
    DownloadState, ModelDownload, Models, Spec, lineup::LINEUP, model_router, modelfile_spec,
};
use crate::{
    daemon::AppState,
    utils::hf_model_downloader::{Hub, repo_dir},
};

const REPO: &str = "org/model-GGUF";
const OTHER: &str = "org/other-GGUF";
const COMMIT: &str = "c0ffee";
const MODEL: &[u8] = b"GGUF the model weights";
const CONFIG: &[u8] = b"{}";

fn model_sha256() -> String {
    format!("{:x}", Sha256::digest(MODEL))
}

/// Serves `repo` as a small model on the mock Hub, its GGUF answered after `delay`
async fn serve_model(server: &MockServer, repo: &str, delay: Duration) {
    Mock::given(method("GET"))
        .and(path(format!("/api/models/{repo}/revision/main")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sha": COMMIT,
            "siblings": [
                { "rfilename": "config.json", "size": CONFIG.len(), "blobId": "a-git-id" },
                {
                    "rfilename": "model-Q4_K_M.gguf",
                    "size": MODEL.len(),
                    "blobId": "another-git-id",
                    "lfs": { "sha256": model_sha256(), "size": MODEL.len() },
                },
            ],
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/{repo}/resolve/{COMMIT}/config.json")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(CONFIG))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/{repo}/resolve/{COMMIT}/model-Q4_K_M.gguf")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(MODEL)
                .set_delay(delay),
        )
        .mount(server)
        .await;
}

async fn hub_with_model(delay: Duration) -> MockServer {
    let server = MockServer::start().await;
    serve_model(&server, REPO, delay).await;
    server
}

struct Daemon {
    state: Arc<AppState>,
    root: TempDir,
}

impl Daemon {
    fn new(server: &MockServer) -> Self {
        Self::with(server, tempdir().unwrap(), |_| {})
    }

    /// A daemon whose default model is `REPO`, with `change` made to its models first
    fn with(server: &MockServer, root: TempDir, change: impl FnOnce(&mut Models)) -> Self {
        let mut models = Models::new(
            Hub::new(&server.uri(), None).unwrap(),
            root.path().to_path_buf(),
            root.path().join("modelfiles"),
            Some(Spec::parse(REPO).unwrap()),
        );
        change(&mut models);
        let mut state = AppState::for_tests();
        state.models = models;
        Self {
            state: Arc::new(state),
            root,
        }
    }

    fn download(&self, model: Option<&str>) -> Arc<ModelDownload> {
        self.state.models.get(model).unwrap()
    }

    async fn send(&self, method: &str, uri: &str, body: Option<Value>) -> Response {
        let req = Request::builder().method(method).uri(uri);
        let req = match body {
            Some(body) => req
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => req.body(Body::empty()),
        };
        model_router()
            .with_state(self.state.clone())
            .oneshot(req.unwrap())
            .await
            .unwrap()
    }

    async fn call(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, String) {
        let res = self.send(method, uri, body).await;
        let status = res.status();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    async fn json(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        let (status, body) = self.call(method, uri, body).await;
        (status, serde_json::from_str(&body).unwrap())
    }

    async fn start(&self, body: Value) -> (StatusCode, Value) {
        self.json("POST", "/v1/tilekit/model/download", Some(body))
            .await
    }

    async fn until(&self, model: Option<&str>, done: impl FnMut(&DownloadState) -> bool) {
        let mut changes = self.download(model).state.subscribe();
        tokio::time::timeout(Duration::from_secs(5), changes.wait_for(done))
            .await
            .expect("the download never got there")
            .unwrap();
    }
}

#[tokio::test]
async fn a_second_start_joins_the_running_download() {
    let server = hub_with_model(Duration::from_millis(300)).await;
    let daemon = Daemon::new(&server);

    let (status, first) = daemon.start(json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["data"]["state"], "downloading");
    // the same model named in full is the same download
    let (_, second) = daemon
        .start(json!({ "model": "org/model-GGUF:q4_k_m" }))
        .await;
    assert_eq!(second["data"]["state"], "downloading");

    daemon
        .until(None, |state| *state == DownloadState::Done)
        .await;
    let gguf_requests = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|req| req.url.path().ends_with(".gguf"))
        .count();
    assert_eq!(gguf_requests, 1);
}

#[tokio::test]
async fn a_named_model_downloads_next_to_the_default() {
    let server = hub_with_model(Duration::from_millis(300)).await;
    serve_model(&server, OTHER, Duration::from_millis(300)).await;
    let daemon = Daemon::new(&server);

    let (_, default) = daemon.start(json!({})).await;
    let (status, other) = daemon.start(json!({ "model": OTHER })).await;

    assert_eq!(default["data"]["state"], "downloading");
    assert_eq!(status, StatusCode::OK);
    assert_eq!(other["data"]["state"], "downloading");
    daemon
        .until(None, |state| *state == DownloadState::Done)
        .await;
    daemon
        .until(Some(OTHER), |state| *state == DownloadState::Done)
        .await;
    assert!(
        repo_dir(daemon.root.path(), OTHER)
            .join("refs/main")
            .exists()
    );
}

#[tokio::test]
async fn pausing_keeps_what_was_downloaded() {
    let server = hub_with_model(Duration::from_secs(10)).await;
    let daemon = Daemon::new(&server);
    daemon.start(json!({})).await;
    // the config comes at once, the model never does
    tokio::time::sleep(Duration::from_millis(200)).await;

    let (status, paused) = daemon
        .json("DELETE", "/v1/tilekit/model/download", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(paused["data"]["state"], "paused");
    assert_eq!(paused["data"]["done"], CONFIG.len());
    assert_eq!(paused["data"]["total"], CONFIG.len() + MODEL.len());

    let (status, _) = daemon
        .json("DELETE", "/v1/tilekit/model/download", None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn closing_a_stream_that_asked_to_pause_pauses_the_download() {
    let server = hub_with_model(Duration::from_secs(10)).await;
    let daemon = Daemon::new(&server);
    daemon.start(json!({})).await;

    // a plain stream, as the UI opens, leaves the download running when it closes
    drop(daemon.send("GET", "/v1/tilekit/model/download", None).await);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(matches!(
        *daemon.download(None).state.borrow(),
        DownloadState::Downloading { .. }
    ));

    // the CLI's stream pauses it, which is what Ctrl-C does
    let cli = daemon
        .send(
            "GET",
            "/v1/tilekit/model/download?pause_on_close=true",
            None,
        )
        .await;
    drop(cli);
    daemon
        .until(None, |state| matches!(state, DownloadState::Paused { .. }))
        .await;
}

#[tokio::test]
async fn status_after_a_restart_shows_a_part_download_as_paused() {
    let server = hub_with_model(Duration::ZERO).await;
    let root = tempdir().unwrap();
    let blobs = repo_dir(root.path(), REPO).join("blobs");
    std::fs::create_dir_all(&blobs).unwrap();
    std::fs::write(blobs.join(format!("{}.part", model_sha256())), &MODEL[..5]).unwrap();
    let daemon = Daemon::with(&server, root, |_| {});

    let (status, body) = daemon.json("GET", "/v1/tilekit/model/status", None).await;

    assert_eq!(status, StatusCode::OK);
    let data = &body["data"];
    assert_eq!(data["model"], "org/model-GGUF:Q4_K_M");
    assert_eq!(data["on_disk"], 5);
    assert_eq!(
        data["download"],
        json!({ "state": "paused", "done": 5, "total": CONFIG.len() + MODEL.len() })
    );
    assert!(data["free_disk"].as_u64().is_some_and(|free| free > 0));
}

#[tokio::test]
async fn the_progress_stream_follows_the_download_to_the_end() {
    let server = hub_with_model(Duration::from_millis(300)).await;
    let daemon = Daemon::new(&server);
    daemon.start(json!({})).await;

    let (status, events) = daemon.call("GET", "/v1/tilekit/model/download", None).await;

    assert_eq!(status, StatusCode::OK);
    let states: Vec<Value> = events
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|data| serde_json::from_str(data).unwrap())
        .collect();
    assert_eq!(states.first().unwrap()["state"], "downloading");
    assert_eq!(states.last().unwrap(), &json!({ "state": "done" }));

    // a client that comes later gets where it ended, and the stream closes
    let (_, late) = daemon.call("GET", "/v1/tilekit/model/download", None).await;
    assert_eq!(late.matches("data: ").count(), 1);
    assert!(late.contains(r#"{"state":"done"}"#));
}

#[tokio::test]
async fn a_download_too_big_for_the_disk_is_refused() {
    let server = hub_with_model(Duration::ZERO).await;
    let daemon = Daemon::with(&server, tempdir().unwrap(), |models| {
        models.spare_room = u64::MAX / 2;
    });

    let (status, body) = daemon.start(json!({})).await;

    assert_eq!(status, StatusCode::INSUFFICIENT_STORAGE);
    assert!(body["reason"].as_str().unwrap().contains("free disk space"));
    assert_eq!(*daemon.download(None).state.borrow(), DownloadState::Idle);
    assert!(!repo_dir(daemon.root.path(), REPO).exists());
}

#[tokio::test]
async fn a_model_the_hub_doesnt_have_is_unprocessable() {
    let server = hub_with_model(Duration::ZERO).await;
    let daemon = Daemon::new(&server);

    let (status, body) = daemon.start(json!({ "model": "org/missing-GGUF" })).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        body["reason"]
            .as_str()
            .unwrap()
            .contains("org/missing-GGUF")
    );
}

#[tokio::test]
async fn a_badly_named_model_is_a_bad_request() {
    let server = hub_with_model(Duration::ZERO).await;
    let daemon = Daemon::new(&server);

    for model in ["gemma", "org/name/extra:Q4_K_M", "/name"] {
        let (status, _) = daemon.start(json!({ "model": model })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{model}");
    }
}

#[tokio::test]
async fn no_model_named_and_no_default_is_a_404() {
    let state = Arc::new(AppState::for_tests());
    let req = Request::builder()
        .uri("/v1/tilekit/model/status")
        .body(Body::empty())
        .unwrap();
    let res = model_router().with_state(state).oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_lineup_lists_each_modelfile_with_its_download() {
    let server = hub_with_model(Duration::ZERO).await;
    serve_model(&server, "org/e2b-GGUF", Duration::ZERO).await;
    serve_model(&server, "org/e4b-GGUF", Duration::ZERO).await;
    let daemon = Daemon::new(&server);
    let modelfiles = daemon.root.path().join("modelfiles");
    std::fs::create_dir_all(&modelfiles).unwrap();
    // the 31B's Modelfile is left out, and the 26B names a model the Hub doesn't have
    for (id, from) in [
        ("gemma-4-e2b-gguf", "org/e2b-GGUF:Q4_K_M"),
        ("gemma-4-e4b-gguf", "org/e4b-GGUF:Q4_K_M"),
        ("gemma-4-12b-gguf", "org/model-GGUF:Q4_K_M"),
        ("gemma-4-26b-a4b-gguf", "org/missing-GGUF:UD-Q4_K_M"),
    ] {
        std::fs::write(modelfiles.join(id), format!("FROM {from}\n")).unwrap();
    }
    let blobs = repo_dir(daemon.root.path(), REPO).join("blobs");
    std::fs::create_dir_all(&blobs).unwrap();
    std::fs::write(blobs.join(format!("{}.part", model_sha256())), &MODEL[..5]).unwrap();

    let lineup = daemon
        .state
        .models
        .lineup(Some("org/e4b-GGUF:q4_k_m"))
        .await;
    let lineup = serde_json::to_value(lineup).unwrap();

    let ids: Vec<&str> = lineup
        .as_array()
        .unwrap()
        .iter()
        .map(|model| model["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, LINEUP.map(|(id, _)| id));
    let total = CONFIG.len() + MODEL.len();
    let [e2b, e4b, b12, b26, b31] = [0, 1, 2, 3, 4].map(|i| &lineup[i]);
    assert_eq!(e2b["label"], "Gemma 4 E2B");
    assert_eq!(e2b["model"], "org/e2b-GGUF:Q4_K_M");
    assert_eq!(e2b["total"], total);
    assert_eq!(e2b["download"], json!({ "state": "idle" }));
    assert_eq!(e4b["current"], true);
    assert_eq!(e2b["current"], false);
    assert_eq!(b12["on_disk"], 5);
    assert_eq!(
        b12["download"],
        json!({ "state": "paused", "done": 5, "total": total })
    );
    assert_eq!(b26["model"], "org/missing-GGUF:UD-Q4_K_M");
    assert!(b26["error"].as_str().unwrap().contains("org/missing-GGUF"));
    assert_eq!(b31["model"], Value::Null);
    assert!(b31["error"].as_str().is_some());
}

#[tokio::test]
async fn a_lineup_download_shows_in_the_lineup() {
    let server = hub_with_model(Duration::from_millis(300)).await;
    let daemon = Daemon::new(&server);
    let modelfiles = daemon.root.path().join("modelfiles");
    std::fs::create_dir_all(&modelfiles).unwrap();
    std::fs::write(
        modelfiles.join("gemma-4-12b-gguf"),
        "FROM org/model-GGUF:Q4_K_M\n",
    )
    .unwrap();

    daemon
        .start(json!({ "model": "org/model-GGUF:Q4_K_M" }))
        .await;
    let lineup = serde_json::to_value(daemon.state.models.lineup(None).await).unwrap();

    assert_eq!(lineup[2]["download"]["state"], "downloading");
}

#[test]
fn every_lineup_model_has_a_bundled_modelfile() {
    let modelfiles = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../modelfiles");
    for (id, _) in LINEUP {
        let spec = modelfile_spec(&modelfiles.join(id)).unwrap();
        assert!(spec.repo.starts_with("unsloth/gemma-4-"), "{id}: {spec}");
    }
}
