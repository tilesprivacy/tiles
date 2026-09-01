//! Local Account
// Stuff related to account and identity system
use anyhow::{Context, Result, anyhow};
use cid::Cid;
use dialog_credentials::{Ed25519KeyResolver, Ed25519Signer, KeyExport};
use dialog_ucan::{
    Delegation, DelegationBuilder, Invocation, InvocationBuilder, future::Sendable,
    subject::Subject, time::timestamp::Timestamp,
};

use dialog_varsig::{Did, Principal, Signature, eddsa::Ed25519Signature};
// use dialog
use iroh::SecretKey;
use log::{info, warn};
use rusqlite::{
    Connection, Row, ToSql, params,
    types::{FromSql, FromSqlError},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    fmt::Display,
    str::FromStr,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use toml::Table;
use uuid::Uuid;

use crate::{
    core::{
        account::{create_identity, get_secret_key, get_signing_key},
        storage::db::{DBTYPE, Dbconn, get_db_conn},
    },
    utils::{
        config::{DefaultProvider, get_app_name, get_or_create_config, save_config},
        get_unix_time_now,
    },
};
const ROOT_USER_CONFIG_KEY: &str = "root-user";

const ROOT_PARSE_ERROR: &str = "Failed to parse root user config";

#[derive(Serialize, Debug)]
pub struct RootUser {
    pub id: String,
    pub nickname: String,
}

// Type of User account
#[derive(Debug, Clone)]
pub enum ACCOUNT {
    // root account, created in the system
    LOCAL,

    // remote account
    PEER,
}

#[derive(Debug)]
pub struct AccountError {
    pub error: String,
}

impl Display for AccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}
impl std::error::Error for AccountError {}
impl TryFrom<String> for ACCOUNT {
    type Error = AccountError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value_lower = value.to_lowercase();
        match value_lower.as_str() {
            "local" => Ok(ACCOUNT::LOCAL),
            "peer" => Ok(ACCOUNT::PEER),
            _ => Err(AccountError {
                error: "Invalid account type".to_owned(),
            }),
        }
    }
}
impl Display for ACCOUNT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LOCAL => write!(f, "{}", String::from("local")),
            Self::PEER => write!(f, "{}", String::from("peer")),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct User {
    // unique uuidv7
    pub id: String,
    // did:key
    pub user_id: String,
    // nickname of the user
    pub username: String,
    // is this identity, user is using everywhere in Tiles
    pub active_profile: bool,
    // LOCAL / PEER(other identities other than user's)
    pub account_type: ACCOUNT,
    // The first identity created locally
    pub root: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug)]
pub struct Token {
    pub id: String,
    pub did: String,
    pub token: String,
    pub cid: String,
    pub r#type: TokenType,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug)]
pub enum TokenType {
    // data syncing
    Sync,
    // Remote connection
    Connect,
}

impl FromSql for TokenType {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str()? {
            "sync" => Ok(Self::Sync),
            "connect" => Ok(Self::Connect),
            _token_type => Err(FromSqlError::InvalidType),
        }
    }
}
impl ToSql for TokenType {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            TokenType::Sync => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Text(String::from("sync")),
            )),
            TokenType::Connect => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Text(String::from("connect")),
            )),
        }
    }
}

impl Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenType::Sync => write!(f, "sync"),
            TokenType::Connect => write!(f, "connect"),
        }
    }
}

impl Display for RootUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "id: {}\nnickname: {}\n", self.id, self.nickname)
    }
}

impl RootUser {
    pub fn new(config: &Table) -> Result<Self> {
        let id = config
            .get("id")
            .ok_or_else(|| anyhow!("Missing ID"))?
            .as_str()
            .ok_or_else(|| anyhow!("ID not a string"))?;
        let nickname = config
            .get("nickname")
            .ok_or_else(|| anyhow!("Missing Nickname"))?
            .as_str()
            .ok_or_else(|| anyhow!("Nickname not a string"))?;
        Ok(RootUser {
            id: id.to_owned(),
            nickname: nickname.to_owned(),
        })
    }

    pub fn to_table(&self) -> Table {
        let mut root_user_table = Table::new();
        root_user_table.insert(String::from("id"), toml::Value::String(self.id.clone()));
        root_user_table.insert(
            String::from("nickname"),
            toml::Value::String(self.nickname.clone()),
        );
        root_user_table
    }
}

/// Returns a `RootUser`, which represents a root user
///
/// # Params
///
/// - config: A `Table` type of entire config.toml file
pub fn get_root_user_details(config: &Table) -> Result<RootUser> {
    let root_user = config
        .get(ROOT_USER_CONFIG_KEY)
        .ok_or_else(|| anyhow!(ROOT_PARSE_ERROR))?;
    let root_user_table = root_user
        .as_table()
        .ok_or_else(|| anyhow!("root user not a table"))?;
    RootUser::new(root_user_table)
}

