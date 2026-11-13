//! The llama.cpp builds Tiles runs models with, and which one suits this computer

mod detect;
mod install;

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

pub use detect::{Gpu, Hardware, Vendor, detect, nvidia_probe};
pub use install::{Installed, Store, check};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Engine {
    #[serde(rename = "cuda-13")]
    Cuda13,
    #[serde(rename = "cuda-12")]
    Cuda12,
    #[serde(rename = "vulkan")]
    Vulkan,
    #[serde(rename = "cpu")]
    Cpu,
    #[serde(rename = "metal")]
    Metal,
}

impl Engine {
    pub const ALL: [Engine; 5] = [
        Engine::Cuda13,
        Engine::Cuda12,
        Engine::Vulkan,
        Engine::Cpu,
        Engine::Metal,
    ];

    /// The name used in the API, config and folder names
    pub fn name(self) -> &'static str {
        match self {
            Engine::Cuda13 => "cuda-13",
            Engine::Cuda12 => "cuda-12",
            Engine::Vulkan => "vulkan",
            Engine::Cpu => "cpu",
            Engine::Metal => "metal",
        }
    }

    pub fn from_name(name: &str) -> Option<Engine> {
        Engine::ALL.into_iter().find(|engine| engine.name() == name)
    }

    /// Whether it comes with Tiles rather than being downloaded
    pub fn is_bundled(self) -> bool {
        matches!(self, Engine::Cpu | Engine::Metal)
    }
}

/// The Linux builds of the pinned llama.cpp release, written by scripts/llama_engines.sh
#[derive(Debug, Deserialize)]
pub struct Builds {
    pub tag: String,
    pub engines: Vec<Build>,
}

#[derive(Debug, Deserialize)]
pub struct Build {
    pub engine: Engine,
    pub arch: String,
    pub version: Option<String>,
    pub files: Vec<BuildFile>,
}

#[derive(Debug, Deserialize)]
pub struct BuildFile {
    pub name: String,
    pub size: u64,
    pub unpacked: u64,
    pub sha256: String,
}

impl Builds {
    /// The build of `engine` for this processor
    pub fn find(&self, engine: Engine) -> Option<&Build> {
        self.engines
            .iter()
            .find(|build| build.engine == engine && build.arch == std::env::consts::ARCH)
    }
}

pub static BUILDS: LazyLock<Builds> = LazyLock::new(|| {
    serde_json::from_str(include_str!("engines.json")).expect("engines.json is valid")
});

/// An engine as the user can pick it
#[derive(Debug, PartialEq, Serialize)]
pub struct Choice {
    pub engine: Engine,
    pub label: String,
    // bytes to download, 0 when it ships with Tiles
    pub download: u64,
    // why it can't run here
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<String>,
}

// the oldest drivers the CUDA builds run on
const CUDA_12_DRIVER: (&str, (u32, u32)) = ("570.26", (12, 8));
const CUDA_13_DRIVER: (&str, (u32, u32)) = ("580.65.06", (13, 0));

// GPUs the CUDA 13 build has compiled code for (ggml-cuda's CMakeLists). others compile it
// when the model loads, which needs a driver as new as the build's toolkit
const CUDA_13_NATIVE: [(u32, u32); 4] = [(8, 6), (8, 9), (12, 0), (12, 1)];

/// Every engine for this computer
pub fn choices(hardware: &Hardware, builds: &Builds) -> Vec<Choice> {
    if hardware.os == "macos" {
        return vec![Choice {
            engine: Engine::Metal,
            label: "Metal".to_owned(),
            download: 0,
            unavailable: None,
        }];
    }
    [Engine::Cuda13, Engine::Cuda12, Engine::Vulkan, Engine::Cpu]
        .into_iter()
        .map(|engine| {
            let build = builds
                .engines
                .iter()
                .find(|build| build.engine == engine && build.arch == hardware.arch);
            Choice {
                engine,
                label: label(engine, build),
                // the CPU build ships with Tiles
                download: match engine {
                    Engine::Cpu => 0,
                    _ => build.map_or(0, |build| build.files.iter().map(|f| f.size).sum()),
                },
                unavailable: unavailable(engine, hardware, build.is_some()),
            }
        })
        .collect()
}

