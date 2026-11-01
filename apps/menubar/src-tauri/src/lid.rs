//! closed-display mode: sleep turned off outright, through the sudoers grant
//! the pkg installs. `SleepDisabled` outlives the app, so every way out resets it

use std::ffi::{c_char, c_void};
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{LazyLock, OnceLock};

use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::runloop::{
    CFRunLoop, CFRunLoopSource, CFRunLoopSourceRef, kCFRunLoopCommonModes,
};
use core_foundation::string::{CFString, CFStringRef};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::awake;

const SUDO: &str = "/usr/bin/sudo";
const PMSET: &str = "/usr/bin/pmset";

/// exactly what the grant allows, sudoers matches the arguments literally
const fn pmset(on: bool) -> [&'static str; 6] {
    [
        SUDO,
        "-n",
        PMSET,
        "-a",
        "disablesleep",
        if on { "1" } else { "0" },
    ]
}

const RESET: [&str; 6] = pmset(false);

/// there while Tiles has sleep off. the pkg's boot guard looks for this name too
const MARKER: &str = "closed-display";

const PKGUTIL: &str = "/usr/sbin/pkgutil";
const RECEIPT: &str = "com.tilesprivacy.tiles";

/// blocks on a pipe only the app holds the other end of, so the app dying in any
/// way ends the read, and release closing it on purpose takes the same path
const SENTINEL: &str = r#"trap '' HUP INT TERM
marker=$1
shift
while read -r _; do :; done
"$@" && rm -f "$marker""#;

const CLAMSHELL_CHANGED: u32 = 0xE003_4100;

/// kCGDisplayBeginConfigurationFlag, the list is still the old one then
const DISPLAYS_CHANGING: u32 = 1 << 0;

/// commands, the poll and the IOKit callbacks only wake the worker that reconciles
static WAKE: OnceLock<Sender<()>> = OnceLock::new();

type IoObject = u32;
type InterestCallback = extern "C" fn(*mut c_void, IoObject, u32, *mut c_void);

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOServiceMatching(name: *const c_char) -> *const c_void;
    fn IOServiceGetMatchingService(main: u32, matching: *const c_void) -> IoObject;
    fn IORegistryEntryCreateCFProperty(
        entry: IoObject,
        key: CFStringRef,
        allocator: *const c_void,
        options: u32,
    ) -> CFTypeRef;
    fn IONotificationPortCreate(main: u32) -> *mut c_void;
    fn IONotificationPortGetRunLoopSource(port: *mut c_void) -> CFRunLoopSourceRef;
    fn IOServiceAddInterestNotification(
        port: *mut c_void,
        service: IoObject,
        interest: *const c_char,
        callback: InterestCallback,
        context: *mut c_void,
        notification: *mut IoObject,
    ) -> i32;
    fn IOPSNotificationCreateRunLoopSource(
        callback: extern "C" fn(*mut c_void),
        context: *mut c_void,
    ) -> CFRunLoopSourceRef;
}

type DisplayCallback = extern "C" fn(u32, u32, *mut c_void);

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGGetOnlineDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayIsBuiltin(display: u32) -> u32;
    fn CGDisplayRegisterReconfigurationCallback(
        callback: DisplayCallback,
        context: *mut c_void,
    ) -> i32;
}

/// never released, it lives as long as the app does
static ROOT: LazyLock<IoObject> = LazyLock::new(|| unsafe {
    IOServiceGetMatchingService(0, IOServiceMatching(c"IOPMrootDomain".as_ptr()))
});

fn flag(key: &str) -> bool {
    if *ROOT == 0 {
        return false;
    }
    unsafe {
        let value = IORegistryEntryCreateCFProperty(
            *ROOT,
            CFString::new(key).as_concrete_TypeRef(),
            std::ptr::null(),
            0,
        );
        if value.is_null() {
            return false;
        }
        CFType::wrap_under_create_rule(value)
            .downcast::<CFBoolean>()
            .is_some_and(bool::from)
    }
}

fn disabled() -> bool {
    flag("SleepDisabled")
}

pub fn closed() -> bool {
    flag("AppleClamshellState")
}

/// connected counts, asleep or mirrored too, the same as clamshell mode goes by
pub fn external() -> bool {
    let mut displays = [0; 16];
    let mut count = 0;
    unsafe {
        CGGetOnlineDisplayList(displays.len() as u32, displays.as_mut_ptr(), &mut count) == 0
            && displays[..count as usize]
                .iter()
                .any(|&display| CGDisplayIsBuiltin(display) == 0)
    }
}

