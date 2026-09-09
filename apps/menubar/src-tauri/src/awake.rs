//! holding the mac up, for as long as there is something worth staying up for

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

pub const STATE_EVENT: &str = "awake://state";

/// the tool holds the assertion, and `-w` releases it even on a kill we never
/// see coming, which a Drop impl cannot promise
const CAFFEINATE: &str = "/usr/bin/caffeinate";

/// -2.0, an external source with no battery to run down. a desktop reads this
/// too, which is the right answer for a machine that is always plugged in
const UNLIMITED: f64 = -2.0;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPSGetTimeRemainingEstimate() -> f64;
}

fn on_ac() -> bool {
    unsafe { IOPSGetTimeRemainingEstimate() == UNLIMITED }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    /// the assertion is held right now
    pub active: bool,
    /// a session exists but is held, its clock stopped and its assertion down
    pub paused: bool,
    /// unix ms an open ended session counts up from, while it is running
    pub since: Option<u64>,
    /// unix ms a timed session counts down to, while it is running
    pub until: Option<u64>,
    /// the reading in ms while paused, which has no wall clock to sit against.
    /// already whole seconds, so it does not step when the clock stops
    pub frozen: Option<u64>,
    /// on battery none of it can be taken at all, and the plate says so
    pub ac: bool,
}

const IDLE: State = State {
    active: false,
    paused: false,
    since: None,
    until: None,
    frozen: None,
    ac: false,
};

/// a run of the clock, which pausing banks and resuming starts again
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Session {
    /// total length in ms, `None` being the open ended one
    length: Option<u64>,
    /// ms banked by earlier runs, which is the whole clock while paused
    elapsed: u64,
    /// unix ms the run in progress began, `None` while paused
    started: Option<u64>,
}

impl Session {
    fn running(&self) -> bool {
        self.started.is_some()
    }

    /// ms on the clock, counting the run in progress
    fn ran(&self, now: u64) -> u64 {
        self.elapsed + self.started.map_or(0, |at| now.saturating_sub(at))
    }

    /// ms before it ends, `None` for the open ended one
    fn left(&self, now: u64) -> Option<u64> {
        self.length.map(|len| len.saturating_sub(self.ran(now)))
    }

    fn expired(&self, now: u64) -> bool {
        self.length.is_some_and(|len| self.ran(now) >= len)
    }

    /// the wall clock an open ended run counts up from. `started - elapsed`
    /// rather than anything involving now, so a tick that changed nothing does
    /// not look to the panel like a change
    fn since(&self) -> Option<u64> {
        if self.length.is_some() {
            return None;
        }
        Some(self.started?.saturating_sub(self.elapsed))
    }

    /// the wall clock a timed run counts down to, steady for the same reason
    fn until(&self) -> Option<u64> {
        Some(self.started? + self.length?.saturating_sub(self.elapsed))
    }

    /// what a stopped clock reads, rounded the way a running one is so it does
    /// not step a second when it is paused
    fn frozen(&self) -> u64 {
        match self.length {
            Some(len) => len.saturating_sub(self.elapsed).div_ceil(1000) * 1000,
            None => self.elapsed / 1000 * 1000,
        }
    }
}

#[derive(Default)]
struct Held {
    /// the tool, alive for exactly as long as the assertion is
    child: Option<Child>,
    session: Option<Session>,
}

struct Awake {
    held: Mutex<Held>,
    state: Mutex<State>,
}

pub fn init(app: &AppHandle) {
    app.manage(Awake {
        held: Mutex::new(Held::default()),
        // the first tick corrects the mains, and off is the safe thing to draw
        state: Mutex::new(IDLE),
    });
}

/// emits on change only, same as the daemon's health
fn set(app: &AppHandle, next: State) {
    let awake = app.state::<Awake>();
    let mut state = awake.state.lock().unwrap();
    if *state == next {
        return;
    }
    *state = next;
    drop(state);

    let _ = app.emit(STATE_EVENT, next);
}

/// what the panel draws, off the session rather than off the clock, so a tick
/// that moved nothing emits nothing
fn describe(held: &Held, ac: bool) -> State {
    let Some(session) = held.session else {
        return State { ac, ..IDLE };
    };

    if session.running() {
        return State {
            active: held.child.is_some(),
            since: session.since(),
            until: session.until(),
            ac,
            ..IDLE
        };
    }

    State {
        paused: true,
        frozen: Some(session.frozen()),
        ac,
        ..IDLE
    }
}

/// the only place the child is started or killed, so the assertion and what the
/// panel was told can never disagree
fn reconcile(app: &AppHandle) {
    let ac = on_ac();
    let now = now_ms();
    let awake = app.state::<Awake>();
    let mut held = awake.held.lock().unwrap();

    // the tool went away on its own, which is `-t` running out or a kill from
    // outside. the assertion went with it, so the session is over
    if let Some(child) = held.child.as_mut()
        && matches!(child.try_wait(), Ok(Some(_)) | Err(_))
    {
        held.child = None;
        held.session = None;
    }

    // the mains going away ends the session rather than holding it, so the
    // cable coming back does not relight the plate on its own
    if !ac {
        held.session = None;
    }

    if held.session.is_some_and(|session| session.expired(now)) {
        held.session = None;
    }

    let running = held.session.is_some_and(|session| session.running());
    match (running, held.child.is_some()) {
        (true, false) => {
            let mut command = Command::new(CAFFEINATE);
            // `-i` keeps the system up and leaves the display free to sleep
            command.arg("-i");
            if let Some(left) = held.session.and_then(|session| session.left(now)) {
                // the kernel holds the deadline, so it survives a sleep the way
                // a timer of our own would not
                command.args(["-t", &left.div_ceil(1000).max(1).to_string()]);
            }
            // no utility to run, so the assertion stands until the tool exits
            command.args(["-w", &std::process::id().to_string()]);

            match command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(child) => held.child = Some(child),
                // nothing to report but the plate staying dark, which the state
                // below already says
                Err(_) => held.session = None,
            }
        }
        (false, true) => {
            if let Some(mut child) = held.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        _ => {}
    }

    let next = describe(&held, ac);
    drop(held);

    set(app, next);
}

