//! the inference server, reachable only through the daemon

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::{daemon, tray};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

const START_TIMEOUT: Duration = Duration::from_secs(20);
const STOP_TIMEOUT: Duration = Duration::from_secs(15);

/// start answers in ~40ms and the port opens ~5s later, past this one that
/// never opened it is called off
const READY_TIMEOUT: Duration = Duration::from_secs(15);

/// how long the user's ask outlives the request meant to satisfy it
const RECONCILE_TIMEOUT: Duration = Duration::from_secs(45);

pub const STATE_EVENT: &str = "inference://state";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Power {
    /// no daemon to ask through
    Unknown,
    Off,
    Starting,
    On,
}

/// the `[llama]` table, the flags the runtime was launched with
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Llama {
    pub context_length: Option<u32>,
    pub gpu_layers: Option<i32>,
    pub offload_kqv: Option<bool>,
    pub batch_size: Option<u32>,
    pub mtp: Option<bool>,
    pub n_cpu_moe: Option<u32>,
    pub flash_attn: Option<bool>,
    pub no_mmap: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct State {
    pub power: Power,
    /// `[model] current`, what the daemon is configured for
    pub model: Option<String>,
    pub llama: Option<Llama>,
}

struct Inference {
    state: Mutex<State>,
    /// set when we ask for a start, the only thing that separates one still
    /// coming up from a server that is simply down
    starting_since: Mutex<Option<Instant>>,
    /// what the user last asked for, held until the server agrees, see [`reconcile`]
    desired: Mutex<Option<(bool, Instant)>>,
    /// a second start inside the boot window spawns a second server, the daemon
    /// only checks the port
    in_flight: AtomicBool,
}

pub fn init(app: &AppHandle) {
    app.manage(Inference {
        state: Mutex::new(State {
            power: Power::Unknown,
            model: None,
            llama: None,
        }),
        starting_since: Mutex::new(None),
        desired: Mutex::new(None),
        in_flight: AtomicBool::new(false),
    });
}

fn current(app: &AppHandle) -> State {
    app.state::<Inference>().state.lock().unwrap().clone()
}

/// emits on change only, same as the daemon's health
fn set(app: &AppHandle, next: State) {
    let inference = app.state::<Inference>();
    let mut state = inference.state.lock().unwrap();
    if *state == next {
        return;
    }
    let power = next.power;
    *state = next.clone();
    drop(state);

    let _ = app.emit(STATE_EVENT, next);

    // the status item dims with the panel's mark, and AppKit wants the main thread
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || tray::set_live(&handle, power == Power::On));
}

fn set_power(app: &AppHandle, power: Power) {
    let mut next = current(app);
    next.power = power;
    set(app, next);
}

fn settle(app: &AppHandle, power: Power) {
    *app.state::<Inference>().starting_since.lock().unwrap() = None;
    set_power(app, power);
}

/// a start in flight resolves in seconds, so the supervisor watches it closely
pub fn is_settling(app: &AppHandle) -> bool {
    app.try_state::<Inference>()
        .is_some_and(|inference| inference.state.lock().unwrap().power == Power::Starting)
}

/// the daemon stopped answering, and it is the only way in
pub fn unknown(app: &AppHandle) {
    *app.state::<Inference>().desired.lock().unwrap() = None;
    settle(app, Power::Unknown);
}

/// one supervisor tick, only while the daemon answers
pub async fn poll(app: &AppHandle, client: &reqwest::Client) {
    let listening = matches!(
        client
            .get(daemon::url("/v1/tilekit/server/ping"))
            .send()
            .await,
        Ok(res) if res.status().is_success()
    );

    let started_at = *app.state::<Inference>().starting_since.lock().unwrap();
    let power = match (listening, started_at) {
        (true, _) => Power::On,
        (false, Some(at)) if at.elapsed() < READY_TIMEOUT => Power::Starting,
        (false, _) => Power::Off,
    };

    if power != Power::Starting {
        *app.state::<Inference>().starting_since.lock().unwrap() = None;
    }

    // /config is a config-file read with no db and no keychain behind it, so it
    // is cheap enough to re-read rather than cache and go stale after a cli
    // `model use`
    let (model, llama) = match config(client).await {
        Some(read) => read,
        None => {
            let held = current(app);
            (held.model, held.llama)
        }
    };

    set(
        app,
        State {
            power,
            model,
            llama,
        },
    );
    reconcile(app, client, power).await;
}

