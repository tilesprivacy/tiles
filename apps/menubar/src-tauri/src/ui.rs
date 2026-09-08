//! the chat window
//!
//! the panel wants the app out of the dock and the chat window wants it in
//! there to take focus, so the activation policy follows whichever is up

use tauri::{
    ActivationPolicy, AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::panel;

pub const LABEL: &str = "ui";

const WIDTH: f64 = 1100.0;
const HEIGHT: f64 = 760.0;
const MIN_WIDTH: f64 = 640.0;
const MIN_HEIGHT: f64 = 480.0;

/// dev points at the vite server, which keeps the page on its own origin and
/// out of the panel's csp. a bundled build has to carry the built ui instead
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
        Ok(WebviewUrl::App(format!("ui{path}").into()))
    }
}

fn build(app: &AppHandle, path: &str) -> Result<WebviewWindow, String> {
    WebviewWindowBuilder::new(app, LABEL, url(path)?)
        .title("Tiles")
        .inner_size(WIDTH, HEIGHT)
        .min_inner_size(MIN_WIDTH, MIN_HEIGHT)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())
}

pub fn open(app: &AppHandle, path: &str) -> Result<(), String> {
    panel::dismiss(app, panel::Dismiss::Instant);

    let window = match app.get_webview_window(LABEL) {
        // reuse means the click lands on the conversation, not wherever it was left
        Some(window) => {
            if let WebviewUrl::External(url) = url(path)? {
                window.navigate(url).map_err(|e| e.to_string())?;
            }
            window
        }
        None => build(app, path)?,
    };

    let _ = app.set_activation_policy(ActivationPolicy::Regular);

    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}

/// only the panel is left, so the app goes back to being invisible to the dock
pub fn on_closed(app: &AppHandle) {
    let _ = app.set_activation_policy(ActivationPolicy::Accessory);
}

#[tauri::command]
pub fn open_session(app: AppHandle, id: String) -> Result<(), String> {
    open(&app, &format!("/chat/{id}"))
}

#[tauri::command]
pub fn open_ui(app: AppHandle) -> Result<(), String> {
    open(&app, "/")
}
