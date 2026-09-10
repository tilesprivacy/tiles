//! Plugin system
//!
//! Plugins follow the Agent Plugins spec (<https://agent-plugins.org/>):
//! a `plugin.json` manifest at the root, skills in `skills/`, MCP servers in
//! `mcp.json`, and client-specific files under a reverse-domain directory.
//! Ours is `run.tiles/`, which is where Pi extensions live.
//!
//! ```text
//! my-plugin/
//!   plugin.json
//!   skills/deploy/SKILL.md
//!   mcp.json
//!   run.tiles/extensions/my-ext/index.ts
//! ```
//!
//! `mcp.json` is read by the bundled MCP adapter, not by us. We only point it
//! at the installed plugin roots via the `agentPluginPaths` setting.

use std::{
    collections::BTreeMap,
    env,
    fs::{File, remove_dir_all},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::Command,
    str::FromStr,
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use log::info;
use reqwest::Client;
use tempfile::{TempDir, tempdir};

use crate::utils::{
    config::{ConfigProvider, DefaultProvider, get_disabled_plugins, set_plugin_disabled},
    copy_recursive,
};

/// Spec versions we accept. The bundled adapter only recognises 1.0.0, so a
/// 1.1.0 plugin installs but its MCP servers stay dormant.
const PLUGIN_SCHEMA_1_0_0: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";
const PLUGIN_SCHEMA_1_1_0: &str = "https://agent-plugins.org/schemas/1.1.0/plugin.schema.json";

/// Every top-level field the spec allows in `plugin.json`.
const MANIFEST_FIELDS: [&str; 10] = [
    "$schema",
    "name",
    "version",
    "description",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
    "extensions",
];

/// Our reverse-domain namespace for client-specific plugin files.
const CLIENT_NAMESPACE: &str = "run.tiles";

#[derive(Debug, PartialEq, Eq)]
pub struct AgentPluginManifest {
    pub name: String,
    /// What the plugin is for, shown to users instead of its internals.
    pub description: Option<String>,
    /// True when the manifest targets a spec version the adapter cannot read.
    pub mcp_dormant: bool,
}

pub async fn install(path: String) -> Result<String> {
    if let Ok(url) = reqwest::Url::parse(&path)
        && matches!(url.scheme(), "http" | "https")
    {
        info!("Online Plugin");

        // No extension check on the url. Hosts commonly serve an archive from a
        // path with no extension, or pick the format from a query parameter, so
        // the downloaded bytes decide the format instead.
        let client = Client::builder().timeout(Duration::from_secs(60)).build()?;

        println!("Downloading plugin from {}..", url);

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|err| anyhow!("Failed to download the plugin due to {:?}", err))?
            .error_for_status()
            .map_err(|err| anyhow!("Failed to download the plugin: {}", err))?;

        let mut tmp_path = env::temp_dir();
        tmp_path.push("tiles-plugin-download");
        let mut plugin_file = File::create(&tmp_path)?;
        plugin_file.write_all(&response.bytes().await?)?;
        info!("Wrote the plugin to tmp path {:?}", tmp_path);
        install_from_local_source(tmp_path)
    } else {
        info!("Local Plugin");
        let local_path = PathBuf::from_str(&path).context("Invalid local path")?;
        install_from_local_source(local_path)
    }
}

fn install_from_local_source(local_path: PathBuf) -> Result<String> {
    if !matches!(local_path.try_exists(), Ok(true)) {
        return Err(anyhow!("{:?} does not exist", local_path));
    }

    // A plugin is just a folder, so take one directly. No archive step means a
    // person or an agent can read a plugin in place with ordinary file tools.
    if local_path.is_dir() {
        if !is_plugin_folder(&local_path) {
            return Err(anyhow!(
                "{:?} has no plugin.json, so it is not a plugin folder. See https://agent-plugins.org/",
                local_path
            ));
        }
        return install_plugin_root(&local_path);
    }

    let kind = ArchiveKind::detect(&local_path)?;

    // `_tmp_dir` must outlive the copy, so bind it here
    let (_tmp_dir, plugin_root) = unpack_archive(&local_path, kind)?;
    install_plugin_root(&plugin_root)
}

