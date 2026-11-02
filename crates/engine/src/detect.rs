//! What GPUs and drivers this computer has

use std::{env, fs, path::Path, process::Stdio, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug)]
pub struct Hardware {
    pub macos: bool,
    pub arch: String,
    pub gpus: Vec<Gpu>,
    pub nvidia_driver: Option<String>,
    // the newest CUDA the NVIDIA driver runs, when the driver libraries answered
    pub nvidia_cuda: Option<(u32, u32)>,
    pub vulkan_loader: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Gpu {
    pub vendor: Vendor,
    pub name: Option<String>,
    #[serde(with = "dotted")]
    pub compute_capability: Option<(u32, u32)>,
    // bytes
    pub vram: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Vendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Other,
}

impl Hardware {
    pub fn nvidia_gpus(&self) -> impl Iterator<Item = &Gpu> {
        self.gpus.iter().filter(|gpu| gpu.vendor == Vendor::Nvidia)
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Nvidia {
    gpus: Vec<Gpu>,
    driver: Option<String>,
    cuda: Option<(u32, u32)>,
}

// nvidia-smi and the driver libraries hang when the driver is stuck
const DEADLINE: Duration = Duration::from_secs(10);

/// Asks nvidia-smi first, then the driver's /proc files and libraries for what it couldn't tell
pub async fn detect() -> Hardware {
    let arch = env::consts::ARCH.to_owned();
    if cfg!(target_os = "macos") {
        return Hardware {
            macos: true,
            arch,
            gpus: apple_chip().await.into_iter().collect(),
            nvidia_driver: None,
            nvidia_cuda: None,
            vulkan_loader: false,
        };
    }

    let mut nvidia = nvidia_smi().await.unwrap_or_default();
    if nvidia.gpus.is_empty() {
        nvidia.gpus = proc_gpus(Path::new("/proc/driver/nvidia/gpus"));
    }
    let incomplete = nvidia.gpus.is_empty()
        || nvidia.driver.is_none()
        || nvidia
            .gpus
            .iter()
            .any(|gpu| gpu.compute_capability.is_none());
    if incomplete && let Some(found) = libraries().await {
        if !found.gpus.is_empty() {
            nvidia.gpus = found.gpus;
        }
        nvidia.driver = nvidia.driver.or(found.driver);
        nvidia.cuda = found.cuda;
    }
    if nvidia.driver.is_none() {
        nvidia.driver = fs::read_to_string("/proc/driver/nvidia/version")
            .ok()
            .and_then(|text| proc_driver_version(&text));
    }

    // an NVIDIA card its driver doesn't run, like under nouveau, only shows up here
    let found_nvidia = !nvidia.gpus.is_empty();
    let mut gpus = nvidia.gpus;
    gpus.extend(
        drm_vendors(Path::new("/sys/class/drm"))
            .into_iter()
            .filter(|vendor| *vendor != Vendor::Nvidia || !found_nvidia)
            .map(|vendor| Gpu {
                vendor,
                name: None,
                compute_capability: None,
                vram: None,
            }),
    );

    Hardware {
        macos: false,
        arch,
        gpus,
        nvidia_driver: nvidia.driver,
        nvidia_cuda: nvidia.cuda,
        vulkan_loader: vulkan_loader(),
    }
}

// WSL keeps it outside PATH
const WSL_NVIDIA_SMI: &str = "/usr/lib/wsl/lib/nvidia-smi";

/// The Apple silicon chip, with its unified memory as the GPU's
async fn apple_chip() -> Option<Gpu> {
    let output = Command::new("/usr/sbin/sysctl")
        .args(["-n", "machdep.cpu.brand_string", "hw.memsize"])
        .kill_on_drop(true)
        .output();
    let output = tokio::time::timeout(DEADLINE, output)
        .await
        .ok()?
        .ok()
        .filter(|output| output.status.success())?;
    parse_sysctl(&String::from_utf8_lossy(&output.stdout))
}

// an Intel Mac names its CPU here, not its GPU, and gets nothing
fn parse_sysctl(text: &str) -> Option<Gpu> {
    let mut lines = text.lines().map(str::trim);
    let name = lines.next().filter(|name| name.starts_with("Apple"))?;
    Some(Gpu {
        vendor: Vendor::Apple,
        name: Some(name.to_owned()),
        compute_capability: None,
        vram: lines.next().and_then(|bytes| bytes.parse().ok()),
    })
}

async fn nvidia_smi() -> Option<Nvidia> {
    let program = if Path::new(WSL_NVIDIA_SMI).exists() {
        WSL_NVIDIA_SMI
    } else {
        "nvidia-smi"
    };
    let output = Command::new(program)
        .args([
            "--query-gpu=name,driver_version,compute_cap,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .kill_on_drop(true)
        .output();
    let output = tokio::time::timeout(DEADLINE, output)
        .await
        .ok()?
        .ok()
        .filter(|output| output.status.success())?;
    Some(parse_nvidia_smi(&String::from_utf8_lossy(&output.stdout)))
}

// memory in MiB
fn parse_nvidia_smi(text: &str) -> Nvidia {
    let mut driver = None;
    let gpus = text
        .lines()
        .filter_map(|line| {
            let mut fields = line.rsplitn(4, ',').map(str::trim);
            let (vram, cc, version, name) = (
                fields.next()?,
                fields.next()?,
                fields.next()?,
                fields.next()?,
            );
            driver.get_or_insert_with(|| version.to_owned());
            Some(Gpu {
                vendor: Vendor::Nvidia,
                name: Some(name.to_owned()),
                compute_capability: dotted::parse(cc),
                vram: vram.parse::<u64>().ok().map(|mib| mib * 1024 * 1024),
            })
        })
        .collect();
    Nvidia {
        gpus,
        driver,
        cuda: None,
    }
}

// the driver keeps a folder per GPU here, even when nvidia-smi is missing
fn proc_gpus(dir: &Path) -> Vec<Gpu> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    let mut folders: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
    folders.sort();
    folders
        .iter()
        .map(|folder| Gpu {
            vendor: Vendor::Nvidia,
            name: fs::read_to_string(folder.join("information"))
                .ok()
                .and_then(|text| proc_gpu_model(&text)),
            compute_capability: None,
            vram: None,
        })
        .collect()
}

fn proc_gpu_model(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix("Model:"))
        .map(|model| model.trim().to_owned())
}

/// Prints what NVML or libcuda tell, for `tiles nvidia-probe`
pub fn nvidia_probe() {
    #[cfg(target_os = "linux")]
    let found = libs::nvml().or_else(libs::cuda_driver);
    #[cfg(not(target_os = "linux"))]
    let found: Option<Nvidia> = None;
    println!("{}", serde_json::to_string(&found).unwrap_or_default());
}

// the driver libraries run in a child, so a stuck or crashing driver can be killed
async fn libraries() -> Option<Nvidia> {
    // the test binary isn't tiles
    if cfg!(test) {
        return None;
    }
    let output = Command::new(env::current_exe().ok()?)
        .arg("nvidia-probe")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .output();
    let output = tokio::time::timeout(DEADLINE, output).await.ok()?.ok()?;
    serde_json::from_slice(&output.stdout).ok().flatten()
}

#[cfg(target_os = "linux")]
mod libs {
    use std::ffi::{CStr, c_char, c_int, c_uint};

    use libloading::Library;
    use nvml_wrapper::Nvml;

    use super::{Gpu, Nvidia, Vendor};

    pub fn nvml() -> Option<Nvidia> {
        let nvml = Nvml::init().ok()?;
        let gpus: Vec<_> = (0..nvml.device_count().ok()?)
            .filter_map(|index| nvml.device_by_index(index).ok())
            .map(|device| Gpu {
                vendor: Vendor::Nvidia,
                name: device.name().ok(),
                compute_capability: device
                    .cuda_compute_capability()
                    .ok()
                    .and_then(|cc| Some((cc.major.try_into().ok()?, cc.minor.try_into().ok()?))),
                vram: device.memory_info().ok().map(|memory| memory.total),
            })
            .collect();
        (!gpus.is_empty()).then(|| Nvidia {
            gpus,
            driver: nvml.sys_driver_version().ok(),
            cuda: nvml.sys_cuda_driver_version().ok().and_then(packed),
        })
    }

    type Init = unsafe extern "C" fn(c_uint) -> c_int;
    type Count = unsafe extern "C" fn(*mut c_int) -> c_int;
    type Device = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
    type Name = unsafe extern "C" fn(*mut c_char, c_int, c_int) -> c_int;
    type Attribute = unsafe extern "C" fn(*mut c_int, c_int, c_int) -> c_int;
    type Memory = unsafe extern "C" fn(*mut usize, c_int) -> c_int;

    const CC_MAJOR: c_int = 75;
    const CC_MINOR: c_int = 76;

    /// The CUDA driver API from libcuda, for drivers without NVML like Jetson's
    pub fn cuda_driver() -> Option<Nvidia> {
        // SAFETY: libcuda is the NVIDIA driver's own library. it's never unloaded,
        // unloading it after cuInit isn't safe, and the probe exits right after
        let lib: &Library = Box::leak(Box::new(unsafe { Library::new("libcuda.so.1") }.ok()?));
        // SAFETY: the types match cuda.h and every pointer is to a local
        unsafe {
            let init = lib.get::<Init>(b"cuInit\0").ok()?;
            let driver_version = lib.get::<Count>(b"cuDriverGetVersion\0").ok()?;
            let device_count = lib.get::<Count>(b"cuDeviceGetCount\0").ok()?;
            let device = lib.get::<Device>(b"cuDeviceGet\0").ok()?;
            let name = lib.get::<Name>(b"cuDeviceGetName\0").ok()?;
            let attribute = lib.get::<Attribute>(b"cuDeviceGetAttribute\0").ok()?;
            let memory = lib.get::<Memory>(b"cuDeviceTotalMem_v2\0").ok()?;

            let (mut version, mut count) = (0, 0);
            if init(0) != 0 || device_count(&mut count) != 0 {
                return None;
            }
            driver_version(&mut version);
            let gpus: Vec<_> = (0..count)
                .filter_map(|index| {
                    let mut handle = 0;
                    if device(&mut handle, index) != 0 {
                        return None;
                    }
                    let mut text = [0u8; 96];
                    let (mut major, mut minor, mut bytes) = (0, 0, 0);
                    Some(Gpu {
                        vendor: Vendor::Nvidia,
                        // a name without its NUL ending is dropped instead of read past the buffer
                        name: (name(text.as_mut_ptr().cast(), text.len() as c_int, handle) == 0)
                            .then(|| CStr::from_bytes_until_nul(&text).ok())
                            .flatten()
                            .map(|name| name.to_string_lossy().into_owned()),
                        compute_capability: (attribute(&mut major, CC_MAJOR, handle) == 0
                            && attribute(&mut minor, CC_MINOR, handle) == 0)
                            .then(|| u32::try_from(major).ok().zip(u32::try_from(minor).ok()))
                            .flatten(),
                        vram: (memory(&mut bytes, handle) == 0).then_some(bytes as u64),
                    })
                })
                .collect();
            (!gpus.is_empty()).then(|| Nvidia {
                gpus,
                driver: None,
                cuda: packed(version),
            })
        }
    }

    // 13010 is CUDA 13.1
    fn packed(version: i32) -> Option<(u32, u32)> {
        let version = u32::try_from(version).ok().filter(|&version| version > 0)?;
        Some((version / 1000, version % 1000 / 10))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn cuda_versions_unpack() {
            assert_eq!(packed(13010), Some((13, 1)));
            assert_eq!(packed(12080), Some((12, 8)));
            assert_eq!(packed(0), None);
        }
    }
}

// "NVRM version"
fn proc_driver_version(text: &str) -> Option<String> {
    text.lines()
        .find(|line| line.starts_with("NVRM version"))?
        .split_whitespace()
        .find(|word| word.contains('.') && word.split('.').all(|part| part.parse::<u32>().is_ok()))
        .map(str::to_owned)
}

// one vendor per graphics card, from card0, card1… but not their outputs like card0-DP-1
fn drm_vendors(drm: &Path) -> Vec<Vendor> {
    let Ok(entries) = fs::read_dir(drm) else {
        return vec![];
    };
    let mut cards: Vec<_> = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_prefix("card"))
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
        .map(|entry| entry.path())
        .collect();
    cards.sort();
    cards
        .iter()
        .filter_map(|card| fs::read_to_string(card.join("device/vendor")).ok())
        .map(|id| match id.trim() {
            "0x10de" => Vendor::Nvidia,
            "0x1002" => Vendor::Amd,
            "0x8086" => Vendor::Intel,
            _ => Vendor::Other,
        })
        .collect()
}

// the loader the Vulkan build links to. without ld.so.cache, like on NixOS, assume it's there
fn vulkan_loader() -> bool {
    const LOADER: &str = "libvulkan.so.1";
    let in_path = env::var("LD_LIBRARY_PATH").is_ok_and(|dirs| {
        dirs.split(':')
            .any(|dir| !dir.is_empty() && Path::new(dir).join(LOADER).exists())
    });
    in_path
        || fs::read("/etc/ld.so.cache").map_or(true, |cache| {
            cache
                .windows(LOADER.len() + 1)
                .any(|window| window.starts_with(LOADER.as_bytes()) && window[LOADER.len()] == 0)
        })
}

// compute capability as "12.0"
mod dotted {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn parse(text: &str) -> Option<(u32, u32)> {
        let (major, minor) = text.split_once('.')?;
        Some((major.parse().ok()?, minor.parse().ok()?))
    }

