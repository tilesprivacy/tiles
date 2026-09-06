//! hand-rolled rather than `TrayIconBuilder`: tray-icon highlights in its
//! mouseDown and unhighlights in its mouseUp, which causes a visible flicker
//! native vehaviour is highlight is tied to menu open state

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_nspanel::objc2::rc::Retained;
use tauri_nspanel::objc2::runtime::{AnyObject, Sel};
use tauri_nspanel::objc2::{AnyThread, DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use tauri_nspanel::objc2_app_kit::{
    NSAutoresizingMaskOptions, NSEvent, NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
    NSVariableStatusItemLength, NSView,
};
use tauri_nspanel::objc2_foundation::{
    MainThreadMarker, NSData, NSObjectProtocol, NSPoint, NSSize, NSString,
};

use crate::{daemon, panel};

/// vector, so it stays sharp at any menu bar height and display scale. alpha
/// only, AppKit tints a template image for the current menu bar
static ICON: &[u8] = include_bytes!("../icons/menubar-template.pdf");

/// the mark is wider than it is tall, and a square box would draw it small
/// beside the system's items. 12:11 is the pdf's own box, anything else squashes
const ICON_WIDTH: f64 = 16.0;
const ICON_HEIGHT: f64 = ICON_WIDTH * 11.0 / 12.0;

/// the mark carries the same reading the panel's does, at full strength only
/// while inference is up
const ICON_ALPHA_LIVE: f64 = 1.0;
const ICON_ALPHA_IDLE: f64 = 0.5;

/// gap between the status item and its right-click menu
const MENU_GAP: f64 = 4.0;

const PEEK_HOLD: Duration = Duration::from_millis(150);

// the menu is built once, so the toggles are found by tag to be re-checked

/// main thread only, where AppKit requires it and every caller below runs
struct StatusItem(Retained<NSStatusItem>);
unsafe impl Send for StatusItem {}
unsafe impl Sync for StatusItem {}

struct StatusMenu(Retained<NSMenu>);
unsafe impl Send for StatusMenu {}
unsafe impl Sync for StatusMenu {}

#[derive(Default)]
struct ClickState {
    /// bumped on press and release, so a hold timer that fires late can tell
    press: AtomicU64,
    /// opened by the hold timer, so the release ending that hold closes it
    peeked: AtomicBool,
}

struct TargetIvars {
    app: AppHandle,
}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "TilesStatusTarget"]
    #[ivars = TargetIvars]
    struct StatusTarget;

    unsafe impl NSObjectProtocol for StatusTarget {}

    impl StatusTarget {
        /// nothing visible on press, the highlight belongs to the panel being
        /// open, not to the button being held
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            let app = self.ivars().app.clone();
            let Some(state) = app.try_state::<ClickState>() else {
                return;
            };

            let id = state.press.fetch_add(1, Ordering::SeqCst) + 1;
            state.peeked.store(false, Ordering::SeqCst);
            self.remember_position();

            // held long enough and the panel opens without committing to a click
            let timer_app = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(PEEK_HOLD);
                let main_app = timer_app.clone();
                let _ = timer_app.run_on_main_thread(move || {
                    let Some(state) = main_app.try_state::<ClickState>() else {
                        return;
                    };
                    if state.press.load(Ordering::SeqCst) != id {
                        return;
                    }
                    state.peeked.store(true, Ordering::SeqCst);
                    panel::show(&main_app);
                });
            });
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, _event: &NSEvent) {
            let app = self.ivars().app.clone();
            let Some(state) = app.try_state::<ClickState>() else {
                return;
            };

            // invalidates a hold timer that has not fired
            state.press.fetch_add(1, Ordering::SeqCst);
            self.remember_position();

            if state.peeked.swap(false, Ordering::SeqCst) {
                panel::hide(&app);
            } else {
                panel::toggle(&app);
            }
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, _event: &NSEvent) {
            let app = self.ivars().app.clone();
            let Some(menu) = app.try_state::<StatusMenu>() else {
                return;
            };

            let below = NSPoint::new(0.0, self.bounds().size.height + MENU_GAP);
            menu.0
                .popUpMenuPositioningItem_atLocation_inView(None, below, Some(self));
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            daemon::quit(&self.ivars().app);
        }
    }
);

