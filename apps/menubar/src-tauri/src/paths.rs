//! where the daemon keeps the user's data, and handing that folder to Finder

use std::path::PathBuf;

use crate::daemon;
use tauri::AppHandle;
use tauri_nspanel::objc2_app_kit::NSWorkspace;
use tauri_nspanel::objc2_foundation::{NSString, NSURL};

/// `data.path` is blank until the user moves it, and the daemon resolves that
/// blank against its own dirs without publishing the result anywhere. this is
/// the same rule for a released daemon, `$XDG_DATA_HOME` or `~/.local/share`
fn default_dir() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_DATA_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => std::env::home_dir()?.join(".local/share"),
    };

    Some(base.join("tiles/data"))
}

/// asked for on demand rather than polled, only the account view reads it
#[tauri::command]
pub async fn data_dir() -> Result<String, String> {
    let client = reqwest::Client::new();
    let res = client
        .get(daemon::url("/config"))
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if !res.status().is_success() {
        return Err(format!("/config answered {}", res.status()));
    }

    let body = res.text().await.map_err(|err| err.to_string())?;
    let payload: serde_json::Value =
        serde_json::from_str(&body).map_err(|_| "/config is not json".to_owned())?;
    let configured = payload
        .get("data")
        .and_then(|d| d.get("path"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    let dir = if configured.is_empty() {
        default_dir().ok_or_else(|| "no data directory to fall back to".to_owned())?
    } else {
        PathBuf::from(configured)
    };

    dir.into_os_string()
        .into_string()
        .map_err(|_| "the data path is not utf-8".to_owned())
}

/// Finder takes focus, so the panel is gone by the time the window opens
#[tauri::command]
pub fn reveal_path(app: AppHandle, path: String) -> Result<(), String> {
    if !std::path::Path::new(&path).is_dir() {
        return Err(format!("{path} is not there"));
    }

    let url = NSURL::fileURLWithPath(&NSString::from_str(&path));
    let opened = NSWorkspace::sharedWorkspace().openURL(&url);
    // the panel hides on blur, but only once something else takes key
    crate::panel::hide(&app);

    opened
        .then_some(())
        .ok_or_else(|| "Finder refused the folder".to_owned())
}
