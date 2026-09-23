//! tiles:// links. one that launches the app has to survive the handover to the
//! daemon's copy, so it crosses in a file

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Manager, Url};
use tauri_plugin_deep_link::DeepLinkExt;

use crate::{boot, lifeline, paths, ui};

const SCHEME: &str = "tiles";

/// older than this, the copy that wrote it was never taken over
const HANDOFF_TTL: Duration = Duration::from_secs(60);

const POLL: Duration = Duration::from_millis(300);
const DRAFT_MAX: usize = 16 * 1024;

/// boot's own limit
const GIVE_UP: Duration = Duration::from_secs(20);
/// a daemon without its own app never takes the link over
const HANDOVER_GRACE: Duration = Duration::from_secs(5);

#[derive(Debug, PartialEq)]
pub enum Target {
    /// like a dock click, the window stays where it was
    Focus,
    Page(String),
}

impl Target {
    fn path(self) -> String {
        match self {
            Target::Focus => "/".to_owned(),
            Target::Page(path) => path,
        }
    }
}

/// held by a hand launched copy until it knows whether it stays
#[derive(Default)]
pub struct Pending(Mutex<Option<Url>>);

/// ids and slugs are held to a safe alphabet, so a link can only name a page
pub fn route(url: &Url) -> Option<Target> {
    if !url.scheme().eq_ignore_ascii_case(SCHEME) {
        return None;
    }

    // tiles://chat/x and tiles:///chat/x are the same page
    let segments: Vec<&str> = url
        .host_str()
        .into_iter()
        .chain(url.path().split('/'))
        .filter(|segment| !segment.is_empty())
        .collect();

    let path = match segments.as_slice() {
        [] => return Some(Target::Focus),
        [page] if page.eq_ignore_ascii_case("open") => return Some(Target::Focus),
        [page] if page.eq_ignore_ascii_case("chat") => new_chat(url),
        [page] if page.eq_ignore_ascii_case("plugins") => "/plugins".to_owned(),
        [page, id] if page.eq_ignore_ascii_case("chat") && is_safe(id) => format!("/chat/{id}"),
        [page, slug] if page.eq_ignore_ascii_case("plugins") && is_safe(slug) => {
            format!("/plugins/{slug}")
        }
        _ => return None,
    };

    Some(Target::Page(path))
}

/// ?draft= is only ever typed in, the ui's ?q= is the one that sends
fn new_chat(url: &Url) -> String {
    let draft = url
        .query_pairs()
        .find(|(key, _)| key == "draft")
        .map(|(_, value)| value)
        .filter(|draft| !draft.trim().is_empty() && draft.len() <= DRAFT_MAX);

    let Some(draft) = draft else {
        return "/".to_owned();
    };

    let mut query = Url::parse("tiles://x/").expect("a fixed url parses");
    query.query_pairs_mut().append_pair("draft", &draft);
    format!("/?{}", query.query().unwrap_or_default())
}

fn is_safe(part: &str) -> bool {
    !part.is_empty()
        && part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// the deep link plugin has already taken these
pub fn in_args(args: &[String]) -> bool {
    args.iter().any(|arg| {
        arg.get(..SCHEME.len() + 1)
            .is_some_and(|head| head.eq_ignore_ascii_case(&format!("{SCHEME}:")))
    })
}

pub fn init(app: &AppHandle) {
    app.manage(Pending::default());

    // linux installs ship no .desktop entry for the scheme
    #[cfg(target_os = "linux")]
    if let Err(err) = app.deep_link().register_all() {
        eprintln!("[deeplink] could not register the tiles scheme: {err}");
    }

    let handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
            open(&handle, &url);
        }
    });

    // linux hands a launching link over as argv, macos as a later apple event
    if let Ok(Some(urls)) = app.deep_link().get_current() {
        for url in urls {
            open(app, &url);
        }
    }
}

fn open(app: &AppHandle, url: &Url) {
    let Some(target) = route(url) else {
        eprintln!("[deeplink] ignored {url}");
        return;
    };

    if lifeline::is_supervised() {
        show(app, target);
        return;
    }

    // this copy may be about to hand over, and a window opened now dies with it
    *app.state::<Pending>().0.lock().unwrap() = Some(url.clone());

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::new();
        if !boot::ping(&client).await {
            let deadline = tokio::time::Instant::now() + GIVE_UP;
            while !boot::ping(&client).await {
                if tokio::time::Instant::now() >= deadline {
                    return;
                }
                tokio::time::sleep(POLL).await;
            }
            // its copy of us is on the way, and this process dies if it arrives
            tokio::time::sleep(HANDOVER_GRACE).await;
        }

        let pending = app.state::<Pending>().0.lock().unwrap().take();
        if let Some(target) = pending.as_ref().and_then(route) {
            show(&app, target);
        }
    });
}

/// appkit traps on windows touched off the main thread
fn show(app: &AppHandle, target: Target) {
    let handle = app.clone();
    let queued = app.run_on_main_thread(move || match target {
        Target::Focus => ui::reopen(&handle),
        Target::Page(path) => {
            if let Err(err) = ui::open(&handle, &path) {
                eprintln!("[deeplink] could not open {path}: {err}");
            }
        }
    });

    if let Err(err) = queued {
        eprintln!("[deeplink] could not reach the main thread: {err}");
    }
}

fn handoff_file() -> Option<PathBuf> {
    Some(paths::default_dir()?.join("deeplink.pending"))
}