/// Unpacks an archive into a temp dir and finds the plugin root inside it.
/// The returned `TempDir` owns the files, so keep it alive while copying.
fn unpack_archive(local_path: &Path, kind: ArchiveKind) -> Result<(TempDir, PathBuf)> {
    let file_name = local_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| anyhow!("{:?} has no file name", local_path))?;
    let archive_stem = file_name.split(".").collect::<Vec<&str>>()[0].to_owned();

    // Reject path traversal before unpacking, not after.
    assert_archive_entries_contained(local_path, kind)?;

    let tmp_dir = tempdir().context("Failed to create tmp dir")?;
    let tmp_path = tmp_dir.path().join("tmp_tiles_plugins");
    std::fs::create_dir_all(&tmp_path).context("Failed to create temporary plugins directory")?;

    let output = kind.extract_command(local_path, &tmp_path).output()?;

    if !output.status.success() {
        return Err(anyhow!("{}", String::from_utf8_lossy(&output.stderr)));
    }

    let plugin_root = locate_plugin_root(&tmp_path, &archive_stem)?;
    Ok((tmp_dir, plugin_root))
}

/// Validates a plugin folder and copies it into the installed plugins dir.
fn install_plugin_root(plugin_root: &Path) -> Result<String> {
    let manifest = read_manifest(plugin_root)?;
    let installed_root = plugins_dir()?.join(&manifest.name);

    // Installing a folder over itself would delete it before the copy.
    let same_place = std::fs::canonicalize(plugin_root)
        .ok()
        .zip(std::fs::canonicalize(&installed_root).ok())
        .is_some_and(|(from, to)| from == to);
    if same_place {
        return Ok(format!(
            "Plugin {} is already installed here",
            manifest.name
        ));
    }

    if installed_root.exists() {
        remove_dir_all(&installed_root)
            .context("Failed to replace the previously installed plugin")?;
    }

    // A half-copied plugin is worse than none, so undo on any failure.
    if let Err(err) = copy_recursive(plugin_root, &installed_root) {
        let _ = remove_dir_all(&installed_root);
        return Err(err);
    }

    let mut message = format!("Successfully installed plugin {}", manifest.name);
    if manifest.mcp_dormant {
        message.push_str(
            "\nNote: this plugin targets Agent Plugins 1.1.0. Skills and \
             extensions work, but its MCP servers stay dormant until the \
             bundled adapter supports 1.1.0.",
        );
    }
    Ok(message)
}

/// Removes skills that older versions copied into the shared skills dir.
///
/// Those copies would now load a second time alongside the plugin's own
/// `--skill` directory. Runs once, guarded by a marker, because matching only
/// by directory name could otherwise keep deleting a hand-written skill that
/// happens to share a plugin skill's name.
pub fn prune_copied_plugin_skills() {
    let Ok(shared) = skills_dir() else {
        return;
    };
    let marker = shared.join(".plugin-skills-migrated");
    if marker.exists() || !shared.is_dir() {
        return;
    }

    for root in all_plugin_roots() {
        let Ok(entries) = std::fs::read_dir(root.join("skills")) else {
            continue;
        };
        for entry in entries.filter_map(|entry| entry.ok()) {
            let copied = shared.join(entry.file_name());
            if copied.is_dir() {
                let _ = remove_dir_all(&copied);
            }
        }
    }
    let _ = std::fs::write(&marker, b"skills now load from plugin roots\n");
}

/// Skill directories from every enabled plugin, passed to Pi as `--skill`.
///
/// Skills used to be copied into the shared skills dir at install time. That
/// left bundled plugins (which never run an install) with dead skills, and kept
/// a disabled plugin's skills loaded. Pointing Pi at the plugin's own directory
/// keeps all three resource types switching on and off together.
pub fn installed_skill_dirs() -> Vec<PathBuf> {
    installed_plugin_roots()
        .into_iter()
        .map(|root| root.join("skills"))
        .filter(|dir| dir.is_dir())
        .collect()
}

