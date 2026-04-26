// Module that handles CLI commands

use std::io;

use anyhow::{Result, anyhow};
use owo_colors::OwoColorize;
use tiles::core::account::local::{
    RootUser, create_root_account, get_peer_list, get_root_user_details, save_root_account,
    set_nickname, unlink,
};
use tiles::core::storage::db::Dbconn;
use tiles::runtime::Runtime;
use tiles::utils::config::{
    ConfigProvider, DefaultProvider, get_or_create_config, set_user_data_path,
};
use tiles::utils::installer::{UpdateInfo, get_update_info, try_update};
use tiles::{core::health, runtime::RunArgs};

pub use tilekit::optimize::optimize;
use toml::Table;

use crate::{AccountArgs, AccountCommands};

const FTUE_VERSION_TITLE: &str = "Tiles";
const FTUE_HEADER: &str = "Initializing local account...";
const FTUE_ASCII_ART: &str = r#"
              ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
             ▓▓                      ▓▓░░▓▒
           ▓▓▓▓▓▓▓▓▓▓▓▓▓     ▓▓▓▓▓▓▓▓▓    ▓▓
            ▓▓░░░░░░░▓▓░    ▓▓▓░░░░░▓▓   ▓▓
             ▓▓     ░▓▒    ▓▓ ▓▒     ▒▓░▓▓
              ▓▓▓▓▓▓▓▒    ▓▓   ▓▓▓▓▓▓▓▓▓▓
                   ▓▓    ▓▓   ░▓░
                  ▓▓    ▒▓░   ▓▒
                 ▒▓    ░▓░   ▓▓
                ▒▓    ░▓▒   ▓▓
               ░▓░    ▓▓   ▓▓
              ░▓▒    ▓▓   ▒▓
              ▓▓▓▓▓▓▓▓   ▒▓░
              ░▓▒    ▓▓ ░▓░
                ▓▓    ▓▓▓▒
                 ▓▓▓▓▓▓▓▓
"#;

// const FTUE_ASCII_ART_NEW: &str = r#"
//                       ▃▅▆▆▇▇▇▇▆▇▇▇▆▆▆▆
//                ░▅▆▆▇▆▇▇▇▇▇▇▇▆▆▆▆▇▇▇▇▆
//         _▃▅▇▆▇▇▆▆▆▇▇▆▆▆▆▆▇▇▆▇▇▇▇▇▆▇▅
//     ▃▆▇▆▇▇▇▆▆▆▆▆▅▆▇▆▇▇▇▇▇▆▆▆▆▆▃
//     ▆▆▇▆▆▆▆▆▇▆▆▇▆▇▇▇▆▇▆▆▆▇▅
//  ▂▆▆▇▇▇▇▇▇▇▇▇▆▆▆▇▇▇▇▇▇▇▇▁
//      ▅▆▆▆▇▆▇▆▆▆▆▇▆▆▇▇▆▅
//              ▆▇▇▇▇▇▆▇▇▅
//             ▅▇▇▇▆▇▇▇▆
//            ▆▆▇▇▇▇▇▇▆
//           ▆▇▇▇▇▆▇▇▇
//          ▆▇▇▇▆▆▆▆▇
//         ▆▇▇▇▆▇▇▇▆
//         ▂▇▇▇▆▇▇▆
//          ▆▆▆▇▆▅
//          ▁▆▇▆▅
//           ▓▆▄

// "#;
const FTUE_REASSURANCE_LOCAL: &str = "On-device by default.";
// const FTUE_REASSURANCE_NO_CLOUD: &str = "Online models and identity optional.";
const FTUE_NICKNAME_PROMPT: &str = "Choose a username:";
const FTUE_NICKNAME_REQUIRED: &str = "Username is required. Please enter a username:";
const FTUE_ACCOUNT_CREATED: &str = "✓ Account created";
const FTUE_ACCOUNT_LABEL: &str = "Account";
const FTUE_ACCOUNT_DETAILS_HINT: &str = "View full details:";
const FTUE_ACCOUNT_DETAILS_COMMAND: &str = "tiles account";
const FTUE_DATA_DIR_PROMPT: &str = "Data directory";
const FTUE_DATA_DIR_CHANGE_HINT: &str = "Change data path later:";
const FTUE_DATA_DIR_CHANGE_COMMAND: &str = "tiles data set-path <PATH>";
const FTUE_CUSTOM_DATA_PROMPT: &str = "Use a custom data directory now? [y/N]";
const FTUE_UPDATE_COMMAND: &str = "tiles update";

