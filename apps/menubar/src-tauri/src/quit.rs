//! quitting from outside the panel
//!
//! `terminate:` asks the delegate first and tao never answers, so the app just
//! exits and the daemon relaunches it. answering hands the quit to the daemon,
//! same as the panel's Quit

use std::ffi::c_char;
use std::sync::OnceLock;

use tauri::AppHandle;
use tauri_nspanel::objc2::ffi::class_addMethod;
use tauri_nspanel::objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
use tauri_nspanel::objc2::{class, msg_send, sel};

use crate::daemon;

const TERMINATE_CANCEL: usize = 0;
const TERMINATE_NOW: usize = 1;

/// `keyAEQuitReason`, set on the quit event by logout, restart and shutdown
const QUIT_REASON: u32 = u32::from_be_bytes(*b"why?");

/// `Q@:@`, an NSUInteger back from self, _cmd and the sender
const SIGNATURE: &[u8] = b"Q@:@\0";

static APP: OnceLock<AppHandle> = OnceLock::new();

/// cancelling these would cancel the logout itself
fn system_is_leaving() -> bool {
    unsafe {
        let manager: *mut AnyObject =
            msg_send![class!(NSAppleEventManager), sharedAppleEventManager];
        let event: *mut AnyObject = msg_send![manager, currentAppleEvent];
        if event.is_null() {
            return false;
        }
        let reason: *mut AnyObject = msg_send![event, attributeDescriptorForKeyword: QUIT_REASON];
        if reason.is_null() {
            return false;
        }
        let code: u32 = msg_send![reason, enumCodeValue];
        code != 0
    }
}

extern "C-unwind" fn should_terminate(_: &AnyObject, _: Sel, _: *mut AnyObject) -> usize {
    if system_is_leaving() {
        return TERMINATE_NOW;
    }
    if let Some(app) = APP.get() {
        daemon::quit(app);
    }
    TERMINATE_CANCEL
}

/// the delegate only exists once the event loop is up
pub fn init(app: &AppHandle) {
    if APP.set(app.clone()).is_err() {
        return;
    }

    let added = unsafe {
        let ns_app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![ns_app, delegate];
        if delegate.is_null() {
            eprintln!("[quit] no app delegate, an outside quit will not be caught");
            return;
        }
        let class: *const AnyClass = msg_send![delegate, class];

        class_addMethod(
            class.cast_mut(),
            sel!(applicationShouldTerminate:),
            std::mem::transmute::<
                extern "C-unwind" fn(&AnyObject, Sel, *mut AnyObject) -> usize,
                Imp,
            >(should_terminate),
            SIGNATURE.as_ptr().cast::<c_char>(),
        )
    };

    if !added.as_bool() {
        eprintln!("[quit] the delegate already answers applicationShouldTerminate:");
    }
}