/// Lists the archive and rejects entries that would escape the extract dir.
fn assert_archive_entries_contained(local_path: &Path, kind: ArchiveKind) -> Result<()> {
    let output = kind.list_command(local_path).output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to read the plugin archive: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    for entry in String::from_utf8_lossy(&output.stdout).lines() {
        if !is_contained_entry(entry) {
            return Err(anyhow!(
                "Refusing to install: plugin archive entry {:?} escapes the plugin directory",
                entry
            ));
        }
    }
    Ok(())
}

/// An archive entry is safe when it is relative and never walks upwards.
fn is_contained_entry(entry: &str) -> bool {
    let entry = entry.trim();
    if entry.is_empty() {
        return true;
    }
    if entry.starts_with('/') || entry.starts_with("\\") || entry.contains(':') {
        return false;
    }
    Path::new(entry)
        .components()
        .all(|part| matches!(part, Component::Normal(_) | Component::CurDir))
}

/// The plugin root is wherever `plugin.json` sits: the extract dir itself, the
/// directory named after the archive, or a lone top-level directory.
fn locate_plugin_root(extract_dir: &Path, archive_stem: &str) -> Result<PathBuf> {
    if extract_dir.join("plugin.json").is_file() {
        return Ok(extract_dir.to_path_buf());
    }

    let stem_root = extract_dir.join(archive_stem);
    if stem_root.join("plugin.json").is_file() {
        return Ok(stem_root);
    }

    let mut dirs = std::fs::read_dir(extract_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<PathBuf>>();

    if dirs.len() == 1
        && let Some(only) = dirs.pop()
        && only.join("plugin.json").is_file()
    {
        return Ok(only);
    }

    Err(anyhow!(
        "Not a valid Agent Plugin: no plugin.json found. Expected a plugin.json \
         at the archive root. See https://agent-plugins.org/"
    ))
}

/// Validates `plugin.json` per the spec: unknown fields are reported and
/// ignored, everything else is fatal.
fn read_manifest(plugin_root: &Path) -> Result<AgentPluginManifest> {
    let manifest_path = plugin_root.join("plugin.json");
    let raw = std::fs::read_to_string(&manifest_path)
        .context("Not a valid Agent Plugin: could not read plugin.json")?;
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).context("Not a valid Agent Plugin: plugin.json is not JSON")?;
    parse_manifest(&parsed)
}

fn parse_manifest(parsed: &serde_json::Value) -> Result<AgentPluginManifest> {
    let object = parsed
        .as_object()
        .ok_or_else(|| anyhow!("Not a valid Agent Plugin: plugin.json must be an object"))?;

    for key in object.keys() {
        if !MANIFEST_FIELDS.contains(&key.as_str()) {
            println!("Ignoring unknown plugin.json field: {}", key);
        }
    }

    let schema = object.get("$schema").and_then(|value| value.as_str());
    let mcp_dormant = match schema {
        Some(PLUGIN_SCHEMA_1_0_0) => false,
        Some(PLUGIN_SCHEMA_1_1_0) => true,
        _ => {
            return Err(anyhow!(
                "Not a valid Agent Plugin: plugin.json $schema must be {} or {}",
                PLUGIN_SCHEMA_1_0_0,
                PLUGIN_SCHEMA_1_1_0
            ));
        }
    };

    let name = object
        .get("name")
        .and_then(|value| value.as_str())
        .filter(|name| is_valid_plugin_name(name))
        .ok_or_else(|| {
            anyhow!(
                "Not a valid Agent Plugin: plugin.json name must be 1-64 chars of \
                 lowercase letters, digits, dots and dashes"
            )
        })?;

    Ok(AgentPluginManifest {
        name: name.to_owned(),
        description: object
            .get("description")
            .and_then(|value| value.as_str())
            .map(str::to_owned),
        mcp_dormant,
    })
}

/// Spec name rule: `^(?!.*(?:--|\.\.))[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?$`, max 64.
fn is_valid_plugin_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    if name.contains("--") || name.contains("..") {
        return false;
    }
    let alnum = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    if !name.chars().all(|c| alnum(c) || c == '.' || c == '-') {
        return false;
    }
    name.starts_with(alnum) && name.ends_with(alnum)
}

/// Which archive tool to use. Detected from the file's leading bytes rather
/// than its name, because plenty of hosts serve an archive from a URL that
/// carries no extension, or picks the format from a query parameter.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ArchiveKind {
    Zip,
    TarGz,
}

