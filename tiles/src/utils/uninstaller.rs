use std::{
    collections::BTreeSet,
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow};

use crate::utils::config::{
    ConfigProvider, DefaultProvider, SYSTEM_BIN_PATH, SYSTEM_LIB_DIR, is_tiles_lib_dir,
};

const LIB_COMPONENT_DIRS: &[&str] = &["server", "modelfiles", "pi", "models"];
const SYSTEM_BIN_DIR: &str = "/usr/local/bin";

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstallLayout {
    bin: PathBuf,
    lib_dir: PathBuf,
}

pub fn uninstall(all: bool) -> Result<()> {
    let plan = UninstallPlan::from_current_system(all)?;

    if plan.is_empty() {
        println!("No Tiles files found to uninstall.");
        return Ok(());
    }

    print_plan(&plan);
    let needs_elevation = plan.needs_elevation();
    confirm_uninstall(all, needs_elevation)?;
    plan.apply()?;

    println!("Tiles uninstalled successfully.");
    Ok(())
}

#[derive(Debug, Default)]
struct UninstallPlan {
    remove_files: BTreeSet<PathBuf>,
    remove_dirs: BTreeSet<PathBuf>,
    clean_config_dir: Option<PathBuf>,
}

impl UninstallPlan {
    fn from_current_system(all: bool) -> Result<Self> {
        let provider = DefaultProvider;
        let layout = InstallLayout::detect(&provider)?;
        let config_dir = provider.get_config_dir()?;
        let data_dir = canonicalize_uninstall_path(&provider.get_data_dir()?)?;
        let lib_dir = canonicalize_uninstall_path(&layout.lib_dir).unwrap_or(layout.lib_dir);
        let mut plan = Self::default();

        plan.remove_files.insert(layout.bin);

        if all {
            let user_data_dir = resolve_user_data_dir_for_uninstall(&data_dir, &config_dir)?;
            plan.remove_dirs.insert(config_dir);
            plan.remove_dirs.insert(data_dir);
            plan.remove_dirs.insert(user_data_dir);
            plan.remove_dirs.insert(lib_dir);
            return Ok(plan);
        }

        plan.clean_config_dir = Some(config_dir);

        if lib_dir != data_dir {
            for component in LIB_COMPONENT_DIRS {
                plan.remove_dirs.insert(lib_dir.join(component));
            }
        }

        Ok(plan)
    }

    fn is_empty(&self) -> bool {
        !self.remove_files.iter().any(|path| path.exists())
            && !self.remove_dirs.iter().any(|path| path.exists())
            && !self
                .clean_config_dir
                .as_ref()
                .is_some_and(|path| path.exists())
    }

    fn needs_elevation(&self) -> bool {
        #[cfg(unix)]
        {
            if is_running_as_root() {
                return false;
            }
            self.remove_files.iter().any(|path| requires_elevation(path))
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
        for dir in sorted_child_first(&self.remove_dirs) {
            if requires_elevation(&dir) {
                elevated_dirs.push(dir);
            } else {
                user_dirs.push(dir);
            }
        }

        for file in user_files {
            remove_file_if_exists(file)?;
        }

        if let Some(config_dir) = &self.clean_config_dir {
            clean_config_dir(config_dir)?;
        }

        for dir in user_dirs {
            remove_dir_if_exists(&dir)?;
        }

        remove_files_elevated(&elevated_files)?;
        remove_dirs_elevated(&elevated_dirs)?;

        Ok(())
    }
}

fn print_plan(plan: &UninstallPlan) {
    println!("Tiles uninstall will remove:");

    for file in plan.remove_files.iter().filter(|path| path.exists()) {
        println!("  {}", file.display());
    }

    if let Some(config_dir) = &plan.clean_config_dir
        && config_dir.exists()
    {
        println!("  {} (everything except config.toml)", config_dir.display());
    }

    for dir in plan.remove_dirs.iter().filter(|path| path.exists()) {
        println!("  {}", dir.display());
    }

    if plan.needs_elevation() {
        println!();
        println!("Administrator privileges are required to remove system files under /usr/local.");
    }
}

fn confirm_uninstall(all: bool, needs_elevation: bool) -> Result<()> {
    let prompt = match (all, needs_elevation) {
        (true, true) => {
            "This will remove all Tiles files, including config and databases. Administrator privileges are required to remove system files. Continue? [y/N] "
        }
        (true, false) => {
            "This will remove all Tiles files, including config and databases. Continue? [y/N] "
        }
        (false, true) => {
            "This will remove Tiles but keep config.toml and your data folder. Administrator privileges are required to remove system files. Continue? [y/N] "
        }
        (false, false) => {
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

fn resolve_user_data_dir_for_uninstall(
    data_dir: &Path,
    config_dir: &Path,
) -> Result<PathBuf> {
    let user_data_dir = match read_user_data_dir(config_dir)? {
        Some(path) => canonicalize_uninstall_path(&path)?,
        None => data_dir.join("data"),
    };
    validate_user_data_dir_for_uninstall(data_dir, &user_data_dir)?;
    Ok(user_data_dir)
}

fn validate_user_data_dir_for_uninstall(data_dir: &Path, user_data_dir: &Path) -> Result<()> {
    if is_filesystem_root(user_data_dir) {
        return Err(anyhow!(
            "Refusing to delete unsafe data path {}",
            user_data_dir.display()
        ));
    }

    if is_strict_ancestor(user_data_dir, data_dir) {
        return Err(anyhow!(
            "Refusing to delete configured data path {} because it contains the Tiles data directory {}",
            user_data_dir.display(),
            data_dir.display()
        ));
    }

    if !path_is_within(data_dir, user_data_dir) {
        return Err(anyhow!(
            "Refusing to delete configured data path {} because it is outside the Tiles data directory {}",
            user_data_dir.display(),
            data_dir.display()
        ));
    }

    Ok(())
}

fn canonicalize_uninstall_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        fs::canonicalize(path)
            .with_context(|| format!("Failed to canonicalize {}", path.display()))
    } else {
        std::path::absolute(path)
            .with_context(|| format!("Failed to resolve absolute path for {}", path.display()))
    }
}

fn is_filesystem_root(path: &Path) -> bool {
    path.parent().is_some_and(|parent| parent == path)
}

fn is_strict_ancestor(ancestor: &Path, path: &Path) -> bool {
    if path == ancestor {
        return false;
    }

    path.strip_prefix(ancestor)
        .is_ok_and(|remainder| !remainder.as_os_str().is_empty())
}

fn path_is_within(base: &Path, candidate: &Path) -> bool {
    if candidate == base {
        return true;
    }

    candidate
        .strip_prefix(base)
        .is_ok_and(|remainder| !remainder.as_os_str().is_empty())
}

fn clean_config_dir(config_dir: &Path) -> Result<()> {
    if !config_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(config_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == "config.toml") {
            continue;
        }
        remove_path(&path)?;
    }

    Ok(())
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
            return Ok(Self {
                bin: provider.get_user_bin_path()?,
                lib_dir: user_lib_dir,
            });
        }

