//! Calls to the Tiles daemon

use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, RequestBuilder, Response, StatusCode, Url};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

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

    pub(crate) fn post_json(&self, path: &str, body: &Value) -> RequestBuilder {
        self.client
            .post(format!("{}{path}", self.address))
            .json(body)
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
        matchers::{body_json, method, path},
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
}
