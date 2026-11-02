//! The engines folder: unpacking a downloaded build, checking it runs, and putting it in place

use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, BufReader},
    path::{Component, Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use tar::{Archive, EntryType};
use tokio::process::Command;

use crate::{Choice, Engine};

// setting up a Vulkan device compiles its shaders, over a minute on a busy computer
const CHECK_TIMEOUT: Duration = Duration::from_mins(3);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub engine: Engine,
    pub tag: String,
    pub dir: PathBuf,
}

impl Installed {
    pub fn server(&self) -> PathBuf {
        self.dir.join("llama-server")
    }
}

#[derive(Serialize, Deserialize)]
struct Failure {
    tag: String,
    reason: String,
}

/// The folder engines are installed in. Names starting with a dot are never run
#[derive(Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn downloads(&self) -> PathBuf {
        self.root.join(".downloads")
    }

    fn staging(&self) -> PathBuf {
        self.root.join(".staging")
    }

    fn failures_path(&self) -> PathBuf {
        self.root.join("failed.json")
    }

    /// The engine on disk. There's only ever one, older builds stay until a newer one passes its check
    pub fn installed(&self) -> Option<Installed> {
        let found: Vec<_> = fs::read_dir(&self.root)
            .ok()?
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                let (engine, tag) = name.rsplit_once('-')?;
                Some(Installed {
                    engine: Engine::from_name(engine)?,
                    tag: tag.to_owned(),
                    dir: entry.path(),
                })
            })
            .filter(|installed| installed.server().is_file())
            .collect();
        let newest = found
            .iter()
            .map(|installed| &installed.tag)
            .max_by_key(|tag| build_number(tag))?;
        found
            .iter()
            .find(|installed| &installed.tag == newest)
            .cloned()
    }

    /// Why each engine failed its check with this llama.cpp build
    pub fn failures(&self, tag: &str) -> HashMap<Engine, String> {
        self.read_failures()
            .into_iter()
            .filter(|(_, failure)| failure.tag == tag)
            .map(|(engine, failure)| (engine, failure.reason))
            .collect()
    }

    /// Marks the engines that failed their check, the way `choices` marks those that can't run
    pub fn mark_failures(&self, choices: &mut [Choice], tag: &str) {
        let failures = self.failures(tag);
        for choice in choices
            .iter_mut()
            .filter(|choice| choice.unavailable.is_none())
        {
            choice.unavailable = failures.get(&choice.engine).cloned();
        }
    }

    pub fn record_failure(&self, engine: Engine, tag: &str, reason: &str) -> io::Result<()> {
        let mut failures = self.read_failures();
        failures.insert(
            engine,
            Failure {
                tag: tag.to_owned(),
                reason: reason.to_owned(),
            },
        );
        self.write_failures(&failures)
    }

    pub fn clear_failure(&self, engine: Engine) -> io::Result<()> {
        let mut failures = self.read_failures();
        if failures.remove(&engine).is_some() {
            self.write_failures(&failures)?;
        }
        Ok(())
    }

    fn read_failures(&self) -> HashMap<Engine, Failure> {
        fs::read(self.failures_path())
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn write_failures(&self, failures: &HashMap<Engine, Failure>) -> io::Result<()> {
        fs::create_dir_all(&self.root)?;
        fs::write(self.failures_path(), serde_json::to_vec_pretty(failures)?)
    }

    /// Unpacks the archives into one fresh staging folder and returns it
    pub fn unpack(&self, archives: &[PathBuf]) -> io::Result<PathBuf> {
        let staging = self.staging();
        remove(&staging)?;
        fs::create_dir_all(&staging)?;
        for archive in archives {
            unpack(archive, &staging)?;
        }
        Ok(staging)
    }

    pub fn discard_staging(&self) -> io::Result<()> {
        remove(&self.staging())
    }

    /// Moves the checked build into place, then removes every other engine and the downloads
    pub fn finish(&self, engine: Engine, tag: &str) -> io::Result<Installed> {
        let dir = self.root.join(format!("{}-{tag}", engine.name()));
        // the move is what makes it installed, a crash before it leaves only the staging folder
        remove(&dir)?;
        fs::rename(self.staging(), &dir)?;
        for entry in fs::read_dir(&self.root)?.flatten() {
            let path = entry.path();
            let hidden = entry.file_name().to_string_lossy().starts_with('.');
            if path != dir && !hidden && path.is_dir() {
                remove(&path)?;
            }
        }
        remove(&self.downloads())?;
        Ok(Installed {
            engine,
            tag: tag.to_owned(),
            dir,
        })
    }

    /// Deletes the engine on disk and any download left half done, and returns the bytes freed
    pub fn delete(&self) -> io::Result<u64> {
        let entries = match fs::read_dir(&self.root) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(0),
            entries => entries?,
        };
        let mut freed = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                freed += size_of(&path);
                remove(&path)?;
            }
        }
        Ok(freed)
    }
}

