//! dying with the daemon
//!
//! the daemon hands us a stdin pipe it never writes to and holds the other end
//! for as long as it lives. reading it blocks forever, and the read only ever
//! returns when that end is gone, which covers a kill the daemon never saw
//! coming as much as an orderly shutdown

use std::io::Read;

use tauri::AppHandle;

/// set by the daemon, absent when the app was launched by hand
const SUPERVISED: &str = "TILES_MENUBAR_SUPERVISED";
const SUPERVISED_ARG: &str = "--tiles-daemon-supervised";

pub fn is_supervised() -> bool {
    std::env::var_os(SUPERVISED).is_some()
}

fn should_yield(supervised: bool, args: &[String]) -> bool {
    !supervised && args.iter().any(|arg| arg == SUPERVISED_ARG)
}

/// A daemon-spawned secondary asks a hand-launched primary to release the
/// singleton. The daemon then retries and its supervised child becomes primary.
pub fn should_yield_to(args: &[String]) -> bool {
    should_yield(is_supervised(), args)
}

pub fn init(app: &AppHandle) {
    if !is_supervised() {
        return;
    }

    let app = app.clone();
    std::thread::spawn(move || {
        let mut byte = [0u8; 1];
        // the daemon writes nothing, so anything but a blocking wait is a bug
        // upstream and still means the pipe is no good to us
        let _ = std::io::stdin().read(&mut byte);
        app.exit(0);
    });
}

#[cfg(test)]
mod tests {
    use super::should_yield;

    #[test]
    fn manual_primary_yields_to_daemon_claim() {
        let args = vec!["tiles-menubar".into(), "--tiles-daemon-supervised".into()];
        assert!(should_yield(false, &args));
    }

    #[test]
    fn supervised_or_manual_launches_do_not_displace_the_primary() {
        let claim = vec!["tiles-menubar".into(), "--tiles-daemon-supervised".into()];
        let manual = vec!["tiles-menubar".into()];
        assert!(!should_yield(true, &claim));
        assert!(!should_yield(false, &manual));
    }
}