pub fn run_setup_for_ftue(_run_args: &RunArgs) -> Result<()> {
    // initializes config directory
    let config_provider = DefaultProvider;
    config_provider.get_or_create_config_dir()?;
    config_provider.get_or_create_data_dir()?;

    let root_config = get_or_create_config()?;
    let root_user_details = get_root_user_details(&root_config)?;
    println!("{}", FTUE_ASCII_ART.blue());
    println!("{} {}", FTUE_VERSION_TITLE, env!("CARGO_PKG_VERSION"));
    println!();

    if root_user_details.id.is_empty() {
        println!("{}", FTUE_HEADER);
        println!();
        println!("{}", FTUE_REASSURANCE_LOCAL);
        println!();
        // FTUE
        setup_root_account(root_config.clone())?;
        setup_default_user_data_dir(&config_provider)?
    } else {
        print_runtime_context(&config_provider, &root_user_details)?;
    }

    Ok(())
}

fn print_runtime_context<T: ConfigProvider>(
    config_provider: &T,
    root_user_details: &RootUser,
) -> Result<()> {
    let directory = config_provider
        .get_user_data_dir()
        .map(|path| path.display().to_string())?;

    let nickname = if root_user_details.nickname.is_empty() {
        "Unknown"
    } else {
        root_user_details.nickname.as_str()
    };

    println!("Account:");
    println!("  {} (DID: {})", nickname, root_user_details.id);
    println!("Directory:");
    println!("  {}", directory);
    println!();
    Ok(())
}

fn setup_root_account(root_config: Table) -> Result<()> {
    println!("{}", FTUE_NICKNAME_PROMPT);
    let nickname = read_required_nickname()?;
    let root_user_config = RootUser::new(&create_root_account(&root_config, Some(nickname))?)?;

    save_root_account(root_config, &root_user_config.to_table())?;
    println!();
    println!("{}", FTUE_ACCOUNT_CREATED);
    println!();
    println!("{}", FTUE_ACCOUNT_LABEL);
    println!("  Nickname: {}", root_user_config.nickname);
    println!("  DID: {}", root_user_config.id);
    println!("{}", FTUE_ACCOUNT_DETAILS_HINT);
    println!("  {}", FTUE_ACCOUNT_DETAILS_COMMAND.bright_blue().bold());
    println!();
    Ok(())
}

fn setup_default_user_data_dir<T: ConfigProvider>(config_provider: &T) -> Result<()> {
    let user_data_dir = config_provider.get_user_data_dir()?;
    println!("{}", FTUE_DATA_DIR_PROMPT);
    println!("  {}", user_data_dir.display());
    println!();
    println!("{}", FTUE_DATA_DIR_CHANGE_HINT);
    println!("  {}", FTUE_DATA_DIR_CHANGE_COMMAND.bright_blue().bold());
    println!();
    println!("{}", FTUE_CUSTOM_DATA_PROMPT);

    let stdin = io::stdin();
    let mut input = String::new();
    loop {
        input.clear();
        stdin.read_line(&mut input)?;
        let choice = input.trim().to_lowercase();

        if choice.is_empty() || choice == "n" {
            match set_user_data_path(
                user_data_dir
                    .to_str()
                    .ok_or_else(|| anyhow!("Failed to parse user data dir"))?,
            ) {
                Ok(_msg) => return Ok(()),
                Err(err) => {
                    let error_msg = format!("Error setting user data path due to {:?}", err);
                    println!("{}", error_msg.red());
                    return Err(anyhow::anyhow!("Error setting default user data path"));
                }
            }
        }

        if choice == "y" {
            println!("Enter custom data path:");
            input.clear();
            stdin.read_line(&mut input)?;
            let custom_path = input.trim();
            if custom_path.is_empty() {
                println!("{}", "Path is required. Try again.".red());
                continue;
            }

            match set_user_data_path(custom_path) {
                Ok(msg) => {
                    println!("{}", msg.green());
                    return Ok(());
                }
                Err(err) => {
                    let error_msg = format!("Try again, error setting user data path: {:?}", err);
                    println!("{}", error_msg.red());
                    continue;
                }
            }
        }

        println!(
            "{}",
            "Please enter y or n (or press Enter for default N).".red()
        );
    }
}

fn read_required_nickname() -> Result<String> {
    let stdin = io::stdin();
    let mut input = String::new();
    loop {
        input.clear();
        stdin.read_line(&mut input)?;
        let nickname = input.trim();
        if nickname.is_empty() {
            println!("{}", FTUE_NICKNAME_REQUIRED);
            continue;
        }
        return Ok(nickname.to_owned());
    }
}

