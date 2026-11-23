//! Signing in to an ATmosphere account

use anyhow::{Context, Result, anyhow};
use serde_json::json;

use crate::api::{AtprotoAccount, DaemonApi};
use crate::browser::open_in_browser;
use crate::sse::take_events;

/// Signs in through the daemon, the same flow every client uses: prints and
/// opens the url, then waits for the browser to come back
pub async fn sign_in(api: &DaemonApi, handle: &str) -> Result<AtprotoAccount> {
    let res = api
        .post_json(
            "/v1/tilekit/atproto/login",
            &json!({ "user_handle": handle }),
        )
        .send()
        .await
        .context("Could not reach the Tiles daemon")?;

    if !res.status().is_success() {
        let body: serde_json::Value = res.json().await.unwrap_or_default();
        let reason = body["reason"].as_str().unwrap_or("The sign in failed");
        return Err(anyhow!(reason.to_owned()));
    }

    let mut res = res;
    let mut pending = String::new();
    while let Some(chunk) = res.chunk().await? {
        pending.push_str(&String::from_utf8_lossy(&chunk));
        for (name, data) in take_events(&mut pending) {
            match name.as_str() {
                "authorize" => {
                    let url = data["url"]
                        .as_str()
                        .context("The sign in came without a url")?;
                    println!(
                        "\nIf the browser doesn't open automatically, open this URL:\n\n{url}\n"
                    );
                    open_in_browser(url);
                }
                "signed_in" => return Ok(serde_json::from_value(data)?),
                "error" => {
                    let reason = data["reason"].as_str().unwrap_or("The sign in failed");
                    return Err(anyhow!(reason.to_owned()));
                }
                _ => {}
            }
        }
    }

    Err(anyhow!("The daemon closed the sign in before it finished"))
}
