use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::{sessions, tray};
use tauri::{AppHandle, Manager};
use tauri_nspanel::objc2::msg_send;
use tauri_nspanel::objc2::rc::Retained;
use tauri_nspanel::objc2::runtime::AnyObject;
use tauri_nspanel::objc2_app_kit::{NSAnimationContext, NSScreen};
use tauri_nspanel::objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use tauri_nspanel::{
    CollectionBehavior, ManagerExt, PanelHandle, PanelLevel, StyleMask, WebviewWindowExt,
};

// quarantined, the macro dumps objc2 names into whatever module it sits in
mod class {
    use tauri::Manager as _;

    tauri_nspanel::tauri_panel! {
        panel!(TilesPanel {
            config: {
                // losing key is the dismiss signal
                can_become_key_window: true,
                // never main, that reads as the foreground app
                can_become_main_window: false,
                is_floating_panel: true
            }
        })
    }
}
use class::TilesPanel;

pub const LABEL: &str = "panel";

/// must equal `--radius-panel` in tokens.css, both clip the same edge
const CORNER_RADIUS: f64 = 10.0;

const EDGE_MARGIN: f64 = 8.0;

/// the frontend reports a height every frame, and a panel shorter than one
/// row means it measured mid-teardown
const MIN_HEIGHT: f64 = 40.0;

/// kept clear of the dock even when the content would fill the screen
const SCREEN_MARGIN: f64 = 16.0;

/// the icon's own click blurs the panel first, so without this it reopens
/// instead of closing
const REOPEN_GUARD: Duration = Duration::from_millis(200);

const FADE_OUT: Duration = Duration::from_millis(160);

/// cold dev server takes ~1.7s to first frame
const WARMUP_TIMEOUT: Duration = Duration::from_secs(5);

/// measured, an ordered-in window still renders at alpha 0
const WARMUP_ALPHA: f64 = 0.0;

#[derive(Default)]
pub struct PanelState {
    // centre of the status item, in points. x places the panel, y picks the
    // display, which stacked screens share an x range and need
    tray_center: Mutex<Option<(f64, f64)>>,
    last_hidden: Mutex<Option<Instant>>,
    // true while launch warm-up owns visibility
    warming: Mutex<bool>,
    warm_started: Mutex<Option<Instant>>,
    // bumped on show and dismiss, so a late fade cannot hide a reopened panel
    fade: AtomicU64,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Dismiss {
    /// same frame, everything but the case below
    Instant,
    /// status item clicked while open, the one dismissal macOS animates
    Fade,
}

pub fn set_tray_center(app: &AppHandle, x: f64, y: f64) {
    if let Some(state) = app.try_state::<PanelState>() {
        *state.tray_center.lock().unwrap() = Some((x, y));
    }
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    app.manage(PanelState::default());

    let window = app
        .get_webview_window(LABEL)
        .expect("the `panel` window is declared in tauri.conf.json");

    let panel = window.to_panel::<TilesPanel>()?;

    // clear window, else it paints square corners behind the layer mask
    panel.set_transparent(true);

    panel.set_level(PanelLevel::Status.value());

    // nonactivating, else opening the panel greys out the user's app
    panel.set_style_mask(StyleMask::empty().nonactivating_panel().value());

    // transient participates in Expose instead of floating above it
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .can_join_all_spaces()
            .transient()
            .full_screen_auxiliary()
            .value(),
    );

    panel.set_has_shadow(true);
    round_corners(&panel);

    Ok(())
}

/// cornerRadius alone leaves subviews square, masksToBounds collapses them,
/// continuous is the squircle rather than a circular arc
fn round_corners(panel: &PanelHandle<tauri::Wry>) {
    let content_view = panel.content_view();

    unsafe {
        let _: () = msg_send![&*content_view, setWantsLayer: true];

        let layer: Retained<AnyObject> = msg_send![&*content_view, layer];
        let _: () = msg_send![&*layer, setCornerRadius: CORNER_RADIUS];
        let _: () = msg_send![&*layer, setMasksToBounds: true];

        // kCACornerCurveContinuous
        let continuous = NSString::from_str("continuous");
        let _: () = msg_send![&*layer, setCornerCurve: &*continuous];
    }
}

