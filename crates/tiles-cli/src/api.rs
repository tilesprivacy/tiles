//! Calls to the Tiles daemon

use std::fmt;

use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, RequestBuilder, Response, StatusCode, Url, header::CONTENT_TYPE};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

/// The running daemon's HTTP API, as the CLI calls it
#[derive(Debug, Clone)]
pub struct DaemonApi {
    client: Client,
    address: String,
}

/// Saved chat session
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
}

/// The ATmosphere account that is signed in
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AtprotoAccount {
    pub handle: String,
    pub did: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShareVisibility {
    /// Encrypted, with the key only in the link
    Private,
    Public,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShareOutcome {
    Shared { url: String },
    NotSignedIn,
    NoSuchSession,
}

#[derive(Deserialize)]
struct SharedLink {
    url: String,
}

/// One llama.cpp setting reported by the daemon
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct LlamaSetting {
    pub name: String,
    pub value: Option<Value>,
    pub default: String,
    #[serde(default)]
    pub choices: Vec<SettingChoice>,
    pub help: String,
}

/// One allowed value for a choice setting
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct SettingChoice {
    pub value: String,
}

#[derive(Deserialize)]
struct SettingsResponse {
    settings: Vec<LlamaSetting>,
}

/// What starting or stopping the inference server did
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ServerState {
    Started,
    AlreadyRunning,
    Stopped,
    NotRunning,
}

impl fmt::Display for ServerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Started => "Inference server started",
            Self::AlreadyRunning => "Inference server is already running",
            Self::Stopped => "Inference server stopped",
            Self::NotRunning => "Inference server is not running",
        })
    }
}

#[derive(Deserialize)]
struct ServerStateResponse {
    state: ServerState,
}

impl DaemonApi {
    /// `address` is where the daemon listens, like `http://127.0.0.1:1729`
    pub fn new(client: Client, address: String) -> Self {
        Self { client, address }
    }

    pub(crate) async fn fetch_sessions(&self) -> Result<Vec<Session>> {
        let res = self
            .client
            .get(format!("{}/v1/tilekit/session/list", self.address))
            .send()
            .await
            .context("Could not reach the Tiles daemon")?;
        response_data(res).await
    }

    /// `None` when nobody is signed in
    pub(crate) async fn fetch_signed_in_account(&self) -> Result<Option<AtprotoAccount>> {
        let res = self
            .client
            .get(format!("{}/v1/tilekit/atproto/status", self.address))
            .send()
            .await
            .context("Could not reach the Tiles daemon")?;
        if res.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        response_data(res).await.map(Some)
    }

    pub(crate) async fn share_session(
        &self,
        session_id: &str,
        visibility: ShareVisibility,
    ) -> Result<ShareOutcome> {
        let is_private = visibility == ShareVisibility::Private;
        let res = self
            .client
            .post(self.share_session_url(session_id)?)
            .json(&json!({ "is_private": is_private }))
            .send()
            .await
            .context("Could not reach the Tiles daemon")?;
        match res.status() {
            StatusCode::FORBIDDEN => Ok(ShareOutcome::NotSignedIn),
            StatusCode::NOT_FOUND => Ok(ShareOutcome::NoSuchSession),
            _ => {
                let link: SharedLink = response_data(res).await?;
                Ok(ShareOutcome::Shared { url: link.url })
            }
        }
    }

    // the id is typed by the user, so it's kept as one part of the path
    fn share_session_url(&self, session_id: &str) -> Result<Url> {
        let mut url = Url::parse(&self.address)?;
        url.path_segments_mut()
            .map_err(|_| anyhow!("{} is not an address to call", self.address))?
            .extend(["v1", "tilekit", "atproto", "share-session", session_id]);
        Ok(url)
    }

    pub(crate) async fn fetch_llama_settings(&self) -> Result<Vec<LlamaSetting>> {
        let res = self
            .client
            .get(format!("{}/v1/tilekit/config/llama", self.address))
            .send()
            .await
            .context("Could not reach the Tiles daemon")?;
        let response: SettingsResponse = response_data(res).await?;
        Ok(response.settings)
    }