/// the daemon's copy picks this up in ui::init
pub fn stash(app: &AppHandle) {
    // single instance listens before setup runs
    let Some(pending) = app.try_state::<Pending>() else {
        return;
    };
    let Some(link) = pending.0.lock().unwrap().take() else {
        return;
    };
    let Some(file) = handoff_file() else {
        return;
    };

    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(err) = std::fs::write(&file, link.as_str()) {
        eprintln!("[deeplink] could not hand over the link: {err}");
    }
}

/// only the daemon's copy is ever handed a link
pub fn take_handoff() -> Option<String> {
    if !lifeline::is_supervised() {
        return None;
    }

    let file = handoff_file()?;
    let modified = std::fs::metadata(&file).and_then(|meta| meta.modified());
    let contents = std::fs::read_to_string(&file);
    let _ = std::fs::remove_file(&file);

    let age = SystemTime::now().duration_since(modified.ok()?).ok()?;
    accept_handoff(&contents.ok()?, age)
}

/// the file sits in a user owned folder, so it is routed like any link
fn accept_handoff(contents: &str, age: Duration) -> Option<String> {
    if age > HANDOFF_TTL {
        return None;
    }

    let url: Url = contents.trim().parse().ok()?;
    // no window yet, so focus means opening one
    route(&url).map(Target::path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route_of(link: &str) -> Option<Target> {
        route(&link.parse().unwrap())
    }

    fn page(path: &str) -> Option<Target> {
        Some(Target::Page(path.to_owned()))
    }

    #[test]
    fn a_bare_link_only_brings_the_app_forward() {
        assert_eq!(route_of("tiles://"), Some(Target::Focus));
        assert_eq!(route_of("tiles:"), Some(Target::Focus));
        assert_eq!(route_of("tiles://open"), Some(Target::Focus));
    }

    #[test]
    fn links_name_pages() {
        assert_eq!(route_of("tiles://chat"), page("/"));
        assert_eq!(route_of("tiles://chat/"), page("/"));
        assert_eq!(route_of("tiles://chat/abc-123_x"), page("/chat/abc-123_x"));
        assert_eq!(route_of("tiles:///chat/abc"), page("/chat/abc"));
        assert_eq!(route_of("tiles:chat/abc"), page("/chat/abc"));
        assert_eq!(route_of("TILES://Chat/abc"), page("/chat/abc"));
        assert_eq!(route_of("tiles://plugins"), page("/plugins"));
        assert_eq!(route_of("tiles://plugins/exa"), page("/plugins/exa"));
        assert_eq!(route_of("tiles://chat/abc?x=1#y"), page("/chat/abc"));
    }

    #[test]
    fn anything_else_is_ignored() {
        assert_eq!(route_of("https://chat/abc"), None);
        assert_eq!(route_of("tiles://settings"), None);
        assert_eq!(route_of("tiles://chat/a/b"), None);
        assert_eq!(route_of("tiles://chat/%2f"), None);
        assert_eq!(route_of("tiles://chat/a.b"), None);
    }

    #[test]
    fn dot_segments_cannot_climb_out() {
        // the url parser resolves these first
        assert_eq!(route_of("tiles://chat/../x"), page("/chat/x"));
        assert_eq!(route_of("tiles://chat/x/.."), page("/"));
        assert_eq!(route_of("tiles://chat/%2e%2e"), page("/"));
        assert_eq!(route_of("tiles://plugins/exa/../../x"), page("/plugins/x"));
    }

    #[test]
    fn a_new_chat_can_carry_a_draft() {
        assert_eq!(
            route_of("tiles://chat?draft=hello%20world"),
            page("/?draft=hello+world")
        );
        assert_eq!(
            route_of("tiles://chat?draft=a%26q%3D1&q=send"),
            page("/?draft=a%26q%3D1")
        );
        assert_eq!(route_of("tiles://chat?draft="), page("/"));
        assert_eq!(route_of("tiles://chat?draft=%20%20"), page("/"));
    }

    #[test]
    fn a_draft_goes_nowhere_else() {
        assert_eq!(route_of("tiles://chat/abc?draft=hi"), page("/chat/abc"));
        assert_eq!(route_of("tiles://plugins?draft=hi"), page("/plugins"));
        assert_eq!(route_of("tiles://?draft=hi"), Some(Target::Focus));
    }

    #[test]
    fn an_oversize_draft_is_dropped() {
        let long = "a".repeat(DRAFT_MAX + 1);
        assert_eq!(route_of(&format!("tiles://chat?draft={long}")), page("/"));
    }

    #[test]
    fn a_fresh_handoff_is_taken() {
        assert_eq!(
            accept_handoff("tiles://chat/abc\n", Duration::from_secs(1)).as_deref(),
            Some("/chat/abc")
        );
        assert_eq!(
            accept_handoff("tiles://chat?draft=hi%20there", Duration::ZERO).as_deref(),
            Some("/?draft=hi+there")
        );
        assert_eq!(
            accept_handoff("tiles://", Duration::ZERO).as_deref(),
            Some("/")
        );
    }

    #[test]
    fn a_stale_or_bad_handoff_is_dropped() {
        assert_eq!(accept_handoff("tiles://chat/abc", HANDOFF_TTL * 2), None);
        assert_eq!(accept_handoff("tiles://chat/a/b", Duration::ZERO), None);
        assert_eq!(accept_handoff("/etc/passwd", Duration::ZERO), None);
        assert_eq!(accept_handoff("https://chat/abc", Duration::ZERO), None);
    }

    #[test]
    fn argv_links_are_spotted() {
        let with = vec!["tiles-menubar".into(), "tiles://chat/abc".into()];
        let without = vec!["tiles-menubar".into(), "--tiles-daemon-supervised".into()];
        assert!(in_args(&with));
        assert!(!in_args(&without));
    }
}
