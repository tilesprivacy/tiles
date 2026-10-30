//! Module that deals with Pi
use crate::core::agent::types::{CommandType, GetStateData, PiResponse, PiResponseMessage};
use crate::utils::config::{
    ConfigProvider, DefaultProvider, create_pi_provider_config, handle_pi_settings_config,
};
use anyhow::{Context, Result, anyhow, bail};
use nix::unistd::setsid;
use serde_json::{Value, json};
use std::{fs, process::Stdio, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

// get_state and new_session answer at once, this only guards against a stuck Pi
const ANSWER_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct PiAgent {
    pub process: Child,
    pub writer: PiWriter,
    pub reader: PiReader,
}

#[derive(Debug)]
pub struct PiWriter {
    stdin: ChildStdin,
}

#[derive(Debug)]
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
        writer.send_to_pi(json!({"type": "get_state"})).await?;
        let answer = self.answer(CommandType::GetState).await?;
        let data = answer
            .data
            .filter(|_| answer.success)
            .ok_or_else(|| anyhow!("Failed to fetch initial state from Pi"))?;
        Ok(serde_json::from_value(data)?)
    }

    /// Reads the next line for Pi's stdout
    pub async fn next_line(&mut self) -> std::result::Result<Option<String>, std::io::Error> {
        self.lines.next_line().await
    }

    /// Creates a new Pi session
    pub async fn create_new_session(&mut self, writer: &mut PiWriter) -> Result<GetStateData> {
        writer.send_to_pi(json!({"type": "new_session"})).await?;
        if !self.answer(CommandType::NewSession).await?.success {
            bail!("Creating new session failed");
        }
        self.get_pi_state(writer).await
    }

    // reads until Pi answers `command`, skipping leftovers like a stopped reply's events
    async fn answer(&mut self, command: CommandType) -> Result<PiResponseMessage> {
        let read = async {
            while let Some(line) = self.lines.next_line().await? {
                if let Ok(PiResponse::Response(answer)) = serde_json::from_str(&line)
                    && answer.command == command
                {
                    return Ok(answer);
                }
            }
            bail!("Pi stopped before answering {command:?}")
        };
        tokio::time::timeout(ANSWER_TIMEOUT, read)
            .await
            .map_err(|_| anyhow!("Pi didn't answer {command:?} in time"))?
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

#[cfg(test)]
mod tests {
    use super::from_test_command;

    const STATE: &str = r#"{"type":"response","command":"get_state","success":true,"data":{"sessionId":"s1","model":{"id":"id","name":"model"},"thinkingLevel":"off","isStreaming":false}}"#;

    #[tokio::test]
    async fn the_state_is_found_past_what_a_stopped_reply_left() {
        let script = format!(
            r#"read request
printf '%s\n' '{{"type":"agent_settled"}}' '{{"type":"response","command":"abort","success":true}}' '{STATE}'"#
        );
        let mut agent = from_test_command("sh", &["-c", &script]).unwrap();
        let state = agent.reader.get_pi_state(&mut agent.writer).await.unwrap();
        assert_eq!(state.session_id, "s1");
    }

    #[tokio::test]
    async fn a_failed_answer_is_an_error_not_a_panic() {
        let script = r#"read request
printf '%s\n' '{"type":"response","command":"get_state","success":false}'"#;
        let mut agent = from_test_command("sh", &["-c", script]).unwrap();
        assert!(agent.reader.get_pi_state(&mut agent.writer).await.is_err());
    }

    #[tokio::test]
    async fn pi_closing_before_it_answers_is_an_error() {
        let mut agent = from_test_command("sh", &["-c", "read request"]).unwrap();
        assert!(agent.reader.get_pi_state(&mut agent.writer).await.is_err());
    }
}
