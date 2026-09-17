//! quits arrive as ExitRequested here, which main.rs already hands to the daemon

use tauri::AppHandle;

pub fn init(_app: &AppHandle) {}