        default_install_layout(provider)
    }

    fn from_executable(provider: &DefaultProvider, exe: &Path) -> Result<Option<Self>> {
        if !exe.file_name().is_some_and(|name| name == "tiles") {
            return Ok(None);
        }

        if exe.starts_with("/usr/local/bin") {
            return Ok(Some(Self {
                bin: exe.to_path_buf(),
                lib_dir: PathBuf::from(SYSTEM_LIB_DIR),
            }));
        }

        if let Ok(user_bin_dir) = provider.get_user_bin_dir()
            && exe.starts_with(&user_bin_dir)
        {
            return Ok(Some(Self {
                bin: exe.to_path_buf(),
                lib_dir: provider.get_data_dir()?,
            }));
        }

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

fn default_install_layout(provider: &DefaultProvider) -> Result<InstallLayout> {
    if uses_user_install_paths() {
        Ok(InstallLayout {
            bin: provider.get_user_bin_path()?,
            lib_dir: provider.get_data_dir()?,
        })
    } else {
        Ok(InstallLayout {
            bin: PathBuf::from(SYSTEM_BIN_PATH),
            lib_dir: PathBuf::from(SYSTEM_LIB_DIR),
        })
    }
}

fn uses_user_install_paths() -> bool {
    #[cfg(target_os = "linux")]
    {
        !is_running_as_root()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
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
        return remove_files_elevated_unix(&existing);
    }

    #[cfg(not(unix))]
    {
        for path in existing {
            remove_file_if_exists(path)?;
        }
        Ok(())
    }
}

fn remove_dirs_elevated(paths: &[PathBuf]) -> Result<()> {
    let existing: Vec<&PathBuf> = paths.iter().filter(|path| path.exists()).collect();
    if existing.is_empty() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        return remove_dirs_elevated_unix(&existing);
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
    #[cfg(target_os = "linux")]
    use std::os::linux::fs::MetadataExt;
    #[cfg(all(unix, not(target_os = "linux")))]
    use std::os::unix::fs::MetadataExt;
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

fn sorted_child_first(paths: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
    let mut paths = paths.iter().cloned().collect::<Vec<_>>();
    paths.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    paths
}

fn remove_path(path: &Path) -> Result<()> {
    if path.is_dir() {
        remove_dir_if_exists(path)
    } else {
        remove_file_if_exists(path)
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

    use crate::utils::config::{
        DefaultProvider, SYSTEM_LIB_DIR, is_tiles_lib_dir,
    };

    use std::path::Path;

    use super::{
        InstallLayout, clean_config_dir, requires_elevation, resolve_user_data_dir_for_uninstall,
        validate_user_data_dir_for_uninstall,
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
        assert!(requires_elevation(Path::new("/usr/local/share/tiles/server")));
        assert!(!requires_elevation(Path::new("/home/user/.local/bin/tiles")));
        assert!(!requires_elevation(Path::new("/home/user/.local/share/tiles/data")));
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
    fn clean_config_dir_preserves_config_toml() -> Result<()> {
        let dir = tempdir()?;
        fs::write(dir.path().join("config.toml"), "config")?;
        fs::write(dir.path().join("server.pid"), "123")?;

        clean_config_dir(dir.path())?;

        assert!(dir.path().join("config.toml").exists());
        assert!(!dir.path().join("server.pid").exists());
        Ok(())
    }
}
