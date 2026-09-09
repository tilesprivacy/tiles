//! the chat window

use tauri::utils::config::Color;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_nspanel::objc2::msg_send;
use tauri_nspanel::objc2::runtime::AnyObject;
use tauri_nspanel::objc2_app_kit::NSWindowCollectionBehavior;

use crate::panel;

pub const LABEL: &str = "ui";

/// set by the daemon when a person started it. launchd starting it at login is
/// not an ask for a window
const SHOW_UI: &str = "TILES_SHOW_UI";

const WIDTH: f64 = 1100.0;
const HEIGHT: f64 = 760.0;
const MIN_WIDTH: f64 = 640.0;
const MIN_HEIGHT: f64 = 480.0;

/// --void, so the window is never white before the page paints
const GROUND: Color = Color(0x11, 0x11, 0x11, 0xff);

/// dev points at the vite server, release at the chat ui bundled into the app.
/// a deep link has no file behind it, but the webview falls back to the root
/// index.html on a miss, which hands the route to the router where it belongs
fn url(path: &str) -> Result<WebviewUrl, String> {
    #[cfg(debug_assertions)]
    {
        let base = std::env::var("TILES_UI_DEV_URL")
            .unwrap_or_else(|_| "http://localhost:5173".to_owned());
        format!("{base}{path}")
            .parse()
            .map(WebviewUrl::External)
            .map_err(|e| format!("bad chat window url: {e}"))
    }

    #[cfg(not(debug_assertions))]
    {
        Ok(WebviewUrl::App(path.trim_start_matches('/').into()))
    }
}

fn build(app: &AppHandle, path: &str) -> Result<WebviewWindow, String> {
    let window = WebviewWindowBuilder::new(app, LABEL, url(path)?)
        .title("Tiles")
        .inner_size(WIDTH, HEIGHT)
        .min_inner_size(MIN_WIDTH, MIN_HEIGHT)
        .background_color(GROUND)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    follow_active_space(&window);

    Ok(window)
}

/// a window belongs to the space it was made on, so without this the user gets
/// dragged to it instead of it coming to them.
///
fn follow_active_space(window: &WebviewWindow) {
    let Ok(ns_window) = window.ns_window() else {
        return;
    };

    unsafe {
        let ns_window = ns_window as *mut AnyObject;
        let behavior = NSWindowCollectionBehavior::MoveToActiveSpace
            | NSWindowCollectionBehavior::FullScreenPrimary;
        let _: () = msg_send![ns_window, setCollectionBehavior: behavior];
    }
}

/// deliberately no app activation here. both setActivationPolicy and
/// activateIgnoringOtherApps send macOS off to whichever space the app counts
/// as its own, which is what was yanking the user across spaces
pub fn open(app: &AppHandle, path: &str) -> Result<(), String> {
    let window = match app.get_webview_window(LABEL) {
        // reuse means the click lands on the conversation, not wherever it was left
        Some(window) => {
            let target = window
                .url()
                .map_err(|e| e.to_string())?
                .join(path)
                .map_err(|e| format!("bad chat window url: {e}"))?;
            window.navigate(target).map_err(|e| e.to_string())?;
            window
        }
        None => build(app, path)?,
    };

    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;

    panel::dismiss(app, panel::Dismiss::Instant);

    Ok(())
}

/// the daemon decides whether the window belongs on screen at startup
pub fn init(app: &AppHandle) {
    if std::env::var_os(SHOW_UI).is_none() {
        return;
    }

    if let Err(err) = open(app, "/") {
        eprintln!("[ui] could not open the chat window: {err}");
    }
}

#[tauri::command]
pub fn open_session(app: AppHandle, id: String) -> Result<(), String> {
    open(&app, &format!("/chat/{id}"))
}

#[tauri::command]
pub fn open_ui(app: AppHandle) -> Result<(), String> {
    open(&app, "/")
}