    pub(crate) async fn set_llama_settings(
        &self,
        values: &Map<String, Value>,
    ) -> Result<Vec<LlamaSetting>> {
        let res = self
            .post_json("/v1/tilekit/config/llama/set", &json!({ "values": values }))
            .send()
            .await
            .context("Could not reach the Tiles daemon")?;
        let response: SettingsResponse = response_data(res).await?;
        Ok(response.settings)
    }

    pub(crate) async fn reset_llama_settings(
        &self,
        names: &[String],
        all: bool,
    ) -> Result<Vec<LlamaSetting>> {
        let res = self
            .post_json(
                "/v1/tilekit/config/llama/reset",
                &json!({ "names": names, "all": all }),
            )
            .send()
            .await
            .context("Could not reach the Tiles daemon")?;
        let response: SettingsResponse = response_data(res).await?;
        Ok(response.settings)
    }

    pub(crate) async fn logout_atproto(&self) -> Result<AtprotoAccount> {
        let res = self
            .post_empty("/v1/tilekit/atproto/logout")
            .send()
            .await
            .context("Could not reach the Tiles daemon")?;
        response_data(res).await
    }

    pub(crate) async fn start_server(&self) -> Result<ServerState> {
        let res = self
            .post_empty("/v1/tilekit/server/start")
            .send()
            .await
            .context("Could not reach the Tiles daemon")?;
        let response: ServerStateResponse = response_data(res).await?;
        Ok(response.state)
    }

    pub(crate) async fn stop_server(&self) -> Result<ServerState> {
        let res = self
            .post_empty("/v1/tilekit/server/stop")
            .send()
            .await
            .context("Could not reach the Tiles daemon")?;
        let response: ServerStateResponse = response_data(res).await?;
        Ok(response.state)
    }

    pub(crate) fn post_json(&self, path: &str, body: &Value) -> RequestBuilder {
        self.client
            .post(format!("{}{path}", self.address))
            .json(body)
    }

    fn post_empty(&self, path: &str) -> RequestBuilder {
        self.client
            .post(format!("{}{path}", self.address))
            .header(CONTENT_TYPE, "application/json")
    }
}

async fn response_data<T: DeserializeOwned>(res: Response) -> Result<T> {
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
                .unwrap_or("The Tiles daemon refused it")
        );
    }
    Ok(serde_json::from_value(body["data"].take())?)
}

