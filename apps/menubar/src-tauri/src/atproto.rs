//! the atproto session, held by the daemon and signed in from the cli

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::daemon;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

pub const STATE_EVENT: &str = "atproto://state";

/// the pds serves the blob at whatever size it was uploaded, and 22px of it is
/// all this draws
const AVATAR_MAX_BYTES: u64 = 2 * 1024 * 1024;

/// what an <img> renders, so a record pointing anywhere else is never fetched
const AVATAR_TYPES: [&str; 4] = ["image/jpeg", "image/png", "image/webp", "image/gif"];

/// the pds is not the daemon, this round trip leaves the machine
const PROFILE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum State {
    /// no daemon to ask through
    Unknown,
    /// the daemon answered, nobody is signed in
    None,
    /// asked for, and the browser has not come back yet
    Pending { handle: String },
    Session {
        handle: String,
        did: String,
        /// the container renames variants and not fields, so this says it
        #[serde(rename = "displayName")]
        display_name: Option<String>,
        /// a data uri, ready for an <img src>
        avatar: Option<String>,
        /// the host holding the repo, which the detail view names
        pds: Option<String>,
    },
}

/// the public half of the profile record, read off the pds and not the daemon
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Profile {
    display_name: Option<String>,
    avatar: Option<String>,
    pds: Option<String>,
}

#[derive(Default)]
struct Profiles {
    /// settled reads, keyed by the did they belong to
    loaded: Option<(String, Profile)>,
    /// the did a task already has out, so a tick does not start a second
    reading: Option<String>,
}

struct Atproto {
    state: Mutex<State>,
    /// the daemon binds 8988 for the callback, so a second login cannot start
    in_flight: AtomicBool,
    profiles: Mutex<Profiles>,
}

pub fn init(app: &AppHandle) {
    app.manage(Atproto {
        state: Mutex::new(State::Unknown),
        in_flight: AtomicBool::new(false),
        profiles: Mutex::new(Profiles::default()),
    });
}

fn current(app: &AppHandle) -> State {
    app.state::<Atproto>().state.lock().unwrap().clone()
}

/// emits on change only, same as the daemon's health
fn set(app: &AppHandle, next: State) {
    let atproto = app.state::<Atproto>();
    let mut state = atproto.state.lock().unwrap();
    if *state == next {
        return;
    }
    *state = next.clone();
    drop(state);

    let _ = app.emit(STATE_EVENT, next);
}

/// the daemon stopped answering, and it is the only way in
pub fn unknown(app: &AppHandle) {
    set(app, State::Unknown);
}

/// one supervisor tick, only while the daemon answers
pub async fn poll(app: &AppHandle, client: &reqwest::Client) {
    // a login owns the state until the browser comes back, and the daemon
    // reports signed out for the whole of that
    if app.state::<Atproto>().in_flight.load(Ordering::SeqCst) {
        return;
    }

    // unlike the local did this goes both ways, `tiles accounts at login` and
    // `logout` flip it under us, so there is nothing to cache
    let next = dress(app, fetch(client).await);
    set(app, next);
}

async fn fetch(client: &reqwest::Client) -> State {
    let Ok(res) = client
        .get(daemon::url("/v1/tilekit/atproto/status"))
        .send()
        .await
    else {
        return State::Unknown;
    };

    // 404 is the answer for nobody signed in, every other failure is the daemon
    // saying nothing, which is not the same as saying there is no session
    if res.status() == reqwest::StatusCode::NOT_FOUND {
        return State::None;
    }
    if !res.status().is_success() {
        return State::Unknown;
    }

    match res.text().await {
        Ok(body) => parse(&body),
        Err(_) => State::Unknown,
    }
}