impl ArchiveKind {
    fn detect(path: &Path) -> Result<Self> {
        let mut magic = [0u8; 4];
        let read = File::open(path)
            .context("Failed to open the plugin archive")?
            .read(&mut magic)?;
        match &magic[..read] {
            [0x1f, 0x8b, ..] => Ok(ArchiveKind::TarGz),
            [0x50, 0x4b, ..] => Ok(ArchiveKind::Zip),
            _ => Err(anyhow!(
                "That is not a plugin. Expected a folder, or a .tar.gz or .zip archive."
            )),
        }
    }

    /// Command that lists entries without extracting them.
    fn list_command(&self, path: &Path) -> Command {
        let mut command = match self {
            ArchiveKind::Zip => {
                let mut c = Command::new("unzip");
                c.arg("-Z1");
                c
            }
            ArchiveKind::TarGz => {
                let mut c = Command::new("tar");
                c.arg("-tzf");
                c
            }
        };
        command.arg(path);
        command
    }

    fn extract_command(&self, path: &Path, into: &Path) -> Command {
        match self {
            ArchiveKind::Zip => {
                let mut c = Command::new("unzip");
                c.arg(path).arg("-d").arg(into);
                c
            }
            ArchiveKind::TarGz => {
                let mut c = Command::new("tar");
                c.arg("-xzf").arg(path).arg("-C").arg(into);
                c
            }
        }
    }
}

/// A folder is a plugin when it has a manifest at its root.
fn is_plugin_folder(path: &Path) -> bool {
    path.join("plugin.json").is_file()
}

fn skills_dir() -> Result<PathBuf> {
    Ok(DefaultProvider.get_user_data_dir()?.join("pi/agent/skills"))
}

/// Where `tiles plugin install` writes. Created on demand.
pub fn plugins_dir() -> Result<PathBuf> {
    let dir = DefaultProvider
        .get_user_data_dir()?
        .join("pi/agent/plugins");
    std::fs::create_dir_all(&dir).context("Failed to create Pi plugins directory")?;
    Ok(dir)
}

/// Plugins shipped with Tiles, beside `vendor/` in the lib dir. Read-only and
/// replaced on upgrade, so never create it: on a normal install the lib dir is
/// root-owned, and a failed mkdir here would hide the user's plugins too.
pub fn bundled_plugins_dir() -> Option<PathBuf> {
    let dir = DefaultProvider.get_lib_dir().ok()?.join("plugins");
    dir.is_dir().then_some(dir)
}

fn plugin_roots_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.join("plugin.json").is_file())
        .collect()
}