fn quiet(command: &mut Command) -> &mut Command {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
}

/// false is a missing grant: a dev build, or a Mac that never ran the pkg
fn set(on: bool) -> bool {
    let [program, args @ ..] = pmset(on);
    quiet(Command::new(program).args(args))
        .status()
        .is_ok_and(|status| status.success())
}

fn marker(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|dir| dir.join(MARKER))
}

/// the grant only ships with the pkg, so no other install offers the mode
pub fn offered() -> bool {
    quiet(Command::new(PKGUTIL).args(["--pkg-info", RECEIPT]))
        .status()
        .is_ok_and(|status| status.success())
}

fn sentinel(marker: &Path, reset: &[&str]) -> Command {
    let mut command = Command::new("/bin/sh");
    command
        .args(["-c", SENTINEL, "tiles-lid"])
        .arg(marker)
        .args(reset)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::piped())
        // out of the app's group, so a signal meant for the app does not take it too
        .process_group(0);
    command
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Lid {
    Held,
    /// no grant, the lid still sleeps the Mac
    Refused,
    /// sleep came back on from outside, and the hold stepped back
    Cleared,
}

pub struct Hold {
    marker: Option<PathBuf>,
    /// this hold turned it on, so it turns it off. another app's setting stays put
    owned: bool,
    /// set once, so a refusal is not retried and an outside change is not fought
    state: Option<Lid>,
    /// dropping it closes the pipe, which resets sleep as surely as a crash
    sentinel: Option<Child>,
}

pub fn hold(app: &AppHandle) -> Hold {
    Hold {
        marker: marker(app),
        owned: false,
        state: None,
        sentinel: None,
    }
}

/// marker and sentinel go in before the setting does, so there is no moment
/// it is on with nothing to turn it off
fn arm(hold: &mut Hold) -> bool {
    let Some(marker) = &hold.marker else {
        return false;
    };

    if !marker.exists() {
        if let Some(dir) = marker.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if fs::write(marker, "").is_err() {
            return false;
        }
    }

    if let Some(child) = hold.sentinel.as_mut()
        && matches!(child.try_wait(), Ok(None))
    {
        return true;
    }
    hold.sentinel = sentinel(marker, &RESET).spawn().ok();
    hold.sentinel.is_some()
}

