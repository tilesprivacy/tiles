use std::{
    collections::BTreeSet,
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow};

use crate::utils::config::{
    ConfigProvider, DefaultProvider, LIB_RUNTIME_DIRS_TO_REMOVE, SYSTEM_BIN_DIR, SYSTEM_BIN_PATH,
    SYSTEM_LIB_DIR, is_tiles_lib_dir,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstallLayout {
    bin: PathBuf,
    lib_dir: PathBuf,
}

pub fn uninstall(all: bool) -> Result<()> {
    let plan = UninstallPlanner::from_current_system(all)?;

    if !plan.remove_files.iter().any(|path| path.exists())
        && !plan.remove_dirs.iter().any(|path| path.exists())
    {
        println!("No Tiles files found to uninstall.");
        return Ok(());
    }

    let needs_elevation = plan.needs_elevation();
    print_plan(&plan, needs_elevation);
    confirm_uninstall(all)?;
    #[cfg(target_os = "macos")]
    crate::core::service::unload().context("Failed to unload Tiles service")?;
    plan.apply()?;

    println!("Tiles uninstalled successfully.");
    Ok(())
}

#[derive(Debug, Default)]
struct UninstallPlanner {
    remove_files: BTreeSet<PathBuf>,
    remove_dirs: BTreeSet<PathBuf>,
}

impl UninstallPlanner {
    fn from_current_system(all: bool) -> Result<Self> {
        let provider = DefaultProvider;
        let layout = InstallLayout::detect(&provider)?;
        let config_dir = provider.get_config_dir()?;
        let data_dir = canonicalize_uninstall_path(&provider.get_data_dir()?)?;
        let lib_dir = canonicalize_uninstall_path(&layout.lib_dir).unwrap_or(layout.lib_dir);
        let mut plan = Self::default();

        plan.remove_files.insert(layout.bin);
        #[cfg(target_os = "macos")]
        add_service_file_to_plan(&mut plan, crate::core::service::plist_path()?);

        if all {
            let user_data_dir = resolve_user_data_dir_for_uninstall(&data_dir, &config_dir)?;
            plan.remove_dirs.insert(config_dir);
            plan.remove_dirs.insert(data_dir);
            plan.remove_dirs.insert(user_data_dir);
            plan.remove_dirs.insert(lib_dir);
            return Ok(plan);
        }

        add_config_dir_entries_to_plan(&mut plan, &config_dir)?;
        add_default_uninstall_cleanup_to_plan(&mut plan, &data_dir, &lib_dir, &config_dir)?;

        Ok(plan)
    }

    fn needs_elevation(&self) -> bool {
        #[cfg(unix)]
        {
            if is_running_as_root() {
                return false;
            }
            self.remove_files
                .iter()
                .any(|path| requires_elevation(path))
                || self.remove_dirs.iter().any(|path| requires_elevation(path))
        }
        #[cfg(not(unix))]
        {
            false
        }
    }

    fn apply(&self) -> Result<()> {
        let mut user_files = Vec::new();
        let mut elevated_files = Vec::new();
        for file in &self.remove_files {
            if requires_elevation(file) {
                elevated_files.push(file);
            } else {
                user_files.push(file);
            }
        }

        let mut user_dirs = Vec::new();
        let mut elevated_dirs = Vec::new();
        for dir in &self.remove_dirs {
            if requires_elevation(dir) {
                elevated_dirs.push(dir);
            } else {
                user_dirs.push(dir);
            }
        }

        remove_files_elevated(&elevated_files)?;
        remove_dirs_elevated(&elevated_dirs)?;

        for file in user_files {
            remove_file_if_exists(file)?;
        }

        for dir in user_dirs {
            remove_dir_if_exists(dir)?;
        }

        Ok(())
    }
}

fn add_service_file_to_plan(plan: &mut UninstallPlanner, path: PathBuf) {
    plan.remove_files.insert(path);
}

fn print_plan(plan: &UninstallPlanner, needs_elevation: bool) {
    println!("Tiles uninstall will remove:");

    for file in plan.remove_files.iter().filter(|path| path.exists()) {
        println!("  {}", file.display());
    }

    for dir in plan.remove_dirs.iter().filter(|path| path.exists()) {
        println!("  {}", dir.display());
    }

    if needs_elevation {
        println!();
        println!("Administrator privileges are required to remove system files under /usr/local.");
        println!();
    }
}

fn confirm_uninstall(all: bool) -> Result<()> {
    let prompt = match all {
        true => {
            "This will remove all Tiles files, including config and databases. Continue? [y/N] "
        }
        false => {
            "This will remove Tiles but keep config.toml and your data folder. Continue? [y/N] "
        }
    };

    print!("{prompt}");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if input.trim().eq_ignore_ascii_case("y") {
        Ok(())
    } else {
        Err(anyhow!("Uninstall cancelled"))
    }
}

fn read_user_data_dir(config_dir: &Path) -> Result<Option<PathBuf>> {
    let config_path = config_dir.join("config.toml");
    if !config_path.exists() {
        return Ok(None);
    }

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    let config = config_str
        .parse::<toml::Table>()
        .with_context(|| format!("Failed to parse {}", config_path.display()))?;

    Ok(config
        .get("data")
        .and_then(|data| data.as_table())
        .and_then(|data| data.get("path"))
        .and_then(|path| path.as_str())
        .filter(|path| !path.is_empty())
        .map(PathBuf::from))
}

fn resolve_user_data_dir_for_uninstall(data_dir: &Path, config_dir: &Path) -> Result<PathBuf> {
    let user_data_dir = match read_user_data_dir(config_dir)? {
        Some(path) => canonicalize_uninstall_path(&path)?,
        None => data_dir.join("data"),
    };
    validate_user_data_dir_for_uninstall(data_dir, &user_data_dir)?;
    Ok(user_data_dir)
}

fn validate_user_data_dir_for_uninstall(data_dir: &Path, user_data_dir: &Path) -> Result<()> {
    if user_data_dir
        .parent()
        .is_some_and(|parent| parent == user_data_dir)
    {
        return Err(anyhow!(
            "Refusing to delete unsafe data path {}",
            user_data_dir.display()
        ));
    }

    if user_data_dir != data_dir
        && data_dir
            .strip_prefix(user_data_dir)
            .is_ok_and(|remainder| !remainder.as_os_str().is_empty())
    {
        return Err(anyhow!(
            "Refusing to delete configured data path {} because it contains the Tiles data directory {}",
            user_data_dir.display(),
            data_dir.display()
        ));
    }

    let within_data_dir = user_data_dir == data_dir
        || user_data_dir
            .strip_prefix(data_dir)
            .is_ok_and(|remainder| !remainder.as_os_str().is_empty());
    if !within_data_dir {
        return Err(anyhow!(
            "Refusing to delete configured data path {} because it is outside the Tiles data directory {}",
            user_data_dir.display(),
            data_dir.display()
        ));
    }

    Ok(())
}

/// Default uninstall keeps only `config.toml` and the configured user data directory.
fn add_default_uninstall_cleanup_to_plan(
    plan: &mut UninstallPlanner,
    data_dir: &Path,
    lib_dir: &Path,
    config_dir: &Path,
) -> Result<()> {
    let user_data_dir = resolve_user_data_dir_for_uninstall(data_dir, config_dir)?;

    if lib_dir != data_dir {
        for component in LIB_RUNTIME_DIRS_TO_REMOVE {
            plan.remove_dirs.insert(lib_dir.join(component));
        }
    }

    add_data_dir_entries_except_user_data(plan, data_dir, &user_data_dir)
}

fn add_data_dir_entries_except_user_data(
    plan: &mut UninstallPlanner,
    data_dir: &Path,
    user_data_dir: &Path,
) -> Result<()> {
    if !data_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(data_dir)? {
        let path = entry?.path();
        if paths_equal_for_uninstall(&path, user_data_dir) {
            continue;
        }
        if path.is_dir() {
            plan.remove_dirs.insert(path);
        } else {
            plan.remove_files.insert(path);
        }
    }

    Ok(())
}

fn paths_equal_for_uninstall(a: &Path, b: &Path) -> bool {
    let a = canonicalize_uninstall_path(a).unwrap_or_else(|_| a.to_path_buf());
    let b = canonicalize_uninstall_path(b).unwrap_or_else(|_| b.to_path_buf());
    a == b
}

fn add_config_dir_entries_to_plan(plan: &mut UninstallPlanner, config_dir: &Path) -> Result<()> {
    if !config_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(config_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == "config.toml") {
            continue;
        }
        if path.is_dir() {
            plan.remove_dirs.insert(path);
        } else {
            plan.remove_files.insert(path);
        }
    }

    Ok(())
}