fn plugin_name_of(root: &Path) -> Option<String> {
    root.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

/// True when this root is shipped with Tiles rather than user-installed.
pub fn is_bundled(root: &Path) -> bool {
    bundled_plugins_dir().is_some_and(|dir| root.starts_with(dir))
}

/// Every plugin root, bundled and user-installed, keyed by name so a
/// user-installed plugin shadows a bundled one and neither is listed twice.
/// Ordering is by name, which keeps `-e` flags and `agentPluginPaths` stable
/// across platforms.
fn plugin_roots_by_name() -> BTreeMap<String, PathBuf> {
    let mut roots = BTreeMap::new();
    let bundled = bundled_plugins_dir().map(|dir| plugin_roots_in(&dir));
    let user = plugins_dir().ok().map(|dir| plugin_roots_in(&dir));

    // user last so it wins on a name collision
    for root in bundled.into_iter().chain(user).flatten() {
        if let Some(name) = plugin_name_of(&root) {
            roots.insert(name, root);
        }
    }
    roots
}

/// Enabled plugin roots, for the adapter's `agentPluginPaths` setting and for
/// the `-e` / `--skill` flags. Disabled plugins are dropped here so all three
/// resource types switch off together.
pub fn installed_plugin_roots() -> Vec<PathBuf> {
    let disabled = get_disabled_plugins();
    plugin_roots_by_name()
        .into_iter()
        .filter(|(name, _)| !disabled.contains(name))
        .map(|(_, root)| root)
        .collect()
}

/// Every plugin root including disabled ones, for listing.
pub fn all_plugin_roots() -> Vec<PathBuf> {
    plugin_roots_by_name().into_values().collect()
}

/// Pi extension entrypoints from every installed plugin, for `pi -e`.
/// Accepts both `run.tiles/extensions/<name>/index.ts` and a bare
/// `run.tiles/extensions/<name>.ts`.
pub fn installed_extension_entrypoints() -> Vec<PathBuf> {
    let mut entrypoints = vec![];
    for root in installed_plugin_roots() {
        let extensions_dir = root.join(CLIENT_NAMESPACE).join("extensions");
        let Ok(entries) = std::fs::read_dir(&extensions_dir) else {
            continue;
        };
        let mut found = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter_map(|path| {
                if path.is_dir() {
                    let index = path.join("index.ts");
                    index.is_file().then_some(index)
                } else {
                    path.extension()
                        .is_some_and(|ext| ext == "ts")
                        .then_some(path)
                }
            })
            .collect::<Vec<PathBuf>>();
        found.sort();
        entrypoints.extend(found);
    }
    entrypoints
}

pub fn uninstall(plugin_name: &str) -> Result<String> {
    let plugin_root = plugins_dir()?.join(plugin_name);
    if !plugin_root.join("plugin.json").is_file() {
        // Shipped plugins live in the read-only lib dir and would come back on
        // the next upgrade, so point at the switch that actually sticks.
        if bundled_plugins_dir().is_some_and(|dir| dir.join(plugin_name).is_dir()) {
            return Err(anyhow!(
                "{} ships with Tiles and cannot be uninstalled. Use `tiles plugin disable {}` instead.",
                plugin_name,
                plugin_name
            ));
        }
        return Err(anyhow!(
            "Plugin {} is not installed. Run `tiles plugin list` to see what is.",
            plugin_name
        ));
    }

    remove_dir_all(&plugin_root)
        .with_context(|| format!("Failed to uninstall plugin {}", plugin_name))?;
    Ok(format!("Uninstalled plugin {} successfully", plugin_name))
}

/// One row of `tiles plugin list`, describing what a plugin is for rather
/// than how it works.
pub struct PluginSummary {
    pub name: String,
    pub description: String,
    pub bundled: bool,
    pub enabled: bool,
}

pub fn summaries() -> Vec<PluginSummary> {
    let disabled = get_disabled_plugins();
    all_plugin_roots()
        .into_iter()
        .filter_map(|root| {
            let name = plugin_name_of(&root)?;
            // A broken manifest in one plugin must not hide the others.
            let manifest = read_manifest(&root).ok();
            Some(PluginSummary {
                description: manifest
                    .as_ref()
                    .and_then(|manifest| manifest.description.clone())
                    .unwrap_or_else(|| {
                        if manifest.is_none() {
                            "(unreadable plugin.json)".to_owned()
                        } else {
                            String::new()
                        }
                    }),
                bundled: is_bundled(&root),
                enabled: !disabled.contains(&name),
                name,
            })
        })
        .collect()
}

pub fn list() -> Result<()> {
    let summaries = summaries();
    if summaries.is_empty() {
        println!("No plugins installed. Add one with `tiles plugin install <path>`.");
        return Ok(());
    }

    let width = summaries
        .iter()
        .map(|summary| summary.name.len())
        .max()
        .unwrap_or(0);

    for summary in summaries {
        let mut tags = vec![];
        if summary.bundled {
            tags.push("built-in");
        }
        if !summary.enabled {
            tags.push("off");
        }
        let tags = if tags.is_empty() {
            String::new()
        } else {
            format!("  [{}]", tags.join(", "))
        };
        println!(
            "{:width$}  {}{}",
            summary.name,
            summary.description,
            tags,
            width = width
        );
    }
    Ok(())
}

/// Turns a plugin on or off without touching its files.
pub fn set_enabled(plugin_name: &str, enabled: bool) -> Result<String> {
    let known = all_plugin_roots()
        .iter()
        .filter_map(|root| plugin_name_of(root))
        .any(|name| name == plugin_name);
    if !known {
        return Err(anyhow!(
            "No plugin named {}. Run `tiles plugin list` to see what is available.",
            plugin_name
        ));
    }

    let changed = set_plugin_disabled(plugin_name, !enabled)?;
    let state = if enabled { "enabled" } else { "disabled" };
    if !changed {
        return Ok(format!("Plugin {} is already {}", plugin_name, state));
    }
    Ok(format!(
        "Plugin {} {}. Restart Tiles to apply.",
        plugin_name, state
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_archive_kind_reads_bytes_not_names() {
        let tmp = tempfile::tempdir().unwrap();

        // a name that lies about its contents must not decide the format
        let lying = tmp.path().join("actually-a-zip.tar.gz");
        std::fs::write(&lying, b"PK\x03\x04rest-of-a-zip").unwrap();
        assert_eq!(ArchiveKind::detect(&lying).unwrap(), ArchiveKind::Zip);

        // and no name at all is fine, which is what url downloads look like
        let unnamed = tmp.path().join("download");
        std::fs::write(&unnamed, b"\x1f\x8b\x08rest-of-a-gzip").unwrap();
        assert_eq!(ArchiveKind::detect(&unnamed).unwrap(), ArchiveKind::TarGz);

        // an html error page is the common accident, and must be rejected
        let html = tmp.path().join("oops.tar.gz");
        std::fs::write(&html, b"<!doctype html><html>404").unwrap();
        assert!(ArchiveKind::detect(&html).is_err());

        // a file too short to have magic bytes
        let tiny = tmp.path().join("tiny");
        std::fs::write(&tiny, b"P").unwrap();
        assert!(ArchiveKind::detect(&tiny).is_err());
    }

    #[test]
    fn test_valid_plugin_names() {
        assert!(is_valid_plugin_name("a"));
        assert!(is_valid_plugin_name("my-plugin"));
        assert!(is_valid_plugin_name("my.plugin-2"));
        assert!(!is_valid_plugin_name(""));
        assert!(!is_valid_plugin_name("-lead"));
        assert!(!is_valid_plugin_name("trail-"));
        assert!(!is_valid_plugin_name("a--b"));
        assert!(!is_valid_plugin_name("a..b"));
        assert!(!is_valid_plugin_name("Caps"));
        assert!(!is_valid_plugin_name("under_score"));
        assert!(!is_valid_plugin_name(&"a".repeat(65)));
        assert!(is_valid_plugin_name(&"a".repeat(64)));
    }

    #[test]
    fn test_manifest_accepts_both_spec_versions() {
        let v100 =
            parse_manifest(&json!({"$schema": PLUGIN_SCHEMA_1_0_0, "name": "demo"})).unwrap();
        assert_eq!(v100.name, "demo");
        assert!(!v100.mcp_dormant);

        let v110 =
            parse_manifest(&json!({"$schema": PLUGIN_SCHEMA_1_1_0, "name": "demo"})).unwrap();
        assert!(v110.mcp_dormant, "1.1.0 cannot drive the bundled adapter");
    }

    #[test]
    fn test_manifest_rejects_bad_input() {
        // unrecognised spec version
        assert!(
            parse_manifest(&json!({
                "$schema": "https://agent-plugins.org/schemas/2.0.0/plugin.schema.json",
                "name": "demo"
            }))
            .is_err()
        );
        // missing $schema
        assert!(parse_manifest(&json!({"name": "demo"})).is_err());
        // missing name
        assert!(parse_manifest(&json!({"$schema": PLUGIN_SCHEMA_1_0_0})).is_err());
        // bad name
        assert!(
            parse_manifest(&json!({"$schema": PLUGIN_SCHEMA_1_0_0, "name": "Bad_Name"})).is_err()
        );
        // not an object
        assert!(parse_manifest(&json!(["nope"])).is_err());
    }

    #[test]
    fn test_manifest_ignores_unknown_fields() {
        let manifest = parse_manifest(&json!({
            "$schema": PLUGIN_SCHEMA_1_0_0,
            "name": "demo",
            "version": "1.0.0",
            "mcpServers": {"nope": {}}
        }))
        .expect("unknown fields are reported, not fatal");
        assert_eq!(manifest.name, "demo");
    }

    #[test]
    fn test_manifest_keeps_description_for_listing() {
        let manifest = parse_manifest(&json!({
            "$schema": PLUGIN_SCHEMA_1_0_0,
            "name": "exa",
            "description": "Web search and page fetch"
        }))
        .unwrap();
        assert_eq!(
            manifest.description.as_deref(),
            Some("Web search and page fetch")
        );

        // optional per the spec
        let bare =
            parse_manifest(&json!({"$schema": PLUGIN_SCHEMA_1_0_0, "name": "bare"})).unwrap();
        assert!(bare.description.is_none());
    }

    #[test]
    fn test_shipped_exa_manifest_is_valid() {
        // the plugin we ship must satisfy our own validator
        let raw = include_str!("../../../plugins/exa/plugin.json");
        let parsed: serde_json::Value = serde_json::from_str(raw).unwrap();
        let manifest = parse_manifest(&parsed).expect("shipped exa manifest must be valid");
        assert_eq!(manifest.name, "exa");
        assert!(manifest.description.is_some(), "users see this text");
        assert!(
            !manifest.mcp_dormant,
            "must target the version the adapter reads"
        );
    }

    #[test]
    fn test_shipped_exa_mcp_config_is_spec_clean() {
        let raw = include_str!("../../../plugins/exa/mcp.json");
        let parsed: serde_json::Value = serde_json::from_str(raw).unwrap();
        let server = &parsed["mcpServers"]["search"];
        assert_eq!(server["type"], "streamable-http");
        // https is required for non-loopback urls, and the url must carry no
        // placeholders or the adapter rejects it outright
        let url = server["url"].as_str().unwrap();
        assert!(url.starts_with("https://"), "{}", url);
        assert!(!url.contains("$env:") && !url.contains("${"), "{}", url);
        // the key rides in a header, where the adapter does interpolate
        assert_eq!(server["headers"]["x-api-key"], "$env:EXA_API_KEY");
    }

    #[test]
    fn test_plugin_folder_is_recognised_without_an_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("my-plugin");
        std::fs::create_dir_all(&root).unwrap();

        // a folder with no manifest is not a plugin
        assert!(!is_plugin_folder(&root));

        std::fs::write(root.join("plugin.json"), b"{}").unwrap();
        assert!(is_plugin_folder(&root));

        // the manifest has to be at the root, not buried
        let nested = tmp.path().join("nested");
        std::fs::create_dir_all(nested.join("inner")).unwrap();
        std::fs::write(nested.join("inner/plugin.json"), b"{}").unwrap();
        assert!(!is_plugin_folder(&nested));
    }

    #[test]
    fn test_shipped_exa_skill_frontmatter_is_valid() {
        // A broken skill is silently skipped by Pi, so catch it here instead.
        let raw = include_str!("../../../plugins/exa/skills/web-research/SKILL.md");
        let frontmatter = raw
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("\n---"))
            .map(|(front, _)| front)
            .expect("SKILL.md must open with yaml frontmatter");

        let field = |key: &str| {
            frontmatter
                .lines()
                .find_map(|line| line.strip_prefix(key))
                .map(str::trim)
        };

        let name = field("name:").expect("name is required");
        assert_eq!(name, "web-research");
        // spec: 1-64 chars, lowercase, digits and single hyphens
        assert!(name.len() <= 64);
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        );
        assert!(!name.starts_with('-') && !name.ends_with('-') && !name.contains("--"));

        // the description is the only part always in the model's prompt, so it
        // is what decides whether the skill ever gets loaded
        let description = field("description:").expect("description is required");
        assert!(description.len() <= 1024, "{}", description.len());
        assert!(
            description.to_lowercase().contains("web"),
            "must be findable for web questions: {}",
            description
        );
    }

    #[test]
    fn test_plugin_name_of_reads_directory_name() {
        assert_eq!(
            plugin_name_of(Path::new("/lib/tiles/plugins/exa")).as_deref(),
            Some("exa")
        );
    }

    #[test]
    fn test_archive_entry_containment() {
        assert!(is_contained_entry("my-plugin/plugin.json"));
        assert!(is_contained_entry("./my-plugin/skills/a/SKILL.md"));
        assert!(is_contained_entry(""));
        assert!(!is_contained_entry("../escape"));
        assert!(!is_contained_entry("my-plugin/../../escape"));
        assert!(!is_contained_entry("/etc/passwd"));
    }
}
