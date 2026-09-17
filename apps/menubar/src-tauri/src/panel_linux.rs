//! the status panel as a plain always-on-top window. the tray cannot say
//! where it sits, so the panel opens in the corner where trays live

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

use crate::sessions;

pub const LABEL: &str = "panel";

const WIDTH: f64 = 380.0;
const MIN_HEIGHT: f64 = 40.0;
const EDGE_MARGIN: f64 = 8.0;
const SCREEN_MARGIN: f64 = 16.0;
const REOPEN_GUARD: Duration = Duration::from_millis(200);

#[derive(Default)]
pub struct PanelState {
    last_hidden: Mutex<Option<Instant>>,
    warming: Mutex<bool>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Dismiss {
    Instant,
    /// no fade here, kept for the shared callers
    Fade,
}

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    app.manage(PanelState::default());
    let window = window(app).expect("the `panel` window is declared in tauri.conf.json");
    window.set_always_on_top(true)?;
    Ok(())
}

/// webkitgtk paints nothing while hidden, so there is nothing to warm
pub fn warm_up(app: &AppHandle) {
    if let Some(state) = app.try_state::<PanelState>() {
        *state.warming.lock().unwrap() = true;
    }
}

fn finish_warm_up(app: &AppHandle) -> bool {
    let Some(state) = app.try_state::<PanelState>() else {
        return false;
    };
    std::mem::replace(&mut *state.warming.lock().unwrap(), false)
}

/// top right of the primary monitor; wayland ignores this and centres
fn place(window: &WebviewWindow) {
    let (Ok(Some(monitor)), Ok(size)) = (window.primary_monitor(), window.outer_size()) else {
        return;
    };
    let scale = monitor.scale_factor();
    let margin = (EDGE_MARGIN * scale) as i32;
    let x = monitor.position().x + monitor.size().width as i32 - size.width as i32 - margin;
    let y = monitor.position().y + margin;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

pub fn show(app: &AppHandle) {
    if let Some(state) = app.try_state::<PanelState>() {
        *state.warming.lock().unwrap() = false;
    }

    let Some(window) = window(app) else {
        return;
    };
    place(&window);
    let _ = window.show();
    let _ = window.set_focus();

    let handle = app.clone();
    tauri::async_runtime::spawn(async move { sessions::refresh(&handle).await });
}

pub fn hide(app: &AppHandle) {
    dismiss(app, Dismiss::Instant);
}

pub fn dismiss(app: &AppHandle, _mode: Dismiss) {
    let Some(window) = window(app) else {
        return;
    };
    if !window.is_visible().unwrap_or(false) {
        return;
    }
    if let Some(state) = app.try_state::<PanelState>() {
        *state.last_hidden.lock().unwrap() = Some(Instant::now());
    }
    let _ = window.hide();
}

pub fn toggle(app: &AppHandle) {
    let Some(window) = window(app) else {
        return;
    };

    let just_hidden = app
        .try_state::<PanelState>()
        .and_then(|state| *state.last_hidden.lock().unwrap())
        .is_some_and(|at| at.elapsed() < REOPEN_GUARD);
    if just_hidden {
        return;
    }

    if window.is_visible().unwrap_or(false) {
        dismiss(app, Dismiss::Instant);
    } else {
        show(app);
    }
}

#[tauri::command]
pub fn hide_panel(app: AppHandle) {
    hide(&app);
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::daemon::quit(&app);
}

#[tauri::command]
pub fn panel_ready(app: AppHandle) {
    finish_warm_up(&app);
}

#[tauri::command]
pub fn resize_panel(app: AppHandle, height: f64) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || set_height(&handle, height));
}

fn set_height(app: &AppHandle, requested: f64) {
    let Some(window) = window(app) else {
        return;
    };

    let ceiling = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|monitor| {
            monitor.work_area().size.height as f64 / monitor.scale_factor() - SCREEN_MARGIN
        })
        .unwrap_or(requested)
        .max(MIN_HEIGHT);
    let height = requested.clamp(MIN_HEIGHT, ceiling);

    let current = window
        .inner_size()
        .ok()
        .zip(window.scale_factor().ok())
        .map(|(size, scale)| size.height as f64 / scale);
    if current.is_some_and(|h| (h - height).abs() < 0.5) {
        return;
    }

    let _ = window.set_size(LogicalSize::new(WIDTH, height));
}