/// an unshown WKWebView has rasterised nothing, so the first orderFront would
/// flash empty. transparent rather than off-screen, AppKit clamps frames
pub fn warm_up(app: &AppHandle) {
    let Ok(panel) = app.get_webview_panel(LABEL) else {
        return;
    };

    if let Some(state) = app.try_state::<PanelState>() {
        *state.warming.lock().unwrap() = true;
        *state.warm_started.lock().unwrap() = Some(Instant::now());
    }

    panel.set_alpha_value(WARMUP_ALPHA);
    // regardless, not key, warm-up must not steal focus at login
    panel.order_front_regardless();

    // in case the frontend never reports a frame
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(WARMUP_TIMEOUT);
        let _ = handle.clone().run_on_main_thread(move || {
            if finish_warm_up(&handle) {
                eprintln!("[panel] warm-up timed out waiting for the first frame");
            }
        });
    });
}

/// idempotent, the ready signal, the timeout and a launch click all race here,
/// returns whether this call ended the warm-up
fn finish_warm_up(app: &AppHandle) -> bool {
    let Some(state) = app.try_state::<PanelState>() else {
        return false;
    };

    let mut warming = state.warming.lock().unwrap();
    if !*warming {
        return false;
    }
    *warming = false;

    // cfg! not #[cfg], keeps the field read in release
    if cfg!(debug_assertions)
        && let Some(started) = *state.warm_started.lock().unwrap()
    {
        eprintln!("[panel] first frame after {:?}", started.elapsed());
    }

    if let Ok(panel) = app.get_webview_panel(LABEL) {
        // hide before restoring alpha, never on screen and opaque at once
        panel.hide();
        panel.set_alpha_value(1.0);
    }

    true
}

/// keyed off the tray point, NSPanel::screen() reports where the window already is
fn visible_frame_for(tray_center: Option<(f64, f64)>) -> Option<NSRect> {
    let mtm = MainThreadMarker::new()?;

    // both axes, else two screens stacked vertically match on x alone and the
    // panel opens on whichever one the list happens to hold first
    if let Some((x, y)) = tray_center {
        let screens = NSScreen::screens(mtm);
        for screen in screens.iter() {
            let frame = screen.frame();
            if x >= frame.origin.x
                && x < frame.origin.x + frame.size.width
                && y >= frame.origin.y
                && y < frame.origin.y + frame.size.height
            {
                return Some(screen.visibleFrame());
            }
        }
    }

    Some(NSScreen::mainScreen(mtm)?.visibleFrame())
}

/// always before ordering in, else the panel shows stale and jumps
fn place(app: &AppHandle) {
    let Ok(panel) = app.get_webview_panel(LABEL) else {
        return;
    };

    let tray_center = app
        .try_state::<PanelState>()
        .and_then(|state| *state.tray_center.lock().unwrap());

    let Some(visible) = visible_frame_for(tray_center) else {
        return;
    };

    let ns_panel = panel.as_panel();
    let frame = ns_panel.frame();
    let size = frame.size;

    let min_x = visible.origin.x + EDGE_MARGIN;
    let max_x = (visible.origin.x + visible.size.width - size.width - EDGE_MARGIN).max(min_x);
    let x = match tray_center {
        Some((center, _)) => (center - size.width / 2.0).clamp(min_x, max_x),
        // no tray rect yet, sit where status items live
        None => max_x,
    };

    // cocoa anchors bottom-left and visibleFrame stops under the menu bar
    let y = visible.origin.y + visible.size.height - size.height;

    ns_panel.setFrame_display(NSRect::new(NSPoint::new(x, y), size), false);
}

pub fn show(app: &AppHandle) {
    // clear first, else a late ready signal or the timeout hides it again
    if let Some(state) = app.try_state::<PanelState>() {
        *state.warming.lock().unwrap() = false;
        // strands a fade in flight
        state.fade.fetch_add(1, Ordering::SeqCst);
    }

    place(app);

    if let Ok(panel) = app.get_webview_panel(LABEL) {
        panel.set_alpha_value(1.0);
        panel.show_and_make_key();
        tray::set_highlighted(app, true);
    }

    // the list is about to be looked at, and this is the only thing that asks
    // for it, see sessions::refresh
    let handle = app.clone();
    tauri::async_runtime::spawn(async move { sessions::refresh(&handle).await });
}

