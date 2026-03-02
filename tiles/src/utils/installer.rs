//! Auto installing Tiles
//!
//! We will be fetching the latest revision form github (where we host the binaries)
//! We will be using the installer script under `tiles/scripts/installer.sh` to
//! install Tiles, just fetching the script and running as bash script from Rust.

use std::{
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{Result, anyhow};
use reqwest::{Client, header::HeaderMap};
use semver::Version;
use serde::Deserialize;

const RELEASES_BASE_ENDPOINT: &str = "https://api.github.com";
const RELEASES_REST_PATH: &str = "repos/tilesprivacy/tiles/releases/latest";
const HEADER_PARSING_ERROR: &str = "Failed to parse header";
#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

pub struct UpdateInfo {
    pub can_update: bool,
    pub latest_version: String,
    pub current_version: String,
}

pub async fn try_update(update_info: Option<UpdateInfo>) -> Result<String> {
    let app_update_info = if let Some(info) = update_info {
        info
    } else {
        get_update_info().await?
    };

    if !app_update_info.can_update {
        let msg = format!(
            "You are up to date, current version: {}",
            app_update_info.current_version
        );
        Ok(msg)
    } else {
        let mut curl_process = Command::new("curl")
            .arg("-fsSL")
            .arg("https://tiles.run/install.sh")
            .stdout(Stdio::piped())
            .spawn()?;

        let _run_sh_cmd = Command::new("sh")
            .stdin(
                curl_process
                    .stdout
                    .take()
                    .ok_or_else(|| anyhow!("Failed to pipe from the curled input"))?,
            )
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        Ok(format!(
            "Tiles upgraded to {}",
            app_update_info.latest_version
        ))
    }
}

pub async fn get_update_info() -> Result<UpdateInfo> {
    let latest_vsn = get_latest_version(RELEASES_BASE_ENDPOINT).await?;
    let req_vsn = Version::parse(&latest_vsn)?;
    let current_vsn = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|e| anyhow!("Failed to parse pkg version due to {}", e))?;

    if req_vsn.cmp_precedence(&current_vsn).is_gt() {
        Ok(UpdateInfo {
            can_update: true,
            latest_version: req_vsn.to_string(),
            current_version: current_vsn.to_string(),
        })
    } else {
        Ok(UpdateInfo {
            can_update: false,
            latest_version: req_vsn.to_string(),
            current_version: current_vsn.to_string(),
        })
    }
}

/// Gets the latest Tiles version
///
/// Returns a Err(String), on API failure
pub async fn get_latest_version(base_url: &str) -> Result<String> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-GitHub-Api-Version",
        "2022-11-28".parse().expect(HEADER_PARSING_ERROR),
    );
    headers.insert(
        "Accept",
        "application/vnd.github+json"
            .parse()
            .expect(HEADER_PARSING_ERROR),
    );
    headers.insert("user-agent", "Tiles".parse().expect(HEADER_PARSING_ERROR));
    let client_builder = Client::builder()
        .timeout(Duration::from_secs(5))
        .default_headers(headers);

    let client = client_builder.build()?;
    let response = client
        .get(format!("{}/{}", base_url, RELEASES_REST_PATH))
        .send()
        .await;

    match response {
        Err(err) if err.is_timeout() => Err(anyhow!("Request failed due to Api timedout")),
        Err(err) => Err(anyhow!("Request failed due to {:?}", err)),
        Ok(res) if res.status() == 200 => {
            let release = res.json::<Release>().await?;
            Ok(release.tag_name)
        }
        Ok(res) => Err(anyhow!("Api failed with status {}", res.status())),
    }
}

#[cfg(test)]
mod tests {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;
    use serde_json::json;
    #[tokio::test]
    async fn test_get_latest_version() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/tilesprivacy/tiles/releases/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!(
                {
                    "tag_name": "0.4.1"
                }
            )))
            .mount(&mock_server)
            .await;

        let tag = get_latest_version(mock_server.uri().as_str())
            .await
            .unwrap();
        assert_eq!(tag, "0.4.1".to_owned())
    }

    #[tokio::test]
    async fn test_get_latest_version_failed_due_to_timeout() {
        let delay = Duration::from_secs(30); // 30s
        let mock_server = MockServer::start().await;
        let path_str = format!("/{}", RELEASES_REST_PATH);
        Mock::given(method("GET"))
            .and(path(path_str))
            .respond_with(ResponseTemplate::new(200).set_delay(delay))
            .mount(&mock_server)
            .await;

        let server = mock_server.uri();
        let res = async_std::future::timeout(delay / 2, get_latest_version(server.as_str())).await;
        assert!(res.unwrap().is_err());
    }

    #[tokio::test]
    async fn test_get_latest_version_err_4xx() {
        let mock_server = MockServer::start().await;
        let path_str = format!("/{}", RELEASES_REST_PATH);

        Mock::given(method("GET"))
            .and(path(path_str))
            .respond_with(ResponseTemplate::new(403).set_body_json(json!(
            {
                "err": "unauth"
            })))
            .mount(&mock_server)
            .await;

        let tag = get_latest_version(mock_server.uri().as_str()).await;
        assert!(tag.is_err())
    }
}
