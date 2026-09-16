//! Auto installing Tiles
//!
//! We will be fetching the latest revision form github (where we host the binaries)
//! We will be using the installer script under `tiles/scripts/installer.sh` to
//! install Tiles, just fetching the script and running as bash script from Rust.

use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{Result, anyhow};
use nix::unistd::{AccessFlags, access};
use reqwest::{Client, header::HeaderMap};
use semver::Version;
use serde::Deserialize;

use crate::utils::config::{
    ConfigProvider, DefaultProvider, SYSTEM_BIN_DIR, SYSTEM_BIN_PATH, SYSTEM_LIB_DIR,
    is_tiles_lib_dir,
};

const RELEASES_BASE_ENDPOINT: &str = "https://api.github.com";
const RELEASES_REST_PATH: &str = "repos/tilesprivacy/tiles/releases/latest";
const HEADER_PARSING_ERROR: &str = "Failed to parse header";
#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

pub struct UpdateInfo {
    pub can_update: bool,
    pub latest_version: String,
    pub current_version: String,
}

#[derive(Debug, PartialEq, Eq)]
struct UpdateInstallLayout {
    install_dir: PathBuf,
    lib_dir: PathBuf,
    is_system_path: bool,
}

fn resolve_update_install_layout(
    current_exe: &Path,
    user_bin_path: &Path,
    user_lib_dir: &Path,
) -> Result<UpdateInstallLayout> {
    if let Some(portable_dir) = current_exe.parent()
        && is_tiles_lib_dir(portable_dir)
    {
        return Ok(UpdateInstallLayout {
            install_dir: portable_dir.to_path_buf(),
            lib_dir: portable_dir.to_path_buf(),
            is_system_path: false,
        });
    }

    if current_exe == Path::new(SYSTEM_BIN_PATH) {
        return Ok(UpdateInstallLayout {
            install_dir: PathBuf::from(SYSTEM_BIN_DIR),
            lib_dir: PathBuf::from(SYSTEM_LIB_DIR),
            is_system_path: true,
        });
    }

    if current_exe == user_bin_path {
        let install_dir = user_bin_path
            .parent()
            .ok_or_else(|| anyhow!("Failed to resolve the user binary directory"))?;
        return Ok(UpdateInstallLayout {
            install_dir: install_dir.to_path_buf(),
            lib_dir: user_lib_dir.to_path_buf(),
            is_system_path: false,
        });
    }

    Err(anyhow!(
        "Cannot update Tiles from unsupported installation path {}",
        current_exe.display()
    ))
}

fn nearest_existing_ancestor(path: &Path) -> Option<&Path> {
    path.ancestors().find(|ancestor| ancestor.exists())
}

fn is_destination_writable(path: &Path) -> bool {
    nearest_existing_ancestor(path).is_some_and(|ancestor| {
        ancestor.is_dir() && access(ancestor, AccessFlags::W_OK | AccessFlags::X_OK).is_ok()
    })
}

fn resolve_update_elevation(
    layout: &UpdateInstallLayout,
    check_writable: impl Fn(&Path) -> bool,
) -> Result<bool> {
    let mut destinations = vec![layout.install_dir.as_path()];
    if layout.lib_dir != layout.install_dir {
        destinations.push(layout.lib_dir.as_path());
    }

    let unwritable = destinations
        .into_iter()
        .filter(|path| !check_writable(path))
        .collect::<Vec<_>>();
    if unwritable.is_empty() {
        return Ok(false);
    }
    if layout.is_system_path {
        return Ok(true);
    }

    let paths = unwritable
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(anyhow!(
        "Tiles installation path is not writable: {paths}. Fix its ownership or permissions and try again. The updater will not use sudo for a non-system installation because that would create root-owned files."
    ))
}