/// Create a root account
/// Stores the private credentials in OS secure password manager
///
/// # Params
///
/// - config: A `Table` type of entire config.toml file
/// - nickname: Nickname for the identity (Optional)
///
/// Returns the root_user_config as a `Table` type
pub async fn create_root_account(config: &Table, nickname: Option<String>) -> Result<Table> {
    let root_user = config
        .get(ROOT_USER_CONFIG_KEY)
        .ok_or_else(|| anyhow!("{} doesn't exist", ROOT_USER_CONFIG_KEY))?;
    let root_user_table = root_user
        .as_table()
        .ok_or_else(|| anyhow!(ROOT_PARSE_ERROR))?;
    let root_user_data = RootUser::new(root_user_table)?;
    let did = root_user_data.id;
    if did.is_empty() {
        Ok(create_root_user(root_user_table, nickname).await?)
    } else {
        Ok(root_user_table.clone())
    }
}

/// Save the root config in `Table` type to config.toml
///
/// # Params
///
/// - config: A `Table` type of entire config.toml file
/// - root_user_config: A `Table` type of root user
pub fn save_root_account(mut config: Table, root_user_config: &Table) -> Result<()> {
    config.insert(
        String::from(ROOT_USER_CONFIG_KEY),
        toml::Value::Table(root_user_config.clone()),
    );
    save_config(&config)
}

/// Sets nickname for the root account
///
/// # Params
///
/// - config: A `Table` type of entire config.toml file
/// - nickname: Nickname for the identity
///
/// Returns the root_user_config as a `Table` type
pub fn set_nickname(config: &Table, nickname: &str) -> Result<Table> {
    let root_user = config
        .get(ROOT_USER_CONFIG_KEY)
        .ok_or_else(|| anyhow!("{} doesn't exist", ROOT_USER_CONFIG_KEY))?;

    let mut root_user_table = root_user
        .as_table()
        .ok_or_else(|| anyhow!(ROOT_PARSE_ERROR))?
        .clone();
    let did = root_user_table
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or(anyhow!("Failed to get id from config"))?;
    if did.is_empty() {
        Err(anyhow::anyhow!("No Root user available"))
    } else {
        root_user_table.insert("id".to_owned(), toml::Value::String(did.to_owned()));
        root_user_table.insert(
            "nickname".to_owned(),
            toml::Value::String(nickname.to_owned()),
        );
        Ok(root_user_table)
    }
}

pub fn get_current_user(conn: &Connection) -> Result<User> {
    let mut fetch_current_user = conn.prepare("select id, user_id, username, account_type, active_profile, root, created_at, updated_at  from users where active_profile= true")?;

    fetch_current_user
        .query_one([], |row| {
            let account_type: String = row.get(3)?;
            let created_at: f64 = row.get(6)?;
            let updated_at: f64 = row.get(7)?;
            Ok(User {
                id: row.get(0)?,
                user_id: row.get(1)?,
                username: row.get(2)?,
                account_type: ACCOUNT::try_from(account_type).map_err(FromSqlError::other)?,
                active_profile: row.get(4)?,
                root: row.get(5)?,

                created_at: created_at as u64,
                updated_at: updated_at as u64,
            })
        })
        .map_err(<rusqlite::Error as Into<anyhow::Error>>::into)
}

pub fn get_user(conn: &Connection, did: &str) -> Result<User> {
    let mut fetch_current_user = conn.prepare("select id, user_id, username, account_type, active_profile, root, created_at, updated_at  from users where user_id= ?1")?;

    fetch_current_user
        .query_one([did], |row| {
            let account_type: String = row.get(3)?;
            let created_at: f64 = row.get(6)?;
            let updated_at: f64 = row.get(7)?;
            Ok(User {
                id: row.get(0)?,
                user_id: row.get(1)?,
                username: row.get(2)?,
                account_type: ACCOUNT::try_from(account_type).map_err(FromSqlError::other)?,
                active_profile: row.get(4)?,
                root: row.get(5)?,

                created_at: created_at as u64,
                updated_at: updated_at as u64,
            })
        })
        .map_err(<rusqlite::Error as Into<anyhow::Error>>::into)
}

pub fn save_root_account_db(db_conn: &Dbconn) -> Result<()> {
    let config = get_or_create_config(DefaultProvider)?;
    let root_user = get_root_user_details(&config)?;
    let user = User {
        id: Uuid::now_v7().to_string(),
        user_id: root_user.id,
        username: root_user.nickname,
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
    };

    let mut fetch_root_user = db_conn
        .common
        .prepare("select id from users where root = true")?;

    match fetch_root_user.query_one([], |_row| Ok(())) {
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            db_conn.common.execute("insert into users (id, user_id, username, active_profile, account_type, root) values
                (?1, ?2, ?3,?4, ?5, ?6)", (&user.id.to_string(), &user.user_id, &user.username, &user.active_profile,
                    user.account_type.to_string(),  &user.root))?;
            Ok(())
        }
        Err(_err) => Err(anyhow!("Fetching user from db failed")),
        _ => Ok(()),
    }
}