// links are counted as links, not what they point to
fn size_of(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    fs::read_dir(path)
        .map(|entries| entries.flatten().map(|entry| size_of(&entry.path())).sum())
        .unwrap_or(0)
}

// renamed away first, a half deleted engine never looks installed
fn remove(dir: &Path) -> io::Result<()> {
    let Some(parent) = dir.parent() else {
        return Ok(());
    };
    let trash = parent.join(".trash");
    match fs::remove_dir_all(&trash) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => return Err(err),
        _ => {}
    }
    match fs::rename(dir, &trash) {
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
        Ok(()) => fs::remove_dir_all(&trash),
    }
}

fn build_number(tag: &str) -> u64 {
    tag.trim_start_matches('b').parse().unwrap_or(0)
}

// upstream archives keep everything in one top folder, it's left out
fn unpack(archive: &Path, into: &Path) -> io::Result<()> {
    let name = archive.display();
    let mut tar = Archive::new(GzDecoder::new(BufReader::new(File::open(archive)?)));
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let inner: PathBuf = path.components().skip(1).collect();
        if inner.as_os_str().is_empty() {
            continue;
        }
        if !is_plain(&inner) {
            return Err(invalid(format!(
                "{name} has an unsafe path: {}",
                path.display()
            )));
        }
        match entry.header().entry_type() {
            EntryType::Regular | EntryType::Directory => {}
            // the libraries link to their versioned files next to them
            EntryType::Symlink => {
                let target = entry.link_name()?.unwrap_or_default();
                if target.components().count() != 1 || !is_plain(&target) {
                    return Err(invalid(format!(
                        "{name} links {} outside its folder",
                        path.display()
                    )));
                }
            }
            _ => continue,
        }
        let dest = into.join(&inner);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        entry.unpack(&dest)?;
    }
    Ok(())
}