pub async fn try_update(update_info: Option<UpdateInfo>) -> Result<String> {
    let app_update_info = if let Some(info) = update_info {
        info
    } else {
        get_update_info().await?
    };

    if !app_update_info.can_update {
        let msg = format!(
            "You are up to date, current version: {}",
            app_update_info.current_version
        );
        Ok(msg)
    } else {
        let provider = DefaultProvider;
        let layout = resolve_update_install_layout(
            &env::current_exe()?,
            &provider.get_user_bin_path()?,
            &provider.get_data_dir()?,
        )?;
        let needs_elevation = resolve_update_elevation(&layout, is_destination_writable)?;
        let mut curl_process = Command::new("curl")
            .arg("-fsSL")
            .arg("https://tiles.run/install.sh")
            .stdout(Stdio::piped())
            .spawn()?;

        let mut install_command = if needs_elevation {
            let mut command = Command::new("sudo");
            command.arg("sh");
            command
        } else {
            Command::new("sh")
        };

        let run_sh_cmd_status = install_command
            .arg("-s")
            .arg("--")
            .arg("--install-dir")
            .arg(layout.install_dir)
            .arg("--lib-dir")
            .arg(layout.lib_dir)
            .stdin(
                curl_process
                    .stdout
                    .take()
                    .ok_or_else(|| anyhow!("Failed to pipe from the curled input"))?,
            )
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if run_sh_cmd_status.success() {
            Ok(format!(
                "Tiles updated to {}",
                app_update_info.latest_version
            ))
        } else {
            Ok("Tiles failed to update".to_owned())
        }
    }
}

pub async fn get_update_info() -> Result<UpdateInfo> {
    let latest_vsn = get_latest_version(RELEASES_BASE_ENDPOINT).await?;
    let req_vsn = Version::parse(&latest_vsn)?;
    let current_vsn = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|e| anyhow!("Failed to parse pkg version due to {}", e))?;

    if req_vsn.cmp_precedence(&current_vsn).is_gt() {
        Ok(UpdateInfo {
            can_update: true,
            latest_version: req_vsn.to_string(),
            current_version: current_vsn.to_string(),
        })
    } else {
        Ok(UpdateInfo {
            can_update: false,
            latest_version: req_vsn.to_string(),
            current_version: current_vsn.to_string(),
        })
    }
}

