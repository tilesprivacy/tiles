//! What the slash commands do and print

use std::io;

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::api::{DaemonApi, Session, ShareOutcome, ShareVisibility};
use crate::atproto::sign_in;

pub async fn show_session_list(api: &DaemonApi) -> Result<()> {
    for line in session_list_lines(&api.fetch_sessions().await?) {
        println!("{line}");
    }
    Ok(())
}

fn session_list_lines(sessions: &[Session]) -> Vec<String> {
    sessions
        .iter()
        .enumerate()
        .map(|(index, session)| format!("{}.\t{}\t{}", index + 1, session.id, session.name))
        .collect()
}

/// `/share`: asks how to share the chat, and offers a sign in when it needs one
pub async fn share_session_command(api: &DaemonApi, session_id: &str) -> Result<()> {
    let visibility = ask_share_visibility()?;

    let mut outcome = api.share_session(session_id, visibility).await?;
    if outcome == ShareOutcome::NotSignedIn {
        if !sign_in_to_share(api).await? {
            return Ok(());
        }
        outcome = api.share_session(session_id, visibility).await?;
    }
    println!("{}", share_outcome_line(&outcome));
    Ok(())
}

// enter takes the default the prompt shows
fn ask_share_visibility() -> Result<ShareVisibility> {
    loop {
        println!(
            "{}",
            "Do you want to share as a private session? (Y/n)".yellow()
        );
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        match answer.trim() {
            "" | "y" | "Y" => return Ok(ShareVisibility::Private),
            "n" | "N" => return Ok(ShareVisibility::Public),
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
    fn sessions_are_numbered_from_one_with_id_and_name() {
        let sessions = [
            Session {
                id: "id-b".to_owned(),
                name: "what is 2+2".to_owned(),
            },
            Session {
                id: "id-a".to_owned(),
                name: "hello".to_owned(),
            },
        ];

        assert_eq!(
            session_list_lines(&sessions),
            ["1.\tid-b\twhat is 2+2", "2.\tid-a\thello"]
        );
    }

    #[test]
    fn no_sessions_print_nothing() {
        assert!(session_list_lines(&[]).is_empty());
    }

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