    pub fn serialize<S: Serializer>(cc: &Option<(u32, u32)>, s: S) -> Result<S::Ok, S::Error> {
        match cc {
            Some((major, minor)) => s.serialize_str(&format!("{major}.{minor}")),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<(u32, u32)>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|text| parse(&text).ok_or_else(|| D::Error::custom("not a compute capability")))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn an_apple_chip_reads_its_name_and_memory() {
        let gpu = parse_sysctl("Apple M4 Pro\n34359738368\n").unwrap();
        assert_eq!(gpu.vendor, Vendor::Apple);
        assert_eq!(gpu.name.as_deref(), Some("Apple M4 Pro"));
        assert_eq!(gpu.vram, Some(32 << 30));
        let intel = "Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz\n17179869184\n";
        assert_eq!(parse_sysctl(intel), None);
    }

    #[test]
    fn nvidia_smi_lists_each_gpu() {
        let nvidia = parse_nvidia_smi(
            "NVIDIA GeForce RTX 5060 Ti, 580.82.07, 12.0, 16311\nTesla P40, 580.82.07, 6.1, 24576\n",
        );
        assert_eq!(nvidia.driver.as_deref(), Some("580.82.07"));
        let gpus = nvidia.gpus;
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[0].name.as_deref(), Some("NVIDIA GeForce RTX 5060 Ti"));
        assert_eq!(gpus[0].compute_capability, Some((12, 0)));
        assert_eq!(gpus[0].vram, Some(16311 * 1024 * 1024));
        assert_eq!(gpus[1].compute_capability, Some((6, 1)));
    }

