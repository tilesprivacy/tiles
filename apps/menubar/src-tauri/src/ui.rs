//! the chat window

use tauri::utils::config::Color;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

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

/// webkitgtk turns gnome's text scaling into the page's device pixel ratio,
/// so a window has to grow by the same factor to give the page the width it
/// gets on macos
#[cfg(target_os = "linux")]
fn text_scale() -> f64 {
    use gtk::prelude::*;

    gtk::Settings::default()
        .map(|settings| settings.gtk_xft_dpi() as f64 / 1024.0 / 96.0)
        .filter(|scale| *scale > 0.0)
        .unwrap_or(1.0)
}

#[cfg(not(target_os = "linux"))]
fn text_scale() -> f64 {
    1.0
}

fn build(app: &AppHandle, path: &str) -> Result<WebviewWindow, String> {
    let scale = text_scale();
    let window = WebviewWindowBuilder::new(app, LABEL, url(path)?)
        .title("Tiles")
        .inner_size(WIDTH * scale, HEIGHT * scale)
        .min_inner_size(MIN_WIDTH * scale, MIN_HEIGHT * scale)
        .background_color(GROUND)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;

    follow_active_space(&window);
    style_titlebar(&window);

    Ok(window)
}

/// gtk draws the title bar itself on wayland. paint it the page's own ground
/// and drop the title, so it reads as part of the window
#[cfg(target_os = "linux")]
fn style_titlebar(window: &WebviewWindow) {
    use gtk::prelude::*;

    const CSS: &str = "
        .titlebar, headerbar {
            background: #111111;
            color: #d4d4d4;
            border: none;
            box-shadow: none;
            min-height: 38px;
        }
        .titlebar .title, headerbar .title { opacity: 0; }
        .titlebar button, headerbar button {
            background: transparent;
            border: none;
            box-shadow: none;
            color: #d4d4d4;
        }
        .titlebar button:hover, headerbar button:hover { background: rgba(255, 255, 255, 0.08); }
    ";

    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };
    let Some(screen) = WidgetExt::screen(&gtk_window) else {
        return;
    };
    let provider = gtk::CssProvider::new();
    if let Err(err) = provider.load_from_data(CSS.as_bytes()) {
        eprintln!("[ui] title bar css rejected: {err}");
        return;
    }
    gtk::StyleContext::add_provider_for_screen(
        &screen,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

#[cfg(not(target_os = "linux"))]
fn style_titlebar(_window: &WebviewWindow) {}

/// a window belongs to the space it was made on, so without this the user gets
/// dragged to it instead of it coming to them.
///
#[cfg(target_os = "macos")]
fn follow_active_space(window: &WebviewWindow) {
    use tauri_nspanel::objc2::msg_send;
    use tauri_nspanel::objc2::runtime::AnyObject;
    use tauri_nspanel::objc2_app_kit::NSWindowCollectionBehavior;

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

#[cfg(not(target_os = "macos"))]
fn follow_active_space(_window: &WebviewWindow) {}

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

/// the daemon decides whether the window belongs on screen at startup. a hand
/// launch on linux is the ask itself
pub fn init(app: &AppHandle) {
    let hand_launched = cfg!(target_os = "linux") && !crate::lifeline::is_supervised();
    if std::env::var_os(SHOW_UI).is_none() && !hand_launched {
        return;
    }

    if let Err(err) = open(app, "/") {
        eprintln!("[ui] could not open the chat window: {err}");
    }
}

/// the dock icon was clicked. a window that is merely buried comes forward
/// where it was left, only a closed one goes back to the start
#[cfg(target_os = "macos")]
pub fn reopen(app: &AppHandle) {
    let result = match app.get_webview_window(LABEL) {
        Some(window) => window
            .show()
            .and_then(|()| window.set_focus())
            .map_err(|e| e.to_string()),
        None => open(app, "/"),
    };

    if let Err(err) = result {
        eprintln!("[ui] could not reopen the chat window: {err}");
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