/// the success body only, a non-2xx never reaches here
fn parse(body: &str) -> State {
    // reqwest is built without its json feature, serde_json is already here
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
        return State::Unknown;
    };

    let data = payload.get("data");
    let handle = data.and_then(|d| d.get("handle")).and_then(|v| v.as_str());
    let did = data.and_then(|d| d.get("did")).and_then(|v| v.as_str());

    match (handle, did) {
        (Some(handle), Some(did)) if !handle.is_empty() && !did.is_empty() => State::Session {
            handle: handle.to_owned(),
            did: did.to_owned(),
            display_name: None,
            avatar: None,
            pds: None,
        },
        // a success carrying neither is the daemon contradicting itself, and a
        // signed-out claim is the one thing it should not be read as
        _ => State::Unknown,
    }
}

/// the status route carries the identity only, so the profile behind it is this
/// app's own read and lands a tick or two later
fn dress(app: &AppHandle, state: State) -> State {
    let State::Session { handle, did, .. } = state else {
        return state;
    };

    let profile = {
        let atproto = app.state::<Atproto>();
        let mut profiles = atproto.profiles.lock().unwrap();
        match &profiles.loaded {
            Some((seen, profile)) if *seen == did => profile.clone(),
            _ => {
                if profiles.reading.as_deref() != Some(did.as_str()) {
                    profiles.reading = Some(did.clone());
                    spawn_read(app.clone(), did.clone());
                }
                Profile::default()
            }
        }
    };

    State::Session {
        handle,
        did,
        display_name: profile.display_name,
        avatar: profile.avatar,
        pds: profile.pds,
    }
}

/// off the watch loop, whose client times out in a second and whose other polls
/// are waiting behind it
fn spawn_read(app: AppHandle, did: String) {
    tauri::async_runtime::spawn(async move {
        let profile = read_profile(&did).await;

        {
            let atproto = app.state::<Atproto>();
            let mut profiles = atproto.profiles.lock().unwrap();
            profiles.reading = None;
            // a pds that said nothing caches nothing, so the next tick asks again
            if let Some(profile) = profile.clone() {
                profiles.loaded = Some((did.clone(), profile));
            }
        }

        let Some(profile) = profile else {
            return;
        };

        // the identity can move while this is out, and the picture belongs to
        // the one that was asked for
        let State::Session {
            handle, did: at, ..
        } = current(&app)
        else {
            return;
        };
        if at != did {
            return;
        }

        set(
            &app,
            State::Session {
                handle,
                did: at,
                display_name: profile.display_name,
                avatar: profile.avatar,
                pds: profile.pds,
            },
        );
    });
}

/// `None` is the pds saying nothing, which is not the same as an account that
/// has set no picture
async fn read_profile(did: &str) -> Option<Profile> {
    let client = reqwest::Client::builder()
        .timeout(PROFILE_TIMEOUT)
        .build()
        .ok()?;

    let pds = resolve_pds(&client, did).await?;
    let res = client
        .get(format!(
            "{pds}/xrpc/com.atproto.repo.getRecord?repo={did}&collection=app.bsky.actor.profile&rkey=self"
        ))
        .send()
        .await
        .ok()?;

    // an account that never wrote the record answers 400 and not 404, so any
    // refusal here is an empty profile rather than a pds that went quiet
    if !res.status().is_success() {
        return Some(Profile {
            pds: Some(pds),
            ..Profile::default()
        });
    }

    let (display_name, blob) = parse_profile(&res.text().await.ok()?);
    let avatar = match blob {
        Some((cid, mime)) => blob_uri(&client, &pds, did, &cid, &mime).await,
        None => None,
    };

    Some(Profile {
        display_name,
        avatar,
        pds: Some(pds),
    })
}

/// the did document says which pds holds the repo, and the daemon is not in
/// this path to have resolved it already
async fn resolve_pds(client: &reqwest::Client, did: &str) -> Option<String> {
    let res = client.get(did_doc_url(did)?).send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }

    pds_from_doc(&res.text().await.ok()?)
}

/// plc keeps its documents in one directory, did:web serves its own
fn did_doc_url(did: &str) -> Option<String> {
    if !is_token(did) {
        return None;
    }

    if did.starts_with("did:plc:") {
        return Some(format!("https://plc.directory/{did}"));
    }

    // the host form only, the path form maps colons to slashes and a guess at
    // one is worse than drawing initials
    let host = did.strip_prefix("did:web:")?;
    if host.is_empty() || host.contains(':') {
        return None;
    }

    Some(format!("https://{host}/.well-known/did.json"))
}

