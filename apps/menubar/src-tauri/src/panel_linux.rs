//! the status panel is not shown on linux yet. its hidden window stays, it is
//! what keeps the app alive once the chat window closes

use tauri::AppHandle;

pub const LABEL: &str = "panel";

#[derive(Clone, Copy, PartialEq)]
pub enum Dismiss {
    Instant,
    Fade,
}

pub fn init(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

pub fn warm_up(_app: &AppHandle) {}

pub fn hide(_app: &AppHandle) {}

pub fn dismiss(_app: &AppHandle, _mode: Dismiss) {}

#[tauri::command]
pub fn hide_panel(_app: AppHandle) {}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::daemon::quit(&app);
}

#[tauri::command]
pub fn panel_ready(_app: AppHandle) {}

#[tauri::command]
pub fn resize_panel(_app: AppHandle, _height: f64) {}