/// Normalize a path for uninstall planning.
///
/// Uses `canonicalize` when the path exists so symlinked or relative paths compare
/// reliably (e.g. `lib_dir != data_dir`). Falls back to `absolute` when the path is
/// missing so planning still works on partial installs.
fn canonicalize_uninstall_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        fs::canonicalize(path).with_context(|| format!("Failed to canonicalize {}", path.display()))
    } else {
        std::path::absolute(path)
            .with_context(|| format!("Failed to resolve absolute path for {}", path.display()))
    }
}

impl InstallLayout {
    fn detect(provider: &DefaultProvider) -> Result<Self> {
        if let Ok(current_exe) = env::current_exe()
            && let Some(layout) = Self::from_executable(provider, &current_exe)?
        {
            return Ok(layout);
        }

        let user_lib_dir = provider.get_data_dir()?;
        if is_tiles_lib_dir(&user_lib_dir) {
            return Self::user_install_layout(provider, provider.get_user_bin_path()?);
        }

        #[cfg(target_os = "linux")]
        if !is_running_as_root() {
            return Self::user_install_layout(provider, provider.get_user_bin_path()?);
        }

        Ok(InstallLayout {
            bin: PathBuf::from(SYSTEM_BIN_PATH),
            lib_dir: PathBuf::from(SYSTEM_LIB_DIR),
        })
    }