/// these get joined into urls, and the daemon is not the only thing that could
/// have put them there
fn is_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'.' | b'_' | b'-'))
}

/// the endpoint of the service entry that holds the repo
fn pds_from_doc(body: &str) -> Option<String> {
    let doc = serde_json::from_str::<serde_json::Value>(body).ok()?;
    let endpoint = doc
        .get("service")?
        .as_array()?
        .iter()
        .find(|entry| {
            entry.get("type").and_then(|v| v.as_str()) == Some("AtprotoPersonalDataServer")
        })?
        .get("serviceEndpoint")?
        .as_str()?;

    // everything after this is built by joining onto it, and only https is
    // worth sending a did to
    if !endpoint.starts_with("https://") {
        return None;
    }

    Some(endpoint.trim_end_matches('/').to_owned())
}

/// the record body, whose avatar is a blob ref and not a url
fn parse_profile(body: &str) -> (Option<String>, Option<(String, String)>) {
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
        return (None, None);
    };

    let value = payload.get("value");
    let display_name = value
        .and_then(|v| v.get("displayName"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned);

    let avatar = value.and_then(|v| v.get("avatar"));
    let cid = avatar
        .and_then(|a| a.get("ref"))
        .and_then(|r| r.get("$link"))
        .and_then(|v| v.as_str());
    let mime = avatar
        .and_then(|a| a.get("mimeType"))
        .and_then(|v| v.as_str());

    let blob = match (cid, mime) {
        (Some(cid), Some(mime)) if AVATAR_TYPES.contains(&mime) && is_token(cid) => {
            Some((cid.to_owned(), mime.to_owned()))
        }
        _ => None,
    };

    (display_name, blob)
}

async fn blob_uri(
    client: &reqwest::Client,
    pds: &str,
    did: &str,
    cid: &str,
    mime: &str,
) -> Option<String> {
    let res = client
        .get(format!(
            "{pds}/xrpc/com.atproto.sync.getBlob?did={did}&cid={cid}"
        ))
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        return None;
    }

    // the header is advisory, so it only saves reading a body that already
    // admits it is too big
    if res
        .content_length()
        .is_some_and(|len| len > AVATAR_MAX_BYTES)
    {
        return None;
    }

    let bytes = res.bytes().await.ok()?;
    if bytes.len() as u64 > AVATAR_MAX_BYTES {
        return None;
    }

    Some(format!(
        "data:{mime};base64,{}",
        data_encoding::BASE64.encode(&bytes)
    ))
}

#[tauri::command]
pub fn atproto_state(app: AppHandle) -> State {
    current(&app)
}

/// the daemon resolves the handle, opens the browser itself and holds the
/// request until the redirect lands, so this waits with no upper bound
#[tauri::command]
pub async fn atproto_login(app: AppHandle, handle: String) -> Result<(), String> {
    let handle = handle.trim().trim_start_matches('@').to_lowercase();
    if handle.is_empty() {
        return Err("A handle is needed".into());
    }

    if app
        .state::<Atproto>()
        .in_flight
        .swap(true, Ordering::SeqCst)
    {
        return Err("A sign-in is already waiting".into());
    }

    set(
        &app,
        State::Pending {
            handle: handle.clone(),
        },
    );

    let outcome = request(&handle).await;
    app.state::<Atproto>()
        .in_flight
        .store(false, Ordering::SeqCst);

    if let Err(err) = outcome {
        // the next tick reports whatever the daemon actually holds
        set(&app, State::Unknown);
        return Err(err);
    }

    Ok(())
}

async fn request(handle: &str) -> Result<(), String> {
    // no timeout on this one, the wait is however long the browser takes
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "user_handle": handle }).to_string();

    let res = client
        .post(daemon::url("/v1/tilekit/atproto/login"))
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|err| err.to_string())?;

    let status = res.status();
    if status.is_success() {
        return Ok(());
    }

    Err(reason(&res.text().await.unwrap_or_default(), status))
}

