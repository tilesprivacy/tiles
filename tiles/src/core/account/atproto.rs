//! Handles atprotocol stuff

use anyhow::{Result, anyhow};
use atrium_api::types::string::Did;
use atrium_common::store::Store;
use atrium_identity::{
    did::{CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL},
    handle::{AtprotoHandleResolver, AtprotoHandleResolverConfig, DnsTxtResolver},
};
use atrium_oauth::{
    AtprotoLocalhostClientMetadata, AuthorizeOptions, CallbackParams, DefaultHttpClient,
    KnownScope, OAuthClient, OAuthClientConfig, OAuthResolverConfig, Scope,
    store::{session::MemorySessionStore, state::MemoryStateStore},
};
use log::info;
use reqwest::Client;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, process::Command, sync::Arc, time::Duration};
use tokio::sync::oneshot;

use std::error::Error;

use hickory_resolver::TokioResolver;

use crate::{core::storage::db::Dbconn, daemon::start_internal_server, utils::get_unix_time_now};

#[derive(Deserialize)]
struct HandleResolve {
    did: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct AtCallbackParams {
    code: Option<String>,
    iss: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

struct AtprotoAuthData {
    // did:plc
    key: String,
    // serialized session data
    session: String,
    // serialized state data
    state: String,
    #[allow(dead_code)]
    is_logged_in: bool,
    created_at: u64,
    updated_at: u64,
    #[allow(dead_code)]
    handle: String,
}

struct HickoryDnsTxtResolver {
    resolver: TokioResolver,
}

impl Default for HickoryDnsTxtResolver {
    fn default() -> Self {
        Self {
            resolver: TokioResolver::builder_tokio()
                .expect("Failed to create TokioResolver builder")
                .build()
                .expect("Failed to build tokio resolver"),
        }
    }
}

impl DnsTxtResolver for HickoryDnsTxtResolver {
    async fn resolve(
        &self,
        query: &str,
    ) -> core::result::Result<Vec<String>, Box<dyn Error + Send + Sync + 'static>> {
        Ok(self
            .resolver
            .txt_lookup(query)
            .await?
            .answers()
            .iter()
            .map(|txt| txt.to_string())
            .collect())
    }
}

pub async fn login(conn: &Dbconn, handle: &str) -> Result<()> {
    let http_client = Arc::new(DefaultHttpClient::default());
    const LOGIN_PORT: u32 = 8988;

    let mem_session_store = MemorySessionStore::default();
    let mem_state_store = MemoryStateStore::default();

    let config = OAuthClientConfig {
        client_metadata: AtprotoLocalhostClientMetadata {
            redirect_uris: Some(vec![String::from("http://127.0.0.1:8988/callback")]),
            scopes: Some(vec![
                Scope::Known(KnownScope::Atproto),
                Scope::Known(KnownScope::TransitionGeneric),
            ]),
        },
        keys: None,
        resolver: OAuthResolverConfig {
            did_resolver: CommonDidResolver::new(CommonDidResolverConfig {
                plc_directory_url: DEFAULT_PLC_DIRECTORY_URL.to_string(),
                http_client: http_client.clone(),
            }),
            handle_resolver: AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
                dns_txt_resolver: HickoryDnsTxtResolver::default(),
                http_client: http_client.clone(),
            }),
            authorization_server_metadata: Default::default(),
            protected_resource_metadata: Default::default(),
        },
        state_store: mem_state_store.clone(),
        session_store: mem_session_store.clone(),
    };

    let Ok(client) = OAuthClient::new(config) else {
        panic!("client fuck up")
    };

    //TODO: This resolve function is hack to convert handle to DID
    // cuz for some reason the authorize fn not working for customd domains
    // it does work for bluesky hosted handles and DIDs.
    // Probably smthng to do w DNS resolver. Will dig more latta
    let did = resolve_handle_to_did(handle)
        .await
        .inspect_err(|_| eprintln!("Failed to resolve handle"))?;

    info!("{}", did);
    let url = client
        .authorize(
            did.clone(),
            AuthorizeOptions {
                scopes: vec![
                    Scope::Known(KnownScope::Atproto),
                    Scope::Known(KnownScope::TransitionGeneric),
                ],
                ..Default::default()
            },
        )
        .await
        .inspect_err(|_| eprintln!("Failed to authorize"))?;

    let mut child = Command::new("open").arg(url).spawn()?;
    child.wait()?;
    let (callback_tx, callback_rx) = oneshot::channel();

    //TODO: can we randomze port
    start_internal_server(Some(LOGIN_PORT), callback_tx).await?;
    let params = callback_rx.await?;
    info!("params recieved {:?}", params);

    if let Some(code) = params.code {
        let cb_params = CallbackParams {
            code,
            state: params.state,
            iss: params.iss,
        };
        let (_auth_session, _) = client.callback(cb_params).await?;

        let did_struct = Did::new(did.clone()).map_err(|_e| anyhow!("Failed to convert to Did"))?;

        let session = mem_session_store
            .get(&did_struct)
            .await?
            .expect("Expected Session");
        let session_string = serde_json::to_string(&session)?;

        let auth_data = AtprotoAuthData {
            key: did.clone(),
            session: session_string,
            state: "".to_owned(),
            is_logged_in: true,
            created_at: get_unix_time_now(),
            updated_at: get_unix_time_now(),
            handle: handle.to_owned(),
        };

        upsert_auth_data(&conn.common, &auth_data)?;
        println!("LoggedIn successfully as {}", handle);
    } else {
        eprintln!(
            "Error authorizing due to {}",
            params
                .error_description
                .unwrap_or("unknow reason".to_owned())
        );
    }
    Ok(())
}