    fn user_install_layout(provider: &DefaultProvider, bin: PathBuf) -> Result<Self> {
        Ok(Self {
            bin,
            lib_dir: provider.get_data_dir()?,
        })
    }

    fn from_executable(provider: &DefaultProvider, exe: &Path) -> Result<Option<Self>> {
        if exe.file_name().is_none_or(|name| name != "tiles") {
            return Ok(None);
        }

        if exe.starts_with(SYSTEM_BIN_DIR) {
            return Ok(Some(Self {
                bin: exe.to_path_buf(),
                lib_dir: PathBuf::from(SYSTEM_LIB_DIR),
            }));
        }

        if let Ok(user_bin_dir) = provider.get_user_bin_dir()
            && exe.starts_with(&user_bin_dir)
        {
            return Ok(Some(Self::user_install_layout(
                provider,
                exe.to_path_buf(),
            )?));
        }

        // The cases when libs are near to the executable, mostly in dev
        if let Some(exe_dir) = exe.parent()
            && is_tiles_lib_dir(exe_dir)
        {
            return Ok(Some(Self {
                bin: exe.to_path_buf(),
                lib_dir: exe_dir.to_path_buf(),
            }));
        }

        Ok(None)
    }
}

fn requires_elevation(path: &Path) -> bool {
    path.starts_with(SYSTEM_BIN_DIR) || path.starts_with(SYSTEM_LIB_DIR)
}

#[cfg(unix)]
fn is_running_as_root() -> bool {
    nix::unistd::geteuid().as_raw() == 0
}

fn remove_files_elevated(paths: &[&PathBuf]) -> Result<()> {
    let existing: Vec<&PathBuf> = paths.iter().copied().filter(|path| path.exists()).collect();
    if existing.is_empty() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        remove_files_elevated_unix(&existing)
    }

    #[cfg(not(unix))]
    {
        for path in existing {
            remove_file_if_exists(path)?;
        }
        Ok(())
    }
}

fn remove_dirs_elevated(paths: &[&PathBuf]) -> Result<()> {
    let existing: Vec<&PathBuf> = paths.iter().copied().filter(|path| path.exists()).collect();
    if existing.is_empty() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        remove_dirs_elevated_unix(&existing)
    }

    #[cfg(not(unix))]
    {
        for path in existing {
            remove_dir_if_exists(path)?;
        }
        Ok(())
    }
}

#[cfg(unix)]
const TRUSTED_SUDO_PATHS: &[&str] = &["/usr/bin/sudo", "/bin/sudo"];

#[cfg(unix)]
fn new_trusted_sudo_command() -> Result<Command> {
    Ok(Command::new(trusted_sudo_path()?))
}

#[cfg(unix)]
fn trusted_sudo_path() -> Result<PathBuf> {
    for path in TRUSTED_SUDO_PATHS {
        let path = Path::new(path);
        if is_trusted_sudo_binary(path)? {
            return Ok(path.to_path_buf());
        }
    }

    Err(anyhow!(
        "Trusted sudo binary not found at {}",
        TRUSTED_SUDO_PATHS.join(" or ")
    ))
}

