//! quitting from the dock icon and the app menu
//!
//! neither route reaches Tauri's ExitRequested. AppKit's `terminate:` asks the
//! delegate for permission first, and tao never implements
//! applicationShouldTerminate:, so the default answer stands and the process
//! ends on the spot. the daemon supervising us then sees a healthy child exit
//! and starts another one, which is why quitting never stuck. adding the method
//! to the live delegate is the veto AppKit was looking for

use std::ffi::c_char;
use std::sync::OnceLock;

use tauri::AppHandle;
use tauri_nspanel::objc2::ffi::class_addMethod;
use tauri_nspanel::objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
use tauri_nspanel::objc2::{class, msg_send, sel};

use crate::daemon;

/// NSTerminateCancel. quitting is the daemon's to do, and it takes us down
/// through the lifeline once it goes
const CANCEL: usize = 0;

/// `Q@:@`, an NSUInteger back from the usual self and _cmd plus the sender
const SIGNATURE: &[u8] = b"Q@:@\0";

/// a C callback has nowhere to carry one
static APP: OnceLock<AppHandle> = OnceLock::new();

extern "C-unwind" fn should_terminate(_: &AnyObject, _: Sel, _: *mut AnyObject) -> usize {
    if let Some(app) = APP.get() {
        daemon::quit(app);
    }

    CANCEL
}

/// call once the event loop is up, the delegate does not exist before that
pub fn init(app: &AppHandle) {
    if APP.set(app.clone()).is_err() {
        return;
    }

    let added = unsafe {
        let ns_app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![ns_app, delegate];

        if delegate.is_null() {
            eprintln!("[quit] no app delegate, the dock's quit will not be caught");
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