pub async fn try_app_update() -> Result<()> {
    // no need to check updates in dev mode
    if cfg!(debug_assertions) {
        return Ok(());
    }
    let update_info: UpdateInfo = get_update_info().await?;
    if update_info.can_update {
        let update_str = format!(
            "Update available {} -> {}\n",
            update_info.current_version, update_info.latest_version
        );

        println!("{}", update_str.yellow());
        println!("You can always update Tiles later via:");
        println!("  {}\n", FTUE_UPDATE_COMMAND.bright_blue().bold());
        println!("{}", "Do you want to update now? (Y/n)".to_string().green());

        let stdin = io::stdin();
        let mut input = String::new();
        stdin.read_line(&mut input)?;
        let clean_input = input.trim();
        if clean_input.to_lowercase() == "y" {
            try_update(Some(update_info)).await?;
        }
    }

    Ok(())
}

pub async fn run(runtime: &Runtime, run_args: RunArgs, db_conn: &Dbconn) -> Result<()> {
    runtime.run(run_args, db_conn).await
}

pub fn set_data(path: &str) {
    match set_user_data_path(path) {
        Ok(msg) => {
            println!("{}", msg.green());
        }
        Err(err) => {
            let error_msg = format!("Error setting memory path due to {:?}", err);
            println!("{}", error_msg.red());
        }
    }
}
pub async fn check_health() -> Result<()> {
    health::check_health().await
}

pub async fn start_server(runtime: &Runtime) {
    let _ = runtime.start_server_daemon().await;
}

pub async fn stop_server(runtime: &Runtime) {
    let _ = runtime.stop_server_daemon().await;
}

/// Runs the account command with the args being passed.
pub fn run_account_commands(account_args: AccountArgs) -> Result<()> {
    let config = get_or_create_config()?;
    let root_user_details = get_root_user_details(&config)?;
    match account_args.command {
        Some(AccountCommands::Create { nickname }) => {
            if !root_user_details.id.is_empty() {
                println!("Local Identity exists with id: {}", root_user_details.id)
            } else {
                let root_user_config = RootUser::new(&create_root_account(&config, nickname)?)?;

                save_root_account(config, &root_user_config.to_table())?;
                println!(
                    "{}",
                    format_args!(
                        "Local Identity has been created with id: {}",
                        root_user_config.id
                    )
                )
            }
        }
        Some(AccountCommands::SetNickname { nickname }) => {
            if root_user_details.id.is_empty() {
                println!("{}", get_account_not_created_msg());
            } else {
                match set_nickname(&config, &nickname) {
                    Ok(root_user_config) => {
                        let id = root_user_config.get("id").unwrap().as_str().unwrap();
                        let nickname = root_user_config.get("nickname").unwrap().as_str().unwrap();
                        save_root_account(config, &root_user_config)?;
                        println!("Nickname {} has been set for ID: {}", nickname, id)
                    }
                    Err(err) => {
                        println!("Failed to set nickname due to {}", err)
                    }
                }
            }
        }
        _ => {
            if root_user_details.id.is_empty() {
                println!("{}", get_account_not_created_msg());
            } else {
                println!("{}", root_user_details);
            }
        }
    }

    Ok(())
}

fn get_account_not_created_msg() -> String {
    format!(
        "Local Identity not created yet, use {}",
        "tiles account create".yellow()
    )
}

pub fn show_peers(db_conn: &Dbconn) -> Result<()> {
    let peers = get_peer_list(&db_conn.common)?;

    println!("DID\tNickname\n");
    for peer in peers {
        println!("{}\t{}", peer.user_id, peer.username)
    }
    Ok(())
}

pub fn unlink_peer(db_conn: &Dbconn, user_id: &str) -> Result<()> {
    if let Err(err) = unlink(&db_conn.common, user_id) {
        println!("{:?}", err)
    } else {
        println!("Succesfully disabled the peer")
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ftue_copy_matches_expected_constants() {
        assert_eq!(FTUE_HEADER, "Initializing local account...");
        assert_eq!(FTUE_REASSURANCE_LOCAL, "On-device by default.");
        assert_eq!(FTUE_NICKNAME_PROMPT, "Choose a username:");
        assert_eq!(FTUE_ACCOUNT_LABEL, "Account");
        assert_eq!(FTUE_ACCOUNT_DETAILS_HINT, "View full details:");
        assert_eq!(FTUE_DATA_DIR_PROMPT, "Data directory");
        assert_eq!(FTUE_DATA_DIR_CHANGE_HINT, "Change data path later:");
        assert_eq!(
            FTUE_CUSTOM_DATA_PROMPT,
            "Use a custom data directory now? [y/N]"
        );
    }

    #[test]
    fn nickname_required_copy_matches_expected_constant() {
        assert_eq!(
            FTUE_NICKNAME_REQUIRED,
            "Username is required. Please enter a username:"
        );
    }
}
