//! The model routes against a mock Hub, with a temp folder as the models folder

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

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
    CLIENT_HEADER, DownloadState, ModelDownload, Models, Picked, Spec, bundled::BUNDLED,
    model_router, modelfile_spec,
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

// the GGUF is answered after `delay`
async fn serve_model(server: &MockServer, repo: &str, delay: Duration) {
    Mock::given(method("GET"))
        .and(path(format!("/api/models/{repo}/revision/main")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sha": COMMIT,
            "siblings": [
                { "rfilename": "config.json", "size": CONFIG.len(), "blobId": "0123456789abcdef0123456789abcdef01234567" },
                {
                    "rfilename": "model-Q4_K_M.gguf",
                    "size": MODEL.len(),
                    "blobId": "89abcdef0123456789abcdef0123456789abcdef",
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

    // a plain stream leaves the download running when it closes
    drop(daemon.send("GET", "/v1/tilekit/model/download", None).await);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(matches!(
        *daemon.download(None).state.borrow(),
        DownloadState::Downloading { .. }
    ));

    // refused without the CLI's header
    let (status, _) = daemon
        .call(
            "GET",
            "/v1/tilekit/model/download?pause_on_close=true",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(matches!(
        *daemon.download(None).state.borrow(),
        DownloadState::Downloading { .. }
    ));

    // the CLI's stream pauses it
    let req = Request::builder()
        .uri("/v1/tilekit/model/download?pause_on_close=true")
        .header(CLIENT_HEADER, "cli")
        .body(Body::empty())
        .unwrap();
    let cli = model_router()
        .with_state(daemon.state.clone())
        .oneshot(req)
        .await
        .unwrap();
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

    let (status, body) = daemon
        .json(
            "GET",
            &format!("/v1/tilekit/model/status?model={REPO}"),
            None,
        )
        .await;

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

    // a later client gets the end state, and the stream closes
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

    for model in [
        "gemma",
        "org/name/extra:Q4_K_M",
        "/name",
        "org/name?x=1",
        "org/..",
        "org/name#top",
    ] {
        let (status, _) = daemon.start(json!({ "model": model })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{model}");
    }
}

#[tokio::test]
async fn starting_with_no_model_and_no_default_is_a_404() {
    let state = Arc::new(AppState::for_tests());
    let req = Request::builder()
        .method("POST")
        .uri("/v1/tilekit/model/download")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let res = model_router().with_state(state).oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn status_with_no_model_lists_every_bundled_model() {
    let server = hub_with_model(Duration::ZERO).await;
    serve_model(&server, "org/e2b-GGUF", Duration::ZERO).await;
    serve_model(&server, "org/e4b-GGUF", Duration::ZERO).await;
    let daemon = Daemon::with(&server, tempdir().unwrap(), |models| {
        models.picked = Picked::Memory(Mutex::new(Some("org/e4b-GGUF:q4_k_m".to_owned())));
    });
    let modelfiles = daemon.root.path().join("modelfiles");
    std::fs::create_dir_all(&modelfiles).unwrap();
    // no 31B Modelfile, and the 26B's model isn't on the Hub
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

    let (status, body) = daemon.json("GET", "/v1/tilekit/model/status", None).await;

    assert_eq!(status, StatusCode::OK);
    let models = &body["data"]["models"];
    let ids: Vec<&str> = models
        .as_array()
        .unwrap()
        .iter()
        .map(|model| model["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, BUNDLED.map(|(id, _)| id));
    let total = CONFIG.len() + MODEL.len();
    let [e2b, e4b, b12, b26, b31] = [0, 1, 2, 3, 4].map(|i| &models[i]);
    assert_eq!(e2b["label"], "Gemma 4 E2B");
    assert_eq!(e2b["model"], "org/e2b-GGUF:Q4_K_M");
    assert_eq!(e2b["total"], total);
    assert_eq!(e2b["download"], json!({ "state": "idle" }));
    assert_eq!(
        (e4b["current"].clone(), e2b["current"].clone()),
        (json!(true), json!(false))
    );
    // the test daemon's default is the 12B's model
    assert_eq!(
        (b12["default"].clone(), e2b["default"].clone()),
        (json!(true), json!(false))
    );
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
async fn a_bundled_model_downloading_shows_in_the_list() {
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
    let (_, body) = daemon.json("GET", "/v1/tilekit/model/status", None).await;

    assert_eq!(
        body["data"]["models"][2]["download"]["state"],
        "downloading"
    );
}

#[test]
fn the_modelfiles_folder_has_exactly_the_bundled_models() {
    let modelfiles = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../modelfiles");
    let mut names: Vec<String> = std::fs::read_dir(&modelfiles)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut bundled = BUNDLED.map(|(id, _)| id.to_owned()).to_vec();
    bundled.sort();
    assert_eq!(names, bundled);
    for (id, _) in BUNDLED {
        let spec = modelfile_spec(&modelfiles.join(id)).unwrap();
        assert!(spec.repo.starts_with("unsloth/gemma-4-"), "{id}: {spec}");
    }
}

#[tokio::test]
async fn the_downloads_list_has_only_unfinished_downloads() {
    let server = hub_with_model(Duration::from_secs(10)).await;
    serve_model(&server, OTHER, Duration::ZERO).await;
    let daemon = Daemon::new(&server);
    daemon.start(json!({})).await;
    // never started
    daemon
        .json(
            "GET",
            &format!("/v1/tilekit/model/status?model={OTHER}"),
            None,
        )
        .await;

    let (status, body) = daemon
        .json("GET", "/v1/tilekit/model/downloads", None)
        .await;

    assert_eq!(status, StatusCode::OK);
    let downloads = body["data"]["downloads"].as_array().unwrap();
    assert_eq!(downloads.len(), 1);
    assert_eq!(downloads[0]["model"], "org/model-GGUF:Q4_K_M");
    assert_eq!(downloads[0]["state"], "downloading");
}

#[tokio::test]
async fn the_events_stream_follows_downloads_started_by_any_client() {
    let server = hub_with_model(Duration::from_secs(10)).await;
    serve_model(&server, OTHER, Duration::from_millis(300)).await;
    let daemon = Daemon::new(&server);
    daemon.start(json!({})).await;

    // the UI opens the stream, then another client starts a download
    let mut stream = daemon
        .send("GET", "/v1/tilekit/model/events", None)
        .await
        .into_body();
    daemon.start(json!({ "model": OTHER })).await;

    let mut seen = String::new();
    let done = r#"data: {"model":"org/other-GGUF:Q4_K_M","state":"done"}"#;
    tokio::time::timeout(Duration::from_secs(5), async {
        while !seen.contains(done) {
            let frame = stream.frame().await.unwrap().unwrap();
            if let Ok(data) = frame.into_data() {
                seen.push_str(&String::from_utf8_lossy(&data));
            }
        }
    })
    .await
    .expect("the other download never showed as done");

    // what was already running comes first
    let first = seen
        .lines()
        .find(|line| line.starts_with("data: "))
        .unwrap();
    assert!(first.contains(r#""model":"org/model-GGUF:Q4_K_M","state":"downloading""#));
    assert!(seen.contains(r#""model":"org/other-GGUF:Q4_K_M","state":"downloading""#));
    assert!(seen.contains("event: model"));
}

#[tokio::test]
async fn only_a_downloaded_bundled_model_can_be_picked() {
    let server = hub_with_model(Duration::ZERO).await;
    serve_model(&server, OTHER, Duration::ZERO).await;
    let daemon = Daemon::new(&server);
    let modelfiles = daemon.root.path().join("modelfiles");
    std::fs::create_dir_all(&modelfiles).unwrap();
    std::fs::write(
        modelfiles.join("gemma-4-12b-gguf"),
        format!("FROM {REPO}\n"),
    )
    .unwrap();
    let pick = |model: &str| json!({ "model": model });
    let select = "/v1/tilekit/model/select";

    let (status, _) = daemon.json("POST", select, Some(pick(REPO))).await;
    assert_eq!(status, StatusCode::CONFLICT, "not downloaded yet");

    daemon.start(json!({})).await;
    daemon.call("GET", "/v1/tilekit/model/download", None).await;
    let (status, body) = daemon.json("POST", select, Some(pick(REPO))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["model"], format!("{REPO}:Q4_K_M"));
    assert_eq!(
        daemon.state.models.current(),
        Some(Spec::parse(REPO).unwrap())
    );

    let (status, _) = daemon.json("POST", select, Some(pick(OTHER))).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Tiles has no Modelfile for it"
    );
}

#[tokio::test]
async fn deleting_a_model_frees_its_files_but_never_the_one_chats_use() {
    let server = hub_with_model(Duration::ZERO).await;
    serve_model(&server, OTHER, Duration::ZERO).await;
    let daemon = Daemon::new(&server);
    let delete = "/v1/tilekit/model/delete";
    let other = json!({ "model": OTHER });

    // the default is what chats use until another is picked
    let (status, _) = daemon
        .json("POST", delete, Some(json!({ "model": REPO })))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = daemon.json("POST", delete, Some(other.clone())).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "nothing on disk yet");

    daemon.start(other.clone()).await;
    let progress = format!("/v1/tilekit/model/download?model={OTHER}");
    daemon.call("GET", &progress, None).await;
    let folder = repo_dir(daemon.root.path(), OTHER);
    assert!(folder.exists());

    let (status, body) = daemon.json("POST", delete, Some(other.clone())).await;
    assert_eq!(status, StatusCode::OK);
    // the blobs, and the ref naming the commit
    assert_eq!(
        body["data"]["freed"],
        CONFIG.len() + MODEL.len() + COMMIT.len()
    );
    assert!(!folder.exists());
    let (_, status_body) = daemon
        .json(
            "GET",
            &format!("/v1/tilekit/model/status?model={OTHER}"),
            None,
        )
        .await;
    assert_eq!(status_body["data"]["download"], json!({ "state": "idle" }));
}

#[tokio::test]
async fn a_picked_model_tiles_doesnt_ship_leaves_the_default_in_use() {
    let server = hub_with_model(Duration::ZERO).await;
    // what `tiles run` saves for a custom Modelfile
    let daemon = Daemon::with(&server, tempdir().unwrap(), |models| {
        models.picked = Picked::Memory(Mutex::new(Some("org/custom-GGUF:Q4_K_M".to_owned())));
    });

    let (status, _) = daemon
        .json(
            "POST",
            "/v1/tilekit/model/delete",
            Some(json!({ "model": REPO })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn a_model_downloading_cant_be_deleted() {
    let server = hub_with_model(Duration::from_millis(500)).await;
    serve_model(&server, OTHER, Duration::from_millis(500)).await;
    let daemon = Daemon::new(&server);
    let other = json!({ "model": OTHER });
    daemon.start(other.clone()).await;

    let (status, body) = daemon
        .json("POST", "/v1/tilekit/model/delete", Some(other))
        .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert!(body["reason"].as_str().unwrap().contains("pause it"));
}
