//! closed-display mode: sleep turned off outright, through the sudoers grant
//! the pkg installs. `SleepDisabled` outlives us, so every way out resets it

use std::ffi::{c_char, c_void};
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::LazyLock;

use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::runloop::{
    CFRunLoop, CFRunLoopSource, CFRunLoopSourceRef, kCFRunLoopCommonModes,
};
use core_foundation::string::{CFString, CFStringRef};
use tauri::{AppHandle, Manager};

use crate::awake;

const SUDO: &str = "/usr/bin/sudo";
const PMSET: &str = "/usr/bin/pmset";

/// exactly what the grant allows, sudoers matches the arguments literally
const RESET: [&str; 6] = [SUDO, "-n", PMSET, "-a", "disablesleep", "0"];

/// our pid while we hold it. the pkg's boot guard looks for this name too
const MARKER: &str = "closed-display";

/// waits on a pipe only we hold the other end of, so any death of ours ends
/// the read. a newer holder's marker means the reset is no longer ours to do
const SENTINEL: &str = r#"trap '' HUP INT TERM
pid=$1 marker=$2
shift 2
while read -r _; do :; done
[ "$(cat "$marker" 2>/dev/null)" = "$pid" ] || exit 0
"$@" && rm -f "$marker""#;

const CLAMSHELL_CHANGED: u32 = 0xE003_4100;

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

const EPERM: i32 = 1;

unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

/// never released, it lives as long as we do
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

fn quiet(command: &mut Command) -> &mut Command {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
}

/// false is a missing grant: a dev build, or a mac that never ran the pkg
fn set(on: bool) -> bool {
    let mut command = Command::new(SUDO);
    command.args([
        "-n",
        PMSET,
        "-a",
        "disablesleep",
        if on { "1" } else { "0" },
    ]);
    quiet(&mut command)
        .status()
        .is_ok_and(|status| status.success())
}

fn owner(marker: &str) -> Option<u32> {
    marker.trim().parse().ok()
}

fn alive(pid: u32) -> bool {
    let Ok(pid) = i32::try_from(pid) else {
        return false;
    };
    let found = unsafe { kill(pid, 0) } == 0;
    found || std::io::Error::last_os_error().raw_os_error() == Some(EPERM)
}

fn marker(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|dir| dir.join(MARKER))
}

fn sentinel(marker: &Path, reset: &[&str]) -> Command {
    let mut command = Command::new("/bin/sh");
    command
        .args(["-c", SENTINEL, "tiles-lid"])
        .arg(std::process::id().to_string())
        .arg(marker)
        .args(reset)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::piped())
        // out of our group, a signal meant for us must not take it too
        .process_group(0);
    command
}

pub struct Hold {
    marker: Option<PathBuf>,
    /// we turned it on, so it is ours to turn off. someone else's stays put
    owned: bool,
    /// sudo said no once, asking every tick would only fill the log
    refused: bool,
    /// dropping it closes the pipe, which resets sleep as surely as a crash
    sentinel: Option<Child>,
}

pub fn hold(app: &AppHandle) -> Hold {
    Hold {
        marker: marker(app),
        owned: false,
        refused: false,
        sentinel: None,
    }
}

/// marker and sentinel go in before the setting does, so there is no moment
/// it is on with nothing to turn it off
fn arm(hold: &mut Hold) -> bool {
    let Some(marker) = &hold.marker else {
        return false;
    };

    let pid = std::process::id().to_string();
    if fs::read_to_string(marker).ok().as_deref() != Some(pid.as_str()) {
        if let Some(dir) = marker.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if fs::write(marker, &pid).is_err() {
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

fn disarm(hold: &mut Hold) {
    // killed before its pipe closes, or it races us to the reset
    if let Some(mut child) = hold.sentinel.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// every tick: macos clears the setting on power and display changes
pub fn keep(hold: &mut Hold) -> Option<bool> {
    if disabled() {
        if hold.owned {
            arm(hold);
        }
        return Some(true);
    }
    if hold.refused {
        return Some(false);
    }

    if arm(hold) && set(true) {
        hold.owned = true;
        return Some(true);
    }

    disarm(hold);
    if let Some(marker) = &hold.marker {
        let _ = fs::remove_file(marker);
    }
    hold.refused = true;
    Some(false)
}

pub fn release(mut hold: Hold) {
    disarm(&mut hold);
    if !hold.owned {
        return;
    }
    // a failed reset keeps the marker for recover and the boot guard
    if set(false)
        && let Some(marker) = &hold.marker
    {
        let _ = fs::remove_file(marker);
    }
}

/// for a holder whose sentinel went down with it
pub fn recover(app: &AppHandle) {
    let Some(marker) = marker(app) else {
        return;
    };
    let Ok(contents) = fs::read_to_string(&marker) else {
        return;
    };
    if let Some(pid) = owner(&contents)
        && pid != std::process::id()
        && alive(pid)
    {
        return;
    }

    if !disabled() || set(false) {
        let _ = fs::remove_file(&marker);
    }
}

fn app_from(context: *mut c_void) -> AppHandle {
    unsafe { &*(context as *const AppHandle) }.clone()
}

extern "C" fn on_root(
    context: *mut c_void,
    _service: IoObject,
    message: u32,
    _argument: *mut c_void,
) {
    if message != CLAMSHELL_CHANGED {
        return;
    }
    let app = app_from(context);
    // off the main thread, sudo is not instant
    std::thread::spawn(move || awake::reconcile(&app));
}

extern "C" fn on_power(context: *mut c_void) {
    let app = app_from(context);
    std::thread::spawn(move || awake::reconcile(&app));
}

/// the poll is seconds apart, and a lid shut on a cleared setting sleeps in less
pub fn watch(app: &AppHandle) {
    let context = Box::into_raw(Box::new(app.clone())).cast::<c_void>();
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
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::{owner, sentinel};

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
    fn the_pipe_closing_resets_and_clears_the_marker() {
        let dir = scratch("reset");
        let marker = dir.join("closed-display");
        let flag = dir.join("reset");
        fs::write(&marker, std::process::id().to_string()).unwrap();

        let mut child = run(&marker, &flag);
        std::thread::sleep(Duration::from_millis(100));
        assert!(!flag.exists(), "it fired before the pipe closed");

        drop(child.stdin.take());
        settle(&mut child);
        assert!(flag.exists());
        assert!(!marker.exists());
    }

    #[test]
    fn a_newer_holder_keeps_its_setting() {
        let dir = scratch("newer");
        let marker = dir.join("closed-display");
        let flag = dir.join("reset");
        fs::write(&marker, "1").unwrap();

        let mut child = run(&marker, &flag);
        drop(child.stdin.take());
        settle(&mut child);
        assert!(!flag.exists());
        assert!(marker.exists());
    }

    #[test]
    fn terminating_it_does_not_skip_the_reset() {
        let dir = scratch("term");
        let marker = dir.join("closed-display");
        let flag = dir.join("reset");
        fs::write(&marker, std::process::id().to_string()).unwrap();

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

    #[test]
    fn a_marker_names_its_holder() {
        assert_eq!(owner("4242"), Some(4242));
        assert_eq!(owner("4242\n"), Some(4242));
        assert_eq!(owner(""), None);
        assert_eq!(owner("tiles"), None);
    }
}