    #[test]
    fn a_gpu_nvidia_smi_cant_fully_read_keeps_what_it_could() {
        let gpus = parse_nvidia_smi("NVIDIA A100, 570.86.15, [N/A], [N/A]\n").gpus;
        assert_eq!(gpus[0].compute_capability, None);
        assert_eq!(gpus[0].vram, None);
    }

    #[test]
    fn the_driver_folders_name_each_gpu() {
        let dir = tempdir().unwrap();
        for (bus, model) in [
            ("0000:01:00.0", "NVIDIA GeForce RTX 5060 Ti"),
            ("0000:02:00.0", "Tesla P40"),
        ] {
            fs::create_dir(dir.path().join(bus)).unwrap();
            fs::write(
                dir.path().join(bus).join("information"),
                format!("Model: \t\t {model}\nIRQ:   \t\t 99\n"),
            )
            .unwrap();
        }
        let names: Vec<_> = proc_gpus(dir.path())
            .into_iter()
            .map(|gpu| gpu.name)
            .collect();
        assert_eq!(
            names,
            [
                Some("NVIDIA GeForce RTX 5060 Ti".to_owned()),
                Some("Tesla P40".to_owned())
            ]
        );
        assert!(proc_gpus(&dir.path().join("missing")).is_empty());
    }

