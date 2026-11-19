//! Signing in to an ATmosphere account and sharing a chat there

use std::io;

use anyhow::{Context, Result, anyhow};
use owo_colors::OwoColorize;
use serde::Deserialize;
use serde_json::json;

use crate::api::{DaemonApi, ShareOutcome};
use crate::browser::open_in_browser;
use crate::sse::take_events;

/// The ATmosphere account a sign in ended up with
#[derive(Debug, Deserialize)]
pub struct AtprotoAccount {
    pub handle: String,
    pub did: String,
}

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

/// `/share`: asks how to share the chat, and offers a sign in when it needs one
pub async fn share_session_command(api: &DaemonApi, session_id: &str) -> Result<()> {
    let is_private = ask_whether_private()?;

    let mut outcome = api.share_session(session_id, is_private).await?;
    if outcome == ShareOutcome::NotSignedIn {
        if !sign_in_to_share(api).await? {
            return Ok(());
        }
        outcome = api.share_session(session_id, is_private).await?;
    }
    println!("{}", share_outcome_line(&outcome));
    Ok(())
}

// asks again until the answer is y or n, so a "yes" is never read as a no.
// enter takes the default the prompt shows
fn ask_whether_private() -> Result<bool> {
    loop {
        println!(
            "{}",
            "Do you want to share as a private session? (Y/n)".yellow()
        );
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        match answer.trim() {
            "" | "y" | "Y" => return Ok(true),
            "n" | "N" => return Ok(false),
            _ => println!("Please answer y or n"),
        }
    }
}

// false when the user says no or the sign in fails
async fn sign_in_to_share(api: &DaemonApi) -> Result<bool> {
    println!("{}", "Sharing a chat session requires logging in, as the data is stored on your ATmosphere PDS.\nDo you want to proceed with the login flow? (Y/n)".yellow());
    if read_answer()? != "y" {
        return Ok(false);
    }

    println!("Please enter your ATmosphere handle (ex: john.bsky.team)");
    let mut handle = String::new();
    io::stdin().read_line(&mut handle)?;
    // a bad handle is reported, it does not end the session
    match sign_in(api, handle.trim()).await {
        Ok(_) => Ok(true),
        Err(err) => {
            eprintln!("Could not sign in: {err}");
            Ok(false)
        }
    }
}

fn read_answer() -> Result<String> {
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(answer.trim().to_lowercase())
}

fn share_outcome_line(outcome: &ShareOutcome) -> String {
    match outcome {
        ShareOutcome::Shared { url } => format!("successfully posted at {url}"),
        ShareOutcome::NoSuchSession => "Session doesn't exist or not started yet".to_owned(),
        ShareOutcome::NotSignedIn => {
            "Sign in to an ATmosphere account to share, the chat is stored there".to_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shared_chat_prints_its_link() {
        let outcome = ShareOutcome::Shared {
            url: "https://tiles.run/share/abc".to_owned(),
        };

        assert_eq!(
            share_outcome_line(&outcome),
            "successfully posted at https://tiles.run/share/abc"
        );
    }

    #[test]
    fn a_missing_session_is_said_plainly() {
        assert_eq!(
            share_outcome_line(&ShareOutcome::NoSuchSession),
            "Session doesn't exist or not started yet"
        );
    }
}