/// The engine Tiles uses unless the user picks one
pub fn auto(hardware: &Hardware, choices: &[Choice]) -> Engine {
    let nvidia: Vec<_> = hardware.nvidia_gpus().collect();
    let native = !nvidia.is_empty()
        && nvidia.iter().all(|gpu| {
            gpu.compute_capability
                .is_some_and(|cc| CUDA_13_NATIVE.contains(&cc))
        });
    // other vendors are mostly VM display devices, where Vulkan only finds the CPU
    let known_gpu = hardware.gpus.iter().any(|gpu| gpu.vendor != Vendor::Other);
    choices
        .iter()
        .filter(|choice| choice.unavailable.is_none())
        .find(|choice| match choice.engine {
            // CUDA 12 covers every GPU, CUDA 13 only those it was compiled for
            Engine::Cuda13 => native || hardware.arch == "aarch64",
            Engine::Vulkan => known_gpu,
            _ => true,
        })
        .map_or(Engine::Cpu, |choice| choice.engine)
}

/// The engine Tiles runs: the backend the user set while it can run here, otherwise auto
pub fn engine_to_run(backend: Option<Engine>, hardware: &Hardware, choices: &[Choice]) -> Engine {
    backend
        .filter(|backend| {
            choices
                .iter()
                .any(|choice| choice.engine == *backend && choice.unavailable.is_none())
        })
        .unwrap_or_else(|| auto(hardware, choices))
}

fn label(engine: Engine, build: Option<&Build>) -> String {
    let version = build.and_then(|build| build.version.as_deref());
    match (engine, version) {
        (Engine::Cuda13 | Engine::Cuda12, Some(version)) => format!("CUDA {version}"),
        (Engine::Cuda13, None) => "CUDA 13".to_owned(),
        (Engine::Cuda12, None) => "CUDA 12".to_owned(),
        (Engine::Vulkan, _) => "Vulkan".to_owned(),
        (Engine::Cpu, _) => "CPU".to_owned(),
        (Engine::Metal, _) => "Metal".to_owned(),
    }
}

fn unavailable(engine: Engine, hardware: &Hardware, built: bool) -> Option<String> {
    match engine {
        Engine::Cpu => None,
        _ if !built => Some("llama.cpp has no build of it for this processor".to_owned()),
        Engine::Cuda13 => cuda(hardware, CUDA_13_DRIVER),
        Engine::Cuda12 => cuda(hardware, CUDA_12_DRIVER),
        Engine::Vulkan if hardware.gpus.is_empty() => Some("No GPU found".to_owned()),
        Engine::Vulkan if !hardware.vulkan_loader => Some(
            "No Vulkan driver found. Install your distro's Vulkan packages, like mesa-vulkan-drivers"
                .to_owned(),
        ),
        Engine::Vulkan => None,
        Engine::Metal => Some("Metal is only on macOS".to_owned()),
    }
}

fn cuda(hardware: &Hardware, (oldest, needs): (&str, (u32, u32))) -> Option<String> {
    if hardware.nvidia_gpus().next().is_none() {
        return Some("No NVIDIA GPU found".to_owned());
    }
    match (&hardware.nvidia_driver, hardware.nvidia_cuda) {
        (Some(driver), _) => (version(driver) < version(oldest))
            .then(|| format!("Needs NVIDIA driver {oldest} or newer, this one is {driver}")),
        // only libcuda answered, with the CUDA version and no driver number
        (None, Some(cuda)) => (cuda < needs).then(|| {
            format!(
                "Needs an NVIDIA driver for CUDA {}.{} or newer, this one is for CUDA {}.{}",
                needs.0, needs.1, cuda.0, cuda.1
            )
        }),
        (None, None) => Some("The NVIDIA driver isn't running".to_owned()),
    }
}