impl StatusTarget {
    fn new(mtm: MainThreadMarker, app: AppHandle) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TargetIvars { app });
        unsafe { msg_send![super(this), init] }
    }

    /// the item's own window is the only thing that knows where it landed, so
    /// the panel centres off that and not a cached click
    fn remember_position(&self) {
        let Some(window) = self.window() else {
            return;
        };
        let frame = window.frame();
        panel::set_tray_center(
            &self.ivars().app,
            frame.origin.x + frame.size.width / 2.0,
            frame.origin.y + frame.size.height / 2.0,
        );
    }
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let mtm = MainThreadMarker::new().expect("setup runs on the main thread");

    let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    let button = item.button(mtm).expect("a status item always has a button");

    let data = NSData::with_bytes(ICON);
    let image = NSImage::initWithData(NSImage::alloc(), &data).expect("the icon is a valid PDF");
    image.setTemplate(true);
    image.setSize(NSSize::new(ICON_WIDTH, ICON_HEIGHT));
    button.setImage(Some(&image));
    button.setAlphaValue(ICON_ALPHA_IDLE);

    // takes the button's mouse events, so it never runs its own press highlight
    let target = StatusTarget::new(mtm, app.clone());
    target.setFrame(button.bounds());
    target.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    button.addSubview(&target);

    app.manage(ClickState::default());
    app.manage(StatusMenu(build_menu(mtm, &target)));
    app.manage(StatusItem(item));

    Ok(())
}

fn item(
    mtm: MainThreadMarker,
    target: &StatusTarget,
    title: &str,
    action: Sel,
    key: &str,
) -> Retained<NSMenuItem> {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(title),
            Some(action),
            &NSString::from_str(key),
        )
    };
    unsafe { item.setTarget(Some(target)) };
    item
}

fn build_menu(mtm: MainThreadMarker, target: &StatusTarget) -> Retained<NSMenu> {
    let menu = NSMenu::new(mtm);

    // start at login belongs to `tiles service` now, and quit is unconditional
    menu.addItem(&item(mtm, target, "Quit Tiles", sel!(quit:), "q"));
    menu
}

/// the panel loses key before the click that caused it is delivered, so this
/// tells a status item dismissal apart from any other blur
pub fn pointer_over_item(app: &AppHandle) -> bool {
    let (Some(item), Some(mtm)) = (app.try_state::<StatusItem>(), MainThreadMarker::new()) else {
        return false;
    };
    let Some(frame) = item
        .0
        .button(mtm)
        .and_then(|b| b.window())
        .map(|w| w.frame())
    else {
        return false;
    };
    let at = NSEvent::mouseLocation();
    at.x >= frame.origin.x
        && at.x <= frame.origin.x + frame.size.width
        && at.y >= frame.origin.y
        && at.y <= frame.origin.y + frame.size.height
}

/// main thread only, so callers off it go through `run_on_main_thread`
pub fn set_live(app: &AppHandle, live: bool) {
    let (Some(item), Some(mtm)) = (app.try_state::<StatusItem>(), MainThreadMarker::new()) else {
        return;
    };
    if let Some(button) = item.0.button(mtm) {
        button.setAlphaValue(if live {
            ICON_ALPHA_LIVE
        } else {
            ICON_ALPHA_IDLE
        });
    }
}

/// the pill AppKit draws behind an active status item
pub fn set_highlighted(app: &AppHandle, on: bool) {
    let (Some(item), Some(mtm)) = (app.try_state::<StatusItem>(), MainThreadMarker::new()) else {
        return;
    };
    if let Some(button) = item.0.button(mtm) {
        button.highlight(on);
    }
}
