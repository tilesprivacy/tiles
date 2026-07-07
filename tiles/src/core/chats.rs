//! Chats.rs
//!
//! Stuff related to chats with the models
//!

use std::collections::HashMap;
use std::str::FromStr;

use crate::core::account::local::User;
use crate::core::storage::db::get_db_conn;
use crate::repl::ChatResponse;
use crate::utils::get_unix_time_now;
use anyhow::{Result, anyhow};
use log::{info, warn};
use rusqlite::types::FromSqlError;
use rusqlite::{Connection, params};
use tilekit::modelfile::Role;
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::oneshot;
use uuid::Uuid;
// model the chats table

// Foreign types on foreign traits, lul
// someday we can do this for traits sake
// https://dev.to/iprosk/generics-in-rust-murky-waters-of-implementing-foreign-traits-on-foreign-types-584n

// impl FromSql for Uuid {
//     fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
//         let value_str = String::column_result(value)?;
//         Uuid::from_str(&value_str).map_err(|_| FromSqlError::InvalidType)
//     }
// }

#[derive(serde::Serialize, Clone, Debug)]
pub struct Message {
    pub r#type: String,
    pub role: Role,
    pub content: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Chats {
    pub id: String,
    pub content: String,
    // The id of the responses api obj
    response_id: Option<String>,
    // The Model chat user role
    pub role: Role,
    user_id: String,
    // The parent Id of a model's reply
    context_id: Option<String>,
    created_at: u64,
    updated_at: u64,
    row_counter: i64,
    session_id: String,
    model_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub created_at: u64,
    creator_id: String,
    pub snapshot: Option<String>,
}

type Responder<T> = oneshot::Sender<T>;
pub enum SyncOp {
    GetLastRowCounter {
        user_id: String,
        resp: Responder<Result<i64>>,
    },
    GetEncodedData {
        user_id: String,
        last_row_counter: i64,
        resp: Responder<Result<Vec<u8>>>,
    },
    ApplyDelta {
        delta: Vec<u8>,
        resp: Responder<Result<()>>,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct DeltaChat {
    pub chats: Vec<Chats>,
    pub sessions: Vec<Session>,
}

pub fn save_chat(conn: &Connection, user: &User, chat_resp: ChatResponse) -> Result<Chats> {
    let row_counter = get_last_row_counter(conn, &user.user_id)?;
    let chat = Chats {
        id: Uuid::now_v7().to_string(),
        user_id: user.user_id.clone(),
        content: chat_resp.input,
        response_id: None,
        role: chat_resp.role,
        context_id: chat_resp.parent_chat_id,
        created_at: get_unix_time_now(),
        updated_at: get_unix_time_now(),
        row_counter: row_counter + 1,
        session_id: chat_resp.session_id,
        model_name: chat_resp.model_used,
    };

    conn.execute("insert into chats(id, user_id, content, resp_id, role, context_id, created_at, updated_at, row_counter, session_id, model_name) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)", (&chat.id.to_string(), &chat.user_id, &chat.content, &chat.response_id, Into::<String>::into(chat.role),  &chat.context_id, &chat.created_at.to_string(), &chat.updated_at.to_string(), &chat.row_counter, &chat.session_id, &chat.model_name))?;

    Ok(chat)
}

/// Returns the `id` of the last entry of the given user_id
/// Used as the offset point for fetching the chat delta from the user_id
pub fn get_last_entry_id(conn: &Connection, user_id: &str) -> Result<Option<Uuid>> {
    match conn.query_row(
        "select id from chats where user_id = ?1 order by id desc limit 1",
        [user_id],
        |row| row.get::<usize, String>(0),
    ) {
        Ok(res) => Uuid::from_str(&res).map_err(Into::into).map(Some),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(<rusqlite::Error as Into<anyhow::Error>>::into(err)),
    }
}

/// Returns the `row_counter` of the last entry of the given user_id
/// Used as the offset point for fetching the chat delta from the user_id
pub fn get_last_row_counter(conn: &Connection, user_id: &str) -> Result<i64> {
    match conn.query_row(
        "select max(row_counter) from chats where user_id = ?1",
        [user_id],
        |row| row.get::<usize, i64>(0),
    ) {
        Ok(res) => Ok(res),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
        // It returns NULL, if there are now no rows
        Err(rusqlite::Error::InvalidColumnType(_, _, _)) => Ok(0),
        Err(err) => Err(<rusqlite::Error as Into<anyhow::Error>>::into(err)),
    }
}

/// Return a Delta of chats and sessions for the given `user_id` since `last_row_counter`
pub fn get_delta(conn: &Connection, user_id: &str, last_row_couter: i64) -> Result<DeltaChat> {
    let query = "select id, user_id, content, resp_id, role, context_id, created_at, updated_at , row_counter, session_id, model_name from chats where user_id = ?1 and row_counter > ?2 order by id";

    let lrc_str = last_row_couter.to_string();

    let params = vec![("?1", user_id), ("?2", &lrc_str)];
    fetch_delta_chats(conn, query, params)
}

fn fetch_delta_chats(
    conn: &Connection,
    query: &str,
    params: Vec<(&str, &str)>,
) -> Result<DeltaChat> {
    let mut stmt = conn.prepare(query)?;

    let mut session_map: HashMap<String, Session> = HashMap::new();
    let chat_rows = stmt.query_map(params.as_slice(), |row| {
        let id: String = row.get(0)?;
        let role: String = row.get(4)?;
        let created_at: f64 = row.get(6)?;
        let updated_at: f64 = row.get(7)?;
        let resp_id: Option<String> = row.get(3)?;
        let ctx_id = row.get(5)?;
        let model_name_db: Option<String> = row.get(9)?;

        let model_name: String = model_name_db.unwrap_or("".to_owned());

        // This is to handle older versions which can have null session_id in DB
        let session_id_db: Option<String> = row.get(9)?;

        let session_id: String = session_id_db.unwrap_or("".to_owned());

        if !session_id.is_empty() && !session_map.contains_key(&session_id) {
            // lets fetch the session details
            match fetch_session(conn, &session_id) {
                Ok(session) => {
                    session_map.insert(session_id.clone(), session);
                }
                Err(err) => {
                    warn!("Fetching session {} failed due to {:?}", &session_id, err);
                }
            }
        }
        Ok(Chats {
            id,
            content: row.get(2)?,
            response_id: resp_id,
            role: Role::from_str(&role).map_err(FromSqlError::other)?,
            user_id: row.get(1)?,
            context_id: ctx_id,
            created_at: created_at as u64,
            updated_at: updated_at as u64,
            row_counter: row.get(8)?,
            session_id,
            model_name,
        })
    })?;

    let mut chats: Vec<Chats> = vec![];

    for chat in chat_rows {
        chats.push(chat?);
    }

    let sessions: Vec<Session> = session_map.into_values().collect();

    Ok(DeltaChat { chats, sessions })
}
pub fn apply_delta(chat_conn: &mut Connection, delta_chats: DeltaChat) -> Result<()> {
    // TODO: Handle primary key conflict, for now reject it (in a way its impossible to have this scenario, and if its occuring then that means
    // some issue in syncing, so ignore it, by rejecting it), later
    // do LWW based on issuer of UCAN
    //

    let chats = delta_chats.chats;
    let sessions = delta_chats.sessions;
    let txn = chat_conn.transaction()?;
    {
        let mut stmt = txn.prepare("insert into chats(id, user_id, content, resp_id, role, context_id, created_at, updated_at, row_counter, session_id) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)")?;

        for chat in chats {
            match stmt.execute(params![
                &chat.id.to_string(),
                &chat.user_id,
                &chat.content,
                &chat.response_id,
                Into::<String>::into(chat.role),
                &chat.context_id,
                &chat.created_at.to_string(),
                &chat.updated_at.to_string(),
                &chat.row_counter,
                &chat.session_id
            ]) {
                Err(rusqlite::Error::SqliteFailure(_, Some(reason)))
                    if reason == "UNIQUE constraint failed: chats.id" =>
                {
                    log::error!(
                        "err in writing row {:?}, already exists, skipping",
                        &chat.id
                    );
                }
                // NOTE: If any other error occurs and write failed we abort the sync, so the the row_counter doesn't get skipped.
                // use RUST_LOG=error tiles to debug the issue
                Err(err) => {
                    log::error!(
                        "err in writing row due to {:?}, Aborting the sync ....",
                        err
                    );
                    break;
                }

                Ok(_) => (),
            }
        }

        // session metadata sync
        let mut session_stmt = txn.prepare(
            "insert into sessions(id, name, created_at, creator_id)  values (?1, ?2, ?3, ?4)",
        )?;

        for session in sessions {
            match session_stmt.execute(params![
                &session.id.to_string(),
                &session.name,
                &session.created_at.to_string(),
                &session.creator_id
            ]) {
                Err(rusqlite::Error::SqliteFailure(_, Some(reason)))
                    if reason == "UNIQUE constraint failed: sessions.id" =>
                {
                    log::error!(
                        "err in writing row {:?}, already exists, skipping",
                        &session.id
                    );
                }
                // NOTE: If any other error occurs and write failed we abort the sync, so the the row_counter doesn't get skipped.
                // use RUST_LOG=error tiles to debug the issue
                Err(err) => {
                    log::error!(
                        "err in writing row due to {:?}, Aborting the sync during session sync....",
                        err
                    );
                    break;
                }

                Ok(_) => (),
            }
        }
    }
    txn.commit()?;

    Ok(())
}

pub fn get_encoded_delta(
    conn: &Connection,
    user_id: &str,
    last_row_couter: i64,
) -> Result<Vec<u8>> {
    let delta = get_delta(conn, user_id, last_row_couter)?;
    Ok(encode_delta_to_bytes(&delta))
}

/// Spawns a concurrent process that can process DB operations for p2p syncing through channel communication
///
/// Returns a sender to the caller.
///
/// This is due to the restrictions on sharing around DB references across threads, due to the Connection object being not thread safe
pub fn create_db_sync_channel() -> Sender<SyncOp> {
    let (sender, mut receiver) = mpsc::channel::<SyncOp>(32);

    tokio::spawn(async move {
        let mut chat_db_conn = get_db_conn(&super::storage::db::DBTYPE::CHAT)?;
        info!("DB sync channel ready..");
        while let Some(msg) = receiver.recv().await {
            match msg {
                SyncOp::GetLastRowCounter { user_id, resp } => {
                    let counter = get_last_row_counter(&chat_db_conn, &user_id);
                    resp.send(counter)
                        .map_err(|_op| anyhow!("Error sending counter"))?;
                }
                SyncOp::GetEncodedData {
                    user_id,
                    last_row_counter,
                    resp,
                } => {
                    let encoded_res = get_encoded_delta(&chat_db_conn, &user_id, last_row_counter);
                    resp.send(encoded_res)
                        .map_err(|_op| anyhow!("Error sending encoded_delta"))?;
                }
                SyncOp::ApplyDelta { delta, resp } => {
                    let chat_rows = decode_delta_from_bytes(&delta)?;

                    let apply_res = apply_delta(&mut chat_db_conn, chat_rows);
                    resp.send(apply_res)
                        .map_err(|_| anyhow!("Error sending apply delta response"))?;
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    });
    sender
}

pub fn create_session(conn: &Connection, id: &str, name: &str, user_id: &str) -> Result<Session> {
    // log a warning if session already exists, and skip the conflict

    let mut stmt = conn.prepare(
        "insert into sessions(id, name, creator_id, created_at, snapshot) values (?1, ?2, ?3, ?4, ?5)",
    )?;

    match stmt.execute(params![
        id.to_owned(),
        name.to_owned(),
        user_id.to_owned(),
        get_unix_time_now() as f64,
        None::<String>
    ]) {
        Ok(_res) => {
            let sesh = fetch_session(conn, id)?;
            Ok(sesh)
        }
        Err(rusqlite::Error::SqliteFailure(_, Some(reason)))
            if reason == "UNIQUE constraint failed: sessions.id" =>
        {
            warn!("Session entry already exists, skipping");
            let sesh = fetch_session(conn, id)?;
            Ok(sesh)
        }
        Err(err) => Err(anyhow!("Err inserting due to {}", err)),
    }
}

pub fn fetch_session(conn: &Connection, session_id: &str) -> Result<Session> {
    let sesh = conn.query_row(
        "SELECT id, name, creator_id, created_at, snapshot FROM sessions WHERE id = ?1",
        [session_id],
        |row| {
            Ok(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                creator_id: row.get(2)?,
                created_at: row.get::<usize, f64>(3)? as u64,
                snapshot: row.get(4)?,
            })
        },
    )?;
    Ok(sesh)
}

pub fn fetch_sessions(conn: &Connection) -> Result<Vec<Session>> {
    let query =
        "select id, name, creator_id, created_at, snapshot from sessions order by created_at desc";

    let mut stmt = conn.prepare(query)?;
    let session_rows = stmt.query_map([], |row| {
        Ok(Session {
            id: row.get(0)?,
            name: row.get(1)?,
            creator_id: row.get(2)?,
            created_at: row.get::<usize, f64>(3)? as u64,
            snapshot: row.get(4)?,
        })
    })?;

    let mut sessions: Vec<Session> = vec![];

    for session in session_rows {
        sessions.push(session?);
    }
    Ok(sessions)
}

pub fn fetch_models_used_by_session(conn: &Connection, session_id: &str) -> Result<Vec<String>> {
    let query = "select distinct model_name from chats where session_id = ?1";

    let mut stmt = conn.prepare(query)?;
    let model_names_rows = stmt.query_map([session_id], |row| {
        let model_opt: Option<String> = row.get(0)?;
        Ok(model_opt.unwrap_or("".to_owned()))
    })?;

    let mut model_names: Vec<String> = vec![];

    for model_name in model_names_rows {
        if let Ok(model) = model_name
            && !model.is_empty()
        {
            model_names.push(model);
        }
    }
    Ok(model_names)
}
fn encode_delta_to_bytes(delta_chats: &DeltaChat) -> Vec<u8> {
    postcard::to_stdvec(delta_chats).expect("Failed to convert to bytes with postcard")
}

fn decode_delta_from_bytes(bytes: &[u8]) -> Result<DeltaChat> {
    postcard::from_bytes(bytes).map_err(Into::into)
}

pub fn fetch_chats_by_session_id(conn: &Connection, session_id: &str) -> Result<DeltaChat> {
    let query = "select id, user_id, content, resp_id, role, context_id, created_at, updated_at , row_counter, session_id  from chats where session_id = ?1 order by id";

    let params = vec![("?1", session_id)];

    fetch_delta_chats(conn, query, params)
}

pub fn update_snapshot(conn: &Connection, id: &str, snapshot: String) -> Result<Session> {
    let mut stmt = conn.prepare("update sessions set snapshot = ?1 where id = ?2")?;
    match stmt.execute(params![snapshot, id.to_owned(),]) {
        Ok(_res) => {
            let sesh = fetch_session(conn, id)?;
            Ok(sesh)
        }
        Err(err) => Err(anyhow!("Err updating session snapshot due to {}", err)),
    }
}
#[cfg(test)]
pub mod tests {

    use std::time::{SystemTime, UNIX_EPOCH};

    use rusqlite::Connection;
    use tilekit::modelfile::Role;
    use uuid::Uuid;

    use crate::{
        core::{
            account::local::{ACCOUNT, User},
            chats::{
                apply_delta, create_session, decode_delta_from_bytes, encode_delta_to_bytes,
                fetch_models_used_by_session, get_delta, get_last_row_counter, save_chat,
            },
        },
        repl::ChatResponse,
        utils::{get_unix_time_now, test_logger},
    };

    #[test]
    fn test_valid_input_save_chat() {
        let conn = setup_db_schema();
        let user = create_user();
        let input = "2+2";

        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let chat = save_chat(&conn, &user, chat_response).expect("chat should be saved");

        assert_eq!(chat.user_id, user.user_id);
        assert!(chat.response_id.is_none());
        assert!(chat.context_id.is_none());

        let saved = fetch_saved_chat_row(&conn, &chat.id);
        assert_eq!(saved.content, input);
        assert_eq!(saved.resp_id, None);
        assert_eq!(saved.role, Into::<String>::into(Role::User));
        assert_eq!(saved.user_id, user.user_id);
        assert_eq!(saved.context_id, None);
    }

    #[test]
    fn test_valid_response_save_chat() {
        let conn = setup_db_schema();
        let user = create_user();
        let parent_chat_id = Uuid::now_v7().to_string();
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::Assistant,
            parent_chat_id: Some(parent_chat_id.clone()),
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let chat = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        assert_eq!(chat.user_id, user.user_id);
        assert_eq!(chat.context_id, Some(parent_chat_id.clone()));

        let saved = fetch_saved_chat_row(&conn, &chat.id);
        assert_eq!(saved.content, input);
        assert_eq!(saved.role, Into::<String>::into(Role::Assistant));
        assert_eq!(saved.user_id, user.user_id);
        assert_eq!(saved.context_id, Some(parent_chat_id.clone()));
    }

    #[test]
    fn test_response_without_parent_chat_id_saves_nil_context() {
        let conn = setup_db_schema();
        let user = create_user();
        let chat_response = ChatResponse {
            input: "".to_owned(),
            session_id: String::from("session_abc"),
            role: Role::Assistant,
            parent_chat_id: Some(Uuid::now_v7().to_string()),
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };

        let chat = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        assert!(chat.context_id.is_some());
        let saved = fetch_saved_chat_row(&conn, &chat.id);
        assert_eq!(saved.role, Into::<String>::into(Role::Assistant));
        assert!(saved.context_id.is_some());
    }

    #[test]
    fn test_empty_input_is_saved() {
        let conn = setup_db_schema();
        let user = create_user();
        let chat_response = ChatResponse {
            input: "".to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let chat =
            save_chat(&conn, &user, chat_response).expect("empty content should still be saved");

        let saved = fetch_saved_chat_row(&conn, &chat.id);
        assert_eq!(saved.content, "");
        assert_eq!(saved.role, Into::<String>::into(Role::User));
    }

    #[test]
    fn test_save_chat_errors_when_table_missing() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        let user = create_user();
        let chat_response = ChatResponse {
            input: "".to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let result = save_chat(&conn, &user, chat_response);

        assert!(result.is_err());
    }

    #[test]
    fn test_last_row_counter() {
        let conn = setup_db_schema();
        let user = create_user();
        let chat_response = ChatResponse {
            input: "".to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let chat = save_chat(&conn, &user, chat_response).expect("chat should be saved");

        assert_eq!(chat.user_id, user.user_id);
        assert!(chat.response_id.is_none());
        assert!(chat.context_id.is_none());

        let saved = get_last_row_counter(&conn, &user.user_id);
        assert_eq!(saved.unwrap(), 1);
    }

    #[test]
    fn test_get_last_row_counter_without_entry() {
        let conn = setup_db_schema();
        let user = create_user();
        let saved = get_last_row_counter(&conn, &user.user_id);
        assert_eq!(saved.unwrap(), 0)
    }

    #[test]
    fn test_get_delta_diff() {
        let conn = setup_db_schema();
        let user = create_user();
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let chat_1 = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        let delta = get_delta(&conn, &user.user_id, chat_1.row_counter).unwrap();
        assert_eq!(delta.sessions.len(), 0);
        assert_eq!(delta.chats.len(), 3);
    }

    #[test]
    fn test_get_delta_diff_chat_without_sessions() {
        let conn = setup_db_schema();
        let user = create_user();
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };

        conn.execute("insert into chats(id, user_id, content, resp_id, role, context_id, created_at, updated_at, row_counter, session_id) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)", (Uuid::now_v7().to_string(), &user.user_id, &chat_response.input, None::<String>, Into::<String>::into(chat_response.role),  &chat_response.parent_chat_id, get_unix_time_now().to_string(), get_unix_time_now().to_string(), 1, None::<String>)).unwrap();

        let delta = get_delta(&conn, &user.user_id, 0).unwrap();
        assert_eq!(delta.sessions.len(), 0);
        assert_eq!(delta.chats.len(), 1);
    }

    #[test]
    fn test_get_delta_diff_empty_last_entry_id() {
        let conn = setup_db_schema();
        let user = create_user();
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let _chat_1 = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        let rows = get_delta(&conn, &user.user_id, 0).unwrap();
        assert_eq!(rows.chats.len(), 4);
    }

    #[test]
    fn test_get_delta_diff_w_same_sessions() {
        let conn = setup_db_schema();
        let user = create_user();
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        create_session(&conn, "session_abc", "sesh", &user.user_id).unwrap();
        let _chat_1 = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        let rows = get_delta(&conn, &user.user_id, 0).unwrap();
        assert_eq!(rows.sessions.len(), 1);
        assert_eq!(rows.chats.len(), 4);
    }

    #[test]
    fn test_get_delta_diff_w_diff_sessions() {
        let conn = setup_db_schema();
        let user = create_user();
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        create_session(&conn, "session_abc", "sesh", &user.user_id).unwrap();
        let _chat_1 = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        create_session(&conn, "session_def", "sesh-2", &user.user_id).unwrap();
        let input = "4+4";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_def"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let _chat_1 = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        let rows = get_delta(&conn, &user.user_id, 0).unwrap();
        assert_eq!(rows.sessions.len(), 2);
        assert_eq!(rows.chats.len(), 6);
    }
    #[test]
    fn test_get_delta_diff_empty_wrong_user_id() {
        let conn = setup_db_schema();
        let user = create_user();
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let _chat_1 = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        let rows = get_delta(&conn, "", 0).unwrap();
        assert_eq!(rows.chats.len(), 0);
    }

    #[test]
    fn test_apply_delta() {
        let conn = setup_db_schema();
        let mut conn_2 = setup_db_schema();
        let user = create_user();
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let _chat_1 = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        let rows = get_delta(&conn, &user.user_id, 0).unwrap();
        assert_eq!(rows.chats.len(), 4);
        assert!(apply_delta(&mut conn_2, rows).is_ok());
        let rows = get_delta(&conn_2, &user.user_id, 0).unwrap();
        assert_eq!(rows.chats.len(), 4);
    }

    #[test]
    fn test_e2e_delta_roundtrip() {
        let conn = setup_db_schema();
        let mut conn_2 = setup_db_schema();
        let user = create_user();
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let _chat_1 = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        let rows = get_delta(&conn, &user.user_id, 0).unwrap();
        assert_eq!(rows.chats.len(), 4);
        let chat_bytes = encode_delta_to_bytes(&rows);
        let decoded_chat = decode_delta_from_bytes(&chat_bytes).unwrap();
        assert!(apply_delta(&mut conn_2, decoded_chat).is_ok());
        let rows = get_delta(&conn_2, &user.user_id, 0).unwrap();
        assert_eq!(rows.chats.len(), 4);
    }

    #[test]
    fn test_e2e_delta_roundtrip_w_empty_bytes() {
        let conn = setup_db_schema();
        let mut conn_2 = setup_db_schema();
        let user = create_user();
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let _chat_1 = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        let rows = get_delta(&conn, &user.user_id, 4).unwrap();
        assert_eq!(rows.chats.len(), 0);
        let chat_bytes = encode_delta_to_bytes(&rows);
        let decoded_chat = decode_delta_from_bytes(&chat_bytes).unwrap();
        assert!(apply_delta(&mut conn_2, decoded_chat).is_ok());
        let rows = get_delta(&conn_2, &user.user_id, 0).unwrap();
        assert_eq!(rows.chats.len(), 0);
    }

    #[test]
    fn test_non_zero_last_counter_delta() {
        let conn = setup_db_schema();
        let mut _conn_2 = setup_db_schema();
        let user = create_user();
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let chat_1 = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let rows = get_delta(&conn, &user.user_id, chat_1.row_counter).unwrap();
        assert_eq!(rows.chats.len(), 3);
    }

    #[test]
    fn test_duplicate_row_apply() {
        test_logger();
        let conn = setup_db_schema();
        let mut conn_2 = setup_db_schema();
        let user = create_user();
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let _chat_1 = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        let rows = get_delta(&conn, &user.user_id, 0).unwrap();
        assert_eq!(rows.chats.len(), 4);
        let chat_bytes = encode_delta_to_bytes(&rows);
        let decoded_chat = decode_delta_from_bytes(&chat_bytes).unwrap();
        assert!(apply_delta(&mut conn_2, decoded_chat.clone()).is_ok());
        let rows = get_delta(&conn_2, &user.user_id, 0).unwrap();
        assert_eq!(rows.chats.len(), 4);
        assert!(apply_delta(&mut conn_2, decoded_chat).is_ok());
        let rows = get_delta(&conn_2, &user.user_id, 0).unwrap();
        assert_eq!(rows.chats.len(), 4);
    }

    #[test]
    fn test_e2e_syncing_both_ways_w_eventual_consistency() {
        test_logger();
        let mut conn = setup_db_schema();
        let mut conn_2 = setup_db_schema();
        let user_a = create_user_by_id("user_a");
        let user_b = create_user_by_id("user_b");

        create_session(&conn, "session_abc", "sesh", &user_a.user_id).unwrap();
        // Node user A adds stuff
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let _chat_1 =
            save_chat(&conn, &user_a, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user_a, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user_a, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user_a, chat_response.clone()).expect("chat should be saved");

        // Node user B adds stuff
        let input = "4+4";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let _chat_1 =
            save_chat(&conn_2, &user_b, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn_2, &user_b, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn_2, &user_b, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn_2, &user_b, chat_response.clone()).expect("chat should be saved");

        // Node A wants to sync with Node B

        // 1. So its sends last_row_counter of Node B to Node B and hopefully
        //  it sends the diff since then and last_row_counter of Node A back..

        let user_b_last_entry_of_user_a = get_last_row_counter(&conn, &user_b.user_id).unwrap();

        // user_b is extracting the row's of it since the given last_row_counter
        let user_bs_diff_rows =
            get_delta(&conn_2, &user_b.user_id, user_b_last_entry_of_user_a).unwrap();

        assert_eq!(user_bs_diff_rows.chats.len(), 4);

        // user_bs diff is encoded
        let user_b_chat_bytes = encode_delta_to_bytes(&user_bs_diff_rows);

        // send to user_a and its decoded
        let user_b_decoded_chat = decode_delta_from_bytes(&user_b_chat_bytes).unwrap();

        // Now user_a is gonna apply the user_b diff
        assert!(apply_delta(&mut conn, user_b_decoded_chat).is_ok());

        // Just checking if we user_a has all 8 rows

        let user_a_rows = conn
            .query_row("select count(*) from chats", [], |row| {
                row.get::<usize, i64>(0)
            })
            .unwrap();

        assert_eq!(user_a_rows, 8);

        // cool, now lets do the reverse sync, user B syncs user A stuff

        let user_a_last_entry_of_user_b = get_last_row_counter(&conn_2, &user_a.user_id).unwrap();

        // user_a is extracting the row's of it since the given last_row_counter
        let user_as_diff_rows =
            get_delta(&conn, &user_a.user_id, user_a_last_entry_of_user_b).unwrap();

        assert_eq!(user_as_diff_rows.chats.len(), 4);

        // user_as diff is encoded
        let user_a_chat_bytes = encode_delta_to_bytes(&user_as_diff_rows);

        // send to user_b and its decoded
        let user_a_decoded_chat = decode_delta_from_bytes(&user_a_chat_bytes).unwrap();

        // Now user_b is gonna apply the user_b diff
        assert!(apply_delta(&mut conn_2, user_a_decoded_chat).is_ok());

        // Just checking eventual consistency

        let user_a_rows = conn
            .query_row("select count(*) from chats", [], |row| {
                row.get::<usize, i64>(0)
            })
            .unwrap();

        let user_b_rows = conn_2
            .query_row("select count(*) from chats", [], |row| {
                row.get::<usize, i64>(0)
            })
            .unwrap();

        assert_eq!(user_a_rows, user_b_rows);

        let user_b_sessions = conn_2
            .query_row("select count(*) from sessions", [], |row| {
                row.get::<usize, i64>(0)
            })
            .unwrap();

        assert_eq!(user_b_sessions, 1);
    }

    #[test]
    fn test_e2e_syncing_both_ways_w_eventual_consistency_multiple_sessions() {
        test_logger();
        let mut conn = setup_db_schema();
        let mut conn_2 = setup_db_schema();
        let user_a = create_user_by_id("user_a");
        let user_b = create_user_by_id("user_b");

        create_session(&conn, "session_abc", "sesh", &user_a.user_id).unwrap();
        // Node user A adds stuff
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let _chat_1 =
            save_chat(&conn, &user_a, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user_a, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user_a, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user_a, chat_response.clone()).expect("chat should be saved");

        // Node user B adds stuff
        create_session(&conn_2, "session_def", "sesh", &user_b.user_id).unwrap();
        let input = "4+4";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_def"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let _chat_1 =
            save_chat(&conn_2, &user_b, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn_2, &user_b, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn_2, &user_b, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn_2, &user_b, chat_response.clone()).expect("chat should be saved");

        // Node A wants to sync with Node B

        // 1. So its sends last_row_counter of Node B to Node B and hopefully
        //  it sends the diff since then and last_row_counter of Node A back..

        let user_b_last_entry_of_user_a = get_last_row_counter(&conn, &user_b.user_id).unwrap();

        // user_b is extracting the row's of it since the given last_row_counter
        let user_bs_diff_rows =
            get_delta(&conn_2, &user_b.user_id, user_b_last_entry_of_user_a).unwrap();

        assert_eq!(user_bs_diff_rows.chats.len(), 4);

        // user_bs diff is encoded
        let user_b_chat_bytes = encode_delta_to_bytes(&user_bs_diff_rows);

        // send to user_a and its decoded
        let user_b_decoded_chat = decode_delta_from_bytes(&user_b_chat_bytes).unwrap();

        // Now user_a is gonna apply the user_b diff
        assert!(apply_delta(&mut conn, user_b_decoded_chat).is_ok());

        // Just checking if we user_a has all 8 rows

        let user_a_rows = conn
            .query_row("select count(*) from chats", [], |row| {
                row.get::<usize, i64>(0)
            })
            .unwrap();

        assert_eq!(user_a_rows, 8);
        let user_a_sessions = conn
            .query_row("select count(*) from sessions", [], |row| {
                row.get::<usize, i64>(0)
            })
            .unwrap();

        assert_eq!(user_a_sessions, 2);

        // cool, now lets do the reverse sync, user B syncs user A stuff

        let user_a_last_entry_of_user_b = get_last_row_counter(&conn_2, &user_a.user_id).unwrap();

        // user_a is extracting the row's of it since the given last_row_counter
        let user_as_diff_rows =
            get_delta(&conn, &user_a.user_id, user_a_last_entry_of_user_b).unwrap();

        assert_eq!(user_as_diff_rows.chats.len(), 4);

        // user_as diff is encoded
        let user_a_chat_bytes = encode_delta_to_bytes(&user_as_diff_rows);

        // send to user_b and its decoded
        let user_a_decoded_chat = decode_delta_from_bytes(&user_a_chat_bytes).unwrap();

        // Now user_b is gonna apply the user_b diff
        assert!(apply_delta(&mut conn_2, user_a_decoded_chat).is_ok());

        // Just checking eventual consistency

        let user_a_rows = conn
            .query_row("select count(*) from chats", [], |row| {
                row.get::<usize, i64>(0)
            })
            .unwrap();

        let user_b_rows = conn_2
            .query_row("select count(*) from chats", [], |row| {
                row.get::<usize, i64>(0)
            })
            .unwrap();

        assert_eq!(user_a_rows, user_b_rows);

        let user_b_sessions = conn_2
            .query_row("select count(*) from sessions", [], |row| {
                row.get::<usize, i64>(0)
            })
            .unwrap();

        assert_eq!(user_b_sessions, 2);
    }
    #[test]
    fn test_valid_input_create_session() {
        let conn = setup_db_schema();
        let user = create_user();

        let session = create_session(&conn, "id-1", "sesh-1", &user.user_id).unwrap();
        assert_eq!(user.user_id, session.creator_id);
    }

    #[test]
    fn test_duplicate_id_create_session() {
        let conn = setup_db_schema();
        let user = create_user();

        let session = create_session(&conn, "id-1", "sesh-1", &user.user_id).unwrap();
        assert_eq!(user.user_id, session.creator_id);

        let session_2 = create_session(&conn, "id-1", "sesh-1", &user.user_id).unwrap();
        assert_eq!(user.user_id, session_2.creator_id);
    }

    #[test]
    fn test_fetching_models_used_in_session() {
        let conn = setup_db_schema();
        let user = create_user();
        let input = "2+2";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };

        let chat_response_2 = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_abc"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "kimi".to_owned(),
        };
        create_session(&conn, "session_abc", "sesh", &user.user_id).unwrap();
        let _chat_1 = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response_2.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        conn.execute("insert into chats(id, user_id, content, resp_id, role, context_id, created_at, updated_at, row_counter, session_id, model_name) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)", (Uuid::now_v7().to_string(), &user.user_id, &chat_response.input, None::<String>, Into::<String>::into(chat_response.role),  &chat_response.parent_chat_id, get_unix_time_now().to_string(), get_unix_time_now().to_string(), 1, "session_abc".to_owned(), None::<String>)).unwrap();

        create_session(&conn, "session_def", "sesh-2", &user.user_id).unwrap();

        let input = "4+4";
        let chat_response = ChatResponse {
            input: input.to_owned(),
            session_id: String::from("session_def"),
            role: Role::User,
            parent_chat_id: None,
            metrics: None,
            model_used: "gpt-oss".to_owned(),
        };
        let _chat_1 = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");
        let _ = save_chat(&conn, &user, chat_response.clone()).expect("chat should be saved");

        let rows = get_delta(&conn, &user.user_id, 0).unwrap();
        assert_eq!(rows.sessions.len(), 2);
        assert_eq!(rows.chats.len(), 7);
        let models = fetch_models_used_by_session(&conn, "session_abc").unwrap();

        assert_eq!(models.len(), 2);
        assert_eq!(models[0], "gpt-oss".to_owned());
        assert_eq!(models[1], "kimi");
    }

    struct SavedChatRow {
        content: String,
        resp_id: Option<String>,
        role: String,
        user_id: String,
        context_id: Option<String>,
    }

    fn fetch_saved_chat_row(conn: &Connection, chat_id: &str) -> SavedChatRow {
        conn.query_row(
            "SELECT content, resp_id, role, user_id, context_id FROM chats WHERE id = ?1",
            [chat_id.to_string()],
            |row| {
                Ok(SavedChatRow {
                    content: row.get(0)?,
                    resp_id: row.get(1)?,
                    role: row.get(2)?,
                    user_id: row.get(3)?,
                    context_id: row.get(4)?,
                })
            },
        )
        .expect("saved chat row should exist")
    }

    pub fn create_user() -> User {
        User {
            id: Uuid::now_v7().to_string(),
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
    fn create_user_by_id(user_id: &str) -> User {
        User {
            id: Uuid::now_v7().to_string(),
            user_id: String::from(user_id),
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
    pub fn setup_db_schema() -> Connection {
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
        updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
        row_counter INTEGER,
        session_id TEXT,
        model_name TEXT
    );",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            creator_id TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            snapshot TEXT
        )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE INDEX idx_chats_session_id ON chats(session_id);",
            [],
        )
        .unwrap();
        conn
    }
}