/// Gets the latest Tiles version
///
/// Returns a Err(String), on API failure
pub async fn get_latest_version(base_url: &str) -> Result<String> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-GitHub-Api-Version",
        "2022-11-28".parse().expect(HEADER_PARSING_ERROR),
    );
    headers.insert(
        "Accept",
        "application/vnd.github+json"
            .parse()
            .expect(HEADER_PARSING_ERROR),
    );
    headers.insert("user-agent", "Tiles".parse().expect(HEADER_PARSING_ERROR));
    let client_builder = Client::builder()
        .timeout(Duration::from_secs(5))
        .default_headers(headers);

    let client = client_builder.build()?;
    let response = client
        .get(format!("{}/{}", base_url, RELEASES_REST_PATH))
        .send()
        .await;

    match response {
        Err(err) if err.is_timeout() => Err(anyhow!("Request failed due to Api timedout")),
        Err(err) => Err(anyhow!("Request failed due to {:?}", err)),
        Ok(res) if res.status() == 200 => {
            let release = res.json::<Release>().await?;
            Ok(release.tag_name)
        }
        Ok(res) => Err(anyhow!("Api failed with status {}", res.status())),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;
    use serde_json::json;

    #[test]
    fn resolves_system_update_layout() {
        let layout = resolve_update_install_layout(
            Path::new(SYSTEM_BIN_PATH),
            Path::new("/home/user/.local/bin/tiles"),
            Path::new("/home/user/.local/share/tiles"),
        )
        .unwrap();

        assert_eq!(layout.install_dir, Path::new(SYSTEM_BIN_DIR));
        assert_eq!(layout.lib_dir, Path::new(SYSTEM_LIB_DIR));
        assert!(layout.is_system_path);
    }

    #[test]
    fn resolves_user_update_layout() {
        let user_bin = Path::new("/home/user/.local/bin/tiles");
        let user_lib = Path::new("/home/user/custom-data/tiles");
        let layout = resolve_update_install_layout(user_bin, user_bin, user_lib).unwrap();

        assert_eq!(layout.install_dir, Path::new("/home/user/.local/bin"));
        assert_eq!(layout.lib_dir, user_lib);
        assert!(!layout.is_system_path);
    }

    #[test]
    fn resolves_portable_update_layout() {
        let root = tempdir().unwrap();
        for component in ["server", "modelfiles", "pi"] {
            std::fs::create_dir(root.path().join(component)).unwrap();
        }

        let layout = resolve_update_install_layout(
            &root.path().join("tiles"),
            Path::new("/home/user/.local/bin/tiles"),
            Path::new("/home/user/.local/share/tiles"),
        )
        .unwrap();

        assert_eq!(layout.install_dir, root.path());
        assert_eq!(layout.lib_dir, root.path());
        assert!(!layout.is_system_path);
    }

    #[test]
    fn rejects_unknown_update_layout() {
        let result = resolve_update_install_layout(
            Path::new("/opt/tiles/bin/tiles"),
            Path::new("/home/user/.local/bin/tiles"),
            Path::new("/home/user/.local/share/tiles"),
        );

        assert!(result.is_err());
    }

    #[test]
    fn uses_elevation_only_for_unwritable_system_layout() {
        let system_layout = UpdateInstallLayout {
            install_dir: PathBuf::from(SYSTEM_BIN_DIR),
            lib_dir: PathBuf::from(SYSTEM_LIB_DIR),
            is_system_path: true,
        };
        assert!(!resolve_update_elevation(&system_layout, |_| true).unwrap());
        assert!(resolve_update_elevation(&system_layout, |_| false).unwrap());

        for layout in [
            UpdateInstallLayout {
                install_dir: PathBuf::from("/home/user/.local/bin"),
                lib_dir: PathBuf::from("/home/user/.local/share/tiles"),
                is_system_path: false,
            },
            UpdateInstallLayout {
                install_dir: PathBuf::from("/opt/portable-tiles"),
                lib_dir: PathBuf::from("/opt/portable-tiles"),
                is_system_path: false,
            },
        ] {
            let error = resolve_update_elevation(&layout, |_| false).unwrap_err();
            assert!(error.to_string().contains("will not use sudo"));
        }
    }

    #[test]
    fn checks_nearest_existing_ancestor_for_new_destinations() {
        let root = tempdir().unwrap();
        let destination = root.path().join("missing/nested/directory");

        assert_eq!(nearest_existing_ancestor(&destination), Some(root.path()));
        assert!(is_destination_writable(&destination));
    }

    #[tokio::test]
    async fn test_get_latest_version() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/tilesprivacy/tiles/releases/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!(
                {
                    "tag_name": "0.4.1"
                }
            )))
            .mount(&mock_server)
            .await;

        let tag = get_latest_version(mock_server.uri().as_str())
            .await
            .unwrap();
        assert_eq!(tag, "0.4.1".to_owned())
    }

    #[tokio::test]
    async fn test_get_latest_version_failed_due_to_timeout() {
        let delay = Duration::from_secs(30); // 30s
        let mock_server = MockServer::start().await;
        let path_str = format!("/{}", RELEASES_REST_PATH);
        Mock::given(method("GET"))
            .and(path(path_str))
            .respond_with(ResponseTemplate::new(200).set_delay(delay))
            .mount(&mock_server)
            .await;

        let server = mock_server.uri();
        let res = async_std::future::timeout(delay / 2, get_latest_version(server.as_str())).await;
        assert!(res.unwrap().is_err());
    }

    #[tokio::test]
    async fn test_get_latest_version_err_4xx() {
        let mock_server = MockServer::start().await;
        let path_str = format!("/{}", RELEASES_REST_PATH);

        Mock::given(method("GET"))
            .and(path(path_str))
            .respond_with(ResponseTemplate::new(403).set_body_json(json!(
            {
                "err": "unauth"
            })))
            .mount(&mock_server)
            .await;

        let tag = get_latest_version(mock_server.uri().as_str()).await;
        assert!(tag.is_err())
    }

    /// The updater once "upgraded" canary builds to the older stable release,
    /// because canary still carried the last stable's version. The downgraded
    /// binary then met a database schema from the future and died on every
    /// start. A canary version must outrank the stable it was cut after, and
    /// still lose to the release that follows it.
    #[test]
    fn a_canary_build_outranks_stable_and_yields_to_the_next_release() {
        let canary = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        let stable = Version::parse("0.4.19").unwrap();
        let next = Version::parse("0.4.20").unwrap();

        assert!(canary.cmp_precedence(&stable).is_gt());
        assert!(next.cmp_precedence(&canary).is_gt());
    }
}
