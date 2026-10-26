//! Talking to the Hugging Face Hub

use std::time::Duration;

use anyhow::{Result, bail};
use reqwest::{Client, RequestBuilder, Response, StatusCode};

const HF_ENDPOINT: &str = "https://huggingface.co";
const USER_AGENT: &str = concat!("tiles/", env!("CARGO_PKG_VERSION"));
// a connection that sends nothing for this long fails instead of hanging
const STALL_TIMEOUT: Duration = Duration::from_secs(30);

/// The Hugging Face Hub, at `HF_ENDPOINT` if set, sending `HF_TOKEN` for private
/// and gated models
pub struct Hub {
    client: Client,
    endpoint: String,
    token: Option<String>,
}

impl Hub {
    pub fn from_env() -> Result<Self> {
        let endpoint = std::env::var("HF_ENDPOINT").unwrap_or_else(|_| HF_ENDPOINT.to_owned());
        let token = std::env::var("HF_TOKEN")
            .ok()
            .filter(|token| !token.is_empty());
        Self::new(&endpoint, token)
    }

    pub fn new(endpoint: &str, token: Option<String>) -> Result<Self> {
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
    pub(super) fn get(&self, path: &str) -> RequestBuilder {
        let request = self.client.get(format!("{}/{path}", self.endpoint));
        match &self.token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }
}

/// Turns the Hub's refusals into messages a user can act on
pub(super) fn check(res: Response, repo: &str) -> Result<Response> {
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
