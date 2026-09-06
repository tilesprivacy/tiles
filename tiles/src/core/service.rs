//! The launchd agent that brings the daemon up at login
//!
//! This has to be a LaunchAgent rather than a LaunchDaemon: the daemon launches
//! the menu bar app as a child, and a status item can only be drawn inside the
//! user's Aqua session. A LaunchDaemon runs in session 0 and could never do it.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow};

use crate::utils::config::{ConfigProvider, DefaultProvider};

pub const LABEL: &str = "com.tilesprivacy.tiles.daemon";

/// Matches the filter `start_daemon` passes to a hand-spawned daemon
const RUST_LOG: &str = "info,iroh=error,tracing=off";

fn require_macos() -> Result<()> {
    if cfg!(target_os = "macos") {
        Ok(())
    } else {
        Err(anyhow!(
            "`tiles service` is only supported on macOS for now"
        ))
    }
}

pub fn plist_path() -> Result<PathBuf> {
    let home = std::env::home_dir().context("Failed to fetch $HOME")?;
    Ok(home.join(format!("Library/LaunchAgents/{LABEL}.plist")))
}

/// launchd addresses a per-user agent by the uid of the session it lives in
fn domain() -> String {
    format!("gui/{}", nix::unistd::getuid())
}

fn target() -> String {
    format!("{}/{}", domain(), LABEL)
}

pub fn is_installed() -> bool {
    plist_path().map(|path| path.exists()).unwrap_or(false)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn plist(program: &Path, out_log: &Path, err_log: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{program}</string>
    <string>daemon</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <dict>
    <key>SuccessfulExit</key>
    <false/>
  </dict>
  <key>LimitLoadToSessionType</key>
  <string>Aqua</string>
  <key>ProcessType</key>
  <string>Interactive</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>RUST_LOG</key>
    <string>{rust_log}</string>
  </dict>
  <key>StandardOutPath</key>
  <string>{out_log}</string>
  <key>StandardErrorPath</key>
  <string>{err_log}</string>
</dict>
</plist>
"#,
        label = LABEL,
        program = xml_escape(&program.to_string_lossy()),
        rust_log = RUST_LOG,
        out_log = xml_escape(&out_log.to_string_lossy()),
        err_log = xml_escape(&err_log.to_string_lossy()),
    )
}

fn launchctl(args: &[&str]) -> Result<std::process::Output> {
    Command::new("launchctl")
        .args(args)
        .output()
        .with_context(|| format!("Failed to run `launchctl {}`", args.join(" ")))
}

/// Errors carry launchctl's own stderr, it is the only useful diagnostic it gives
fn launchctl_checked(args: &[&str]) -> Result<()> {
    let output = launchctl(args)?;
    if output.status.success() {
        return Ok(());
    }
    let reason = String::from_utf8_lossy(&output.stderr);
    Err(anyhow!(
        "`launchctl {}` failed: {}",
        args.join(" "),
        reason.trim()
    ))
}

pub fn install() -> Result<()> {
    require_macos()?;

    // a debug daemon resolves .tiles_dev off its working directory and launchd
    // hands it `/`, so an installed one would write its config to the root
    if cfg!(debug_assertions) {
        return Err(anyhow!(
            "Refusing to install a debug build, it resolves its config off the working directory"
        ));
    }

    // the binary we are running is the one the agent should run, so a cargo
    // install and a pkg install both point at themselves
    let program = std::env::current_exe().context("Failed to resolve the tiles binary path")?;
    let data_dir = DefaultProvider.get_or_create_data_dir()?;
    let path = plist_path()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create ~/Library/LaunchAgents")?;
    }
    std::fs::write(
        &path,
        plist(
            &program,
            &data_dir.join("logs/daemon.out.log"),
            &data_dir.join("logs/daemon.err.log"),
        ),
    )
    .with_context(|| format!("Failed to write {}", path.display()))?;

    // an older copy has to go first, bootstrap refuses a label already loaded
    let _ = launchctl(&["bootout", &target()]);
    launchctl_checked(&["bootstrap", &domain(), &path.to_string_lossy()])?;
    // a user who disabled it once stays disabled across reinstalls otherwise
    let _ = launchctl(&["enable", &target()]);

    println!("Service installed at {}", path.display());
    Ok(())
}

pub fn uninstall() -> Result<()> {
    require_macos()?;

    let path = plist_path()?;
    unload()?;
    if !path.exists() {
        println!("Service is not installed");
        return Ok(());
    }

    std::fs::remove_file(&path).with_context(|| format!("Failed to remove {}", path.display()))?;

    println!("Service uninstalled");
    Ok(())
}

/// Remove the job from launchd if it is loaded, including partial installs
/// where the plist has already disappeared.
pub(crate) fn unload() -> Result<()> {
    require_macos()?;
    if launchctl(&["print", &target()])?.status.success() {
        launchctl_checked(&["bootout", &target()])?;
    }
    Ok(())
}

/// Kickstart rather than a plain start, so a wedged daemon is replaced
pub fn start() -> Result<()> {
    require_macos()?;
    if !is_installed() {
        return Err(anyhow!(
            "Service is not installed, run `tiles service install`"
        ));
    }
    launchctl_checked(&["kickstart", "-k", &target()])
}

/// Booting the agent out sends SIGTERM, which the daemon turns into a clean shutdown
pub fn stop() -> Result<()> {
    require_macos()?;
    if !is_installed() {
        return Err(anyhow!(
            "Service is not installed, run `tiles service install`"
        ));
    }
    launchctl_checked(&["bootout", &target()])
}

pub async fn status() -> Result<()> {
    require_macos()?;

    let path = plist_path()?;
    if !path.exists() {
        println!("Service:  not installed");
        println!("Run `tiles service install` to start Tiles at login");
        return Ok(());
    }
    println!("Service:  installed at {}", path.display());

    let loaded = launchctl(&["print", &target()])?;
    println!(
        "launchd:  {}",
        if loaded.status.success() {
            "loaded"
        } else {
            "not loaded"
        }
    );

    match crate::daemon::ping(None).await {
        Ok(vsn) => println!("Daemon:   up, version {}", vsn.trim()),
        Err(_) => println!("Daemon:   down"),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// launchd refuses a malformed plist without saying so, and the template is
    /// hand-written, so the parse is the thing worth checking. plutil ships with
    /// the os, and ci runs the suite on linux
    #[test]
    #[cfg(target_os = "macos")]
    fn plist_is_well_formed() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let path = dir.path().join("agent.plist");
        std::fs::write(
            &path,
            plist(
                Path::new("/usr/local/bin/tiles"),
                Path::new("/tmp/daemon.out.log"),
                Path::new("/tmp/daemon.err.log"),
            ),
        )
        .expect("the plist writes");

        let lint = Command::new("plutil")
            .arg("-lint")
            .arg(&path)
            .output()
            .expect("plutil is on every mac");
        assert!(
            lint.status.success(),
            "{}",
            String::from_utf8_lossy(&lint.stdout)
        );
    }

    #[test]
    fn paths_with_xml_metacharacters_survive() {
        let rendered = plist(
            Path::new("/Users/a&b/<tiles>"),
            Path::new("/tmp/out.log"),
            Path::new("/tmp/err.log"),
        );
        assert!(rendered.contains("/Users/a&amp;b/&lt;tiles&gt;"));
    }
}
