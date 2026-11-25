//! Module that deals with Pi
use crate::core::agent::types::{
    CommandType, GetStateData, PiResponse, PiResponseMessage, ReasoningEffort, Skill,
};
use crate::utils::config::{
    ConfigProvider, DefaultProvider, create_pi_provider_config, handle_pi_settings_config,
};
use crate::utils::shell_path;
use anyhow::{Context, Result, anyhow, bail};
use nix::unistd::setsid;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{self, Stdio},
    time::Duration,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use uuid::Uuid;

// get_state and new_session answer at once, this only guards against a stuck Pi
const ANSWER_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct PiAgent {
    pub process: Child,
    pub writer: PiWriter,
    pub reader: PiReader,
    /// The system prompt Pi was started with, it can't be changed afterwards
    pub system_prompt: String,
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
    let pi_agent_dir = write_models_json(DefaultProvider, model_name, port)?;

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
            .env("PATH", shell_path::for_tools())
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
        system_prompt: system_prompt.to_owned(),
    })
}

/// Writes the model and its context window to Pi's models.json and returns Pi's folder.
/// Pi reads the file when it starts and again with each new session
pub fn write_models_json(
    provider: impl ConfigProvider,
    model_name: &str,
    port: u32,
) -> Result<PathBuf> {
    let pi_agent_dir = provider.get_user_data_dir()?.join("pi/agent/");
    fs::create_dir_all(&pi_agent_dir).context("Failed to create Pi agent directory")?;

    let endpoint_url = format!("http://127.0.0.1:{}/v1", port);
    let model_config = create_pi_provider_config(model_name, &endpoint_url)?;
    fs::write(pi_agent_dir.join("models.json"), model_config)?;

    Ok(pi_agent_dir)
}

