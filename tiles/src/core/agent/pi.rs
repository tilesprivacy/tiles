//! Module that deals with Pi
use crate::core::agent::types::{Commands, GetStateData, PiResponse};
use crate::core::plugin::{
    installed_extension_entrypoints, installed_skill_dirs, prune_copied_plugin_skills,
};
use crate::utils::config::{
    ConfigProvider, DefaultProvider, create_pi_provider_config, handle_pi_mcp_config,
    handle_pi_settings_config,
};
use anyhow::{Context, Result, anyhow};
use chrono::Local;
use log::{info, warn};
use nix::unistd::setsid;
use serde_json::{Value, json};
use std::{collections::HashMap, fs, process::Stdio};
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

/// Startup events we will read past while waiting for a response.
const MAX_EVENTS_BEFORE_RESPONSE: usize = 64;

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

    let mcp_config_file_path = pi_agent_dir.join("mcp.json");
    handle_pi_mcp_config(&mcp_config_file_path)?;

    let pi_exec_path = tiles_lib_dir.join("pi/pi");

    prune_copied_plugin_skills();

    // Extensions are loaded explicitly, never discovered: the bundled MCP
    // adapter plus whatever installed plugins ship under `run.tiles/`.
    let mut extensions = vec![tiles_lib_dir.join("vendor/node_modules/pi-mcp-adapter")];
    extensions.extend(installed_extension_entrypoints());

    // Skills come from the plugins that own them, so disabling a plugin takes
    // its skills with it.
    let skill_dirs = installed_skill_dirs();

    let mut pi_process = unsafe {
        let mut command = Command::new(pi_exec_path);
        command
            .arg("--mode")
            .arg("rpc")
            .arg("--append-system-prompt")
            .arg(with_current_date(system_prompt))
            .arg("--no-session")
            .arg("--no-extensions");

        for extension in extensions {
            if extension.exists() {
                command.arg("-e").arg(extension);
            } else {
                warn!("Skipping missing Pi extension {:?}", extension);
            }
        }

        for skill_dir in skill_dirs {
            command.arg("--skill").arg(skill_dir);
        }

        command
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

/// Appends today's date to the system prompt.
///
/// The model does not know the date, so it fills the gap from its training
/// data. Seen in a real session: asked for today's news, it searched for
/// "latest news today October 24 2024" and got news from that day in 2024.
/// The search worked; the date in the query was wrong.
fn with_current_date(system_prompt: &str) -> String {
    let today = Local::now().format("%A, %-d %B %Y");
    format!(
        "{}\n\nCurrent date: {}\n\
         Use this whenever a question depends on the current date, including when \
         building a search query. Never take the date from your training data.",
        system_prompt.trim_end(),
        today
    )
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
    /// Sends a request and returns the matching `response`.
    ///
    /// Extensions emit events on the same stream, and the bundled MCP adapter
    /// always announces itself before Pi answers, so anything that is not the
    /// response we asked for is skipped rather than treated as a failure.
    async fn request(&mut self, writer: &mut PiWriter, payload: Value) -> Result<Value> {
        let request_type = payload
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned();

        self.request_optional(writer, payload)
            .await?
            .ok_or_else(|| anyhow!("Pi returned no data for {}", request_type))
    }

    /// Like `request`, for commands whose response carries no data. An ack is
    /// the whole answer to `new_session`, and demanding data of it would turn
    /// every success into an error.
    async fn request_optional(
        &mut self,
        writer: &mut PiWriter,
        payload: Value,
    ) -> Result<Option<Value>> {
        let request_type = payload
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned();

        writer
            .send_to_pi(payload)
            .await
            .inspect_err(|_e| eprintln!("sending command to pi failed"))?;

        // bounded so a stream of events can never hang startup
        for _ in 0..MAX_EVENTS_BEFORE_RESPONSE {
            let Some(line) = self.lines.next_line().await? else {
                return Err(anyhow!("Pi closed the connection during {}", request_type));
            };
            match serde_json::from_str::<PiResponse>(&line) {
                Ok(PiResponse::Response(msg)) => {
                    if !msg.success {
                        return Err(anyhow!("Pi answered {} with a failure", request_type));
                    }
                    return Ok(msg.data);
                }
                _ => {
                    info!(
                        "skipping event while waiting for {}: {}",
                        request_type, line
                    );
                    continue;
                }
            }
        }
        Err(anyhow!(
            "Gave up waiting for a response to {}",
            request_type
        ))
    }

    /// Gets current Pi State
    pub async fn get_pi_state(&mut self, writer: &mut PiWriter) -> Result<GetStateData> {
        let data = self.request(writer, json!({ "type": "get_state" })).await?;
        serde_json::from_value(data).context("Failed to parse Pi state")
    }

    /// Every command Pi knows about: skills, plus anything extensions added.
    pub async fn get_pi_commands(&mut self, writer: &mut PiWriter) -> Result<Vec<Commands>> {
        let data = self
            .request(writer, json!({ "type": "get_commands" }))
            .await?;
        let mut by_key: HashMap<String, Vec<Commands>> =
            serde_json::from_value(data).context("Failed to parse Pi commands")?;
        Ok(by_key.remove("commands").unwrap_or_default())
    }

    /// Reads the next line for Pi's stdout
    pub async fn next_line(&mut self) -> std::result::Result<Option<String>, std::io::Error> {
        self.lines.next_line().await
    }

    /// Creates a new Pi session
    /// Resets Pi's one conversation.
    ///
    /// Goes through the event-skipping request loop: extensions announce
    /// themselves on the same stream, and the naive single read this used to
    /// do would take an `extension_ui_request` for the answer, fail the call,
    /// and leave the real response behind to desync the next reader.
    pub async fn create_new_session(&mut self, writer: &mut PiWriter) -> Result<GetStateData> {
        self.request_optional(writer, json!({ "type": "new_session" }))
            .await
            .context("Creating new session failed")?;

        self.get_pi_state(writer).await
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
    use super::*;

    #[test]
    fn test_current_date_is_appended_without_losing_the_prompt() {
        let prompt = "You are Tiles.\n\nBehavior:\n- Be concise.";
        let result = with_current_date(prompt);

        // the modelfile's prompt has to survive intact
        assert!(result.starts_with("You are Tiles."));
        assert!(result.contains("- Be concise."));

        // and the date has to be a real one, not a placeholder
        let year = Local::now().format("%Y").to_string();
        assert!(result.contains("Current date: "), "{}", result);
        assert!(result.contains(&year), "{}", result);
        // the instruction matters as much as the date: without it the model
        // still reached for a training-data date when building a query
        assert!(result.contains("search query"));
    }

    #[test]
    fn test_current_date_handles_an_empty_prompt() {
        // modelfile.system is optional, so this is a real case
        let result = with_current_date("");
        assert!(
            result.trim_start().starts_with("Current date: "),
            "{}",
            result
        );
    }
}
