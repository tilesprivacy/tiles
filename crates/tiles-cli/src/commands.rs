//! What the slash commands do and print

use anyhow::Result;

use crate::api::{DaemonApi, Session};

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
}
