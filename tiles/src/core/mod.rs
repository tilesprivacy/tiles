//! tiles-core
//!
//! The core runtime which different UI apps can leverage
//! Generally the core will be run as daemon and interact with other sub components

use anyhow::{Context, Result};

use crate::{
    core::{
        account::local::save_root_account_db,
        storage::db::{Dbconn, init_db},
    },
    utils::config::{ConfigProvider, DefaultProvider},
};

pub mod account;
pub mod agent;
pub mod chats;
pub mod health;
pub mod network;
pub mod plugin;
pub mod server;
pub mod storage;
// Entrypoint of the core
pub fn init() -> Result<Dbconn> {
    let config_provider = DefaultProvider;
    config_provider
        .get_or_create_config_dir()
        .context("Failed in creating config folder")?;
    config_provider
        .get_or_create_data_dir()
        .context("Failed to create data dir")?;
    init_db()
}

pub fn init_account(db_conn: &Dbconn) -> Result<()> {
    save_root_account_db(db_conn)
}