fn version(text: &str) -> Vec<u32> {
    text.split('.')
        .map_while(|part| part.parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linux(gpus: Vec<Gpu>, driver: Option<&str>) -> Hardware {
        Hardware {
            os: "linux".to_owned(),
            arch: "x86_64".to_owned(),
            gpus,
            nvidia_driver: driver.map(str::to_owned),
            nvidia_cuda: None,
            vulkan_loader: true,
        }
    }

    fn nvidia(cc: Option<(u32, u32)>) -> Gpu {
        Gpu {
            vendor: Vendor::Nvidia,
            name: Some("NVIDIA GPU".to_owned()),
            compute_capability: cc,
            vram: None,
        }
    }

    fn other(vendor: Vendor) -> Gpu {
        Gpu {
            vendor,
            name: None,
            compute_capability: None,
            vram: None,
        }
    }

    fn picked(hardware: &Hardware) -> Engine {
        auto(hardware, &choices(hardware, &BUILDS))
    }

    fn reason(hardware: &Hardware, engine: Engine) -> Option<String> {
        choices(hardware, &BUILDS)
            .into_iter()
            .find(|choice| choice.engine == engine)
            .and_then(|choice| choice.unavailable)
    }

    #[test]
    fn names_match_the_api() {
        for engine in Engine::ALL {
            assert_eq!(serde_json::to_value(engine).unwrap(), engine.name());
            assert_eq!(Engine::from_name(engine.name()), Some(engine));
        }
    }

    #[test]
    fn the_pinned_builds_load() {
        assert!(BUILDS.tag.starts_with('b'));
        for engine in [Engine::Cuda13, Engine::Vulkan, Engine::Cpu] {
            for arch in ["x86_64", "aarch64"] {
                assert!(
                    BUILDS
                        .engines
                        .iter()
                        .any(|b| b.engine == engine && b.arch == arch),
                    "{engine:?} on {arch}"
                );
            }
        }
    }

    #[test]
    fn a_new_nvidia_gpu_and_driver_get_cuda_13() {
        let rtx_50 = linux(vec![nvidia(Some((12, 0)))], Some("580.82.07"));
        assert_eq!(picked(&rtx_50), Engine::Cuda13);
    }

    #[test]
    fn older_drivers_step_down_to_cuda_12_then_vulkan() {
        let below_580 = linux(vec![nvidia(Some((8, 9)))], Some("575.64"));
        assert_eq!(picked(&below_580), Engine::Cuda12);
        assert!(
            reason(&below_580, Engine::Cuda13)
                .unwrap()
                .contains("580.65.06")
        );

        let below_570 = linux(vec![nvidia(Some((8, 6)))], Some("550.54.14"));
        assert_eq!(picked(&below_570), Engine::Vulkan);
        assert!(
            reason(&below_570, Engine::Cuda12)
                .unwrap()
                .contains("570.26")
        );
    }

    #[test]
    fn gpus_cuda_13_doesnt_compile_for_get_cuda_12() {
        for cc in [Some((6, 1)), Some((7, 5)), Some((8, 0)), Some((9, 0)), None] {
            let hardware = linux(vec![nvidia(cc)], Some("590.10"));
            assert_eq!(picked(&hardware), Engine::Cuda12, "{cc:?}");
        }
        // one old card among new ones decides
        let mixed = linux(
            vec![nvidia(Some((12, 0))), nvidia(Some((7, 5)))],
            Some("590.10"),
        );
        assert_eq!(picked(&mixed), Engine::Cuda12);
    }

    #[test]
    fn an_nvidia_gpu_without_its_driver_gets_vulkan() {
        let hardware = linux(vec![nvidia(None)], None);
        assert_eq!(picked(&hardware), Engine::Vulkan);
        assert_eq!(
            reason(&hardware, Engine::Cuda13).unwrap(),
            "The NVIDIA driver isn't running"
        );
    }

    #[test]
    fn arm_has_no_cuda_12_and_takes_cuda_13() {
        let mut gb10 = linux(vec![nvidia(Some((12, 1)))], Some("580.95.05"));
        gb10.arch = "aarch64".to_owned();
        assert_eq!(picked(&gb10), Engine::Cuda13);
        assert!(reason(&gb10, Engine::Cuda12).is_some());

        // a Jetson knows its driver only by the CUDA version it supports
        let mut orin = linux(vec![nvidia(Some((8, 7)))], None);
        orin.arch = "aarch64".to_owned();
        orin.nvidia_cuda = Some((12, 6));
        assert_eq!(picked(&orin), Engine::Vulkan);
        assert!(reason(&orin, Engine::Cuda13).unwrap().contains("CUDA 12.6"));

        orin.nvidia_cuda = Some((13, 0));
        assert_eq!(picked(&orin), Engine::Cuda13);
    }

    #[test]
    fn amd_and_intel_get_vulkan() {
        for vendor in [Vendor::Amd, Vendor::Intel] {
            let hardware = linux(vec![other(vendor)], None);
            assert_eq!(picked(&hardware), Engine::Vulkan);
            assert_eq!(
                reason(&hardware, Engine::Cuda13).unwrap(),
                "No NVIDIA GPU found"
            );
        }
    }

    #[test]
    fn a_vm_display_device_gets_the_cpu_but_can_pick_vulkan() {
        let vm = linux(vec![other(Vendor::Other)], None);
        assert_eq!(picked(&vm), Engine::Cpu);
        assert_eq!(reason(&vm, Engine::Vulkan), None);
    }

    #[test]
    fn without_a_vulkan_driver_or_a_gpu_its_the_cpu() {
        let mut no_loader = linux(vec![other(Vendor::Amd)], None);
        no_loader.vulkan_loader = false;
        assert_eq!(picked(&no_loader), Engine::Cpu);
        assert!(
            reason(&no_loader, Engine::Vulkan)
                .unwrap()
                .contains("Vulkan packages")
        );

        let no_gpu = linux(vec![], None);
        assert_eq!(picked(&no_gpu), Engine::Cpu);
        assert_eq!(reason(&no_gpu, Engine::Vulkan).unwrap(), "No GPU found");
    }

    #[test]
    fn a_pick_that_cant_run_here_falls_back_to_auto() {
        let hardware = linux(vec![other(Vendor::Amd)], None);
        let choices = choices(&hardware, &BUILDS);
        assert_eq!(
            engine_to_run(Some(Engine::Cpu), &hardware, &choices),
            Engine::Cpu
        );
        assert_eq!(
            engine_to_run(Some(Engine::Cuda13), &hardware, &choices),
            Engine::Vulkan
        );
        assert_eq!(engine_to_run(None, &hardware, &choices), Engine::Vulkan);
    }

    #[test]
    fn macos_has_metal_only() {
        let mac = Hardware {
            os: "macos".to_owned(),
            arch: "aarch64".to_owned(),
            gpus: vec![],
            nvidia_driver: None,
            nvidia_cuda: None,
            vulkan_loader: false,
        };
        let choices = choices(&mac, &BUILDS);
        assert_eq!(choices.len(), 1);
        assert_eq!(auto(&mac, &choices), Engine::Metal);
    }

    #[test]
    fn downloads_count_every_file_and_the_cpu_ships_with_tiles() {
        let hardware = linux(vec![nvidia(Some((12, 0)))], Some("580.82.07"));
        let choices = choices(&hardware, &BUILDS);
        let cuda = &choices[0];
        assert_eq!(cuda.label, "CUDA 13.5");
        // the build and its runtime
        assert!(cuda.download > 500_000_000);
        assert_eq!(choices.last().unwrap().download, 0);
    }
}
