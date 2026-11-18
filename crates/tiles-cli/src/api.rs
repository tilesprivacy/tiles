//! Calls to the Tiles daemon

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub struct DaemonApi {
    client: Client,
    address: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
}

impl DaemonApi {
    /// `address` is where the daemon listens, like `http://127.0.0.1:1729`
    pub fn new(client: Client, address: String) -> Self {
        Self { client, address }
    }

    pub async fn fetch_sessions(&self) -> Result<Vec<Session>> {
        self.get_response_data("/v1/tilekit/session/list").await
    }

    async fn get_response_data<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let res = self
            .client
            .get(format!("{}{path}", self.address))
            .send()
            .await
            .context("Could not reach the Tiles daemon")?;
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
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;

    async fn daemon_answering(status: u16, body: Value) -> (MockServer, DaemonApi) {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/tilekit/session/list"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .mount(&server)
            .await;
        let api = DaemonApi::new(Client::new(), server.uri());
        (server, api)
    }

    #[tokio::test]
    async fn sessions_are_read_from_the_daemons_answer() {
        // as the daemon sends it, with the fields the CLI doesn't use
        let (_server, api) = daemon_answering(
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

        let sessions = api.fetch_sessions().await.unwrap();

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
        let (_server, api) = daemon_answering(
            500,
            json!({ "status": "failed", "reason": "database is locked" }),
        )
        .await;

        let err = api.fetch_sessions().await.unwrap_err();

        assert_eq!(err.to_string(), "database is locked");
    }

    #[tokio::test]
    async fn a_daemon_that_is_not_running_is_reported() {
        let api = DaemonApi::new(Client::new(), "http://127.0.0.1:1".to_owned());

        let err = api.fetch_sessions().await.unwrap_err();

        assert_eq!(err.to_string(), "Could not reach the Tiles daemon");
    }
}
