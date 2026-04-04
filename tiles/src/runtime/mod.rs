#[allow(unused_imports)]
use crate::runtime::cpu::CPURuntime;
use crate::{core::storage::db::Dbconn, runtime::mlx::MLXRuntime};
use anyhow::Result;
pub mod cpu;
pub mod mlx;

pub struct RunArgs {
    pub modelfile_path: Option<String>,
    pub relay_count: u32,
    pub memory: bool, // Future flags go here
    pub pi: bool,
}

pub enum Runtime {
    Mlx(MLXRuntime),
    Cpu(CPURuntime),
}

impl Runtime {
    pub async fn run(&self, run_args: RunArgs, db_conn: &Dbconn) -> Result<()> {
        match self {
            Runtime::Mlx(runtime) => runtime.run(run_args, db_conn).await,
            Runtime::Cpu(runtime) => runtime.run(run_args).await,
        }
    }

    pub async fn start_server_daemon(&self) -> Result<()> {
        match self {
            Runtime::Mlx(runtime) => runtime.start_server_daemon().await,
            Runtime::Cpu(runtime) => runtime.start_server_daemon().await,
        }
    }

    pub async fn stop_server_daemon(&self) -> Result<()> {
        match self {
            Runtime::Mlx(runtime) => runtime.stop_server_daemon().await,
            Runtime::Cpu(runtime) => runtime.stop_server_daemon().await,
        }
    }
}

#[cfg(target_os = "macos")]
pub fn build_runtime() -> Runtime {
    Runtime::Mlx(MLXRuntime::new())
}

#[cfg(not(target_os = "macos"))]
pub fn build_runtime() -> Runtime {
    Runtime::Cpu(CPURuntime::new())
}