/// the daemon says why in the body, and its reasons are the readable half of
/// what went wrong
fn reason(body: &str, status: reqwest::StatusCode) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .as_ref()
        .and_then(|payload| payload.get("reason"))
        .and_then(|v| v.as_str())
        .map(|reason| reason.to_owned())
        .unwrap_or_else(|| format!("The daemon answered {status}"))
}

#[cfg(test)]
mod tests {
    use super::{State, did_doc_url, parse, parse_profile, pds_from_doc};

    /// the shape `status` builds, tiles/src/daemon/atproto.rs
    const SUCCESS: &str = r#"{"status":"success","data":{"handle":"codelif.in","did":"did:plc:7iza6de2dwap2sbkpav7c6c6"}}"#;

    /// the shape a pds returns for app.bsky.actor.profile/self
    const RECORD: &str = r#"{"uri":"at://did:plc:x/app.bsky.actor.profile/self","value":{"$type":"app.bsky.actor.profile","displayName":"Harsh Sharma","avatar":{"$type":"blob","ref":{"$link":"bafkreiabc123"},"mimeType":"image/jpeg","size":91234}}}"#;

    #[test]
    fn reads_the_daemons_success_body() {
        assert_eq!(
            parse(SUCCESS),
            State::Session {
                handle: "codelif.in".to_owned(),
                did: "did:plc:7iza6de2dwap2sbkpav7c6c6".to_owned(),
                display_name: None,
                avatar: None,
                pds: None,
            }
        );
    }

    #[test]
    fn a_body_without_an_identity_is_not_a_sign_out() {
        assert_eq!(parse(r#"{"status":"success","data":{}}"#), State::Unknown);
        assert_eq!(parse("not json"), State::Unknown);
    }

    #[test]
    fn finds_the_pds_in_a_did_document() {
        // the id carries a fragment, hence the wider raw string
        let doc = r##"{"service":[{"id":"#atproto_pds","type":"AtprotoPersonalDataServer","serviceEndpoint":"https://shimeji.us-east.host.bsky.network/"}]}"##;
        assert_eq!(
            pds_from_doc(doc).as_deref(),
            Some("https://shimeji.us-east.host.bsky.network")
        );

        // a document with no pds entry, and one served over plain http
        assert_eq!(pds_from_doc(r#"{"service":[]}"#), None);
        let plain = r#"{"service":[{"type":"AtprotoPersonalDataServer","serviceEndpoint":"http://pds.example"}]}"#;
        assert_eq!(pds_from_doc(plain), None);
    }

    #[test]
    fn reads_the_profile_record() {
        let (name, blob) = parse_profile(RECORD);
        assert_eq!(name.as_deref(), Some("Harsh Sharma"));
        assert_eq!(
            blob,
            Some(("bafkreiabc123".to_owned(), "image/jpeg".to_owned()))
        );
    }

    #[test]
    fn a_record_without_a_picture_is_not_a_failure() {
        let (name, blob) = parse_profile(r#"{"value":{"displayName":"Harsh"}}"#);
        assert_eq!(name.as_deref(), Some("Harsh"));
        assert_eq!(blob, None);

        // a blob that an <img> would not render is left alone
        let odd = r#"{"value":{"avatar":{"ref":{"$link":"bafkrei1"},"mimeType":"video/mp4"}}}"#;
        assert_eq!(parse_profile(odd), (None, None));
    }

    #[test]
    fn only_resolvable_dids_become_urls() {
        assert_eq!(
            did_doc_url("did:plc:abc").as_deref(),
            Some("https://plc.directory/did:plc:abc")
        );
        assert_eq!(
            did_doc_url("did:web:example.com").as_deref(),
            Some("https://example.com/.well-known/did.json")
        );
        // the path form, and a did carrying url of its own
        assert_eq!(did_doc_url("did:web:example.com:u:alice"), None);
        assert_eq!(did_doc_url("did:plc:a/../../evil"), None);
    }
}
