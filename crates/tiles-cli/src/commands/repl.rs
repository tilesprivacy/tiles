//! What the slash commands do and print

use std::env;
use std::io;
use std::path::Path;

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::api::{AtprotoAccount, DaemonApi, Session, ShareOutcome, ShareVisibility};
use crate::atproto::sign_in;

/// `/sessions`: prints the saved sessions, newest first
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
    // bad handle is reported, it does not end the session
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

/// `/status`: prints the session, model, reasoning level, working directory and ATmosphere
/// sign in. `session` is `None` until the first message is saved
pub async fn show_status(
    api: &DaemonApi,
    session: Option<&Session>,
    model: &str,
    reasoning: &str,
) -> Result<()> {
    let cwd = env::current_dir()?;
    let signed_in = api.fetch_signed_in_account().await?;

    println!("\n");
    for line in status_lines(&cwd, session, model, reasoning, signed_in.as_ref()) {
        println!("{line}");
    }
    Ok(())
}

fn status_lines(
    cwd: &Path,
    session: Option<&Session>,
    model: &str,
    reasoning: &str,
    signed_in: Option<&AtprotoAccount>,
) -> Vec<String> {
    let session_status = match session {
        Some(session) => format!("{} ({})", session.name.yellow(), session.id.dimmed()),
        None => "Session not started yet".to_owned(),
    };
    let atproto_status = match signed_in {
        Some(account) => format!(
            "{}{} ({})",
            "@".blue(),
            account.handle.blue(),
            account.did.dimmed()
        ),
        None => "Not logged-in".to_owned(),
    };
    let statuses = [
        ("Session", session_status),
        ("Model", model.to_owned()),
        ("Reasoning", reasoning.to_owned()),
        ("Working Directory", cwd.display().to_string()),
        ("ATProto", atproto_status),
    ];

    let label_width = statuses
        .iter()
        .map(|(label, _)| label.len())
        .max()
        .unwrap_or(0);
    statuses
        .into_iter()
        .map(|(label, status)| {
            let padding = " ".repeat(label_width - label.len());
            format!("{label}:{padding}\t{status}")
        })
        .collect()
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

    #[test]
    fn status_lines_show_defaults_without_session_or_atproto_login() {
        let lines = status_lines(Path::new("/tmp/tiles"), None, "model", "low", None);

        assert_eq!(
            lines,
            [
                "Session:          \tSession not started yet",
                "Model:            \tmodel",
                "Reasoning:        \tlow",
                "Working Directory:\t/tmp/tiles",
                "ATProto:          \tNot logged-in",
            ]
        );
    }

    #[test]
    fn status_lines_include_session_and_atproto_login() {
        let session = Session {
            id: "session-1".to_owned(),
            name: "First chat".to_owned(),
        };
        let signed_in = AtprotoAccount {
            handle: "alice.test".to_owned(),
            did: "did:plc:alice".to_owned(),
        };

        let lines = status_lines(
            Path::new("/work/tiles"),
            Some(&session),
            "llama",
            "low",
            Some(&signed_in),
        );

        assert!(lines[0].starts_with("Session:          \t"), "{lines:?}");
        assert!(lines[0].contains("First chat"), "{lines:?}");
        assert!(lines[0].contains("session-1"), "{lines:?}");
        assert_eq!(lines[1], "Model:            \tllama");
        assert_eq!(lines[3], "Working Directory:\t/work/tiles");
        assert!(lines[4].starts_with("ATProto:          \t"), "{lines:?}");
        assert!(lines[4].contains('@'), "{lines:?}");
        assert!(lines[4].contains("alice.test"), "{lines:?}");
        assert!(lines[4].contains("did:plc:alice"), "{lines:?}");
    }
}
