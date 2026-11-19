//! Opening a link in the user's browser

/// Opens `url` in the default browser, if there's one. Needs a tokio runtime
pub fn open_in_browser(url: &str) {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    match tokio::process::Command::new(opener).arg(url).spawn() {
        // waited on, so finished openers don't pile up as zombies
        Ok(mut child) => {
            tokio::spawn(async move {
                let _ = child.wait().await;
            });
        }
        Err(err) => log::debug!("Could not open {url} with {opener}: {err}"),
    }
}