#[cfg(test)]
mod tests {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_json, header, method, path},
    };

    use super::*;

    const SHARE_PATH: &str = "/v1/tilekit/atproto/share-session/abc";

    async fn daemon_answering(verb: &str, route: &str, status: u16, body: Value) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method(verb))
            .and(path(route))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .mount(&server)
            .await;
        server
    }

    fn api_for(server: &MockServer) -> DaemonApi {
        DaemonApi::new(Client::new(), server.uri())
    }

    #[tokio::test]
    async fn sessions_are_read_from_the_daemons_answer() {
        // as the daemon sends it, with the fields the CLI doesn't use
        let server = daemon_answering(
            "GET",
            "/v1/tilekit/session/list",
            200,
            json!({
                "status": "success",
                "data": [
                    {
                        "id": "019a2b6e-0000-7000-8000-000000000002",
                        "name": "what is 2+2",
                        "created_at": 1763540000,
                        "creator_id": "did:key:abc",
                        "snapshot": null
                    },
                    {
                        "id": "019a2b6e-0000-7000-8000-000000000001",
                        "name": "hello",
                        "created_at": 1763530000,
                        "creator_id": "did:key:abc",
                        "snapshot": "{}"
                    }
                ]
            }),
        )
        .await;

        let sessions = api_for(&server).fetch_sessions().await.unwrap();

        assert_eq!(
            sessions,
            vec![
                Session {
                    id: "019a2b6e-0000-7000-8000-000000000002".to_owned(),
                    name: "what is 2+2".to_owned(),
                },
                Session {
                    id: "019a2b6e-0000-7000-8000-000000000001".to_owned(),
                    name: "hello".to_owned(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn a_refusal_gives_the_daemons_reason() {
        let server = daemon_answering(
            "GET",
            "/v1/tilekit/session/list",
            500,
            json!({ "status": "failed", "reason": "database is locked" }),
        )
        .await;

        let err = api_for(&server).fetch_sessions().await.unwrap_err();

        assert_eq!(err.to_string(), "database is locked");
    }

    #[tokio::test]
    async fn a_daemon_that_is_not_running_is_reported() {
        let api = DaemonApi::new(Client::new(), "http://127.0.0.1:1".to_owned());

        let err = api.fetch_sessions().await.unwrap_err();

        assert_eq!(err.to_string(), "Could not reach the Tiles daemon");
    }

    #[tokio::test]
    async fn the_signed_in_account_is_read_and_nobody_signed_in_is_none() {
        let route = "/v1/tilekit/atproto/status";
        let signed_in = json!({
            "status": "success",
            "data": { "handle": "alice.test", "did": "did:plc:alice" }
        });
        let server = daemon_answering("GET", route, 200, signed_in).await;
        let account = api_for(&server).fetch_signed_in_account().await.unwrap();
        assert_eq!(
            account,
            Some(AtprotoAccount {
                handle: "alice.test".to_owned(),
                did: "did:plc:alice".to_owned(),
            })
        );

        let nobody = json!({ "status": "failed", "reason": "Not logged-in" });
        let server = daemon_answering("GET", route, 404, nobody).await;
        let account = api_for(&server).fetch_signed_in_account().await.unwrap();
        assert_eq!(account, None);
    }

    #[tokio::test]
    async fn a_share_sends_the_private_choice_and_reads_the_link() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(SHARE_PATH))
            .and(body_json(json!({ "is_private": true })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "success",
                "data": { "url": "https://tiles.run/share/abc", "is_private": true }
            })))
            .mount(&server)
            .await;

        let outcome = api_for(&server)
            .share_session("abc", ShareVisibility::Private)
            .await
            .unwrap();

        assert_eq!(
            outcome,
            ShareOutcome::Shared {
                url: "https://tiles.run/share/abc".to_owned()
            }
        );
    }

    #[tokio::test]
    async fn a_share_without_a_sign_in_or_a_session_is_told_apart() {
        let refusal = |reason: &str| json!({ "status": "failed", "reason": reason });

        let server = daemon_answering("POST", SHARE_PATH, 403, refusal("Sign in first")).await;
        let outcome = api_for(&server)
            .share_session("abc", ShareVisibility::Public)
            .await
            .unwrap();
        assert_eq!(outcome, ShareOutcome::NotSignedIn);

        let server = daemon_answering("POST", SHARE_PATH, 404, refusal("No chat abc")).await;
        let outcome = api_for(&server)
            .share_session("abc", ShareVisibility::Public)
            .await
            .unwrap();
        assert_eq!(outcome, ShareOutcome::NoSuchSession);
    }

    #[tokio::test]
    async fn a_failed_share_gives_the_daemons_reason() {
        let server = daemon_answering(
            "POST",
            SHARE_PATH,
            500,
            json!({ "status": "failed", "reason": "The PDS refused the record" }),
        )
        .await;

        let err = api_for(&server)
            .share_session("abc", ShareVisibility::Public)
            .await
            .unwrap_err();

        assert_eq!(err.to_string(), "The PDS refused the record");
    }

    #[test]
    fn a_typed_session_id_cannot_reach_another_route() {
        let api = DaemonApi::new(Client::new(), "http://127.0.0.1:1729".to_owned());

        let url = api.share_session_url("../logout").unwrap();

        assert_eq!(url.path(), "/v1/tilekit/atproto/share-session/..%2Flogout");
    }

    #[tokio::test]
    async fn llama_settings_are_read_from_the_daemons_answer() {
        let server = daemon_answering(
            "GET",
            "/v1/tilekit/config/llama",
            200,
            json!({
                "status": "success",
                "data": {
                    "settings": [
                        {
                            "name": "context_length",
                            "label": "Context length",
                            "value": 65536,
                            "default": "65536",
                            "kind": "number",
                            "min": 4096,
                            "help": "Max context size"
                        },
                        {
                            "name": "load_mode",
                            "label": "Load mode",
                            "value": null,
                            "default": "auto",
                            "kind": "choice",
                            "choices": [
                                { "value": "auto", "label": "Auto" },
                                { "value": "mmap", "label": "Memory map" }
                            ],
                            "help": "How the model is loaded"
                        }
                    ]
                }
            }),
        )
        .await;

        let settings = api_for(&server).fetch_llama_settings().await.unwrap();

        assert_eq!(
            settings,
            vec![
                LlamaSetting {
                    name: "context_length".to_owned(),
                    value: Some(json!(65536)),
                    default: "65536".to_owned(),
                    choices: vec![],
                    help: "Max context size".to_owned(),
                },
                LlamaSetting {
                    name: "load_mode".to_owned(),
                    value: None,
                    default: "auto".to_owned(),
                    choices: vec![
                        SettingChoice {
                            value: "auto".to_owned(),
                        },
                        SettingChoice {
                            value: "mmap".to_owned(),
                        },
                    ],
                    help: "How the model is loaded".to_owned(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn set_and_reset_llama_settings_send_their_bodies() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/tilekit/config/llama/set"))
            .and(body_json(json!({ "values": { "flash_attn": "false" } })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "success",
                "data": {
                    "settings": [{
                        "name": "flash_attn",
                        "value": false,
                        "default": "true",
                        "help": "Flash attention"
                    }]
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/tilekit/config/llama/reset"))
            .and(body_json(json!({ "names": ["flash_attn"], "all": false })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "success",
                "data": {
                    "settings": [{
                        "name": "flash_attn",
                        "value": null,
                        "default": "true",
                        "help": "Flash attention"
                    }]
                }
            })))
            .mount(&server)
            .await;

        let mut values = Map::new();
        values.insert("flash_attn".to_owned(), json!("false"));
        let updated = api_for(&server).set_llama_settings(&values).await.unwrap();
        assert_eq!(updated[0].value, Some(json!(false)));

        let reset = api_for(&server)
            .reset_llama_settings(&["flash_attn".to_owned()], false)
            .await
            .unwrap();
        assert_eq!(reset[0].value, None);
    }

    #[tokio::test]
    async fn atproto_logout_sends_json_content_type_and_reads_account() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/tilekit/atproto/logout"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "success",
                "data": { "handle": "alice.test", "did": "did:plc:alice" }
            })))
            .mount(&server)
            .await;

        let account = api_for(&server).logout_atproto().await.unwrap();
        assert_eq!(
            account,
            AtprotoAccount {
                handle: "alice.test".to_owned(),
                did: "did:plc:alice".to_owned(),
            }
        );

        let not_signed_in = daemon_answering(
            "POST",
            "/v1/tilekit/atproto/logout",
            404,
            json!({ "status": "failed", "reason": "No ATmosphere account is signed in" }),
        )
        .await;
        let err = api_for(&not_signed_in).logout_atproto().await.unwrap_err();
        assert_eq!(err.to_string(), "No ATmosphere account is signed in");
    }

    #[tokio::test]
    async fn server_start_and_stop_send_json_content_type_and_read_state() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/tilekit/server/start"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "success",
                "data": { "state": "already_running" }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/tilekit/server/stop"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "success",
                "data": { "state": "stopped" }
            })))
            .mount(&server)
            .await;

        assert_eq!(
            api_for(&server).start_server().await.unwrap(),
            ServerState::AlreadyRunning
        );
        assert_eq!(
            api_for(&server).stop_server().await.unwrap(),
            ServerState::Stopped
        );

        let port_in_use = daemon_answering(
            "POST",
            "/v1/tilekit/server/start",
            409,
            json!({ "status": "failed", "reason": "Port 6969 is in use by another process" }),
        )
        .await;
        let err = api_for(&port_in_use).start_server().await.unwrap_err();
        assert_eq!(err.to_string(), "Port 6969 is in use by another process");
    }
}