pub fn logout(conn: &Dbconn) -> Result<()> {
    if let Some(auth_user) = fetch_logged_in_data(&conn.common)? {
        let key = auth_user.key.clone();
        let logout_user = AtprotoAuthData {
            is_logged_in: false,
            ..auth_user
        };
        upsert_auth_data(&conn.common, &logout_user)?;
        println!("Loggedout successfully as {}", key);
    } else {
        println!("No logged-in user, please login")
    }
    Ok(())
}

async fn resolve_handle_to_did(handle: &str) -> Result<String> {
    let client_builder = Client::builder().timeout(Duration::from_secs(5));
    let client = client_builder.build()?;
    let response = client
        .get(format!(
            "https://bsky.social/xrpc/com.atproto.identity.resolveHandle?handle={}",
            handle
        ))
        .send()
        .await;

    match response {
        Err(err) if err.is_timeout() => Err(anyhow!("Request failed due to Api timedout")),
        Err(err) => Err(anyhow!("Request failed due to {:?}", err)),
        Ok(res) if res.status() == 200 => {
            let resolve_data = res.json::<HandleResolve>().await?;
            Ok(resolve_data.did)
        }
        Ok(res) => Err(anyhow!("Api failed with status {}", res.status())),
    }
}

fn upsert_auth_data(conn: &Connection, data: &AtprotoAuthData) -> Result<()> {
    let mut stmt = conn.prepare(
        "insert into atproto_auth_data(key, session, state, is_logged_in, created_at, updated_at, handle) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         on conflict(key)
         do update set session = ?2, updated_at = ?6, is_logged_in = ?4
         ",
    )?;

    match stmt.execute(params![
        data.key.to_owned(),
        data.session.to_owned(),
        data.state.to_owned(),
        data.is_logged_in,
        data.created_at as f64,
        data.updated_at as f64,
        data.handle.to_owned()
    ]) {
        Ok(_res) => Ok(()),
        Err(err) => Err(anyhow!("Err inserting due to {}", err)),
    }
}

#[allow(dead_code)]
fn fetch_auth_data(conn: &Connection, did: &str) -> Result<AtprotoAuthData> {
    let data = conn.query_row(
        "SELECT key, session, state, is_logged_in, created_at , updated_at, handle FROM atproto_auth_data WHERE key = ?1",
        [did],
        |row| {
            Ok(AtprotoAuthData {
                key: row.get(0)?,
                session: row.get(1)?,
                state: row.get(2)?,
                is_logged_in: row.get(3)?,
                created_at: row.get::<usize, f64>(4)? as u64,
                updated_at: row.get::<usize, f64>(5)? as u64,
                handle: row.get(6)?,
            })
        },
    )?;
    Ok(data)
}