pub fn hide(app: &AppHandle) {
    dismiss(app, Dismiss::Instant);
}

pub fn dismiss(app: &AppHandle, mode: Dismiss) {
    let Ok(panel) = app.get_webview_panel(LABEL) else {
        return;
    };

    // before the visibility check, a window hidden behind our back would leave
    // the highlight stuck on
    tray::set_highlighted(app, false);

    if !panel.is_visible() {
        return;
    }

    let Some(state) = app.try_state::<PanelState>() else {
        panel.hide();
        return;
    };

    // stamped now, not when the fade ends, so the guard covers the animation
    *state.last_hidden.lock().unwrap() = Some(Instant::now());
    let generation = state.fade.fetch_add(1, Ordering::SeqCst) + 1;

    if mode == Dismiss::Instant {
        panel.hide();
        panel.set_alpha_value(1.0);
        return;
    }

    let ns_panel = panel.as_panel();
    NSAnimationContext::beginGrouping();
    NSAnimationContext::currentContext().setDuration(FADE_OUT.as_secs_f64());
    unsafe {
        let animator: Retained<AnyObject> = msg_send![ns_panel, animator];
        let _: () = msg_send![&*animator, setAlphaValue: 0.0f64];
    }
    NSAnimationContext::endGrouping();

    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(FADE_OUT);
        let main = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            let Some(state) = main.try_state::<PanelState>() else {
                return;
            };
            if state.fade.load(Ordering::SeqCst) != generation {
                return;
            }
            if let Ok(panel) = main.get_webview_panel(LABEL) {
                panel.hide();
                panel.set_alpha_value(1.0);
            }
        });
    });
}

pub fn toggle(app: &AppHandle) {
    let Ok(panel) = app.get_webview_panel(LABEL) else {
        return;
    };

    // warming is visible to AppKit but not to the user, so treat it as closed
    let warming = app
        .try_state::<PanelState>()
        .is_some_and(|state| *state.warming.lock().unwrap());

    // before the visibility check, a fade leaves the window visible while it runs
    let just_hidden = app
        .try_state::<PanelState>()
        .and_then(|state| *state.last_hidden.lock().unwrap())
        .is_some_and(|at| at.elapsed() < REOPEN_GUARD);

    if just_hidden {
        return;
    }

    if panel.is_visible() && !warming {
        dismiss(app, Dismiss::Fade);
        return;
    }

    show(app);
}

#[tauri::command]
pub fn hide_panel(app: AppHandle) {
    hide(&app);
}

/// the tray menu's Quit and the footer's are the same ask, and the daemon runs
/// the whole teardown from there
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::daemon::quit(&app);
}

/// frontend reporting it has painted a frame, see [`warm_up`]
#[tauri::command]
pub fn panel_ready(app: AppHandle) {
    finish_warm_up(&app);
}

/// from the frontend's ResizeObserver, one call per animation frame
#[tauri::command]
pub fn resize_panel(app: AppHandle, height: f64) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || set_height(&handle, height));
}

/// cocoa anchors frames bottom-left, so a taller frame set without moving the
/// origin grows upwards and tears the panel off the menu bar
fn set_height(app: &AppHandle, requested: f64) {
    let Ok(panel) = app.get_webview_panel(LABEL) else {
        return;
    };

    let ns_panel = panel.as_panel();
    let frame = ns_panel.frame();

    // visibleFrame stops under the menu bar and above the dock, Tauri's
    // Monitor reports the whole screen
    let ceiling = match ns_panel.screen() {
        Some(screen) => (screen.visibleFrame().size.height - SCREEN_MARGIN).max(MIN_HEIGHT),
        None => requested.max(MIN_HEIGHT),
    };
    let height = requested.clamp(MIN_HEIGHT, ceiling);

    // the observer fires on every frame of the height transition, so equal
    // heights must cost nothing
    if (height - frame.size.height).abs() < 0.5 {
        return;
    }

    let top = frame.origin.y + frame.size.height;
    ns_panel.setFrame_display(
        NSRect::new(
            NSPoint::new(frame.origin.x, top - height),
            NSSize::new(frame.size.width, height),
        ),
        true,
    );
}