// TODO: We could add unique user_id constraints, but
// we will wait for it until we solve the sync part
pub fn save_peer_account_db(db_conn: &Connection, user_id: &str, nickname: &str) -> Result<()> {
    let user = User {
        id: Uuid::now_v7().to_string(),
        user_id: String::from(user_id),
        username: String::from(nickname),
        account_type: ACCOUNT::PEER,
        active_profile: false,
        root: false,
        created_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_secs(),
        updated_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_secs(),
    };
    db_conn.execute(
        "insert into users (id, user_id, username, active_profile, account_type, root) values
                (?1, ?2, ?3,?4, ?5, ?6)",
        (
            &user.id.to_string(),
            &user.user_id,
            &user.username,
            &user.active_profile,
            user.account_type.to_string(),
            &user.root,
        ),
    )?;
    Ok(())
}

pub async fn create_token(aud_did: &str, db_conn: &Dbconn) -> Result<String> {
    let user = get_current_user(&db_conn.common)?;
    let app_name = get_app_name();
    let signing_key = get_signing_key(&app_name, &user.user_id)?;
    let keyexport = KeyExport::from(&signing_key.to_bytes());
    let issuer: Ed25519Signer = Ed25519Signer::import(keyexport).await?;
    info!("issuer did {}", issuer.ed25519_did());
    let token = generate_delegation_token(issuer, aud_did, db_conn).await?;
    Ok(token.token)
}

async fn generate_delegation_token(
    issuer: Ed25519Signer,
    aud_did: &str,
    db_conn: &Dbconn,
) -> Result<Token> {
    let aud_did = Did::from_str(aud_did)?;
    let subject = Subject::Specific(Did::from_str(&issuer.ed25519_did().to_string())?);
    let delegation = DelegationBuilder::<Ed25519Signature>::new()
        .issuer(issuer)
        .audience(&aud_did)
        .subject(subject)
        .policy(vec![])
        //generating token with an expiry of an year, assuming this is for powerline user, will make it configurable later
        .expiration(Timestamp::new(
            SystemTime::now() + Duration::from_secs(86400 * 365 * 10),
        )?)
        .command(vec![])
        .try_build()
        .await?;

    let delegation_token = save_token(db_conn, delegation.issuer().did().as_str(), delegation)?;
    Ok(delegation_token)
}

pub fn add_token(delegation_token: &str, db_conn: &Dbconn) -> Result<Token> {
    let delegation_token_bytes = data_encoding::BASE64
        .decode(delegation_token.as_bytes())
        .context("Delegation token is in invalid base64")?;
    let delegation: Delegation<Ed25519Signature> =
        serde_ipld_dagcbor::from_slice(&delegation_token_bytes).context("Invalid DID")?;

    let issuer_did = delegation.issuer().did();
    let token = save_token(db_conn, issuer_did.as_str(), delegation)
        .context("Saving delegation token failed")?;
    Ok(token)
}

