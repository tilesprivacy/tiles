//! Core Database Handling
//!
//! Uses sqlite as the underlying database
//!

use std::{env, path::PathBuf};

use anyhow::{Result, anyhow};
use log::info;
use rusqlite::Connection;
use tilekit::accounts::{create_and_save_passkey, get_passkey};

use crate::utils::config::{ConfigProvider, DefaultProvider, get_app_name};
use rusqlite_migration::{M, Migrations};

#[derive(Debug)]
pub enum DBTYPE {
    COMMON,
    CHAT,
}

pub struct Dbconn {
    pub chat: Connection,
    pub common: Connection,
}

// DEFINE MIGRATIONS

const COMMON_MIGRATION_ARRAY: &[M] = &[
    M::up(
        "CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            username TEXT NOT NULL,
            active_profile INTEGER NOT NULL DEFAULT 0 CHECK (active_profile IN (0,1)),
            account_type TEXT NOT NULL,
            root INTEGER NOT NULL DEFAULT 0 CHECK (root IN (0,1)),
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
            UNIQUE(account_type, user_id)
        );
        ",
    ),
    M::up(
        "CREATE TABLE IF NOT EXISTS atproto_auth_data(
            key TEXT PRIMARY KEY,
            session TEXT ,
            state TEXT,
            is_logged_in INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            handle TEXT NOT NULL
        )",
    ),
];

const COMMON_MIGRATIONS: Migrations = Migrations::from_slice(COMMON_MIGRATION_ARRAY);

const CHATS_MIGRATION_ARRAY: &[M] = &[
    M::up(
        "CREATE TABLE IF NOT EXISTS chats (
        id TEXT PRIMARY KEY,
        content TEXT NOT NULL,
        resp_id TEXT,
        role TEXT NOT NULL,
        user_id TEXT NOT NULL,
        context_id TEXT,
        created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
        updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
    )",
    ),
    // After creating row_counter, we backfill the row_counter for existing rows
    // which doesnt have any
    M::up(
        "
        ALTER TABLE CHATS ADD COLUMN row_counter INTEGER;
        UPDATE chats SET row_counter = (
            SELECT rn FROM (
                SELECT id, ROW_NUMBER() OVER ( PARTITION BY user_id ORDER BY id ) as rn FROM chats
            ) t WHERE t.id = chats.id );

        ALTER TABLE CHATS ADD COLUMN session_id TEXT;
        ",
    ),
    M::up(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            creator_id TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )",
    ),
    M::up("CREATE INDEX idx_chats_session_id ON chats(session_id);"),
];

const CHATS_MIGRATIONS: Migrations = Migrations::from_slice(CHATS_MIGRATION_ARRAY);

pub fn init_db() -> Result<Dbconn> {
    let mut chat_conn = get_db_conn(&DBTYPE::CHAT)?;
    let mut common_conn = get_db_conn(&DBTYPE::COMMON)?;

    apply_migrations(&mut common_conn, &mut chat_conn)?;

    Ok(Dbconn {
        chat: chat_conn,
        common: common_conn,
    })
}

pub fn get_db_conn(db_type: &DBTYPE) -> Result<Connection> {
    let db_path = get_db_path(db_type)?;
    let conn = Connection::open(db_path)
        .map_err(|e| anyhow!("Failed to create db connection due to {:?}", e))?;

    let passkey = fetch_passkey()?;
    let cipher_format = format!("x'{}'", passkey);
    conn.pragma_update(None, "KEY", cipher_format)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    info!("DB {:?} Opened", db_type);
    Ok(conn)
}

fn apply_migrations(common_conn: &mut Connection, chat_conn: &mut Connection) -> Result<()> {
    COMMON_MIGRATIONS
        .to_latest(common_conn)
        .map_err(<rusqlite_migration::Error as Into<anyhow::Error>>::into)?;
    CHATS_MIGRATIONS.to_latest(chat_conn).map_err(|e| e.into())
}
fn get_db_path(db_type: &DBTYPE) -> Result<PathBuf> {
    let user_data_dir = DefaultProvider.get_user_data_dir()?;
    match db_type {
        DBTYPE::COMMON => Ok(user_data_dir.join("common_v2.db")),
        DBTYPE::CHAT => Ok(user_data_dir.join("chats_v2.db")),
    }
}

fn fetch_passkey() -> Result<String> {
    let app_name = get_app_name();
    // handling db passwords in dev mode separately
    // This is to suppress keychain popups during development

    if cfg!(debug_assertions) {
        if let Ok(passwd) = env::var("TILES_DEV_DB_PASSWORD") {
            return Ok(passwd);
        } else {
            info!("DB passkey not found in development, creating one..");
            let passwd = create_and_save_passkey(&app_name, "db_passkey")?;
            info!(
                "Save this password {} as an environment variable with name `TILES_DEV_DB_PASSWORD`",
                passwd
            );
            return Ok(passwd);
        }
    }

    if let Ok(passkey) = get_passkey(&app_name, "db_passkey") {
        Ok(passkey)
    } else {
        info!("DB passkey not found, creating one..");
        create_and_save_passkey(&app_name, "db_passkey")
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn migrations_test() {
        assert!(COMMON_MIGRATIONS.validate().is_ok());
        assert!(CHATS_MIGRATIONS.validate().is_ok());
    }
}
