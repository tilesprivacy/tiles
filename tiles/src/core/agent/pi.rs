//! Module that deals with Pi
use crate::core::agent::types::{GetStateData, PiResponse};
use crate::utils::config::{
    ConfigProvider, DefaultProvider, create_pi_provider_config, handle_pi_settings_config,
};
use anyhow::{Context, Result, anyhow};
use nix::unistd::setsid;
use serde_json::{Value, json};
use std::{fs, process::Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

pub struct PiAgent {
    pub process: Child,
    pub writer: PiWriter,
    pub reader: PiReader,
}

pub struct PiWriter {
    stdin: ChildStdin,
}

pub struct PiReader {
    lines: Lines<BufReader<ChildStdout>>,
}

//TODO: check if we need a use case of kill_on_drop(true)
/// Creates a Pi Agent instance with writer and reader for comms with Pi
pub fn new(model_name: &str, system_prompt: &str, port: u32) -> Result<PiAgent> {
    let tiles_lib_dir = DefaultProvider.get_lib_dir()?;
    let user_data_dir = DefaultProvider.get_user_data_dir()?;
    let pi_agent_dir = user_data_dir.join("pi/agent/");
    std::fs::create_dir_all(&pi_agent_dir).context("Failed to create Pi agent directory")?;

    let provider_config_file_path = pi_agent_dir.join("models.json");
    let endpoint_url = format!("http://127.0.0.1:{}/v1", port);
    let model_config = create_pi_provider_config(model_name, &endpoint_url)?;

    fs::write(provider_config_file_path, model_config)?;

    let settings_file_path = pi_agent_dir.join("settings.json");
    handle_pi_settings_config(&settings_file_path)?;

    let pi_exec_path = tiles_lib_dir.join("pi/pi");

    let mut pi_process = unsafe {
        Command::new(pi_exec_path)
            .arg("--mode")
            .arg("rpc")
            .arg("--append-system-prompt")
            .arg(system_prompt)
            .arg("--no-session")
            .env("PI_CODING_AGENT_DIR", pi_agent_dir)
            .env("PI_OFFLINE", "true")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .pre_exec(|| {
                if let Err(err) = setsid() {
                    Err(Into::into(err))
                } else {
                    Ok(())
                }
            })
            .spawn()?
    };

    let pi_stdin = pi_process
        .stdin
        .take()
        .ok_or(anyhow!("Failed to get pi stdin"))?;

    let pi_stdout = pi_process
        .stdout
        .take()
        .ok_or(anyhow!("Failed to get pi stdout"))?;

    Ok(PiAgent {
        process: pi_process,
        reader: PiReader {
            lines: BufReader::new(pi_stdout).lines(),
        },
        writer: PiWriter { stdin: pi_stdin },
    })
}

/// Gracefully exit an ongoing Pi agent session.
/// NOTE: This doesnot kill Pi background process
pub async fn handle_graceful_exit(writer: &mut PiWriter) -> Result<()> {
    let end_payload = json!({
        "type": "abort",
    });
    writer.send_to_pi(end_payload).await
}

impl PiAgent {
    pub fn split(self) -> (Child, PiReader, PiWriter) {
        (self.process, self.reader, self.writer)
    }
}

impl PiWriter {
    /// Send requests to Pi in json
    pub async fn send_to_pi(&mut self, payload_json: Value) -> Result<()> {
        let payload_str = format!(
            "{}\n",
            serde_json::to_string(&payload_json).map_err(|e| {
                log::error!("{}", e);
                anyhow!("Error sending command to Pi due to {}", e)
            })?
        );
        self.stdin
            .write_all(payload_str.as_bytes())
            .await
            .context("Failed to send to Pi's stdin")
            .map_err(|e| {
                log::error!("{}", e);
                anyhow!("Error sending command to Pi due to {}", e)
            })?;
        self.stdin.flush().await.map_err(|e| {
            log::error!("{}", e);
            anyhow!("Error sending command to Pi due to {}", e)
        })
    }
}

impl PiReader {
    /// Gets current Pi State
    pub async fn get_pi_state(&mut self, writer: &mut PiWriter) -> Result<GetStateData> {
        let init_cmd_payload = json!({
            "type": "get_state",
        });

        writer.send_to_pi(init_cmd_payload).await?;

        if let Some(line) = self.lines.next_line().await? {
            let response: PiResponse = serde_json::from_str(&line)?;
            if let PiResponse::Response(msg) = response
                && msg.success
            {
                let state: GetStateData =
                    serde_json::from_value(msg.data.expect("get state parsing failed"))?;
                Ok(state)
            } else {
                Err(anyhow!("Failed to fetch initial state from Pi"))
            }
        } else {
            Err(anyhow!("Failed to fetch session_id from Pi"))
        }
    }

    /// Reads the next line for Pi's stdout
    pub async fn next_line(&mut self) -> std::result::Result<Option<String>, std::io::Error> {
        self.lines.next_line().await
    }

    /// Creates a new Pi session
    pub async fn create_new_session(&mut self, writer: &mut PiWriter) -> Result<GetStateData> {
        let cmd_payload = json!({
            "type": "new_session",
        });

        writer.send_to_pi(cmd_payload).await?;

        if let Some(line) = self.lines.next_line().await? {
            let response: PiResponse = serde_json::from_str(&line)?;
            if let PiResponse::Response(msg) = response
                && msg.success
            {
                let state = self.get_pi_state(writer).await?;
                Ok(state)
            } else {
                Err(anyhow!("Creating new session failed"))
            }
        } else {
            Err(anyhow!("Failed to fetch session_id from Pi"))
        }
    }
}

#[cfg(test)]
pub fn from_test_command(program: &str, args: &[&str]) -> anyhow::Result<PiAgent> {
    let mut process = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let stdin = process.stdin.take().unwrap();
    let stdout = process.stdout.take().unwrap();

    Ok(PiAgent {
        process,
        writer: PiWriter { stdin },
        reader: PiReader {
            lines: BufReader::new(stdout).lines(),
        },
    })
}