pub fn fetch_token(did: &str, conn: &Connection, token_type: TokenType) -> Result<Option<Token>> {
    let fetch_resp = conn.query_row(
        "SELECT id, did, token, cid, created_at, updated_at, type FROM tokens WHERE did= ?1 and type=?2 order by id desc limit 1",
        [did, token_type.to_string().as_str()],
        |row| {
            Ok(Token {
                id: row.get(0)?,
                did: row.get(1)?,
                token: row.get(2)?,
                cid: row.get(3)?,
                created_at: row.get::<usize, f64>(4)? as u64,
                updated_at: row.get::<usize, f64>(5)? as u64,
                r#type: row.get(6)?,
            })
        },
    );
    match fetch_resp {
        Ok(token) => Ok(Some(token)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(Into::into(err)),
    }
}

pub fn fetch_tokens(conn: &Connection) -> Result<Vec<Token>> {
    let query = "SELECT id, did, token, cid, created_at, updated_at, type FROM tokens";

    let mut stmt = conn.prepare(query)?;
    let token_rows = stmt.query_map([], |row| {
        Ok(Token {
            id: row.get(0)?,
            did: row.get(1)?,
            token: row.get(2)?,
            cid: row.get(3)?,
            created_at: row.get::<usize, f64>(4)? as u64,
            updated_at: row.get::<usize, f64>(5)? as u64,
            r#type: row.get(6)?,
        })
    })?;

    let mut tokens: Vec<Token> = vec![];

    for token in token_rows {
        tokens.push(token?);
    }
    Ok(tokens)
}

fn save_token<S: Signature>(conn: &Dbconn, did: &str, delegation: Delegation<S>) -> Result<Token> {
    let mut stmt = conn.common.prepare(
        "insert into tokens(id, did, token, cid, created_at, updated_at, type) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;

    let token_cid = delegation
        .to_cid()
        .to_string_of_base(cid::multibase::Base::Base64)?;
    let token_serialized = serde_ipld_dagcbor::to_vec(&delegation)?;

    let token = data_encoding::BASE64.encode(&token_serialized);

    match stmt.execute(params![
        Uuid::now_v7().to_string(),
        did.to_owned(),
        token,
        token_cid,
        get_unix_time_now() as f64,
        get_unix_time_now() as f64,
        TokenType::Sync
    ]) {
        Ok(_res) => {
            let token = fetch_token(did, &conn.common, TokenType::Sync)?;
            Ok(token.expect("Expected token"))
        }
        Err(err) => Err(anyhow!("Err inserting token due to {}", err)),
    }
}

/// Create an invocation token from the delegation token
pub async fn create_invocation_token(token_delegated: &str, conn: &Connection) -> Result<String> {
    let user = get_current_user(conn)?;
    let app_name = get_app_name();
    let signing_key = get_signing_key(&app_name, &user.user_id)?;
    let keyexport = KeyExport::from(&signing_key.to_bytes());
    let issuer: Ed25519Signer = Ed25519Signer::import(keyexport).await?;
    generate_invocation_token(issuer, token_delegated).await
}

// TODO: Later add more validations as in  expiry, signature or are theses
// done while parsingto a Delegation struct
pub fn is_valid_delegation(token: &str) -> Result<()> {
    if let Ok(token_bytes) = data_encoding::BASE64.decode(token.as_bytes())
        && let Ok(_delegation) =
            serde_ipld_dagcbor::from_slice::<Delegation<Ed25519Signature>>(&token_bytes)
    {
        Ok(())
    } else {
        Err(anyhow!("Invalid UCAN token"))
    }
}

async fn generate_invocation_token(issuer: Ed25519Signer, token_delegated: &str) -> Result<String> {
    let token_delegated_in_bytes = data_encoding::BASE64.decode(token_delegated.as_bytes())?;
    let delegation: Delegation<Ed25519Signature> =
        serde_ipld_dagcbor::from_slice(&token_delegated_in_bytes)?;

    let token_cid = delegation.to_cid();

    let invocation = InvocationBuilder::<Ed25519Signature>::new()
        .issuer(issuer)
        .audience(delegation.issuer())
        .subject(delegation.issuer())
        .command(vec![])
        .proofs(vec![token_cid])
        .try_build()
        .await?;

    let invocation_serialized = serde_ipld_dagcbor::to_vec(&invocation)
        .context("Failed to serialize invocation to dag cbor bytes")?;

    let invocation_token = data_encoding::BASE64.encode(&invocation_serialized);

    Ok(invocation_token)
}

pub async fn verify_invocation(invocation_token: &str) -> Result<()> {
    let db_conn: Connection = get_db_conn(&DBTYPE::COMMON)?;

    process_invocation_verification(invocation_token, db_conn).await
}

async fn process_invocation_verification(
    invocation_token: &str,
    db_conn: Connection,
) -> Result<()> {
    let invocation_token_bytes = data_encoding::BASE64.decode(invocation_token.as_bytes())?;

    let invocation: Invocation<Ed25519Signature> =
        serde_ipld_dagcbor::from_slice(&invocation_token_bytes)?;

    let hash_store: HashMap<Cid, Arc<Delegation<Ed25519Signature>>> = HashMap::new();

    let delegation_store: Arc<Mutex<HashMap<Cid, Arc<Delegation<Ed25519Signature>>>>> =
        Arc::new(Mutex::new(hash_store));

    let tokens = fetch_tokens(&db_conn)?;

    {
        let mut delegation_store_guard = delegation_store.lock().unwrap();

        for token in tokens {
            let delegation_token_bytes = data_encoding::BASE64.decode(token.token.as_bytes())?;
            let delegation: Delegation<Ed25519Signature> =
                serde_ipld_dagcbor::from_slice(&delegation_token_bytes)?;

            delegation_store_guard.insert(Cid::from_str(&token.cid)?, Arc::new(delegation));
        }
    }

    match invocation
        .check::<Sendable, _, _, _>(&delegation_store, &Ed25519KeyResolver)
        .await
    {
        Ok(_res) => Ok(()),
        Err(err) => {
            warn!("Invocation verification failed due to {:?}", err);
            Err(anyhow!("Invocation verification failed"))
        }
    }
}

async fn create_root_user(root_user_config: &Table, nickname: Option<String>) -> Result<Table> {
    let mut root_user_table = root_user_config.clone();
    let app_name = get_app_name();
    println!("{}", app_name);
    match create_identity(&app_name).await {
        Ok(did) => {
            root_user_table.insert("id".to_owned(), toml::Value::String(did));
            if let Some(nickname) = nickname {
                root_user_table.insert("nickname".to_owned(), toml::Value::String(nickname));
            }
            Ok(root_user_table)
        }
        Err(err) => Err(err),
    }
}

fn parse_user_from_row(row: &Row<'_>) -> Result<User, rusqlite::Error> {
    let account_type: String = row.get(3)?;
    let created_at: f64 = row.get(6)?;
    let updated_at: f64 = row.get(7)?;
    Ok(User {
        id: row.get(0)?,
        user_id: row.get(1)?,
        username: row.get(2)?,
        account_type: ACCOUNT::try_from(account_type).map_err(FromSqlError::other)?,
        active_profile: row.get(4)?,
        root: row.get(5)?,

        created_at: created_at as u64,
        updated_at: updated_at as u64,
    })
}
/// Gets a user account by its DID
pub fn get_user_info(conn: &Connection, did: &str) -> Result<User> {
    let mut fetch_user = conn.prepare("select id, user_id, username, account_type, active_profile, root, created_at, updated_at from users    where user_id = ?1")?;

    match fetch_user.query_one([did], parse_user_from_row) {
        Ok(user) => Ok(user),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(anyhow!("Peer doesnt exist")),
        Err(err) => {
            log::error!("{:?}", err);
            Err(anyhow!("Fetching user from db failed due to {:?}", err))
        }
    }
}

pub fn get_peer_list(db_conn: &Connection) -> Result<Vec<User>> {
    let mut stmt= db_conn.prepare("select id, user_id, username, account_type, active_profile, root, created_at, updated_at  from users where account_type != \'local\'")?;

    let user_rows = stmt
        .query_map([], parse_user_from_row)
        .map_err(<rusqlite::Error as Into<anyhow::Error>>::into)?;

    let mut peer_list: Vec<User> = vec![];

    for peer in user_rows {
        peer_list.push(peer?);
    }

    Ok(peer_list)
}

//TODO: Revoke the peers connected to online too
pub fn unlink(db_conn: &Connection, user_id: &str) -> Result<()> {
    let user = get_current_user(db_conn)?;
    if user.user_id == user_id {
        return Err(anyhow!("Cannot unlink yourself"));
    }

    match db_conn.execute(
        "delete from users where user_id = ?1 and account_type != \'local\'",
        [user_id],
    ) {
        Ok(0) => Err(anyhow!("A peer with DID {} doesn't exist", user_id)),
        Ok(_) => Ok(()),
        Err(err) => Err(anyhow!("Unable to unlink the peer due to {:?}", err)),
    }
}

pub fn get_app_secret_key(did: &str) -> Result<SecretKey> {
    let app_name = get_app_name();
    let signing_key = get_secret_key(&app_name, did)?;
    Ok(SecretKey::from_bytes(&signing_key))
}

pub fn create_dummy_user(conn: &Connection, did: Option<String>) -> User {
    let user_id = if let Some(did_str) = did {
        did_str
    } else {
        let uuid_did = Uuid::now_v7().to_string();
        format!("did:key:{}", uuid_did.split('-').collect::<Vec<&str>>()[0])
    };
    let chunk = "nickname";
    let id = Uuid::now_v7().to_string();
    let username = format!("nickname-{}", chunk);
    let user = User {
        id,
        user_id,
        username,
        account_type: ACCOUNT::PEER,
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
    };
    conn.execute(
        "insert into users (id, user_id, username, active_profile, account_type, root) values
                (?1, ?2, ?3,?4, ?5, ?6)",
        (
            &user.id.to_string(),
            &user.user_id,
            &user.username,
            &user.active_profile,
            user.account_type.to_string(),
            &user.root,
        ),
    )
    .unwrap();
    user
}
#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::core::account::local::{
        RootUser, create_root_account, get_current_user, get_root_user_details, set_nickname,
    };
    use anyhow::Result;
    use rusqlite::Connection;
    use toml::Table;

    fn use_sample_keyring_store() -> Result<()> {
        keyring_core::set_default_store(keyring_core::sample::Store::new()?);
        Ok(())
    }

    #[test]
    fn test_get_root_user_details_empty_id() -> Result<()> {
        let config: Table = toml::from_str(
            r#"
                [root-user]
                id = ''
                nickname = ''
            "#,
        )
        .unwrap();
        let acc_details = get_root_user_details(&config)?;
        assert!(acc_details.id.is_empty());
        Ok(())
    }

    #[test]
    fn test_get_root_user_details_valid_id() -> Result<()> {
        let config: Table = toml::from_str(
            r#"
                [root-user]
                id = 'did:key:xyz'
                nickname = ''
            "#,
        )
        .unwrap();
        let acc_details = get_root_user_details(&config)?;
        assert!(acc_details.id.contains("did:key"));
        Ok(())
    }

    #[tokio::test]
    async fn test_create_root_account_but_exists() {
        let config: Table = toml::from_str(
            r#"
                [root-user]
                id = 'did:key:xyz'
                nickname = ''
            "#,
        )
        .unwrap();
        let root_user = create_root_account(&config, None).await.unwrap();

        assert_eq!(
            root_user.get("id").unwrap().as_str().unwrap(),
            "did:key:xyz"
        );
    }

    #[tokio::test]
    async fn test_create_root_account_new() -> Result<()> {
        use_sample_keyring_store()?;
        let config: Table = toml::from_str(
            r#"
                [root-user]
                id = ''
                nickname = ''
            "#,
        )
        .unwrap();
        let root_user = create_root_account(&config, None).await.unwrap();

        assert_ne!(
            root_user.get("id").unwrap().as_str().unwrap(),
            "did:key:xyz"
        );

        assert!(
            root_user
                .get("id")
                .unwrap()
                .as_str()
                .unwrap()
                .starts_with("did:key")
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_create_root_account_new_w_nickname() -> Result<()> {
        use_sample_keyring_store()?;
        let config: Table = toml::from_str(
            r#"
                [root-user]
                id = ''
                nickname = ''
            "#,
        )
        .unwrap();
        let root_user = create_root_account(&config, Some(String::from("madclaws")))
            .await
            .unwrap();

        assert_ne!(
            root_user.get("id").unwrap().as_str().unwrap(),
            "did:key:xyz"
        );

        assert!(
            root_user
                .get("id")
                .unwrap()
                .as_str()
                .unwrap()
                .starts_with("did:key")
        );

        assert_eq!(
            root_user.get("nickname").unwrap().as_str().unwrap(),
            "madclaws"
        );
        Ok(())
    }

    #[test]
    fn test_get_root_user_details_missing_key() {
        let config: Table = toml::from_str(
            r#"
                # no root-user table
                [other]
                foo = "bar"
            "#,
        )
        .unwrap();

        let res = get_root_user_details(&config);
        assert!(res.is_err(), "Expected error when root-user key is missing");
    }

    #[test]
    fn test_root_user_new_wrong_types() {
        // id is integer, nickname is table
        let config: Table = toml::from_str(
            r#"
                [root-user]
                id = 123
                nickname = { nested = "value" }
            "#,
        )
        .unwrap();

        let root_tbl = config.get("root-user").unwrap().as_table().unwrap().clone();
        assert!(
            RootUser::new(&root_tbl).is_err(),
            "Expected error for wrong types"
        );
    }

    #[test]
    fn test_root_user_roundtrip_table() -> Result<()> {
        let user = RootUser {
            id: "did:key:abc".into(),
            nickname: "nick".into(),
        };
        let tbl = user.to_table();
        let parsed = RootUser::new(&tbl)?;
        assert_eq!(parsed.id, user.id);
        assert_eq!(parsed.nickname, user.nickname);
        Ok(())
    }

    #[test]
    fn test_set_nickname_but_invalid_config() {
        let config: Table = toml::from_str(
            r#"
                [ruser]
                id = ''
            "#,
        )
        .unwrap();

        assert!(set_nickname(&config, "madclaws").is_err())
    }

    #[test]
    fn test_set_nickname_success() {
        let config: Table = toml::from_str(
            r#"
                [root-user]
                id = 'did:key:xyz'
                nickname = ''
            "#,
        )
        .unwrap();

        let updated = set_nickname(&config, "madclaws").expect("nickname update should succeed");
        assert_eq!(
            updated.get("id").and_then(|v| v.as_str()),
            Some("did:key:xyz")
        );
        assert_eq!(
            updated.get("nickname").and_then(|v| v.as_str()),
            Some("madclaws")
        );
    }

    #[test]
    fn test_set_nickname_with_empty_id_fails() {
        let config: Table = toml::from_str(
            r#"
                [root-user]
                id = ''
                nickname = ''
            "#,
        )
        .unwrap();

        let err = set_nickname(&config, "madclaws").expect_err("empty id should fail");
        assert!(err.to_string().contains("No Root user available"));
    }

    pub fn setup_db_schema() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "
    CREATE TABLE IF NOT EXISTS users (
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
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tokens (
            id TEXT PRIMARY KEY,
            did TEXT NOT NULL,
            cid TEXT NOT NULL,
            token TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            type TEXT NOT NULL
        );",
            [],
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_get_user_when_no_user() {
        let conn = setup_db_schema();
        assert!(get_current_user(&conn).is_err())
    }

    #[test]
    fn test_get_current_user_valid() {
        let conn = setup_db_schema();
        let user = User {
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
        };

        let mut fetch_root_user = conn
            .prepare("select id from users where root = true")
            .unwrap();

        match fetch_root_user.query_one([], |_row| Ok(())) {
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                conn.execute("insert into users (id, user_id, username, active_profile, account_type, root) values
                (?1, ?2, ?3,?4, ?5, ?6)", (&user.id.to_string(), &user.user_id, &user.username, &user.active_profile,
                    user.account_type.to_string(),  &user.root)).unwrap();
            }
            Err(_err) => (),
            _ => (),
        }

        assert!(get_current_user(&conn).is_ok())
    }

    #[test]
    fn test_get_current_user_invalid_account_type_fails() {
        let conn = setup_db_schema();
        conn.execute(
            "insert into users (id, user_id, username, active_profile, account_type, root, created_at, updated_at)
            values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                Uuid::now_v7().to_string(),
                "did:key:test",
                "nickname",
                true,
                "unknown",
                true,
                1_i64,
                1_i64,
            ),
        )
        .unwrap();

        assert!(get_current_user(&conn).is_err());
    }

    #[test]
    fn test_get_current_user_inactive_only_rows_fails() {
        let conn = setup_db_schema();
        conn.execute(
            "insert into users (id, user_id, username, active_profile, account_type, root, created_at, updated_at)
            values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                Uuid::now_v7().to_string(),
                "did:key:test",
                "nickname",
                false,
                "local",
                true,
                1_i64,
                1_i64,
            ),
        )
        .unwrap();

        assert!(get_current_user(&conn).is_err());
    }

    fn create_user(conn: &Connection, account_type: ACCOUNT) -> User {
        let user = User {
            id: Uuid::now_v7().to_string(),
            user_id: String::from("did"),
            username: String::from("nickname"),
            account_type,
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
        };

        conn.execute("insert into users (id, user_id, username, active_profile, account_type, root) values (?1, ?2, ?3,?4, ?5, ?6)", (&user.id.to_string(), &user.user_id, &user.username, &user.active_profile,
        user.account_type.to_string(),  &user.root)).unwrap();
        user
    }

    #[test]
    fn test_list_peers_with_atleast_0_peer() {
        let conn = setup_db_schema();
        let _local_user = create_user(&conn, ACCOUNT::LOCAL);

        let user_list = get_peer_list(&conn).unwrap();

        assert!(user_list.is_empty())
    }

    #[test]
    fn test_list_peers_with_more_than_0_peer() {
        let conn = setup_db_schema();
        let _local_user = create_user(&conn, ACCOUNT::LOCAL);
        save_peer_account_db(&conn, "did:jey:varathan", "varathan").unwrap();
        let user_list = get_peer_list(&conn).unwrap();

        assert!(!user_list.is_empty())
    }

    #[test]
    fn test_unlink_valid_peer() {
        let conn = setup_db_schema();
        let _local_user = create_user(&conn, ACCOUNT::LOCAL);
        save_peer_account_db(&conn, "did:jey:varathan", "varathan").unwrap();
        let user_list = get_peer_list(&conn).unwrap();

        assert!(!user_list.is_empty());

        unlink(&conn, "did:jey:varathan").unwrap();
        let user_list = get_peer_list(&conn).unwrap();
        assert!(user_list.is_empty());
    }

    #[test]
    fn test_try_unlink_local() {
        let conn = setup_db_schema();
        let local_user = create_user(&conn, ACCOUNT::LOCAL);

        assert!(unlink(&conn, &local_user.user_id).is_err())
    }

    #[test]
    fn test_get_user_info() {
        let conn = setup_db_schema();
        let _local_user = create_user(&conn, ACCOUNT::LOCAL);
        save_peer_account_db(&conn, "did:jey:varathan", "varathan").unwrap();
        let user_info = get_user_info(&conn, "did:jey:varathan");
        assert!(user_info.is_ok())
    }

    #[test]
    fn test_valid_add_token() {
        let token = "glhAPHPmeDM0le3YVN4oBkDEg6Yz0lqOIRo5HqkUQbbv3Kdh1jvig7YhpfC9fSO8FXaDP1MZXnz+nnuAT/YfwJ/KAqJhaEg0Ae0B7QETcXN1Y2FuL2RsZ0AxLjAuMC1yYy4xp2NhdWR4IGRpZDpwbGM6bWJrNndnbXhpYXRvdHp5NWIzcTU3bmF3Y2NtZGEvY2V4cBp87SFjY2lzc3g4ZGlkOmtleTp6Nk1rcWtQWVUzZVVTczdQZzROc1NUTmJtOWhLWjRNVTk5N3dLRmJCd3Q5Z0Q1azVjcG9sgGNzdWJ4OGRpZDprZXk6ejZNa3FrUFlVM2VVU3M3UGc0TnNTVE5ibTloS1o0TVU5OTd3S0ZiQnd0OWdENWs1ZW5vbmNlUEHHpLSbdxgpK1QfeHvBxmQ=";

        let db_conn = setup_db_conn_v2();

        let resp = add_token(token, &db_conn);

        assert!(resp.is_ok());

        assert_eq!(resp.unwrap().token, token);
    }

    #[test]
    fn test_valid_add_token_multiple_same_did() {
        let token = "glhAPHPmeDM0le3YVN4oBkDEg6Yz0lqOIRo5HqkUQbbv3Kdh1jvig7YhpfC9fSO8FXaDP1MZXnz+nnuAT/YfwJ/KAqJhaEg0Ae0B7QETcXN1Y2FuL2RsZ0AxLjAuMC1yYy4xp2NhdWR4IGRpZDpwbGM6bWJrNndnbXhpYXRvdHp5NWIzcTU3bmF3Y2NtZGEvY2V4cBp87SFjY2lzc3g4ZGlkOmtleTp6Nk1rcWtQWVUzZVVTczdQZzROc1NUTmJtOWhLWjRNVTk5N3dLRmJCd3Q5Z0Q1azVjcG9sgGNzdWJ4OGRpZDprZXk6ejZNa3FrUFlVM2VVU3M3UGc0TnNTVE5ibTloS1o0TVU5OTd3S0ZiQnd0OWdENWs1ZW5vbmNlUEHHpLSbdxgpK1QfeHvBxmQ=";

        let did = "did:key:z6MkqkPYU3eUSs7Pg4NsSTNbm9hKZ4MU997wKFbBwt9gD5k5";
        let db_conn = setup_db_conn_v2();

        let resp = add_token(token, &db_conn);
        println!("{:?}", resp);
        assert!(resp.is_ok());

        assert_eq!(resp.unwrap().token, token);

        let tokenb = "glhACMCMJFAYFQBP/AwhUuH6A1B5eQWo1EWBg5X8B5CXAyDAb/LhTSM6ndct/N/0rz2K2tdOLkUFAkowwR4sd02zCKJhaEg0Ae0B7QETcXN1Y2FuL2RsZ0AxLjAuMC1yYy4xp2NhdWR4IGRpZDpwbGM6bWJrNndnbXhpYXRvdHp5NWIzcTU3bmF3Y2NtZGEvY2V4cBp87SJYY2lzc3g4ZGlkOmtleTp6Nk1rcWtQWVUzZVVTczdQZzROc1NUTmJtOWhLWjRNVTk5N3dLRmJCd3Q5Z0Q1azVjcG9sgGNzdWJ4OGRpZDprZXk6ejZNa3FrUFlVM2VVU3M3UGc0TnNTVE5ibTloS1o0TVU5OTd3S0ZiQnd0OWdENWs1ZW5vbmNlUFSdn3+p0ErihX4qr3oZZFo=";

        let resp = add_token(tokenb, &db_conn);
        assert_eq!(resp.unwrap().token, tokenb);
        assert_eq!(
            fetch_token(did, &db_conn.common, TokenType::Sync)
                .unwrap()
                .unwrap()
                .token,
            tokenb
        );
    }
    #[test]
    fn test_invalid_token_in_add_token() {
        let token = "glhAPHPmeDM0le3YVN4oBkDEg6Yz0lqOIRo5HqkUQbbv3Kdh1jvig7YhpfC9fSO8FXaDP1MZXnz+nnuAT/YfwJ/KAqJhaEg0Ae0B7QETcXN1Y2FuL2RsZ0AxLjAuMC1yYy4xp2NhdWR4IGRpZDpwbGM6bWJrNndnbXhpYXRvdHp5NWIzcTU3bmF3Y2NtZGEvY2V4cBp87SFjY2lzc3g4ZGlkOmtleTp6Nk1rcWtQWVUzZVVTczdQZzROc1NUTmJtOWhLWjRNVTk5N3dLRmJCd3Q5Z0Q1azVjcG9sgGNzdWJ4OGRpZDprZXk6ejZNa3FrUFlVM2VVU3M3UGc0TnNTVE5ibTloS1o0TVU5OTd3S0ZiQnd0OWdENWs1ZW5vbmNlUEHHpLSbdxgpK1QfeHvBxmQ";

        let db_conn = setup_db_conn_v2();

        let resp = add_token(token, &db_conn);

        assert!(resp.is_err());
    }

    #[tokio::test]
    async fn test_generate_token() {
        let signer = Ed25519Signer::import(&[80; 32]).await.unwrap();
        let db_conn = setup_db_conn_v2();
        assert!(
            generate_delegation_token(
                signer,
                "did:key:z6Mkp1F7iJfUaj8Yp9nBNEvL3pCz42QBHtzaV4JQw3xjn5ww",
                &db_conn
            )
            .await
            .is_ok()
        );
    }

    #[tokio::test]
    async fn test_invocation_verification() {
        let db_conn = setup_db_conn_v2();
        let issued_signer = Ed25519Signer::import(&[80; 32]).await.unwrap();
        let audience_signer = Ed25519Signer::import(&[100; 32]).await.unwrap();
        let token_delegated = generate_delegation_token(
            issued_signer,
            &audience_signer.ed25519_did().to_string(),
            &db_conn,
        )
        .await
        .unwrap();

        let token_delegated_in_bytes = data_encoding::BASE64
            .decode(token_delegated.token.as_bytes())
            .unwrap();
        let delegation: Delegation<Ed25519Signature> =
            serde_ipld_dagcbor::from_slice(&token_delegated_in_bytes).unwrap();

        println!("Delegated token\n{:?}", delegation);
        let invocation_token = generate_invocation_token(audience_signer, &token_delegated.token)
            .await
            .unwrap();

        let invocation_token_bytes = data_encoding::BASE64
            .decode(invocation_token.as_bytes())
            .unwrap();
        let inv: Invocation<Ed25519Signature> =
            serde_ipld_dagcbor::from_slice(&invocation_token_bytes).unwrap();

        println!("Invocation token\n{:?}", inv);

        assert!(
            process_invocation_verification(&invocation_token, db_conn.common)
                .await
                .is_ok()
        );
    }
    pub fn setup_db_conn_v2() -> Dbconn {
        Dbconn {
            chat: crate::core::chats::tests::setup_db_schema(),
            common: setup_db_schema(),
        }
    }
}