/// the daemon decides "is it running" by pinging rather than by pid, so a stop
/// inside a start's boot window reports success and kills nothing, and a start
/// inside a stop's reports "already up" and spawns nothing. either way the
/// switch would settle opposite to the tap, so the ask is re-issued until the
/// server agrees with it
async fn reconcile(app: &AppHandle, client: &reqwest::Client, power: Power) {
    let Some((on, asked_at)) = *app.state::<Inference>().desired.lock().unwrap() else {
        return;
    };

    let settled = match power {
        Power::On => true,
        Power::Off => false,
        // still moving, nothing to disagree with yet
        Power::Starting | Power::Unknown => return,
    };

    if settled == on || asked_at.elapsed() > RECONCILE_TIMEOUT {
        *app.state::<Inference>().desired.lock().unwrap() = None;
        return;
    }

    let _ = request(app, client, on).await;
}

/// the config blob also carries the user's did and data paths, so only these two
/// branches cross into the webview
async fn config(client: &reqwest::Client) -> Option<(Option<String>, Option<Llama>)> {
    let res = client.get(daemon::url("/config")).send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }
    // reqwest is built without its json feature, serde_json is already here
    let body: serde_json::Value = serde_json::from_str(&res.text().await.ok()?).ok()?;

    Some((model_spec(&body), llama(&body)))
}

fn model_spec(body: &serde_json::Value) -> Option<String> {
    let spec = body.get("model")?.get("current")?.as_str()?;

    (!spec.is_empty()).then(|| spec.to_owned())
}

/// every field is optional in the daemon too, an absent flag means its default
fn llama(body: &serde_json::Value) -> Option<Llama> {
    let table = body.get("llama")?;
    let int = |key: &str| table.get(key).and_then(|v| v.as_u64()).map(|v| v as u32);
    let flag = |key: &str| table.get(key).and_then(|v| v.as_bool());

    Some(Llama {
        context_length: int("context_length"),
        gpu_layers: table
            .get("gpu_layers")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        offload_kqv: flag("offload_kqv"),
        batch_size: int("batch_size"),
        mtp: flag("mtp"),
        n_cpu_moe: int("n_cpu_moe"),
        flash_attn: flag("flash_attn"),
        no_mmap: flag("no_mmap"),
    })
}

async fn request(app: &AppHandle, client: &reqwest::Client, on: bool) -> Result<(), String> {
    if app
        .state::<Inference>()
        .in_flight
        .swap(true, Ordering::SeqCst)
    {
        return Err("a start or stop is already in flight".into());
    }

    let (path, timeout) = if on {
        ("/v1/tilekit/server/start", START_TIMEOUT)
    } else {
        ("/v1/tilekit/server/stop", STOP_TIMEOUT)
    };

    if on {
        *app.state::<Inference>().starting_since.lock().unwrap() = Some(Instant::now());
        set_power(app, Power::Starting);
    }

    let outcome = match client.get(daemon::url(path)).timeout(timeout).send().await {
        Ok(res) if res.status().is_success() => Ok(()),
        Ok(res) => Err(format!("{path} answered {}", res.status())),
        Err(err) => Err(err.to_string()),
    };

    match (on, &outcome) {
        // a start that was refused never comes up
        (true, Err(_)) => settle(app, Power::Off),
        // stop closes the port before the next tick, no reason to wait to say so
        (false, Ok(())) => settle(app, Power::Off),
        // a start that landed is the poll's to confirm
        _ => {}
    }

    app.state::<Inference>()
        .in_flight
        .store(false, Ordering::SeqCst);

    outcome
}

#[tauri::command]
pub fn inference_state(app: AppHandle) -> State {
    current(&app)
}

#[tauri::command]
pub async fn inference_set(app: AppHandle, on: bool) -> Result<(), String> {
    // the ask outlives this request, the daemon may quietly ignore it
    *app.state::<Inference>().desired.lock().unwrap() = Some((on, Instant::now()));

    request(&app, &reqwest::Client::new(), on).await
}