#[allow(dead_code)]
fn delete_auth_data(conn: &Connection, did: &str) -> Result<()> {
    let mut stmt = conn.prepare("delete from atproto_auth_data where key = ?1")?;

    match stmt.execute(params![did]) {
        Ok(_res) => Ok(()),
        Err(err) => Err(anyhow!("Err deleting due to {}", err)),
    }
}

fn fetch_logged_in_data(conn: &Connection) -> Result<Option<AtprotoAuthData>> {
    conn.query_row(
        "SELECT key, session, state, is_logged_in, created_at, updated_at, handle FROM atproto_auth_data WHERE is_logged_in = true",
        [],
        |row| {
            Ok(AtprotoAuthData {
                key: row.get(0)?,
                session: row.get(1)?,
                state: row.get(2)?,
                is_logged_in: row.get(3)?,
                created_at: row.get::<usize, f64>(4)? as u64,
                updated_at: row.get::<usize, f64>(5)? as u64,
                handle: row.get(6)?,
            })
        },
    ).optional().map_err(Into::<anyhow::Error>::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_add_a_new_auth_entry() {
        let conn = setup_db_schema();

        let auth_data = AtprotoAuthData {
            key: String::from("did:plc:wth"),
            session: String::from("session_stuff"),
            state: "".to_owned(),
            is_logged_in: true,
            created_at: get_unix_time_now(),
            updated_at: get_unix_time_now(),
            handle: "madcla.ws".to_owned(),
        };

        upsert_auth_data(&conn, &auth_data).unwrap();
        let auth_data_2 = fetch_auth_data(&conn, "did:plc:wth").unwrap();

        assert_eq!(auth_data.key, auth_data_2.key)
    }

    #[test]
    fn test_add_same_auth_entry() {
        let conn = setup_db_schema();

        let auth_data = AtprotoAuthData {
            key: String::from("did:plc:wth"),
            session: String::from("session_stuff"),
            state: "".to_owned(),
            is_logged_in: true,
            created_at: get_unix_time_now(),
            updated_at: get_unix_time_now(),
            handle: "madcla.ws".to_owned(),
        };

        upsert_auth_data(&conn, &auth_data).unwrap();
        let auth_data_2 = fetch_auth_data(&conn, "did:plc:wth").unwrap();

        assert_eq!(auth_data.key, auth_data_2.key);

        let auth_data_2 = AtprotoAuthData {
            key: String::from("did:plc:wth"),
            session: String::from("session_stuff_2"),
            state: "".to_owned(),
            is_logged_in: true,
            created_at: get_unix_time_now(),
            updated_at: get_unix_time_now(),
            handle: "madcla.ws".to_owned(),
        };

        upsert_auth_data(&conn, &auth_data_2).unwrap();

        let auth_data_2 = fetch_auth_data(&conn, "did:plc:wth").unwrap();

        assert_eq!(auth_data_2.session, "session_stuff_2");
    }

    #[test]
    fn test_fetch_valid_logged_in_auth_entry() {
        let conn = setup_db_schema();

        let auth_data = AtprotoAuthData {
            key: String::from("did:plc:wth"),
            session: String::from("session_stuff"),
            state: "".to_owned(),
            is_logged_in: true,
            created_at: get_unix_time_now(),
            updated_at: get_unix_time_now(),
            handle: "madcla.ws".to_owned(),
        };

        upsert_auth_data(&conn, &auth_data).unwrap();
        let auth_data_2 = fetch_logged_in_data(&conn).unwrap();

        assert!(auth_data_2.is_some())
    }

    #[test]
    fn test_fetch_zero_logged_in_auth_entry() {
        let conn = setup_db_schema();

        let auth_data_2 = fetch_logged_in_data(&conn).unwrap();

        assert!(auth_data_2.is_none())
    }
    fn setup_db_schema() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS atproto_auth_data(
            key TEXT PRIMARY KEY,
            session TEXT ,
            state TEXT,
            is_logged_in INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            handle TEXT NOT NULL
        )",
            [],
        )
        .unwrap();

        conn
    }
}
