// Module that handles CLI commands

use std::io;

use anyhow::{Result, anyhow};
use owo_colors::OwoColorize;
use tiles::core::accounts::{
    RootUser, create_root_account, get_peer_list, get_root_user_details, save_root_account,
    set_nickname, unlink,
};
use tiles::core::license::{
    activate_license, deactivate_license, get_active_license, get_license_details,
    get_license_status_string, purge_local_license, validate_license, ProductType,
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

pub fn run_setup_for_ftue(_run_args: &RunArgs, db_conn: &Dbconn) -> Result<()> {
    // initializes config directory
    let config_provider = DefaultProvider;
    config_provider.get_or_create_config_dir()?;
    config_provider.get_or_create_data_dir()?;

    let root_config = get_or_create_config()?;
    let root_user_details = get_root_user_details(&root_config)?;

    // Get license status
    let license_status = get_license_status_string(&db_conn.common);

    println!("{}", FTUE_ASCII_ART.blue());
    println!("{} {} {}", FTUE_VERSION_TITLE, env!("CARGO_PKG_VERSION"), license_status);
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

/// Activates a license key
pub async fn activate_license_cmd(license_key: &str, db_conn: &Dbconn) -> Result<()> {
    println!("Activating license...");

    match activate_license(license_key, &db_conn.common).await {
        Ok(license_info) => {
            println!("{}", "✓ License activated successfully!".green());
            println!();
            println!("License Details:");
            println!("  Type: {:?}", license_info.product_type);
            println!("  Status: {:?}", license_info.status);
            if let Some(expires_at) = license_info.expires_at {
                println!("  Expires: {}", expires_at.format("%Y-%m-%d %H:%M:%S UTC"));
            } else {
                println!("  Expires: Never (Lifetime)");
            }
        }
        Err(e) => {
            println!("{}", format!("✗ License activation failed: {}", e).red());
            println!();
            println!("Common issues:");
            println!("  - Invalid license key format");
            println!("  - License already activated on 5 devices (deactivate one first)");
            println!("  - License has been revoked");
            println!("  - Network connectivity issues");
        }
    }

    Ok(())
}

/// Deactivates the current license
pub async fn deactivate_license_cmd(db_conn: &Dbconn) -> Result<()> {
    let license = get_active_license(&db_conn.common)?;

    match license {
        Some(license_info) => {
            println!("Deactivating license...");

            // Validate before deactivating
            match validate_license(&license_info).await {
                Ok(true) => {
                    match deactivate_license(&license_info, &db_conn.common).await {
                        Ok(_) => {
                            println!("{}", "✓ License deactivated successfully!".green());
                            println!();
                            println!("This device is no longer using a license activation.");
                            println!("You can activate it again on this or another device using:");
                            println!("  {}", "tiles license activate <license-key>".bright_blue().bold());
                        }
                        Err(e) => {
                            println!("{}", format!("✗ License deactivation failed: {}", e).red());
                        }
                    }
                }
                Ok(false) => {
                    println!("{}", "✗ License is no longer valid or has been revoked".red());
                    println!();
                    println!("Removing local license data...");
                    purge_local_license(&db_conn.common, &license_info)?;
                    println!("{}", "✓ Local license data removed".green());
                }
                Err(e) => {
                    println!("{}", format!("✗ Error validating license: {}", e).red());
                    println!();
                    println!("Do you want to force remove the local license? (y/N)");
                    let stdin = io::stdin();
                    let mut input = String::new();
                    stdin.read_line(&mut input)?;
                    if input.trim().to_lowercase() == "y" {
                        // Try a clean deactivation first so the Polar slot is freed.
                        match deactivate_license(&license_info, &db_conn.common).await {
                            Ok(_) => {
                                println!("{}", "✓ License deactivated and local data removed".green());
                            }
                            Err(deactivate_err) => {
                                // Still offline / Polar unreachable — warn before purging locally.
                                println!();
                                println!("{}", "⚠ Could not reach Polar to free the activation slot:".yellow());
                                println!("  {}", deactivate_err.to_string().yellow());
                                println!();
                                println!("Your license has a limited number of activations. If you");
                                println!("remove local data now without deactivating, this device will");
                                println!("still count against that limit.");
                                println!();
                                println!("Free the slot manually at:");
                                println!("  {}", "https://polar.sh/tilesprivacy/portal/".bright_blue().underline());
                                println!();
                                println!("Remove local data anyway? (y/N)");
                                let mut confirm = String::new();
                                io::stdin().read_line(&mut confirm)?;
                                if confirm.trim().to_lowercase() == "y" {
                                    purge_local_license(&db_conn.common, &license_info)?;
                                    println!("{}", "✓ Local license data removed".green());
                                    println!("{}", "  Remember to free the activation slot via the portal.".yellow());
                                }
                            }
                        }
                    }
                }
            }
        }
        None => {
            println!("{}", "No active license found on this device.".yellow());
            println!();
            println!("To activate a license, use:");
            println!("  {}", "tiles license activate <license-key>".bright_blue().bold());
        }
    }

    Ok(())
}

/// Shows detailed license status
pub async fn show_license_status(db_conn: &Dbconn) -> Result<()> {
    let license = get_active_license(&db_conn.common)?;

    match license {
        Some(license_info) => {
            println!("License Status");
            println!("═══════════════════════════════════════════════");
            println!();

            // Get detailed info from Polar API
            match get_license_details(&license_info).await {
                Ok(details) => {
                    // License Type
                    let license_type = match license_info.product_type {
                        ProductType::Backer => "Backer License (Lifetime)",
                        ProductType::Commercial => "Commercial License (Annual Subscription)",
                    };
                    println!("License Type:      {}", license_type.green());

                    // Status
                    let status_display = match details.status.as_str() {
                        "granted" => "Active".green().to_string(),
                        "revoked" => "Revoked".red().to_string(),
                        "disabled" => "Disabled".red().to_string(),
                        _ => details.status.yellow().to_string(),
                    };
                    println!("Status:            {}", status_display);
                    println!();

                    // Expiration / Days Remaining
                    if let Some(expires_at_str) = &details.expires_at {
                        if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(expires_at_str) {
                            let expires_at_utc = expires_at.with_timezone(&chrono::Utc);
                            let now = chrono::Utc::now();

                            if expires_at_utc > now {
                                let duration = expires_at_utc.signed_duration_since(now);
                                let days_remaining = duration.num_days();

                                println!("Expires:           {}", expires_at_utc.format("%Y-%m-%d %H:%M:%S UTC"));

                                let days_display = if days_remaining > 30 {
                                    format!("{} days", days_remaining).green().to_string()
                                } else if days_remaining > 7 {
                                    format!("{} days", days_remaining).yellow().to_string()
                                } else {
                                    format!("{} days", days_remaining).red().to_string()
                                };
                                println!("Days Remaining:    {}", days_display);
                            } else {
                                println!("Expires:           {} {}", expires_at_utc.format("%Y-%m-%d %H:%M:%S UTC"), "(EXPIRED)".red());
                                println!("Days Remaining:    {}", "0 days".red());
                            }
                        }
                    } else {
                        println!("Expires:           {}", "Never (Lifetime)".green());
                    }
                    println!();

                    // Activations
                    println!("This Device:       {}", "Activated".green());
                    if let Some(limit) = details.limit_activations {
                        println!("Device Limit:      {} devices maximum", limit);
                        println!();
                        println!("{}", "Note:".yellow().bold());
                        println!("To view all active devices and manage activations,");
                        println!("visit the Polar customer portal (link below).");
                    } else {
                        println!("Device Limit:      {}", "Unlimited".green());
                    }
                    println!();
                    println!("Activated On:      {}", license_info.activated_at.format("%Y-%m-%d %H:%M:%S UTC"));
                }
                Err(e) => {
                    println!("{}", "Unable to fetch current license details from Polar".red());
                    println!("{}", format!("Error: {}", e).red());
                    println!();
                    println!("Local License Information:");
                    println!("─────────────────────────────────────────────");

                    let license_type = match license_info.product_type {
                        ProductType::Backer => "Backer License (Lifetime)",
                        ProductType::Commercial => "Commercial License (Annual Subscription)",
                    };
                    println!("License Type:      {}", license_type);
                    println!("Activated On:      {}", license_info.activated_at.format("%Y-%m-%d %H:%M:%S UTC"));

                    if let Some(expires_at) = license_info.expires_at {
                        println!("Expected Expiry:   {}", expires_at.format("%Y-%m-%d %H:%M:%S UTC"));
                    }
                }
            }

            println!();
            println!("Manage License");
            println!("─────────────────────────────────────────────");
            println!("You can manage your license, view all activations, and");
            println!("deactivate devices through the Polar customer portal:");
            println!();
            println!("  {}", "https://polar.sh/tilesprivacy/portal/".bright_blue().underline());
            println!();
            println!("Log in with the email address used to purchase your license.");
        }
        None => {
            println!("{}", "No Active License".yellow());
            println!("═══════════════════════════════════════════════");
            println!();
            println!("This device does not have an active license.");
            println!();
            println!("To activate a license, use:");
            println!("  {}", "tiles license activate <license-key>".bright_blue().bold());
            println!();
            println!("Manage License");
            println!("─────────────────────────────────────────────");
            println!("View your licenses and manage activations at:");
            println!("  {}", "https://polar.sh/tilesprivacy/portal/".bright_blue().underline());
        }
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