    #[test]
    fn the_kernel_module_reports_the_driver() {
        let text = "NVRM version: NVIDIA UNIX Open Kernel Module for x86_64  580.82.07  Release Build  (dvs-builder@U22-I3-AF03-09-1)  Fri Aug 29 2025\nGCC version:  gcc version 12.3.0\n";
        assert_eq!(proc_driver_version(text).as_deref(), Some("580.82.07"));
        assert_eq!(proc_driver_version("GCC version: 12.3.0"), None);
    }

    #[test]
    fn graphics_cards_are_read_but_not_their_outputs() {
        let drm = tempdir().unwrap();
        for (card, vendor) in [
            ("card0", "0x8086"),
            ("card1", "0x10de"),
            ("card2", "0x1002"),
        ] {
            fs::create_dir_all(drm.path().join(card).join("device")).unwrap();
            fs::write(
                drm.path().join(card).join("device/vendor"),
                format!("{vendor}\n"),
            )
            .unwrap();
        }
        fs::create_dir_all(drm.path().join("card0-DP-1/device")).unwrap();
        fs::write(drm.path().join("card0-DP-1/device/vendor"), "0x8086\n").unwrap();
        fs::create_dir_all(drm.path().join("renderD128")).unwrap();

        assert_eq!(
            drm_vendors(drm.path()),
            [Vendor::Intel, Vendor::Nvidia, Vendor::Amd]
        );
    }
}
