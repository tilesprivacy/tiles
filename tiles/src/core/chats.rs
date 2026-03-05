//! Chats.rs
//!
//! Stuff related to chats with the models
//!

use crate::core::accounts::User;
use crate::runtime::mlx::ChatResponse;
use crate::utils::get_unix_time_now;
use anyhow::Result;
use rusqlite::Connection;
use tilekit::modelfile::Role;
use uuid::Uuid;
// model the chats table
pub struct Chats {
    pub id: Uuid,
    content: String,
    // The id of the responses api obj
    response_id: Option<String>,
    // The Model chat user role
    role: Role,
    user_id: String,
    // The parent Id of a model's reply
    context_id: Option<Uuid>,
    created_at: u64,
    updated_at: u64,
}

pub fn save_chat(
    conn: &Connection,
    user: &User,
    input: &str,
    chat_resp: Option<&ChatResponse>,
) -> Result<Chats> {
    if let Some(chat_response) = chat_resp {
        let chat_resp_cloned = chat_response.clone();
        let chat = Chats {
            id: Uuid::now_v7(),
            user_id: user.user_id.clone(),
            content: input.to_owned(),
            response_id: Some(chat_resp_cloned.prev_response_id),
            role: Role::Assistant,
            context_id: chat_resp_cloned.parent_chat_id,
            created_at: get_unix_time_now(),
            updated_at: get_unix_time_now(),
        };

        conn.execute("insert into chats(id, user_id, content, resp_id, role, context_id, created_at, updated_at) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", (&chat.id.to_string(), &chat.user_id, &chat.content, &chat.response_id, Into::<String>::into(chat.role),  &chat.context_id.unwrap_or(Uuid::nil()).to_string(), &chat.created_at.to_string(), &chat.updated_at.to_string()))?;

        Ok(chat)
    } else {
        let chat = Chats {
            id: Uuid::now_v7(),
            user_id: user.user_id.clone(),
            content: input.to_owned(),
            response_id: None,
            role: Role::User,
            context_id: None,
            created_at: get_unix_time_now(),
            updated_at: get_unix_time_now(),
        };

        conn.execute("insert into chats(id, user_id, content, resp_id, role, context_id, created_at, updated_at) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", (&chat.id.to_string(), &chat.user_id, &chat.content, &chat.response_id, Into::<String>::into(chat.role),  &chat.context_id.unwrap_or(Uuid::nil()).to_string(), &chat.created_at.to_string(), &chat.updated_at.to_string()))?;

        Ok(chat)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use rusqlite::Connection;
    use uuid::Uuid;

    use crate::{
        core::{
            accounts::{ACCOUNT, User},
            chats::save_chat,
        },
        runtime::mlx::ChatResponse,
    };

    #[test]
    fn test_valid_input_save_chat() {
        let conn = setup_db_schema();
        let user = create_user();
        let chat = save_chat(&conn, &user, "2+2", None);

        assert!(chat.is_ok())
    }

    #[test]
    fn test_valid_response_save_chat() {
        let conn = setup_db_schema();
        let user = create_user();
        let chat_resp = ChatResponse {
            reply: "reply".to_owned(),
            code: "code".to_owned(),
            prev_response_id: String::from("resp_prev"),
            parent_chat_id: Some(Uuid::now_v7()),
            metrics: None,
        };
        let chat = save_chat(&conn, &user, "2+2", Some(&chat_resp));
        assert!(chat.is_ok())
    }

    fn create_user() -> User {
        User {
            id: Uuid::now_v7(),
            user_id: String::from("did"),
            username: String::from("nickname"),
            account_type: ACCOUNT::LOCAL,
            active_profile: true,
            root: true,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_secs(),
            updated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_secs(),
        }
    }
    fn setup_db_schema() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS chats (
        id TEXT PRIMARY KEY,
        content TEXT NOT NULL,
        resp_id TEXT,
        role TEXT NOT NULL,
        user_id TEXT NOT NULL,
        context_id TEXT ,
        created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
        updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
    );",
            [],
        )
        .unwrap();

        conn
    }
}
