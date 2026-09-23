//! Module for inference server

use anyhow::{Context, Result, anyhow};
use nix::unistd::setsid;
use reqwest::Client;
use std::{
    fs::OpenOptions,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use crate::utils::config::{ConfigProvider, DefaultProvider, PY_PORT};

#[allow(clippy::zombie_processes)]
pub async fn start_server_daemon() -> Result<String> {
    // check if the server is running
    // start server as a child process
    // save the pid in a file under ~/.config/tiles/server_pid

    if (ping().await).is_ok() {
        let msg = "server already up";
        log::info!("{}", msg);
        return Ok(msg.to_owned());
    }

    let config_dir = DefaultProvider.get_config_dir()?;
    let data_dir = DefaultProvider.get_data_dir()?;
    let server_dir = DefaultProvider.get_lib_dir()?;
    let pid_file = config_dir.join("server.pid");
    let stdout_log = OpenOptions::new()
        .append(true)
        .open(data_dir.join("logs/server.out.log"))?;
    let stderr_log = OpenOptions::new()
        .append(true)
        .open(data_dir.join("logs/server.err.log"))?;
    let server_path = if cfg!(debug_assertions) {
        server_dir.join("server/.venv/bin/python3")
    } else {
        server_dir.join("server/stack_export_prod/app-server/bin/python")
    };
    let child = unsafe {
        Command::new(server_path)
            .args(["-m", "server.main"])
            .current_dir(server_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout_log))
            .stderr(Stdio::from(stderr_log))
            .pre_exec(|| {
                if let Err(err) = setsid() {
                    Err(Into::into(err))
                } else {
                    Ok(())
                }
            })
            .spawn()
            .map_err(|e| anyhow!("Error starting server due to {:?}", e))?
    };

    if std::fs::write(pid_file, child.id().to_string()).is_err() {
        stop_server_daemon().await?;
        return Err(anyhow!(
            "Server start failed due to cannot write PID to file"
        ));
    }
    log::info!("Server started with PID {}", child.id());
    Ok("server started successfully".to_owned())
}

pub async fn stop_server_daemon() -> Result<&'static str> {
    if (ping().await).is_err() {
        let msg = "Server is not running";
        println!("{}", msg);
        return Ok(msg);
    }

    let pid_file = DefaultProvider.get_config_dir()?.join("server.pid");

    if !pid_file.exists() {
        let msg = "server pid doesnt exist";
        eprintln!("{}", msg);
        return Ok(msg);
    }

    let pid = std::fs::read_to_string(&pid_file).context("Failed to read the string")?;
    Command::new("kill")
        .arg(pid.trim())
        .status()
        .map_err(|e| anyhow!("Failed to kill the server due to {}", e))?;

    std::fs::remove_file(pid_file)
        .map_err(|e| anyhow!("Failed to remove the server pid file due to {}", e))?;
    let success_msg = "Server stopped";

    println!("{}", success_msg);
    Ok(success_msg)
}

pub async fn ping() -> Result<String> {
    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/ping", PY_PORT);
    let res = client.get(url).send().await;

    match res {
        Err(err) => Err(anyhow!("Server down due to {:?}", err)),
        _ => Ok("pong".to_owned()),
    }
}

/// Load the model and prefill the agent's system prompt before the first
/// message needs either. Waits for a just-spawned server to start listening.
/// Returns whether a prompt was prefilled, which needs one earlier request.
pub async fn warm_up(model: &str) -> Result<bool> {
    wait_until_up().await?;

    let answer: serde_json::Value = Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()?
        .post(format!("http://127.0.0.1:{}/v1/warmup", PY_PORT))
        .json(&serde_json::json!({ "model": model }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(answer["prefilled"].as_bool().unwrap_or(false))
}

/// Waits for a just-spawned inference server to start listening.
pub async fn wait_until_up() -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while ping().await.is_err() {
        if std::time::Instant::now() > deadline {
            return Err(anyhow!("the inference server did not come up"));
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    Ok(())
}

/// Starts the inference server if it is not running, and waits for it.
pub async fn ensure_up() -> Result<()> {
    if ping().await.is_ok() {
        return Ok(());
    }
    start_server_daemon().await?;
    wait_until_up().await
}

fn py_url(path: &str) -> String {
    format!("http://127.0.0.1:{}{}", PY_PORT, path)
}

/// Devices the inference server can run on, with their free memory.
pub async fn hardware() -> Result<serde_json::Value> {
    Ok(Client::new()
        .get(py_url("/v1/hardware"))
        .timeout(std::time::Duration::from_secs(90))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

/// Memory a model file needs, estimated by the inference server from its
/// header without downloading it.
pub async fn estimate(url: &str, size: u64) -> Result<crate::core::models::Need> {
    let answer: serde_json::Value = Client::new()
        .post(py_url("/v1/estimate"))
        .timeout(std::time::Duration::from_secs(120))
        .json(&serde_json::json!({ "url": url, "size": size }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let bytes = |key: &str| {
        answer[key]
            .as_u64()
            .ok_or_else(|| anyhow!("estimate is missing {key}"))
    };
    Ok(crate::core::models::Need {
        vram_bytes: bytes("vram_bytes")?,
        expert_bytes: bytes("expert_bytes")?,
    })
}
