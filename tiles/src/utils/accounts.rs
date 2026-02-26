// Stuff related to account and identity system
use anyhow::{Result, anyhow};
use std::fmt::Display;
use tilekit::accounts::create_identity;
use toml::Table;

use crate::utils::config::save_config;
const ROOT_USER_CONFIG_KEY: &str = "root-user";

const ROOT_PARSE_ERROR: &str = "Failed to parse root user config";
#[allow(dead_code)]
pub struct RootUser {
    pub id: String,
    pub nickname: String,
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
pub fn create_root_account(config: &Table, nickname: Option<String>) -> Result<Table> {
    let root_user = config
        .get(ROOT_USER_CONFIG_KEY)
        .ok_or_else(|| anyhow!("{} doesn't exist", ROOT_USER_CONFIG_KEY))?;
    let root_user_table = root_user
        .as_table()
        .ok_or_else(|| anyhow!(ROOT_PARSE_ERROR))?;
    let root_user_data = RootUser::new(root_user_table)?;
    let did = root_user_data.id;
    if did.is_empty() {
        Ok(create_root_user(root_user_table, nickname)?)
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

fn create_root_user(root_user_config: &Table, nickname: Option<String>) -> Result<Table> {
    let mut root_user_table = root_user_config.clone();
    match create_identity("tiles") {
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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use keyring::{mock, set_default_credential_builder};
    use toml::Table;

    use crate::utils::accounts::{
        RootUser, create_root_account, get_root_user_details, set_nickname,
    };

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

    #[test]
    fn test_create_root_account_but_exists() {
        let config: Table = toml::from_str(
            r#"
                [root-user]
                id = 'did:key:xyz'
                nickname = ''
            "#,
        )
        .unwrap();
        let root_user = create_root_account(&config, None).unwrap();

        assert_eq!(
            root_user.get("id").unwrap().as_str().unwrap(),
            "did:key:xyz"
        );
    }

    #[test]
    fn test_create_root_account_new() {
        set_default_credential_builder(mock::default_credential_builder());
        let config: Table = toml::from_str(
            r#"
                [root-user]
                id = ''
                nickname = ''
            "#,
        )
        .unwrap();
        let root_user = create_root_account(&config, None).unwrap();

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
    }

    #[test]
    fn test_create_root_account_new_w_nickname() {
        set_default_credential_builder(mock::default_credential_builder());
        let config: Table = toml::from_str(
            r#"
                [root-user]
                id = ''
                nickname = ''
            "#,
        )
        .unwrap();
        let root_user = create_root_account(&config, Some(String::from("madclaws"))).unwrap();

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
}