#[cfg(unix)]
fn is_trusted_sudo_binary(path: &Path) -> Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    if !path.is_absolute() || !path.exists() {
        return Ok(false);
    }

    let canonical = fs::canonicalize(path)
        .with_context(|| format!("Failed to resolve sudo path {}", path.display()))?;
    if !TRUSTED_SUDO_PATHS
        .iter()
        .any(|trusted| canonical == Path::new(trusted))
    {
        return Ok(false);
    }

    let metadata = fs::metadata(&canonical)
        .with_context(|| format!("Failed to read metadata for {}", canonical.display()))?;
    if !metadata.is_file() {
        return Ok(false);
    }

    let mode = metadata.permissions().mode();
    #[cfg(all(unix, not(target_os = "linux")))]
    use std::os::darwin::fs::MetadataExt;
    #[cfg(target_os = "linux")]
    use std::os::linux::fs::MetadataExt;
    Ok(metadata.st_uid() == 0 && mode & 0o4000 != 0)
}

#[cfg(unix)]
fn remove_files_elevated_unix(paths: &[&PathBuf]) -> Result<()> {
    if is_running_as_root() {
        for path in paths {
            remove_file_if_exists(path)?;
        }
        return Ok(());
    }

    let mut command = new_trusted_sudo_command()?;
    command.arg("rm").arg("-f");
    for path in paths {
        command.arg(path);
    }
    run_elevated_command(&mut command, "remove system files")
}

#[cfg(unix)]
fn remove_dirs_elevated_unix(paths: &[&PathBuf]) -> Result<()> {
    if is_running_as_root() {
        for path in paths {
            remove_dir_if_exists(path)?;
        }
        return Ok(());
    }

    let mut command = new_trusted_sudo_command()?;
    command.arg("rm").arg("-rf");
    for path in paths {
        command.arg(path);
    }
    run_elevated_command(&mut command, "remove system directories")
}

#[cfg(unix)]
fn run_elevated_command(command: &mut Command, action: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("Failed to run sudo to {action}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "Failed to {action} (sudo exited with {status}). Administrator privileges are required."
        ))
    }
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(path).with_context(|| format!("Failed to remove {}", path.display()))
}

