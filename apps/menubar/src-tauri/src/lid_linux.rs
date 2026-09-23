//! logind owns the lid, and polkit can refuse a block on it, so linux leaves it alone for now

use tauri::AppHandle;

pub struct Hold;

pub fn hold(_app: &AppHandle) -> Hold {
    Hold
}

/// `None` is no closed-display mode at all, not a failed one
pub fn keep(_hold: &mut Hold) -> Option<bool> {
    None
}

pub fn release(_hold: Hold) {}

pub fn recover(_app: &AppHandle) {}

pub fn watch(_app: &AppHandle) {}