fn is_plain(path: &Path) -> bool {
    path.components()
        .all(|part| matches!(part, Component::Normal(_)))
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Runs `llama-server --list-devices` from `dir`. A GPU engine passes when llama.cpp lists its GPU
pub async fn check(dir: &Path, engine: Engine) -> Result<(), String> {
    let libraries = match env::var("LD_LIBRARY_PATH") {
        Ok(prev) if !prev.is_empty() => format!("{}:{prev}", dir.display()),
        _ => dir.display().to_string(),
    };
    let output = Command::new(dir.join("llama-server"))
        .arg("--list-devices")
        .env("LD_LIBRARY_PATH", libraries)
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .output();
    let output = tokio::time::timeout(CHECK_TIMEOUT, output)
        .await
        .map_err(|_| "llama.cpp didn't list the GPUs within 3 minutes".to_owned())?
        .map_err(|err| format!("llama.cpp didn't start: {err}"))?;
    if !output.status.success() {
        return Err(start_failure(&String::from_utf8_lossy(&output.stderr)));
    }
    let backend = match engine {
        Engine::Cuda13 | Engine::Cuda12 => "CUDA",
        Engine::Vulkan => "Vulkan",
        Engine::Cpu | Engine::Metal => return Ok(()),
    };
    if lists(&String::from_utf8_lossy(&output.stdout), backend) {
        Ok(())
    } else {
        Err(format!("llama.cpp found no GPU it can use with {backend}"))
    }
}

// lines like "  CUDA0: NVIDIA GeForce RTX 5060 Ti (16310 MiB, 15671 MiB free)"
fn lists(devices: &str, backend: &str) -> bool {
    devices.lines().any(|line| {
        line.trim_start()
            .strip_prefix(backend)
            .and_then(|rest| rest.split_once(':'))
            .is_some_and(|(index, _)| {
                !index.is_empty() && index.chars().all(|c| c.is_ascii_digit())
            })
    })
}

fn start_failure(stderr: &str) -> String {
    if let Some(at) = stderr.find("GLIBC_") {
        let version: String = stderr[at + 6..]
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        return format!(
            "This build needs glibc {version} or newer, which comes with newer Linux versions"
        );
    }
    match stderr.lines().rev().find(|line| !line.trim().is_empty()) {
        Some(line) => format!("llama.cpp didn't start: {}", line.trim()),
        None => "llama.cpp didn't start".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Write, os::unix::fs::PermissionsExt};

    use flate2::{Compression, write::GzEncoder};
    use tar::{Builder, Header};

    use super::*;

    enum Item<'a> {
        File(&'a str, &'a [u8]),
        Link(&'a str, &'a str),
    }

    fn archive(dir: &Path, name: &str, items: &[Item]) -> PathBuf {
        let path = dir.join(name);
        let mut tar = Builder::new(GzEncoder::new(
            File::create(&path).unwrap(),
            Compression::fast(),
        ));
        for item in items {
            let mut header = Header::new_gnu();
            match item {
                Item::File(path, bytes) => {
                    header.set_size(bytes.len() as u64);
                    header.set_mode(0o755);
                    header.set_entry_type(EntryType::Regular);
                    // set_path refuses "..", the bytes are written as they are
                    header.as_old_mut().name[..path.len()].copy_from_slice(path.as_bytes());
                    header.set_cksum();
                    tar.append(&header, *bytes).unwrap();
                }
                Item::Link(path, target) => {
                    header.set_size(0);
                    header.set_entry_type(EntryType::Symlink);
                    header.as_old_mut().name[..path.len()].copy_from_slice(path.as_bytes());
                    header.as_old_mut().linkname[..target.len()].copy_from_slice(target.as_bytes());
                    header.set_cksum();
                    tar.append(&header, io::empty()).unwrap();
                }
            }
        }
        tar.into_inner().unwrap().finish().unwrap().flush().unwrap();
        path
    }

    fn server_script(dir: &Path, script: &str) -> PathBuf {
        archive(
            dir,
            "build.tar.gz",
            &[Item::File(
                "llama-b1/llama-server",
                format!("#!/bin/sh\n{script}\n").as_bytes(),
            )],
        )
    }

    #[test]
    fn archives_unpack_into_one_folder_without_their_top_folder() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("engines"));
        let build = archive(
            dir.path(),
            "build.tar.gz",
            &[
                Item::File("llama-b1/llama-server", b"server"),
                Item::File("llama-b1/libllama.so.0.8.0", b"lib"),
                Item::Link("llama-b1/libllama.so.0", "libllama.so.0.8.0"),
            ],
        );
        let runtime = archive(
            dir.path(),
            "runtime.tar.gz",
            &[Item::File("cudart-b1/libcudart.so.13", b"cudart")],
        );
        let staging = store.unpack(&[build, runtime]).unwrap();
        assert_eq!(fs::read(staging.join("libllama.so.0")).unwrap(), b"lib");
        assert_eq!(
            fs::read(staging.join("libcudart.so.13")).unwrap(),
            b"cudart"
        );
        let mode = fs::metadata(staging.join("llama-server"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111);
    }

    #[test]
    fn paths_and_links_out_of_the_folder_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("engines"));
        let escape = archive(
            dir.path(),
            "a.tar.gz",
            &[Item::File("top/../../evil", b"x")],
        );
        assert!(store.unpack(&[escape]).is_err());
        assert!(!dir.path().join("evil").exists());

        let link = archive(
            dir.path(),
            "b.tar.gz",
            &[Item::Link("top/lib.so", "/etc/passwd")],
        );
        assert!(store.unpack(&[link]).is_err());
        let up = archive(
            dir.path(),
            "c.tar.gz",
            &[Item::Link("top/lib.so", "../lib.so")],
        );
        assert!(store.unpack(&[up]).is_err());
    }

    #[test]
    fn finishing_keeps_only_the_new_engine() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("engines");
        let store = Store::new(root.clone());
        let old = root.join("cuda-12-b1");
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("llama-server"), "").unwrap();
        // newer, but without a server
        fs::create_dir_all(root.join("cuda-13-b9")).unwrap();
        fs::create_dir_all(store.downloads()).unwrap();
        assert_eq!(store.installed().unwrap().engine, Engine::Cuda12);

        let build = archive(
            dir.path(),
            "build.tar.gz",
            &[Item::File("top/llama-server", b"")],
        );
        store.unpack(&[build]).unwrap();
        // the old engine is still the one used while the new one is checked
        assert_eq!(store.installed().unwrap().tag, "b1");
        let installed = store.finish(Engine::Vulkan, "b2").unwrap();

        assert_eq!(store.installed(), Some(installed));
        assert!(!old.exists());
        assert!(!store.downloads().exists());
        assert!(!store.staging().exists());
    }

    #[test]
    fn deleting_removes_the_engine_and_any_half_download() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("engines");
        let store = Store::new(root.clone());
        assert_eq!(store.delete().unwrap(), 0);

        fs::create_dir_all(root.join("vulkan-b1")).unwrap();
        fs::write(root.join("vulkan-b1/llama-server"), "12345").unwrap();
        fs::create_dir_all(store.downloads()).unwrap();
        fs::write(store.downloads().join("cuda.tar.gz.part"), "123").unwrap();
        store
            .record_failure(Engine::Cuda13, "b1", "no GPU")
            .unwrap();

        assert_eq!(store.delete().unwrap(), 8);
        assert_eq!(store.installed(), None);
        assert!(!store.downloads().exists());
        // what failed its check still did
        assert!(!store.failures("b1").is_empty());
    }

    #[test]
    fn failures_count_only_for_their_build() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("engines"));
        store
            .record_failure(Engine::Cuda13, "b1", "no GPU")
            .unwrap();
        assert_eq!(store.failures("b1")[&Engine::Cuda13], "no GPU");
        assert!(store.failures("b2").is_empty());

        let mut choices = vec![Choice {
            engine: Engine::Cuda13,
            label: "CUDA 13".to_owned(),
            download: 1,
            unavailable: None,
        }];
        store.mark_failures(&mut choices, "b1");
        assert_eq!(choices[0].unavailable.as_deref(), Some("no GPU"));

        store.clear_failure(Engine::Cuda13).unwrap();
        assert!(store.failures("b1").is_empty());
    }

    #[test]
    fn an_old_glibc_says_what_it_needs() {
        let stderr = "./llama-server: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.34' not found (required by ./llama-server)";
        assert!(start_failure(stderr).contains("glibc 2.34 or newer"));
        assert_eq!(
            start_failure("error while loading shared libraries: libvulkan.so.1\n"),
            "llama.cpp didn't start: error while loading shared libraries: libvulkan.so.1"
        );
    }

    #[tokio::test]
    async fn the_check_reads_the_device_list() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("engines"));
        let lists_cuda = server_script(
            dir.path(),
            "echo 'Available devices:'; echo '  CUDA0: GPU (1 MiB, 1 MiB free)'",
        );
        let staging = store.unpack(&[lists_cuda]).unwrap();
        assert_eq!(check(&staging, Engine::Cuda13).await, Ok(()));
        assert!(
            check(&staging, Engine::Vulkan)
                .await
                .unwrap_err()
                .contains("Vulkan")
        );
        let not_a_device = server_script(dir.path(), "echo '  CUDA: not a device'");
        let staging = store.unpack(&[not_a_device]).unwrap();
        assert!(check(&staging, Engine::Cuda13).await.is_err());

        let crashes = server_script(dir.path(), "echo 'libcuda.so.1: cannot open' >&2; exit 127");
        let staging = store.unpack(&[crashes]).unwrap();
        assert_eq!(
            check(&staging, Engine::Cuda13).await.unwrap_err(),
            "llama.cpp didn't start: libcuda.so.1: cannot open"
        );
        // the CPU build only has to start
        let none = server_script(dir.path(), "echo '  (none)'");
        let staging = store.unpack(&[none]).unwrap();
        assert_eq!(check(&staging, Engine::Cpu).await, Ok(()));
    }
}