fn remove_dir_if_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    fs::remove_dir_all(path).with_context(|| format!("Failed to remove {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use anyhow::Result;
    use tempfile::tempdir;

    use std::path::PathBuf;

    use crate::utils::config::{DefaultProvider, SYSTEM_LIB_DIR, is_tiles_lib_dir};

    use std::path::Path;

    use super::{
        InstallLayout, UninstallPlanner, add_config_dir_entries_to_plan,
        add_data_dir_entries_except_user_data, remove_file_if_exists, requires_elevation,
        resolve_user_data_dir_for_uninstall, validate_user_data_dir_for_uninstall,
    };

    #[cfg(unix)]
    #[test]
    fn trusted_sudo_path_uses_absolute_location() -> Result<()> {
        let sudo_path = super::trusted_sudo_path()?;
        assert!(sudo_path.is_absolute());
        assert!(super::is_trusted_sudo_binary(&sudo_path)?);
        Ok(())
    }

    #[test]
    fn install_layout_from_system_executable_path() -> Result<()> {
        let provider = DefaultProvider;
        let bin = PathBuf::from("/usr/local/bin/tiles");
        let layout = InstallLayout::from_executable(&provider, &bin)?.expect("layout");
        assert_eq!(layout.bin, bin);
        assert_eq!(layout.lib_dir, PathBuf::from(SYSTEM_LIB_DIR));
        Ok(())
    }

    #[test]
    fn install_layout_from_portable_executable() -> Result<()> {
        let provider = DefaultProvider;
        let root = tempdir()?;
        fs::create_dir_all(root.path().join("server"))?;
        fs::create_dir_all(root.path().join("modelfiles"))?;
        fs::create_dir_all(root.path().join("pi"))?;
        let bin = root.path().join("tiles");
        fs::write(&bin, "")?;

        let layout = InstallLayout::from_executable(&provider, &bin)?.expect("layout");
        assert_eq!(layout.bin, bin);
        assert_eq!(layout.lib_dir, root.path());
        assert!(is_tiles_lib_dir(&layout.lib_dir));
        Ok(())
    }

    #[test]
    fn requires_elevation_for_system_paths() {
        assert!(requires_elevation(Path::new("/usr/local/bin/tiles")));
        assert!(requires_elevation(Path::new(
            "/usr/local/share/tiles/server"
        )));
        assert!(!requires_elevation(Path::new(
            "/home/user/.local/bin/tiles"
        )));
        assert!(!requires_elevation(Path::new(
            "/home/user/.local/share/tiles/data"
        )));
    }

    #[test]
    fn validate_user_data_dir_rejects_unsafe_targets() -> Result<()> {
        let root = tempdir()?;
        fs::create_dir_all(root.path().join("tiles"))?;
        let data_dir = fs::canonicalize(root.path().join("tiles"))?;
        let safe_child = data_dir.join("data/memory");
        fs::create_dir_all(&safe_child)?;

        validate_user_data_dir_for_uninstall(&data_dir, &safe_child)?;

        let outside = root.path().join("outside");
        fs::create_dir_all(&outside)?;
        assert!(validate_user_data_dir_for_uninstall(&data_dir, &outside).is_err());

        let ancestor = root.path();
        assert!(validate_user_data_dir_for_uninstall(&data_dir, ancestor).is_err());

        assert!(validate_user_data_dir_for_uninstall(&data_dir, Path::new("/")).is_err());

        let prefix_trap = data_dir.with_file_name("tiles-evil");
        fs::create_dir_all(&prefix_trap)?;
        assert!(validate_user_data_dir_for_uninstall(&data_dir, &prefix_trap).is_err());
        Ok(())
    }

    #[test]
    fn resolve_user_data_dir_uses_default_child_when_unconfigured() -> Result<()> {
        let root = tempdir()?;
        fs::create_dir_all(root.path().join("tiles"))?;
        let data_dir = fs::canonicalize(root.path().join("tiles"))?;
        let config_dir = root.path().join("config");
        fs::create_dir_all(&config_dir)?;

        let resolved = resolve_user_data_dir_for_uninstall(&data_dir, &config_dir)?;
        assert_eq!(resolved, data_dir.join("data"));
        Ok(())
    }

    #[test]
    fn resolve_user_data_dir_rejects_configured_path_outside_data_dir() -> Result<()> {
        let root = tempdir()?;
        fs::create_dir_all(root.path().join("tiles"))?;
        let data_dir = fs::canonicalize(root.path().join("tiles"))?;
        let config_dir = root.path().join("config");
        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(root.path().join("outside"))?;
        let outside = fs::canonicalize(root.path().join("outside"))?;
        fs::write(
            config_dir.join("config.toml"),
            format!("[data]\npath = \"{}\"\n", outside.display()),
        )?;

        assert!(resolve_user_data_dir_for_uninstall(&data_dir, &config_dir).is_err());
        Ok(())
    }

    #[test]
    fn data_dir_cleanup_preserves_only_user_data() -> Result<()> {
        let root = tempdir()?;
        let data_dir = root.path().join("tiles");
        fs::create_dir_all(data_dir.join("data/memory"))?;
        fs::create_dir_all(data_dir.join("server"))?;
        fs::create_dir_all(data_dir.join("logs"))?;
        fs::write(data_dir.join("logs/server.out.log"), "log")?;

        let user_data_dir = data_dir.join("data");
        let mut plan = UninstallPlanner::default();
        add_data_dir_entries_except_user_data(&mut plan, &data_dir, &user_data_dir)?;

        assert!(plan.remove_dirs.contains(&data_dir.join("server")));
        assert!(plan.remove_dirs.contains(&data_dir.join("logs")));
        assert!(!plan.remove_dirs.contains(&user_data_dir));
        Ok(())
    }

    #[test]
    fn plan_config_cleanup_preserves_config_toml() -> Result<()> {
        let dir = tempdir()?;
        fs::write(dir.path().join("config.toml"), "config")?;
        fs::write(dir.path().join("server.pid"), "123")?;

        let mut plan = UninstallPlanner::default();
        add_config_dir_entries_to_plan(&mut plan, dir.path())?;

        assert!(plan.remove_files.contains(&dir.path().join("server.pid")));
        assert!(!plan.remove_files.contains(&dir.path().join("config.toml")));

        for file in &plan.remove_files {
            remove_file_if_exists(file)?;
        }

        assert!(dir.path().join("config.toml").exists());
        assert!(!dir.path().join("server.pid").exists());
        Ok(())
    }

    #[test]
    fn service_plist_is_an_uninstall_artifact() -> Result<()> {
        let dir = tempdir()?;
        let plist = dir.path().join("com.tilesprivacy.tiles.daemon.plist");
        fs::write(&plist, "plist")?;

        let mut plan = UninstallPlanner::default();
        super::add_service_file_to_plan(&mut plan, plist.clone());
        assert!(plan.remove_files.contains(&plist));

        plan.apply()?;
        assert!(!plist.exists());
        Ok(())
    }
}
