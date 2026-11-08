//! The login shell's PATH, for a daemon started with launchd's bare one

use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{LazyLock, mpsc},
    time::{Duration, Instant},
};

const LAUNCHD: [&str; 4] = ["/usr/bin", "/bin", "/usr/sbin", "/sbin"];

const MARK: &str = "_TILES_PATH_";

const TIMEOUT: Duration = Duration::from_secs(10);

const MAX_OUTPUT: usize = 1 << 20;

static PATH: LazyLock<OsString> = LazyLock::new(|| {
    let inherited = env::var_os("PATH").unwrap_or_default();
    if !is_launchds(&inherited) {
        return inherited;
    }
    login_shell()
        .and_then(|shell| login_shell_path(&shell, &probe(&shell), TIMEOUT))
        .unwrap_or_else(|| with_install_dirs(&inherited, env::home_dir().as_deref()))
});

pub fn for_tools() -> &'static OsStr {
    &PATH
}

pub fn warm_up() {
    std::thread::spawn(|| {
        for_tools();
    });
}

/// The shell in the user's account, not `$SHELL`, and only one /etc/shells lists
fn login_shell() -> Option<String> {
    let user = nix::unistd::User::from_uid(nix::unistd::getuid()).ok()??;
    let shell = user.shell.to_str()?.to_owned();
    let listed = fs::read_to_string("/etc/shells").ok()?;
    is_listed(&shell, &listed).then_some(shell)
}

fn is_listed(shell: &str, shells: &str) -> bool {
    shells
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .any(|line| line == shell)
}

fn is_launchds(path: &OsStr) -> bool {
    env::split_paths(path)
        .filter(|dir| !dir.as_os_str().is_empty())
        .all(|dir| LAUNCHD.iter().any(|bare| dir == Path::new(bare)))
}

fn probe(shell: &str) -> String {
    let print = format!(r#"exec /bin/sh -c 'printf "%s%s%s" {MARK} "$PATH" {MARK}'"#);
    let name = Path::new(shell)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    if matches!(name, "bash" | "zsh" | "sh" | "dash" | "ksh") {
        // a -c bash or zsh saves its history on exit without loading it first
        format!("unset HISTFILE 2>/dev/null; {print}")
    } else {
        print
    }
}

fn login_shell_path(shell: &str, probe: &str, timeout: Duration) -> Option<OsString> {
    let mut command = Command::new(shell);
    command
        .args(["-ilc", probe])
        // Oh My Zsh would ask about updating
        .env("DISABLE_AUTO_UPDATE", "true")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(home) = env::home_dir() {
        command.current_dir(home);
    }
    let mut child = command.spawn().ok()?;
    let mut stdout = child.stdout.take()?;

    // bounded, a chatty startup script waits instead of filling memory
    let (tx, rx) = mpsc::sync_channel(16);
    std::thread::spawn(move || {
        let mut chunk = [0; 4096];
        while let Ok(read) = stdout.read(&mut chunk) {
            if read == 0 || tx.send(chunk[..read].to_vec()).is_err() {
                break;
            }
        }
    });

    let deadline = Instant::now() + timeout;
    let mut output = Vec::new();
    let path = loop {
        if let Some(path) = between_marks(&output) {
            break Some(path);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(chunk) if output.len() < MAX_OUTPUT => output.extend(chunk),
            _ => break None,
        }
    };
    let _ = child.kill();
    std::thread::spawn(move || child.wait());
    path.filter(|path| !path.is_empty())
}

fn between_marks(output: &[u8]) -> Option<OsString> {
    let text = String::from_utf8_lossy(output);
    let (_, rest) = text.split_once(MARK)?;
    let (path, _) = rest.split_once(MARK)?;
    Some(path.into())
}

fn with_install_dirs(inherited: &OsStr, home: Option<&Path>) -> OsString {
    let mut dirs: Vec<PathBuf> = env::split_paths(inherited)
        .filter(|dir| !dir.as_os_str().is_empty())
        .collect();
    let installs = [
        Some(PathBuf::from("/usr/local/bin")),
        Some(PathBuf::from("/opt/homebrew/bin")),
        home.map(|home| home.join(".local/bin")),
    ];
    for dir in installs.into_iter().flatten() {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    env::join_paths(dirs).unwrap_or_else(|_| inherited.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHORT: Duration = Duration::from_millis(500);

    #[test]
    fn only_launchds_path_needs_the_shell() {
        assert!(is_launchds(OsStr::new("/usr/bin:/bin:/usr/sbin:/sbin")));
        assert!(is_launchds(OsStr::new("")));
        assert!(!is_launchds(OsStr::new("/usr/bin:/bin:/opt/homebrew/bin")));
    }

    #[test]
    fn the_login_shell_gives_its_path() {
        let path = login_shell_path("/bin/sh", &probe("/bin/sh"), TIMEOUT).unwrap();
        assert!(
            env::split_paths(&path)
                .any(|dir| dir == Path::new("/bin") || dir == Path::new("/usr/bin")),
            "{path:?}"
        );
    }

    #[test]
    fn a_shell_that_never_answers_is_given_up_on() {
        let started = Instant::now();
        assert_eq!(login_shell_path("/bin/sh", "sleep 30", SHORT), None);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_job_left_holding_stdout_doesnt_hold_the_answer() {
        let probe = format!("sleep 30 & printf '%s/opt/tools%s' {MARK} {MARK}");
        let started = Instant::now();
        assert_eq!(
            login_shell_path("/bin/sh", &probe, TIMEOUT),
            Some(OsString::from("/opt/tools"))
        );
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_missing_shell_falls_back() {
        assert_eq!(login_shell_path("/nonexistent/shell", "true", SHORT), None);
    }

    #[test]
    fn only_a_shell_etc_shells_lists_runs() {
        let shells =
            "# Pathnames of valid login shells.\n/bin/sh\n/bin/zsh\n/opt/homebrew/bin/fish\n";
        assert!(is_listed("/bin/zsh", shells));
        assert!(is_listed("/opt/homebrew/bin/fish", shells));
        assert!(!is_listed("/tmp/shell", shells));
        assert!(!is_listed("# Pathnames of valid login shells.", shells));
    }

    #[test]
    fn bash_and_zsh_keep_their_history() {
        assert!(probe("/bin/zsh").starts_with("unset HISTFILE"));
        assert!(!probe("/opt/homebrew/bin/fish").contains("HISTFILE"));
    }

    #[test]
    fn the_fallback_adds_the_usual_install_folders() {
        let home = Some(Path::new("/home/ada"));
        assert_eq!(
            with_install_dirs(OsStr::new("/usr/bin:/bin:/usr/sbin:/sbin"), home),
            "/usr/bin:/bin:/usr/sbin:/sbin:/usr/local/bin:/opt/homebrew/bin:/home/ada/.local/bin"
        );
        assert_eq!(
            with_install_dirs(OsStr::new("/home/ada/.local/bin:/usr/local/bin"), home),
            "/home/ada/.local/bin:/usr/local/bin:/opt/homebrew/bin"
        );
        assert_eq!(
            with_install_dirs(OsStr::new(""), None),
            "/usr/local/bin:/opt/homebrew/bin"
        );
    }
}