/// one supervisor pass, outside the health branch. the mains and the deadline
/// are not the daemon's business, and a session outlives it going away
pub fn tick(app: &AppHandle) {
    reconcile(app);
}

#[tauri::command]
pub fn awake_state(app: AppHandle) -> State {
    *app.state::<Awake>().state.lock().unwrap()
}

/// `seconds` of `None` is the open ended one the menu offers last. a pick while
/// one is already going replaces it rather than stacking on it
#[tauri::command]
pub fn awake_start(app: AppHandle, seconds: Option<u64>) {
    {
        let awake = app.state::<Awake>();
        let mut held = awake.held.lock().unwrap();
        held.session = Some(Session {
            length: seconds.map(|s| s * 1000),
            elapsed: 0,
            started: Some(now_ms()),
        });
    }
    reconcile(&app);
}

#[tauri::command]
pub fn awake_stop(app: AppHandle) {
    {
        let awake = app.state::<Awake>();
        let mut held = awake.held.lock().unwrap();
        held.session = None;
    }
    reconcile(&app);
}

/// banks the run so far and drops the assertion. the clock keeps its reading
#[tauri::command]
pub fn awake_pause(app: AppHandle) {
    {
        let now = now_ms();
        let awake = app.state::<Awake>();
        let mut held = awake.held.lock().unwrap();
        if let Some(session) = held.session.as_mut()
            && let Some(at) = session.started.take()
        {
            session.elapsed += now.saturating_sub(at);
        }
    }
    reconcile(&app);
}

#[tauri::command]
pub fn awake_resume(app: AppHandle) {
    {
        let awake = app.state::<Awake>();
        let mut held = awake.held.lock().unwrap();
        if let Some(session) = held.session.as_mut()
            && session.started.is_none()
        {
            session.started = Some(now_ms());
        }
    }
    reconcile(&app);
}

#[cfg(test)]
mod tests {
    use super::Session;

    const NOW: u64 = 1_700_000_000_000;

    fn timed(minutes: u64) -> Session {
        Session {
            length: Some(minutes * 60_000),
            elapsed: 0,
            started: Some(NOW),
        }
    }

    /// the run in progress counts, and the banked part counts with it
    #[test]
    fn the_clock_is_the_banked_time_plus_the_run_in_progress() {
        let session = Session {
            elapsed: 20_000,
            started: Some(NOW),
            ..timed(15)
        };
        assert_eq!(session.ran(NOW + 5_000), 25_000);
        assert_eq!(session.left(NOW + 5_000), Some(15 * 60_000 - 25_000));
    }

    /// pausing banks the run, and what is left does not move while it is held
    #[test]
    fn a_paused_clock_does_not_move() {
        let held = Session {
            elapsed: 60_000,
            started: None,
            ..timed(15)
        };
        assert_eq!(held.left(NOW), held.left(NOW + 3_600_000));
        assert!(!held.running());
    }

    /// resuming picks the run up where it was banked rather than restarting it
    #[test]
    fn resuming_carries_the_banked_time_over() {
        let resumed = Session {
            elapsed: 60_000,
            started: Some(NOW),
            ..timed(15)
        };
        assert_eq!(resumed.ran(NOW + 30_000), 90_000);
    }

    /// the anchors the panel counts against are steady, so a tick that moved
    /// nothing is not an emit
    #[test]
    fn the_anchors_do_not_drift_with_the_clock() {
        let session = Session {
            elapsed: 60_000,
            started: Some(NOW),
            ..timed(15)
        };
        assert_eq!(session.until(), Some(NOW + 15 * 60_000 - 60_000));
        assert_eq!(session.since(), None);

        let open = Session {
            length: None,
            elapsed: 60_000,
            started: Some(NOW),
        };
        assert_eq!(open.since(), Some(NOW - 60_000));
        assert_eq!(open.until(), None);
    }

    #[test]
    fn a_session_is_over_once_it_has_run_its_length() {
        let session = timed(15);
        assert!(!session.expired(NOW + 15 * 60_000 - 1));
        assert!(session.expired(NOW + 15 * 60_000));
    }

    /// an open ended one has nothing to run out of
    #[test]
    fn an_open_ended_session_never_expires() {
        let open = Session {
            length: None,
            elapsed: 0,
            started: Some(NOW),
        };
        assert!(!open.expired(NOW + 86_400_000));
        assert_eq!(open.left(NOW), None);
    }

    /// a stopped clock reads in whole seconds, the way the running one does
    #[test]
    fn a_frozen_reading_is_rounded_like_a_running_one() {
        let timed_held = Session {
            elapsed: 500,
            started: None,
            ..timed(1)
        };
        assert_eq!(timed_held.frozen(), 60_000);

        let open_held = Session {
            length: None,
            elapsed: 1_500,
            started: None,
        };
        assert_eq!(open_held.frozen(), 1_000);
    }
}