/// Saves the thinking level as Pi's default, so it is still set after Pi restarts.
/// The other settings in the file are kept
pub fn save_default_thinking_level(
    provider: impl ConfigProvider,
    level: ReasoningEffort,
) -> Result<()> {
    let pi_agent_dir = provider.get_user_data_dir()?.join("pi/agent/");
    let settings_path = pi_agent_dir.join("settings.json");

    let mut settings: Value = match fs::read_to_string(&settings_path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|_| json!({})),
        Err(_) => json!({}),
    };
    settings
        .as_object_mut()
        .context("settings.json is not a JSON object")?
        .insert("defaultThinkingLevel".to_owned(), json!(level));

    fs::create_dir_all(&pi_agent_dir).context("Failed to create Pi agent directory")?;
    // written beside the file and renamed over it, a reader never sees half a file
    let tmp_path = pi_agent_dir.join(format!("settings.{}.tmp.json", process::id()));
    fs::write(&tmp_path, serde_json::to_string_pretty(&settings)?)
        .context("Failed to write settings.json")?;
    fs::rename(&tmp_path, &settings_path).context("Failed to write settings.json")
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
        let answer = self.ask(writer, CommandType::GetState).await?;
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
        let answer = self.ask(writer, CommandType::NewSession).await?;
        // an extension can cancel it
        if !answer.success || answer.data.is_some_and(|data| data["cancelled"] == true) {
            bail!("Creating new session failed");
        }
        self.get_pi_state(writer).await
    }

    /// Sets Pi's thinking level and returns the level Pi is on afterwards. Pi picks
    /// another one when the model doesn't support the level asked for
    pub async fn set_thinking_level(
        &mut self,
        writer: &mut PiWriter,
        level: ReasoningEffort,
    ) -> Result<String> {
        // Pi holds its answer to this back until the next request, so it isn't waited
        // for. the state asked for next says which level Pi is on
        writer
            .send_to_pi(json!({ "type": CommandType::SetThinkingLevel, "level": level }))
            .await?;
        Ok(self.get_pi_state(writer).await?.thinking_level)
    }

    /// The skills Pi has loaded. Pi lists them among its other commands as `skill:<name>`
    pub async fn fetch_skills(&mut self, writer: &mut PiWriter) -> Result<Vec<Skill>> {
        #[derive(Deserialize)]
        struct PiCommands {
            commands: Vec<PiCommand>,
        }
        #[derive(Deserialize)]
        struct PiCommand {
            name: String,
            // extension commands may come without one
            #[serde(default)]
            description: String,
        }

        let answer = self.ask(writer, CommandType::GetCommands).await?;
        let data = answer
            .data
            .filter(|_| answer.success)
            .ok_or_else(|| anyhow!("Failed to fetch the commands from Pi"))?;
        let listed: PiCommands = serde_json::from_value(data)?;
        let skills = listed
            .commands
            .into_iter()
            .filter_map(|command| {
                let name = command.name.strip_prefix("skill:")?.to_owned();
                Some(Skill {
                    name,
                    description: command.description.trim().to_owned(),
                })
            })
            .collect();
        Ok(skills)
    }

    // sends `command` and waits for its answer, skipping anything else Pi prints
    async fn ask(
        &mut self,
        writer: &mut PiWriter,
        command: CommandType,
    ) -> Result<PiResponseMessage> {
        let id = Uuid::now_v7().to_string();
        writer
            .send_to_pi(json!({"type": command, "id": id}))
            .await?;
        let read = async {
            while let Some(line) = self.lines.next_line().await? {
                if let Ok(PiResponse::Response(answer)) = serde_json::from_str(&line)
                    && answer.id.as_deref() == Some(id.as_str())
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
        system_prompt: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::from_test_command;

    // a fake Pi that answers the request it reads with `answers`, where $id is the request's id
    fn pi(answers: &str) -> super::PiAgent {
        let script = format!(
            r#"read request; id=${{request#*'"id":"'}}; id=${{id%%'"'*}}
printf '%s\n' {answers}"#
        );
        from_test_command("sh", &["-c", &script]).unwrap()
    }

    const STATE: &str = r#"'{"type":"response","id":"'"$id"'","command":"get_state","success":true,"data":{"sessionId":"s1","model":{"id":"id","name":"model"},"thinkingLevel":"off","isStreaming":false}}'"#;

    #[tokio::test]
    async fn the_state_is_found_past_what_a_stopped_reply_left() {
        let mut agent = pi(&format!(
            r#"'{{"type":"agent_settled"}}' '{{"type":"response","command":"abort","success":true}}' {STATE}"#
        ));
        let state = agent.reader.get_pi_state(&mut agent.writer).await.unwrap();
        assert_eq!(state.session_id, "s1");
    }

    #[tokio::test]
    async fn a_late_answer_to_an_earlier_call_is_skipped() {
        let mut agent = pi(&format!(
            r#"'{{"type":"response","id":"earlier","command":"get_state","success":true,"data":{{"sessionId":"old","model":{{"id":"id","name":"model"}},"thinkingLevel":"off","isStreaming":false}}}}' {STATE}"#
        ));
        let state = agent.reader.get_pi_state(&mut agent.writer).await.unwrap();
        assert_eq!(state.session_id, "s1");
    }

    #[tokio::test]
    async fn a_failed_answer_is_an_error_not_a_panic() {
        let mut agent =
            pi(r#"'{"type":"response","id":"'"$id"'","command":"get_state","success":false}'"#);
        assert!(agent.reader.get_pi_state(&mut agent.writer).await.is_err());
    }

    #[tokio::test]
    async fn a_cancelled_new_session_is_an_error() {
        let mut agent = pi(
            r#"'{"type":"response","id":"'"$id"'","command":"new_session","success":true,"data":{"cancelled":true}}'"#,
        );
        assert!(
            agent
                .reader
                .create_new_session(&mut agent.writer)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn pi_closing_before_it_answers_is_an_error() {
        let mut agent = from_test_command("sh", &["-c", "read request"]).unwrap();
        assert!(agent.reader.get_pi_state(&mut agent.writer).await.is_err());
    }
}
