//! holding the machine up

use std::process::{Child, Stdio};
use std::sync::{LazyLock, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::{lid, power};

pub const STATE_EVENT: &str = "awake://state";

const MINIMUM_BATTERY_PERCENT: u8 = 10;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevicePower {
    pub plugged_in: bool,
    pub battery_percent: Option<u8>,
}

impl DevicePower {
    fn allows_stay_awake(self) -> bool {
        self.plugged_in
            || self
                .battery_percent
                .is_some_and(|percent| percent > MINIMUM_BATTERY_PERCENT)
    }
}

/// monotonic: an ntp step must not stall or end a session
static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

fn now_ms() -> u64 {
    EPOCH.elapsed().as_millis() as u64
}

/// run-clock ms as unix ms
fn wall_ms(at: u64) -> u64 {
    let wall = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let now = now_ms();

    if at >= now {
        wall.saturating_add(at - now)
    } else {
        wall.saturating_sub(now - at)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub active: bool,
    pub paused: bool,
    /// unix ms an open ended session counts up from
    pub since: Option<u64>,
    /// unix ms a timed session counts down to
    pub until: Option<u64>,
    /// the paused reading, already whole seconds
    pub frozen: Option<u64>,
    /// gates new sessions
    pub power: DevicePower,
    /// whether a running session survives the lid closing, `None` where
    /// there is no closed-display mode
    pub lid: Option<bool>,
}

const IDLE: State = State {
    active: false,
    paused: false,
    since: None,
    until: None,
    frozen: None,
    power: DevicePower {
        plugged_in: false,
        battery_percent: None,
    },
    lid: None,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Session {
    /// total ms, `None` being open ended
    length: Option<u64>,
    /// ms banked by earlier runs
    elapsed: u64,
    /// run-clock ms the run in progress began, `None` while paused
    started: Option<u64>,
}

impl Session {
    fn running(&self) -> bool {
        self.started.is_some()
    }

    /// ms on the clock, counting the run in progress
    fn ran(&self, now: u64) -> u64 {
        self.elapsed
            .saturating_add(self.started.map_or(0, |at| now.saturating_sub(at)))
    }

    /// ms before it ends
    fn left(&self, now: u64) -> Option<u64> {
        self.length.map(|len| len.saturating_sub(self.ran(now)))
    }

    fn expired(&self, now: u64) -> bool {
        self.length.is_some_and(|len| self.ran(now) >= len)
    }

    fn since(&self) -> Option<u64> {
        if self.length.is_some() {
            return None;
        }
        Some(self.started?.saturating_sub(self.elapsed))
    }

    fn until(&self) -> Option<u64> {
        Some(
            self.started?
                .saturating_add(self.length?.saturating_sub(self.elapsed)),
        )
    }

    fn frozen(&self) -> u64 {
        match self.length {
            Some(len) => len
                .saturating_sub(self.elapsed)
                .div_ceil(1000)
                .saturating_mul(1000),
            None => self.elapsed / 1000 * 1000,
        }
    }
}

#[derive(Default)]
struct Held {
    child: Option<Child>,
    /// taken and let go with the child, the two are one mode
    lid: Option<lid::Hold>,
    session: Option<Session>,
}

struct Awake {
    held: Mutex<Held>,
    state: Mutex<State>,
}

pub fn init(app: &AppHandle) {
    app.manage(Awake {
        held: Mutex::new(Held::default()),
        state: Mutex::new(IDLE),
    });
    lid::recover(app);
    lid::watch(app);
}

// the caller still holds `held`, so the snapshot cannot be overtaken by a later one
fn store(app: &AppHandle, next: State) -> bool {
    let awake = app.state::<Awake>();
    let mut state = awake.state.lock().unwrap();
    if *state == next {
        return false;
    }
    *state = next;
    true
}

fn describe(held: &Held, power: DevicePower, lid: Option<bool>) -> State {
    let Some(session) = held.session else {
        return State { power, ..IDLE };
    };

    if session.running() {
        return State {
            active: held.child.is_some(),
            since: session.since().map(wall_ms),
            until: session.until().map(wall_ms),
            power,
            lid,
            ..IDLE
        };
    }

    State {
        paused: true,
        frozen: Some(session.frozen()),
        power,
        ..IDLE
    }
}

fn release(held: &mut Held) {
    if let Some(mut child) = held.child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    if let Some(hold) = held.lid.take() {
        lid::release(hold);
    }
}

pub fn reconcile(app: &AppHandle) {
    let power = power::device_power();
    let now = now_ms();
    let awake = app.state::<Awake>();
    let mut held = awake.held.lock().unwrap();

    if let Some(Ok(Some(_)) | Err(_)) = held.child.as_mut().map(Child::try_wait) {
        release(&mut held);
        held.session = None;
    }

    // low battery ends the session, a charger later does not bring it back
    if !power.allows_stay_awake() {
        held.session = None;
    }

    if held.session.is_some_and(|session| session.expired(now)) {
        held.session = None;
    }

    let running = held.session.is_some_and(|session| session.running());
    match (running, held.child.is_some()) {
        (true, false) => {
            let left = held.session.and_then(|session| session.left(now));
            let mut command = power::inhibitor(left);

            match command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(child) => {
                    held.child = Some(child);
                    held.lid = Some(lid::hold(app));
                }
                Err(_) => held.session = None,
            }
        }
        (false, true) => release(&mut held),
        _ => {}
    }

    let lid = held.lid.as_mut().and_then(lid::keep);
    let next = describe(&held, power, lid);
    let changed = store(app, next);
    drop(held);

    if changed {
        let _ = app.emit(STATE_EVENT, next);
    }
}

/// the chirp says the mac is staying up, so only when it is
pub fn lid_moved(app: &AppHandle, closed: bool) {
    reconcile(app);
    let covered = app.state::<Awake>().state.lock().unwrap().lid == Some(true);
    if closed && covered {
        lid::chirp(app);
    }
}

/// the sentinel would reset it too, this is only sooner
pub fn shutdown(app: &AppHandle) {
    let Some(awake) = app.try_state::<Awake>() else {
        return;
    };
    let mut held = awake.held.lock().unwrap();
    release(&mut held);
    held.session = None;
}

#[tauri::command]
pub fn awake_state(app: AppHandle) -> State {
    *app.state::<Awake>().state.lock().unwrap()
}

#[tauri::command]
pub fn awake_start(app: AppHandle, seconds: Option<u64>) -> Result<(), String> {
    if !power::device_power().allows_stay_awake() {
        return Err("Connect a charger or charge above 10% to use Stay Awake".into());
    }

    {
        let awake = app.state::<Awake>();
        let mut held = awake.held.lock().unwrap();
        release(&mut held);
        held.session = Some(Session {
            length: seconds.map(|s| s.saturating_mul(1000)),
            elapsed: 0,
            started: Some(now_ms()),
        });
    }
    reconcile(&app);
    Ok(())
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

#[tauri::command]
pub fn awake_pause(app: AppHandle) {
    {
        let now = now_ms();
        let awake = app.state::<Awake>();
        let mut held = awake.held.lock().unwrap();
        if let Some(session) = held.session.as_mut()
            && let Some(at) = session.started.take()
        {
            session.elapsed = session.elapsed.saturating_add(now.saturating_sub(at));
        }
    }
    reconcile(&app);
}

#[tauri::command]
pub fn awake_resume(app: AppHandle) -> Result<(), String> {
    if !power::device_power().allows_stay_awake() {
        return Err("Connect a charger or charge above 10% to resume Stay Awake".into());
    }

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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DevicePower, Session};

    const NOW: u64 = 1_700_000_000_000;

    fn timed(minutes: u64) -> Session {
        Session {
            length: Some(minutes * 60_000),
            elapsed: 0,
            started: Some(NOW),
        }
    }

    #[test]
    fn stay_awake_needs_power_or_more_than_ten_percent() {
        assert!(
            DevicePower {
                plugged_in: true,
                battery_percent: Some(1),
            }
            .allows_stay_awake()
        );
        assert!(
            DevicePower {
                plugged_in: false,
                battery_percent: Some(11),
            }
            .allows_stay_awake()
        );
        assert!(
            !DevicePower {
                plugged_in: false,
                battery_percent: Some(10),
            }
            .allows_stay_awake()
        );
        assert!(!DevicePower::default().allows_stay_awake());
    }

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

    #[test]
    fn resuming_carries_the_banked_time_over() {
        let resumed = Session {
            elapsed: 60_000,
            started: Some(NOW),
            ..timed(15)
        };
        assert_eq!(resumed.ran(NOW + 30_000), 90_000);
    }

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
    fn an_absurd_length_saturates_rather_than_wrapping() {
        let session = Session {
            length: Some(u64::MAX.saturating_mul(1000)),
            elapsed: 0,
            started: Some(NOW),
        };

        assert!(!session.expired(NOW));
        assert_eq!(session.left(NOW), Some(u64::MAX));
        assert_eq!(session.until(), Some(u64::MAX));
        assert_eq!(session.frozen(), u64::MAX);
    }

    #[test]
    fn a_session_is_over_once_it_has_run_its_length() {
        let session = timed(15);
        assert!(!session.expired(NOW + 15 * 60_000 - 1));
        assert!(session.expired(NOW + 15 * 60_000));
    }

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