/// for a sentinel that never had anything to reset
fn disarm(hold: &mut Hold) {
    if let Some(mut child) = hold.sentinel.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub fn keep(hold: &mut Hold) -> Lid {
    let lid = match hold.state {
        None => take(hold),
        Some(Lid::Held) if !disabled() => {
            settle(hold);
            Lid::Cleared
        }
        Some(Lid::Held) => {
            if hold.owned {
                arm(hold);
            }
            Lid::Held
        }
        Some(lid) => lid,
    };
    hold.state = Some(lid);
    lid
}

fn take(hold: &mut Hold) -> Lid {
    if disabled() {
        return Lid::Held;
    }
    if arm(hold) && set(true) {
        hold.owned = true;
        return Lid::Held;
    }

    disarm(hold);
    if let Some(marker) = &hold.marker {
        let _ = fs::remove_file(marker);
    }
    Lid::Refused
}

/// waits for the reset, so a hold that follows cannot be undone by this one
fn settle(hold: &mut Hold) {
    if !hold.owned {
        return;
    }
    hold.owned = false;
    if let Some(mut child) = hold.sentinel.take() {
        drop(child.stdin.take());
        if child.wait().is_ok_and(|status| status.success()) {
            return;
        }
    }
    // a sentinel that died early never reset. a failed reset keeps the marker
    // for recover and the boot guard
    if set(false)
        && let Some(marker) = &hold.marker
    {
        let _ = fs::remove_file(marker);
    }
}

/// with sleep off the built-in screen can stay lit under the lid
pub fn blank() {
    let _ = quiet(Command::new(PMSET).arg("displaysleepnow")).status();
}

pub fn release(mut hold: Hold) {
    settle(&mut hold);
    disarm(&mut hold);
}

/// a marker at launch means the last sentinel went down with the app
pub fn recover(app: &AppHandle) {
    let Some(marker) = marker(app) else {
        return;
    };
    if !marker.exists() {
        return;
    }
    if !disabled() || set(false) {
        let _ = fs::remove_file(&marker);
    }
}

/// queues a reconcile, sudo there is nobody's to wait on
pub fn wake() {
    if let Some(wake) = WAKE.get() {
        let _ = wake.send(());
    }
}

extern "C" fn on_root(_: *mut c_void, _: IoObject, message: u32, _: *mut c_void) {
    if message == CLAMSHELL_CHANGED {
        wake();
    }
}

extern "C" fn on_power(_: *mut c_void) {
    wake();
}

extern "C" fn on_displays(_: u32, flags: u32, _: *mut c_void) {
    if flags & DISPLAYS_CHANGING == 0 {
        wake();
    }
}

/// the poll is seconds apart, and a lid shut on a cleared setting sleeps in less
pub fn watch(app: &AppHandle) {
    let (sender, events) = mpsc::channel();
    if WAKE.set(sender).is_err() {
        return;
    }
    let app = app.clone();
    // the one place reconcile runs, off the main thread, and a burst is one pass
    std::thread::spawn(move || {
        while events.recv().is_ok() {
            while events.try_recv().is_ok() {}
            awake::reconcile(&app);
        }
    });

    let context = std::ptr::null_mut();
    let main = CFRunLoop::get_main();

    unsafe {
        let port = IONotificationPortCreate(0);
        let mut notification = 0;
        if !port.is_null()
            && *ROOT != 0
            && IOServiceAddInterestNotification(
                port,
                *ROOT,
                c"IOGeneralInterest".as_ptr(),
                on_root,
                context,
                &mut notification,
            ) == 0
        {
            let source =
                CFRunLoopSource::wrap_under_get_rule(IONotificationPortGetRunLoopSource(port));
            main.add_source(&source, kCFRunLoopCommonModes);
        } else {
            eprintln!("[lid] no clamshell notifications");
        }

        let power = IOPSNotificationCreateRunLoopSource(on_power, context);
        if power.is_null() {
            eprintln!("[lid] no power source notifications");
        } else {
            let source = CFRunLoopSource::wrap_under_create_rule(power);
            main.add_source(&source, kCFRunLoopCommonModes);
        }

        // an external going away under a shut lid is a lid to blank now
        if CGDisplayRegisterReconfigurationCallback(on_displays, context) != 0 {
            eprintln!("[lid] no display notifications");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::{pmset, sentinel};

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tiles-lid-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(flag: &Path) -> [String; 2] {
        ["/usr/bin/touch".into(), flag.display().to_string()]
    }

    fn run(marker: &Path, flag: &Path) -> std::process::Child {
        let reset = touch(flag);
        let reset: Vec<&str> = reset.iter().map(String::as_str).collect();
        sentinel(marker, &reset).spawn().unwrap()
    }

    fn settle(child: &mut std::process::Child) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while child.try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "sentinel never finished");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn the_grant_allows_exactly_what_set_runs() {
        let postinstall = include_str!("../../../../pkg/scripts/postinstall");
        for on in [true, false] {
            let [_, _, command @ ..] = pmset(on);
            assert!(postinstall.contains(&command.join(" ")), "{command:?}");
        }
    }

    #[test]
    fn the_pipe_closing_resets_and_clears_the_marker() {
        let dir = scratch("reset");
        let marker = dir.join("closed-display");
        let flag = dir.join("reset");
        fs::write(&marker, "").unwrap();

        let mut child = run(&marker, &flag);
        std::thread::sleep(Duration::from_millis(100));
        assert!(!flag.exists(), "it fired before the pipe closed");

        drop(child.stdin.take());
        settle(&mut child);
        assert!(flag.exists());
        assert!(!marker.exists());
    }

    #[test]
    fn terminating_it_does_not_skip_the_reset() {
        let dir = scratch("term");
        let marker = dir.join("closed-display");
        let flag = dir.join("reset");
        fs::write(&marker, "").unwrap();

        let mut child = run(&marker, &flag);
        std::thread::sleep(Duration::from_millis(100));
        let _ = std::process::Command::new("/bin/kill")
            .args(["-TERM", &child.id().to_string()])
            .status();
        std::thread::sleep(Duration::from_millis(100));
        assert!(child.try_wait().unwrap().is_none(), "TERM took it down");

        drop(child.stdin.take());
        settle(&mut child);
        assert!(flag.exists());
    }
}
