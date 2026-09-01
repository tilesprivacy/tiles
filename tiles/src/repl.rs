use crate::core::account::atproto::{fetch_logged_in_data, login, share_session};
use crate::core::account::local::get_current_user;
use crate::core::agent::pi::{PiAgent, PiWriter};
use crate::core::agent::types::{
    CommandType, Commands, PiAgentEndEvent, PiMsgContent, PiResponse, PiResponseMessage,
    ReasoningEffort,
};
use crate::core::agent::{pi, types};
use crate::core::chats::{
    Session, create_session, fetch_chats_by_session_id, fetch_session, fetch_sessions, save_chat,
    update_snapshot,
};
use crate::core::network;
use crate::core::server::{ping, start_server_daemon, stop_server_daemon};
use crate::core::storage::db::Dbconn;
use crate::utils::config::{
    ConfigProvider, DefaultProvider, LlamaConfig, PY_PORT, REMOTE_BOUND_PORT, get_inference_config,
    get_memory_path, get_model_cache, update_current_model, update_llama_config,
};
use crate::utils::hf_model_downloader::*;
use crate::utils::lexicons::{SessionSnapshotRecord, Turn};
use anyhow::{Context, Result, anyhow};
use log::{info, warn};
use owo_colors::OwoColorize;
use reqwest::{Client, StatusCode};
use rustyline::completion::Completer;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{Config, Editor, Helper};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs::{self};
use std::io::{self};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tilekit::modelfile::Modelfile;
use tilekit::modelfile::Role;
use tokio::time::sleep;

const MAX_LOAD_MODEL_RETRIES: u8 = 3;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BenchmarkMetrics {
    ttft_ms: f64,
    total_tokens: i32,
    tokens_per_second: f64,
    total_latency_s: f64,
}

#[allow(dead_code)]
impl BenchmarkMetrics {
    fn update(&mut self, metrics: BenchmarkMetrics) -> &Self {
        if self.ttft_ms == 0.0 {
            self.ttft_ms += metrics.ttft_ms;
        }
        self.total_tokens += metrics.total_tokens;
        self.tokens_per_second += metrics.tokens_per_second;
        self.total_latency_s += metrics.total_latency_s;
        self
    }
}

pub struct RunArgs {
    pub modelfile_path: Option<String>,
    pub relay_count: u32,
    pub llama_config: Option<LlamaConfig>,
    pub remote: Option<String>,
}
#[derive(Clone, Debug)]
pub struct ChatResponse {
    // text content
    pub input: String,
    pub session_id: String,
    pub role: Role,
    pub parent_chat_id: Option<String>,
    pub metrics: Option<BenchmarkMetrics>,
    pub model_used: String,
}

enum InputCommandResponse {
    WaitForNextLine,
    ProcessNextInput,
}
impl FromStr for ReasoningEffort {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "high" => Ok(ReasoningEffort::High),
            "medium" => Ok(ReasoningEffort::Medium),
            "low" => Ok(ReasoningEffort::Low),
            _ => Err(anyhow!("Invalid Reasoning value, use /help".to_owned())),
        }
    }
}

impl From<ReasoningEffort> for String {
    fn from(value: ReasoningEffort) -> Self {
        match value {
            ReasoningEffort::High => "high".to_owned(),
            ReasoningEffort::Medium => "medium".to_owned(),
            ReasoningEffort::Low => "low".to_owned(),
        }
    }
}

pub async fn run(run_args: RunArgs, db_conn: &Dbconn) -> Result<()> {
    let (modelfile, default_modelfile) = if let Some(modelfile_str) = &run_args.modelfile_path {
        let modelfile = match tilekit::modelfile::parse_from_file(modelfile_str.as_str()) {
            Ok(mf) => mf,
            Err(err) => {
                eprintln!("Invalid Modelfile due to {:?}", err);
                return Ok(());
            }
        };
        let default_modelfile = get_default_modelfile(DefaultProvider)
            .ok()
            .and_then(|path| tilekit::modelfile::parse_from_file(path.to_str()?).ok())
            .unwrap_or_else(|| modelfile.clone());
        (modelfile, default_modelfile)
    } else {
        let default_modelfile_path = get_default_modelfile(DefaultProvider)?;
        let default_modelfile = match tilekit::modelfile::parse_from_file(
            default_modelfile_path
                .to_str()
                .expect("default_modelfile_path: Failed PathBuf to str"),
        ) {
            Ok(mf) => mf,
            Err(err) => {
                eprintln!("Invalid default Modelfile due to {:?}", err);
                return Ok(());
            }
        };
        (default_modelfile.clone(), default_modelfile)
    };

    run_model_with_server(modelfile, default_modelfile, &run_args, db_conn).await
}

struct TilesHinter;

impl Hinter for TilesHinter {
    type Hint = String;

    fn hint(&self, line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<Self::Hint> {
        if line.is_empty() {
            Some("Send a message (/help to show available commands)".to_string())
        } else {
            None
        }
    }
}

impl Completer for TilesHinter {
    type Candidate = String;
}

impl Highlighter for TilesHinter {
    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        std::borrow::Cow::Owned(format!("\x1b[2m{}\x1b[0m", hint))
    }
}

impl Validator for TilesHinter {}

impl Helper for TilesHinter {}

enum InputType {
    Skip,
    Command(String),
    Exit,
    Prompt,
    Skill,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SharedSession {
    #[serde(rename = "$type")]
    pub r#type: String,
    pub session_id: String,
    pub name: String,
    pub contents: Vec<SharedContent>,
    pub created_at: String,
    pub models_used: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SharedContent {
    role: Role,
    content: String,
}

struct ReplSession {
    pub session_id: String,
    // if true, we will prepend the resumed session history to the input
    resume_session_pending: bool,
    resumed_session: String,
    pub current_modelname: String,
    pub last_chat_id: Option<String>,
    pub session_started: bool,
    pub reasoning: ReasoningEffort,
    pub session_snapshot: SessionSnapshotRecord,
}

impl ReplSession {
    pub fn new(state: &types::GetStateData) -> Self {
        ReplSession {
            session_id: state.session_id.clone(),
            resume_session_pending: false,
            resumed_session: String::from(""),
            current_modelname: state.model.name.to_owned(),
            last_chat_id: None,
            session_started: false,
            reasoning: state.thinking_level.parse().unwrap_or(ReasoningEffort::Low),
            session_snapshot: SessionSnapshotRecord::new("", &state.session_id),
        }
    }

    pub fn set_pending_resume_session(&mut self, flag: bool) {
        self.resume_session_pending = flag
    }

    pub fn get_pending_resume_session(&self) -> bool {
        self.resume_session_pending
    }

    pub fn get_resumed_session(&self) -> String {
        self.resumed_session.clone()
    }

    pub fn set_resumed_session(&mut self, session: String) {
        self.resumed_session = session
    }
}
fn handle_input(input: &str) -> InputType {
    if let Some(cmd) = input.strip_prefix('/') {
        let keyword = cmd.split_whitespace().next().unwrap_or("");
        match keyword.to_lowercase().as_str() {
            "help" | "?" => {
                show_help();
                InputType::Skip
            }
            "bye" => InputType::Exit,
            "" => {
                println!("Empty command. Type /help for available commands.");
                InputType::Skip
            }
            _ => InputType::Command(cmd.to_owned()),
        }
    } else if let Some(_skill) = input.strip_prefix('$') {
        InputType::Skill
    } else {
        InputType::Prompt
    }
}

fn show_help() {
    let help_groups = vec![
        (
            "Session",
            vec![
                (
                    "/reasoning <effort>",
                    "Sets the reasoning effort of current model (none, low, medium, high, xhigh)",
                ),
                ("/status", "Show the current session state"),
                ("/sessions", "List all available sessions"),
                (
                    "/resume <sessionId>",
                    "Load and resume a specific session (requires <sessionId>)",
                ),
            ],
        ),
        (
            "Sharing",
            vec![
                (
                    "/share",
                    "Create a shareable link for the current session (via ATProto)",
                ),
                (
                    "/share <sessionId>",
                    "Create a shareable link for a specific session (via ATProto)",
                ),
            ],
        ),
        (
            "Chat",
            vec![
                ("/help", "Show this help message"),
                ("/bye", "Exit the Chat"),
            ],
        ),
        (
            "Plugins",
            vec![
                ("/skills", "List all the available skills"),
                ("$<skill-name>", "Use the skill directly"),
            ],
        ),
    ];

    println!("Available Commands:");

    for (heading, commands) in help_groups {
        println!();
        println!("  {heading}");

        for (command, description) in commands {
            println!("    {command:<24}{description}");
        }
    }

    println!();
    println!("Documentation: https://tiles.run/book");
    println!("Report issues: https://github.com/tilesprivacy/tiles/issues");
    println!();
}

async fn run_model_with_server(
    modelfile: Modelfile,
    default_modelfile: Modelfile,
    run_args: &RunArgs,
    db_conn: &Dbconn,
) -> Result<()> {
    //TODO: deprecated remove..
    let memory_path = get_memory_path().context("Setting/Retrieving memory_path failed")?;
    if let Some(llama_config) = &run_args.llama_config {
        update_llama_config(llama_config).context("Failed to update llama config")?;
    }
    //TODO: Remove this remote infy code, refactor it
    // If remote inference mode then no need to load or leverage local server
    if let Some(ticket) = run_args.remote.clone() {
        tokio::spawn(async move {
            let _ = network::connect(&ticket)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
            Result::<()>::Ok(())
        });
        start_repl(&modelfile, run_args, db_conn)
            .await
            .map_err(|e| anyhow!(e))
    } else {
        if !cfg!(debug_assertions) {
            let _ = start_server_daemon().await.inspect_err(|e| {
                eprintln!("Failed to start inference server due to {:?}", e);
            });
            let _ = wait_until_server_is_up().await;
        }
        match load_model(&modelfile, &default_modelfile, &memory_path, 0).await {
            Ok(_) => start_repl(&modelfile, run_args, db_conn)
                .await
                .map_err(|e| anyhow!(e)),
            Err(err) => Err(anyhow!(err)),
        }
    }
}

#[allow(unused_assignments)]
async fn start_repl(modelfile: &Modelfile, run_args: &RunArgs, db_conn: &Dbconn) -> Result<()> {
    let modelname = model_spec(modelfile)?;

    update_current_model(&modelname).context("Failed to update current model in config.toml")?;
    let system_prompt = modelfile.system.clone().unwrap_or("".to_owned());
    println!("Running {}", modelname);
    let current_user = get_current_user(&db_conn.common)?;

    let config = Config::builder().auto_add_history(true).build();
    let mut editor = Editor::<TilesHinter, DefaultHistory>::with_config(config)
        .context("Failed to create editor")?;
    editor.set_helper(Some(TilesHinter));

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let port = if run_args.remote.is_none() {
        PY_PORT
    } else {
        REMOTE_BOUND_PORT
    };
    // Setting up Pi rpc process handles
    let mut pi_agent = pi::new(&modelname, &system_prompt, port)?;

    let pi_session_state = pi_agent.reader.get_pi_state(&mut pi_agent.writer).await?;
    let mut repl_session = ReplSession::new(&pi_session_state);

    // The great REPL loop
    loop {
        // Reads the user input
        let readline = editor.readline(">>> ");
        let input = match readline {
            Ok(line) => line.trim().to_string(),
            Err(_) => {
                handle_repl_exit(&mut pi_agent.writer).await?;
                break;
            }
        };

        if input.is_empty() {
            continue;
        }

        // Process the user input in the repl
        match handle_input(&input.to_lowercase()) {
            InputType::Skip => continue,
            InputType::Exit => {
                handle_repl_exit(&mut pi_agent.writer).await?;
                break;
            }
            InputType::Prompt => {
                handle_input_prompt(&mut pi_agent.writer, &mut repl_session, &input).await?;
            }
            InputType::Skill => {
                let (_, skill_name) = input.split_at(1);
                let skill_prompt = format!("/skill:{}", skill_name);
                handle_input_prompt(&mut pi_agent.writer, &mut repl_session, &skill_prompt).await?;
            }
            InputType::Command(cmd) => {
                let res =
                    handle_input_commands(cmd, &mut repl_session, db_conn, &mut pi_agent.writer)
                        .await?;

                if let InputCommandResponse::ProcessNextInput = res {
                    continue;
                }
            }
        }

        // This is to prevent the session name being messedup if user starts
        // with skills. Pi unfurls skills command to entire skill doc.So we
        // can't put that as session name.
        if !repl_session.session_started {
            repl_session.session_snapshot.name = input.clone();
        };

        // Reads the output from Pi and process and responds to the repl

        while let Some(line) = pi_agent.reader.next_line().await? {
            if !running.load(std::sync::atomic::Ordering::SeqCst) {
                info!("Ctrlc detected, aborting Pi ops");
                let end_payload = json!({
                    "type": "abort",
                });
                pi_agent.writer.send_to_pi(end_payload).await?;
                running.store(true, std::sync::atomic::Ordering::SeqCst);
                continue;
            }
            let response: PiResponse =
                serde_json::from_str(&line).context("Failed to parse Pi response")?;
            match response {
                PiResponse::AgentStart => {
                    info!("agent start")
                }
                PiResponse::MessageUpdate(msg_update) => {
                    handle_pi_message_update(msg_update);
                }
                PiResponse::AgentEnd(agent_end_event) => {
                    info!("agent end - {}", &line);
                    process_pi_agent_end_event(
                        &mut repl_session,
                        agent_end_event,
                        &mut pi_agent.writer,
                        db_conn,
                        &current_user,
                    )
                    .await?;
                    break;
                }
                PiResponse::AgentSettled => {
                    info!("agent settled")
                }
                PiResponse::TurnEnd(_turn_event) => {
                    println!("\n");
                }
                PiResponse::Response(response_msg) => {
                    if response_msg.success {
                        match response_msg.command {
                            CommandType::Unknown => {
                                continue;
                            }
                            CommandType::Abort => {
                                info!("Abort command received");
                                // continuing as we need to process the
                                // agent_end event from Pi
                                continue;
                            }
                            _ => {
                                process_command(response_msg, &mut repl_session, &mut pi_agent)
                                    .await?;
                                break;
                            }
                        }
                    } else {
                        println!("Command failed, try again")
                    }
                    break;
                }
                PiResponse::Unknown => {
                    info!("Unsupported response {}", &line);
                    continue;
                }
                _ => (),
            }
        }
    }
    Ok(())
}

/// Full `repo:quant` spec used as the model identity across config, Pi and the py server
pub fn model_spec(modelfile: &Modelfile) -> Result<String> {
    let from = modelfile
        .from
        .clone()
        .ok_or_else(|| anyhow!("Modelfile missing FROM instruction"))?;
    Ok(match &modelfile.quant {
        Some(quant) => format!("{}:{}", from, quant),
        None => from,
    })
}

fn resolve_gguf_path(model_cache_path: &PathBuf, quant: Option<&str>) -> Result<PathBuf> {
    let Some(quant) = quant else {
        return Ok(model_cache_path.clone());
    };
    let suffix = format!("{}.gguf", quant.to_lowercase());
    let matches: Vec<PathBuf> = fs::read_dir(model_cache_path)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().is_some_and(|ext| ext == "gguf")
                && p.file_name().is_some_and(|n| {
                    let name = n.to_string_lossy().to_lowercase();
                    name.ends_with(&suffix) && !name.contains("mmproj") && !name.contains("mtp")
                })
        })
        .collect();
    match matches.len() {
        0 => Err(anyhow!(
            "No gguf matching quant '{}' found in {}",
            quant,
            model_cache_path.display()
        )),
        1 => Ok(matches[0].clone()),
        _ => Err(anyhow!(
            "Multiple gguf files matching quant '{}' in {}: {:?}",
            quant,
            model_cache_path.display(),
            matches
        )),
    }
}

async fn load_model(
    modelfile: &Modelfile,
    default_modelfile: &Modelfile,
    memory_path: &str,
    retries: u8,
) -> Result<()> {
    if retries > MAX_LOAD_MODEL_RETRIES {
        return Err(anyhow!(
            "Model loading retried failed after {} times",
            retries
        ));
    }
    // TODO: why `from` is even an Option, gotta check latta
    let model_name = modelfile
        .from
        .clone()
        .expect("Modelfile not found, this is impossible to occur");

    let quant = modelfile.quant.as_deref();
    let model_cache_res = get_model_cache(&model_name);

    if model_cache_res.is_err() {
        download_model(&model_name, quant).await?;
        return Box::pin(load_model(modelfile, default_modelfile, memory_path, 0)).await;
    }

    // If loading fails it most probably a partial downloaded
    // model present, so we try to resume the download
    let model_cache_path = model_cache_res.unwrap();
    let load_res = match resolve_gguf_path(&model_cache_path, quant) {
        Ok(gguf_path) => {
            load_model_in_py(modelfile, default_modelfile, memory_path, &gguf_path).await
        }
        Err(err) => {
            log::warn!("Failed to resolve GGUF for the model: {}", err);
            Err(anyhow!(err))
        }
    };
    if load_res.is_err() {
        log::warn!("Load model failed, resuming the partial download");
        download_model(&model_name, quant).await?;
        Box::pin(load_model(
            modelfile,
            default_modelfile,
            memory_path,
            retries + 1,
        ))
        .await
    } else {
        Ok(())
    }
}

async fn wait_until_server_is_up() {
    loop {
        match ping().await {
            Ok(_) => {
                break;
            }
            Err(_err) => {
                println!("### tiling ###...");
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

pub fn get_default_modelfile(provider: impl ConfigProvider) -> Result<PathBuf> {
    let path = provider.get_lib_dir()?.join("modelfiles/gemma-4-12b-gguf");
    Ok(path)
}

async fn load_model_in_py(
    modelfile: &Modelfile,
    default_modelfile: &Modelfile,
    memory_path: &str,
    model_cache_path: &PathBuf,
) -> Result<()> {
    let client = Client::new();
    let model_name = model_spec(modelfile)?;
    let body = json!({
        "model": model_name,
        "memory_path": memory_path,
        "model_cache_path": model_cache_path,
        "system_prompt": modelfile.system.clone().unwrap_or(default_modelfile.system.clone().unwrap_or("".to_owned()))
    });
    let res = client
        .post("http://127.0.0.1:6969/start")
        .json(&body)
        .send()
        .await?;
    match res.status() {
        StatusCode::OK => {
            let payload: serde_json::Value = res.json().await?;
            for warning in model_load_warnings(&payload) {
                println!("{} {}", "WARNING:".yellow(), warning);
            }
            Ok(())
        }
        _ => Err(anyhow::anyhow!(format!(
            "Failed to load model {} due to {:?}",
            model_name, res
        ))),
    }
}

/// Extract the human-readable warnings reported by the inference server
/// while loading the model (e.g. MTP requested but no MTP head found).
fn model_load_warnings(payload: &serde_json::Value) -> Vec<String> {
    payload
        .get("warnings")
        .and_then(|w| w.as_array())
        .map(|warnings| {
            warnings
                .iter()
                .filter_map(|w| w.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

async fn download_model(model_name: &str, quant: Option<&str>) -> Result<()> {
    match pull_model(model_name, quant).await {
        Ok(_) => {
            println!("\nDownloading completed \n");
            Ok(())
        }
        Err(err) => Err(anyhow::anyhow!(format!("Download failed due to {:?}", err))),
    }
}

async fn process_command(
    response_msg: PiResponseMessage,
    repl_session: &mut ReplSession,
    agent: &mut PiAgent,
) -> Result<()> {
    match response_msg.command {
        CommandType::SetThinkingLevel => {
            let state = agent.reader.get_pi_state(&mut agent.writer).await?;
            match state.thinking_level.parse::<ReasoningEffort>() {
                Ok(effort) => {
                    repl_session.reasoning = effort;
                    if let Err(err) = persist_default_thinking_level(&state.thinking_level) {
                        warn!("Failed to persist thinking level across restarts: {}", err);
                    }
                    println!("Reasoning settings updated successfully")
                }
                Err(_) => {
                    println!(
                        "Pi reported reasoning level '{}', which Tiles does not recognize; leaving it unchanged",
                        state.thinking_level
                    )
                }
            }
        }
        CommandType::GetCommands => {
            if let Some(commands) = response_msg.data {
                let commands_obj: HashMap<String, Vec<Commands>> =
                    serde_json::from_value(commands)?;

                if let Some(commands) = commands_obj.get("commands") {
                    let mut index = 0;
                    commands.iter().for_each(|cmd| {
                        index += 1;
                        // chucking off `skill:` from the name
                        let (_, skill_name) = cmd.name.split_at(6);
                        println!(
                            "{}. {}{} - {}",
                            index.purple(),
                            "$".yellow(),
                            skill_name.bright_green(),
                            cmd.description.bright_cyan()
                        );
                    });
                } else {
                    println!("No commands found")
                }
            } else {
                println!("No commands found")
            }
        }
        _ => (),
    }

    Ok(())
}

async fn process_share_session(
    conn: &Dbconn,
    current_session_id: &str,
    args: &[&str],
) -> Result<()> {
    let args = if let Some((_main_command, sub_commands)) = args.split_first() {
        sub_commands
    } else {
        println!("Not a valid command");
        return Ok(());
    };

    let session_id = if args.is_empty() {
        current_session_id
    } else {
        args[0]
    };
    // fetch session and the chats for the session_id

    let delta_chats = fetch_chats_by_session_id(&conn.chat, session_id)?;

    if delta_chats.sessions.is_empty() {
        println!("Session {} not available", session_id);
    }

    if delta_chats.sessions.is_empty() {
        println!("Session doesn't exist or not started yet");
        return Ok(());
    }
    let session = &delta_chats.sessions[0];
    let shared_session: SessionSnapshotRecord = if let Some(snapshot_record) = &session.snapshot {
        serde_json::from_str(snapshot_record)?
    } else {
        eprintln!("Older sessions are not supported");
        return Ok(());
    };

    let share_choice_prompt = format!(
        "{}",
        "Do you want to share as a private session? (Y/n)".yellow()
    );

    println!("{}", share_choice_prompt);

    let stdin = io::stdin();
    let mut input = String::new();
    stdin.read_line(&mut input)?;
    let clean_input = input.trim();
    let is_private = clean_input.is_empty() || clean_input.to_lowercase() == "y";

    match share_session(&conn.common, &shared_session, is_private).await {
        Err(err) if &err.to_string() == "NOT_LOGGED_IN" => {
            let login_prompt = format!("{}", "Sharing a chat session requires logging in, as the data is stored on your ATmosphere PDS.\nDo you want to proceed with the login flow? (Y/n)".yellow());

            println!("{}", login_prompt);

            let stdin = io::stdin();
            let mut input = String::new();
            stdin.read_line(&mut input)?;
            let clean_input = input.trim();
            if clean_input.to_lowercase() == "y" {
                input.clear();
                println!("Please enter your ATmosphere handle (ex: john.bsky.team)");
                stdin.read_line(&mut input)?;
                login(conn, input.trim()).await?;
                share_session(&conn.common, &shared_session, is_private).await?;
            }
        }
        Err(err) => {
            eprintln!("Failed to share session due to {:?}", err)
        }
        Ok(_) => {
            info!("Session shared successfully")
        }
    }
    Ok(())
}

fn show_session_info(db_conn: &Dbconn) -> Result<()> {
    let sessions = fetch_sessions(&db_conn.chat, None)?;

    let mut count = 0;
    for session in sessions {
        count += 1;
        println!("{}.\t{}\t{}", count, session.id, session.name);
    }
    Ok(())
}

fn load_session(db_conn: &Dbconn, args: &[&str], repl_session: &mut ReplSession) -> Result<()> {
    let args = if let Some((_main_command, sub_commands)) = args.split_first() {
        sub_commands
    } else {
        return Err(anyhow!("Not a valid command"));
    };

    let session_id = if args.is_empty() {
        println!("Please provide sessionId");
        return Err(anyhow!("Please provide sessionId"));
    } else {
        args[0]
    };

    // fetch session and the chats for the session_id

    let delta_chats = fetch_chats_by_session_id(&db_conn.chat, session_id)?;

    if delta_chats.sessions.is_empty() {
        println!("Session {} not available", session_id);
    }

    //TODO: we will later implement a decent compaction based on Pi's for
    // history
    // https://github.com/badlogic/pi-mono/blob/182d4ceea33beabe7c4712b04f1f5459e613de44/packages/coding-agent/docs/compaction.md

    let mut chat_history: String = "".to_owned();
    for chat in &delta_chats.chats {
        chat_history.push_str(&chat.content);
        println!("{}", chat.content);
    }
    let last_chat_id = delta_chats.chats.last().map(|chat| chat.id.clone());
    let saved_session = delta_chats.sessions[0].clone();
    repl_session.session_id = session_id.to_string();
    repl_session.set_pending_resume_session(true);
    repl_session.set_resumed_session(chat_history);
    repl_session.last_chat_id = last_chat_id;
    repl_session.session_started = true;
    repl_session.session_snapshot = if let Some(snapshot_record) = saved_session.snapshot {
        serde_json::from_str(&snapshot_record)?
    } else {
        SessionSnapshotRecord::new(&saved_session.name, &saved_session.id)
    };
    Ok(())
}

fn show_status(repl_session: &ReplSession, db_conn: &Dbconn) -> Result<()> {
    let cwd_pathbuf = std::env::current_dir()?;
    let cwd = cwd_pathbuf.to_string_lossy();
    let logged_in_atproto = fetch_logged_in_data(&db_conn.common)?;
    let session_data: Option<Session> = fetch_session(&db_conn.chat, &repl_session.session_id).ok();
    let status_lines = build_status_lines(&cwd, session_data, repl_session, logged_in_atproto);

    println!("\n");
    for line in status_lines {
        println!("{}", line);
    }
    Ok(())
}

fn build_status_lines(
    cwd: &str,
    session_data: Option<Session>,
    repl_session: &ReplSession,
    logged_in_atproto: Option<crate::core::account::atproto::AtprotoAuthData>,
) -> Vec<String> {
    let mut status_map: Vec<(&str, String)> = vec![];
    let session_status = if let Some(session) = session_data {
        format!("{} ({})", session.name.yellow(), session.id.dimmed())
    } else {
        "Session not started yet".to_owned()
    };
    status_map.push(("Session", session_status));
    status_map.push(("Model", repl_session.current_modelname.clone()));
    status_map.push(("Reasoning", String::from(repl_session.reasoning)));
    status_map.push(("Working Directory", cwd.to_owned()));
    let at_proto_status = if let Some(at_auth_user) = logged_in_atproto {
        format!(
            "{}{} ({})",
            "@".blue(),
            at_auth_user.handle.blue(),
            at_auth_user.key.dimmed()
        )
    } else {
        "Not logged-in".to_owned()
    };
    status_map.push(("ATProto", at_proto_status));

    // for padding
    let max_length = status_map
        .iter()
        .fold(0, |acc, x| if x.0.len() > acc { x.0.len() } else { acc });

    status_map
        .into_iter()
        .map(|status| {
            format!(
                "{}:{}\t{}",
                status.0,
                " ".repeat(max_length - status.0.len()),
                status.1
            )
        })
        .collect()
}

fn handle_pi_message_update(msg_update: types::PiMessageUpdate) {
    match msg_update.assistant_message_event.r#type {
        types::AsstMsgEventType::TextStart => {
            println!();
            println!("{}\n", "**[Answer]**".bold());
            info!("msg text_start")
        }
        types::AsstMsgEventType::TextDelta => {
            if let Some(delta) = msg_update.assistant_message_event.delta {
                print!("{}", delta);
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }
        types::AsstMsgEventType::TextEnd => {
            info!("msg text_end")
        }
        types::AsstMsgEventType::ThinkingStart => {
            println!();
            println!("{}\n", "**[Reasoning]**".dimmed());
        }
        types::AsstMsgEventType::ThinkingDelta => {
            if let Some(delta) = msg_update.assistant_message_event.delta {
                print!("{}", delta.dimmed());
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }
        types::AsstMsgEventType::ThinkingEnd => {
            println!();
        }
        types::AsstMsgEventType::ToolcallStart => {
            info!("Selecting tool to execute");
            println!();
            let delta = "**[Tool Calling]**";
            println!("{}", delta.dimmed());
        }
        types::AsstMsgEventType::ToolcallDelta => {
            if let Some(delta) = msg_update.assistant_message_event.delta {
                print!("{}", delta.dimmed());
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }
        types::AsstMsgEventType::ToolcallEnd => {
            info!("Tool call selected");
        }
        types::AsstMsgEventType::Done => {
            info!("msg done event")
        }
        types::AsstMsgEventType::Error => {
            warn!("msg error event")
        }
        _ => (),
    }
}

async fn handle_repl_exit(agent_writer: &mut PiWriter) -> Result<()> {
    pi::handle_graceful_exit(agent_writer).await?;
    println!("Exiting interactive mode");
    if !cfg!(debug_assertions) {
        match get_inference_config()? {
            Some(inference_config) if !inference_config.daemon => {
                let _res = stop_server_daemon().await;
            }
            None => {
                let _res = stop_server_daemon().await;
            }
            _ => (),
        }
    }
    Ok(())
}

async fn handle_input_prompt(
    agent_writer: &mut PiWriter,
    repl_session: &mut ReplSession,
    input: &str,
) -> Result<()> {
    let final_input = if repl_session.get_pending_resume_session() {
        repl_session.set_pending_resume_session(false);
        info!("Pending resumed session, prepend the history");
        format!(
            "user_chat_history:\n{}.\nUse the history as context.\n[Followup question] - {}",
            repl_session.get_resumed_session(),
            input
        )
    } else {
        input.to_owned()
    };
    let payload = json!({
        "type": "prompt",
        "message": final_input
    });
    agent_writer.send_to_pi(payload).await
}

//TODO: command type should be in repl.rs not in agent::types
async fn handle_input_commands(
    cmd: String,
    repl_session: &mut ReplSession,
    db_conn: &Dbconn,
    agent_writer: &mut PiWriter,
) -> Result<InputCommandResponse> {
    let args: Vec<&str> = cmd.split(" ").collect();
    let main_cmd = args.first().expect("Main command should be there");

    let cmd_json = json!(main_cmd.to_lowercase());

    let command: CommandType = serde_json::from_value(cmd_json)?;
    let res = match command {
        CommandType::Unknown => {
            println!(
                "Unknown command: /{}. Type /help for available commands.",
                cmd
            );
            InputCommandResponse::ProcessNextInput
        }
        CommandType::Share => {
            process_share_session(db_conn, &repl_session.session_id, &args).await?;
            InputCommandResponse::ProcessNextInput
        }
        CommandType::Sessions => {
            show_session_info(db_conn)?;
            InputCommandResponse::ProcessNextInput
        }
        CommandType::Resume => {
            if let Err(err) = load_session(db_conn, &args, repl_session) {
                println!("{}", err)
            };
            InputCommandResponse::ProcessNextInput
        }
        CommandType::Status => {
            if let Err(err) = show_status(repl_session, db_conn) {
                println!("Failed to display status due to {}", err);
            };
            InputCommandResponse::ProcessNextInput
        }
        CommandType::Reasoning => {
            if let Err(err) = set_reasoning_effort(agent_writer, &args).await {
                println!("Failed to set reasoning effort due to {}", err);
                InputCommandResponse::ProcessNextInput
            } else {
                InputCommandResponse::WaitForNextLine
            }
        }
        CommandType::Skills => {
            let pi_cmd = json!({
                "type": "get_commands",
            });

            agent_writer.send_to_pi(pi_cmd).await?;
            InputCommandResponse::WaitForNextLine
        }
        _ => InputCommandResponse::ProcessNextInput,
    };
    Ok(res)
}

async fn process_pi_agent_end_event(
    repl_session: &mut ReplSession,
    agent_end_event: PiAgentEndEvent,
    agent: &mut PiWriter,
    db_conn: &Dbconn,
    current_user: &crate::core::account::local::User,
) -> Result<()> {
    if let Some(last_msg) = agent_end_event.messages.last()
        && last_msg.role == Role::Assistant
        && let Some(reason) = &last_msg.stop_reason
        && reason == "error"
    {
        // agent fooked up, lets show a UX friendly msg to try again
        // TODO: Send the err log to local daemon, so we can log it in daemon logs for later debuggin
        let payload = json!({
            "type": "abort"
        });
        agent.send_to_pi(payload).await?;
        println!("An issue occurred, please try again!");
    }

    save_agent_session(repl_session, agent_end_event, db_conn, current_user)?;
    Ok(())
}

fn save_agent_session(
    repl_session: &mut ReplSession,
    agent_end_event: PiAgentEndEvent,
    db_conn: &Dbconn,
    current_user: &crate::core::account::local::User,
) -> Result<()> {
    let mut assistant_parts: Vec<PiMsgContent> = vec![];
    let mut turn = Turn {
        api: Some(String::from("open-responses")),
        provider: Some(String::from("tiles")),
        model: repl_session.current_modelname.clone(),
        messages: vec![],
    };
    //TODO: We need to these at the `message_end` event maybe, need to check
    for msg in agent_end_event.messages {
        //TODO: could avoid this cloning here
        let msg_copy = msg.clone();
        match msg.role {
            Role::User => {
                let input = get_pi_msg_content(msg.content);
                let parent_chat_id = if !repl_session.session_started {
                    create_session(
                        &db_conn.chat,
                        &repl_session.session_id,
                        &repl_session.session_snapshot.name,
                        &current_user.user_id,
                    )?;
                    repl_session.session_started = true;
                    None
                } else {
                    repl_session.last_chat_id.clone()
                };
                let chat_response = ChatResponse {
                    input,
                    session_id: repl_session.session_id.clone(),
                    role: Role::User,
                    parent_chat_id,
                    metrics: None,
                    model_used: repl_session.current_modelname.clone(),
                };
                let prompt_chat = save_chat(&db_conn.chat, current_user, chat_response)?;
                repl_session.last_chat_id = Some(prompt_chat.id);
            }
            Role::Assistant => {
                assistant_parts.extend(msg.content);
            }
            _ => (),
        }
        turn.messages.push(msg_copy);
    }
    let full_response = format_assistant_content(assistant_parts);
    let chat_response = ChatResponse {
        input: full_response,
        session_id: repl_session.session_id.clone(),
        role: Role::Assistant,
        parent_chat_id: repl_session.last_chat_id.clone(),
        metrics: None,
        model_used: repl_session.current_modelname.clone(),
    };
    let chat = save_chat(&db_conn.chat, current_user, chat_response)?;
    repl_session.last_chat_id = Some(chat.id);
    repl_session.session_snapshot.turns.push(turn);
    update_snapshot(
        &db_conn.chat,
        &repl_session.session_id,
        serde_json::to_string(&repl_session.session_snapshot)?,
    )?;
    Ok(())
}

fn get_pi_msg_content(msgs: Vec<PiMsgContent>) -> String {
    let mut content: Vec<String> = vec![];
    for msg in msgs {
        if msg.r#type == "text" {
            content.push(msg.text.unwrap_or(String::from("")));
        } else if msg.r#type == "toolCall"
            && let Some(args) = msg.arguments
        {
            content.push("\n**[ToolCall]**\n".to_string());
            content.push(format!("Tool: {}", msg.name.unwrap_or("None".to_string())));
            content.push(format!("Arguments: {}", &args));
        }
    }
    content.join("\n")
}

/// Rebuild an assistant turn into the marker format the tiles.run/share
/// frontend understands. Reasoning and any tool calls it makes are delimited by
/// `**[Reasoning]**` ... `**[Answer]**`, so the frontend nests them into the
/// collapsible "Reasoning details" block; the final answer follows. Reasoning is
/// only opened when the model actually produced thinking or a tool call, so a
/// plain answer with no reasoning stays clean (matching the previous backends).
fn format_assistant_content(msgs: Vec<PiMsgContent>) -> String {
    let mut out = String::new();
    let mut reasoning_open = false;
    let mut answer_open = false;

    for msg in msgs {
        match msg.r#type.as_str() {
            "thinking" => {
                if !reasoning_open {
                    out.push_str("**[Reasoning]**\n\n");
                    reasoning_open = true;
                }
                out.push_str(&msg.thinking.unwrap_or_default());
            }
            "toolCall" => {
                let Some(args) = msg.arguments else { continue };
                if !reasoning_open {
                    out.push_str("**[Reasoning]**\n\n");
                    reasoning_open = true;
                }
                let arguments = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string());
                out.push_str(&format!(
                    "\n\n**[ToolCall]**\nTool: {}\nArguments: {}",
                    msg.name.unwrap_or_else(|| "None".to_string()),
                    arguments
                ));
            }
            "text" => {
                if reasoning_open && !answer_open {
                    out.push_str("\n\n---\n\n**[Answer]**\n\n");
                    answer_open = true;
                }
                out.push_str(&msg.text.unwrap_or_default());
            }
            _ => {}
        }
    }
    out
}

async fn set_reasoning_effort(agent_writer: &mut PiWriter, args: &[&str]) -> Result<()> {
    let args = if let Some((_main_command, sub_commands)) = args.split_first() {
        sub_commands
    } else {
        return Err(anyhow!("Not a valid command"));
    };

    let reasoning_effort: ReasoningEffort = if args.is_empty() {
        return Err(anyhow!(
            "Please provide Reasoning effort (none, low, medium, high, xhigh)"
        ));
    } else {
        args[0].parse()?
    };

    let effort_str = String::from(reasoning_effort);

    let pi_cmd = json!({
        "type": "set_thinking_level",
        "level": &effort_str
    });

    agent_writer.send_to_pi(pi_cmd).await
}

/// Persist the reasoning level to Pi's settings.json so it survives restarts.
/// Startup reads the level back from Pi's state, which Pi seeds from
/// `defaultThinkingLevel`. Existing settings are preserved.
fn persist_default_thinking_level(level: &str) -> Result<()> {
    let user_data_dir = DefaultProvider.get_user_data_dir()?;
    let settings_path = user_data_dir.join("pi/agent/settings.json");

    let mut settings: Value = match fs::read_to_string(&settings_path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|_| json!({})),
        Err(_) => json!({}),
    };

    let obj = settings
        .as_object_mut()
        .context("settings.json is not a JSON object")?;
    obj.insert("defaultThinkingLevel".to_owned(), json!(level));

    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent).context("Failed to create Pi agent directory")?;
    }
    fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)
        .context("Failed to write settings.json")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::types::{
        GetStateData, PiAgentEndEvent, PiModelInfo, PiMsgContent, PiMsgEvent,
    };
    use crate::core::chats::create_session;
    use crate::core::chats::tests::create_user;
    use rusqlite::Connection;

    #[test]
    fn default_modelfile_uses_platform_default() {
        let path =
            get_default_modelfile(DefaultProvider).expect("default modelfile should resolve");
        assert!(path.ends_with("modelfiles/gemma-4-12b-gguf"));
    }

    #[test]
    fn model_load_warnings_extracts_string_list() {
        let payload = serde_json::json!({
            "message": "Model loaded",
            "warnings": ["MTP enabled but no MTP GGUF found next to /x/model.gguf."]
        });
        assert_eq!(
            model_load_warnings(&payload),
            vec!["MTP enabled but no MTP GGUF found next to /x/model.gguf.".to_owned()]
        );
    }

    #[test]
    fn model_load_warnings_handles_missing_or_malformed() {
        assert!(model_load_warnings(&serde_json::json!({})).is_empty());
        assert!(model_load_warnings(&serde_json::json!({"warnings": null})).is_empty());
        // Non-string entries are skipped, strings are kept.
        let payload = serde_json::json!({"warnings": [42, "valid", null]});
        assert_eq!(model_load_warnings(&payload), vec!["valid".to_owned()]);
    }

    #[test]
    fn status_lines_show_defaults_without_session_or_atproto_login() {
        let state = GetStateData {
            model: PiModelInfo {
                id: String::from("model"),
                name: String::from("model"),
            },
            thinking_level: "low".to_string(),
            is_streaming: true,
            session_id: "def".to_string(),
        };
        let repl_session_b = ReplSession::new(&state);
        let lines = build_status_lines("/tmp/tiles", None, &repl_session_b, None);

        assert_eq!(
            lines,
            vec![
                "Session:          \tSession not started yet",
                "Model:            \tmodel",
                "Reasoning:        \tlow",
                "Working Directory:\t/tmp/tiles",
                "ATProto:          \tNot logged-in",
            ]
        );
    }

    #[test]
    fn status_lines_include_session_and_atproto_login() {
        let db_conn = setup_db_conn();
        create_session(&db_conn.chat, "session-1", "First chat", "user-1")
            .expect("session should be created");
        insert_atproto_login(&db_conn.common);

        let session = fetch_session(&db_conn.chat, "session-1").ok();
        let atproto_login =
            fetch_logged_in_data(&db_conn.common).expect("login lookup should succeed");
        let state = GetStateData {
            model: PiModelInfo {
                id: String::from("llam"),
                name: String::from("llama"),
            },
            thinking_level: "low".to_string(),
            is_streaming: true,
            session_id: "def".to_string(),
        };
        let repl_session_b = ReplSession::new(&state);
        let lines = build_status_lines("/work/tiles", session, &repl_session_b, atproto_login);

        assert!(lines[0].starts_with("Session:          \t"));
        assert!(lines[0].contains("First chat"));
        assert!(lines[0].contains("session-1"));
        assert_eq!(lines[1], "Model:            \tllama");
        assert_eq!(lines[3], "Working Directory:\t/work/tiles");
        assert!(lines[4].starts_with("ATProto:          \t"));
        assert!(lines[4].contains("@"));
        assert!(lines[4].contains("alice.test"));
        assert!(lines[4].contains("did:plc:alice"));
    }

    fn setup_db_conn() -> Dbconn {
        Dbconn {
            chat: setup_chat_db(),
            common: setup_common_db(),
        }
    }

    fn setup_db_conn_v2() -> Dbconn {
        Dbconn {
            chat: crate::core::chats::tests::setup_db_schema(),
            common: crate::core::account::local::tests::setup_db_schema(),
        }
    }

    #[test]
    fn test_saving_valid_agent_session() {
        let state = GetStateData {
            model: PiModelInfo {
                id: String::from("model"),
                name: String::from("model"),
            },
            thinking_level: "low".to_string(),
            is_streaming: true,
            session_id: "abc".to_string(),
        };
        let mut repl_session = ReplSession::new(&state);

        let db_conn = setup_db_conn_v2();
        let current_user = create_user();

        let agent_end_event = PiAgentEndEvent {
            messages: vec![
                PiMsgEvent {
                    role: Role::User,
                    content: vec![PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("what is capital of sweden".to_string()),
                        thinking: None,
                        arguments: None,
                        name: None
                    }],
                    stop_reason: None,
                    timestamp: 1783321582953,
                    tool_name: None
                },
                PiMsgEvent {
                    role: Role::Assistant,
                    content: vec![
                        PiMsgContent {
                        r#type: String::from("thinking"),
                        text: None,
                        thinking: Some(
                            "User asks: \"what is capital of sweden\". Likely they mean Sweden. Answer: Stockholm.".to_string()),
                        arguments: None,
                        name: None
                    },
                    PiMsgContent {
                        r#type: String::from("toolCall"),
                        text: None,
                        thinking: None,
                        arguments: None,
                        name: None
                     },
                    PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("The capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).".to_string()),
                        thinking: None,
                        arguments: None,
                        name: None
                    },
                 ],
                 stop_reason: None,
                 timestamp: 1783321582953,
                 tool_name: None
                },
                PiMsgEvent {
                    role: Role::ToolResult,
                    content: vec![PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("Validation failed for tool \"read\":\n  - path: must have required property 'path'\n\nReceived arguments:\n{}".to_string()),
                        thinking: None,
                        arguments: None,
                        name: None
                    }],
                    stop_reason: None,
                    timestamp: 1783321582953,
                    tool_name: None
                },
                PiMsgEvent {
                    role: Role::Assistant,
                    content: vec![PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("The capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).".to_string()),
                        thinking: None,
                        arguments: None,
                        name: None
                    }],
                    stop_reason: Some("stop".to_string()),
                    timestamp: 1783321582953,
                    tool_name: None
                },
            ],
        };

        assert!(
            save_agent_session(&mut repl_session, agent_end_event, &db_conn, &current_user).is_ok()
        );

        let session = fetch_session(&db_conn.chat, "abc").unwrap();

        let chats = fetch_chats_by_session_id(&db_conn.chat, &session.id).unwrap();

        assert_eq!(chats.chats.len(), 2);
        assert_eq!(
            chats.chats.first().unwrap().content,
            "what is capital of sweden".to_string()
        );
        let last_session = chats.chats.last().unwrap();
        assert_eq!(last_session.content, "**[Reasoning]**\n\nUser asks: \"what is capital of sweden\". Likely they mean Sweden. Answer: Stockholm.\n\n---\n\n**[Answer]**\n\nThe capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).The capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).".to_string());

        assert_eq!(repl_session.last_chat_id.unwrap(), last_session.id);
    }

    #[test]
    fn test_saving_session_after_resuming() {
        // session 1 - abc
        let state = GetStateData {
            model: PiModelInfo {
                id: String::from("model"),
                name: String::from("model"),
            },
            thinking_level: "low".to_string(),
            is_streaming: true,
            session_id: "abc".to_string(),
        };
        let mut repl_session = ReplSession::new(&state);

        let db_conn = setup_db_conn_v2();
        let current_user = create_user();

        let agent_end_event = PiAgentEndEvent {
            messages: vec![
                PiMsgEvent {
                    role: Role::User,
                    content: vec![PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("what is capital of sweden".to_string()),
                        thinking: None,
                        arguments: None,
                        name: None
                    }],
                    stop_reason: None,
                    timestamp: 1783321582953,
                    tool_name: None
                },
                PiMsgEvent {
                    role: Role::Assistant,
                    content: vec![
                        PiMsgContent {
                        r#type: String::from("thinking"),
                        text: None,
                        thinking: Some(
                            "User asks: \"what is capital of sweden\". Likely they mean Sweden. Answer: Stockholm.".to_string()),
                        arguments: None,
                        name: None
                    },
                    PiMsgContent {
                        r#type: String::from("toolCall"),
                        text: None,
                        thinking: None,
                        arguments: Some(serde_json::to_string(&json!({"command": "bash"})).unwrap()),
                        name: Some(String::from("bash"))
                     },
                    PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("The capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).".to_string()),
                        thinking: None,
                        arguments: None,
                        name: None
                    },
                 ],
                 stop_reason: None,
                    timestamp: 1783321582953,
                    tool_name: None
                },
                PiMsgEvent {
                    role: Role::ToolResult,
                    content: vec![PiMsgContent {
                        r#type: String::from("text"),
                        text: None,
                        thinking: None,
                        arguments: Some(serde_json::to_string(&json!({"command": "bash"})).unwrap()),
                        name: Some(String::from("bash"))
                    }],
                    stop_reason: None,
                    timestamp: 1783321582953,
                    tool_name: None
                },
                PiMsgEvent {
                    role: Role::Assistant,
                    content: vec![PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("The capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).".to_string()),
                        thinking: None,
                        arguments: None,
                        name: None
                    }],
                    stop_reason: Some("stop".to_string()),
                    timestamp: 1783321582953,
                    tool_name: None
                },
            ],
        };

        assert!(
            save_agent_session(&mut repl_session, agent_end_event, &db_conn, &current_user).is_ok()
        );

        let session = fetch_session(&db_conn.chat, "abc").unwrap();

        let chats = fetch_chats_by_session_id(&db_conn.chat, &session.id).unwrap();

        assert_eq!(chats.chats.len(), 2);
        assert_eq!(
            chats.chats.first().unwrap().content,
            "what is capital of sweden".to_string()
        );
        let last_session = chats.chats.last().unwrap();
        assert_eq!(last_session.content, "**[Reasoning]**\n\nUser asks: \"what is capital of sweden\". Likely they mean Sweden. Answer: Stockholm.\n\n**[ToolCall]**\nTool: bash\nArguments: \"{\\\"command\\\":\\\"bash\\\"}\"\n\n---\n\n**[Answer]**\n\nThe capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).The capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).".to_string());

        assert_eq!(repl_session.last_chat_id.clone().unwrap(), last_session.id);

        // session 2: def

        let state = GetStateData {
            model: PiModelInfo {
                id: String::from("model"),
                name: String::from("model"),
            },
            thinking_level: "low".to_string(),
            is_streaming: true,
            session_id: "def".to_string(),
        };
        let mut repl_session_b = ReplSession::new(&state);

        let agent_end_event = PiAgentEndEvent{
            messages: vec![
                PiMsgEvent{
                    role: Role::User,
                    content: vec![PiMsgContent{
                        r#type: String::from("text"),
                        text: Some("what is capital of India".to_string()),
                        thinking: None,
                        arguments: None,
                        name: None
                    }],
                    stop_reason: None,
                    timestamp: 1783321582953,
                    tool_name: None
                },
                PiMsgEvent {
                    role: Role::Assistant,
                    content: vec![PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("The capital of India is **Delhi** (often spelled \"Delhi\" in English).".to_string()),
                        thinking: None,
                        arguments: None,
                        name: None
                    }],
                    stop_reason: Some("stop".to_string()),
                    timestamp: 1783321582953,
                    tool_name: None
                },
            ],
        };

        assert!(
            save_agent_session(
                &mut repl_session_b,
                agent_end_event,
                &db_conn,
                &current_user
            )
            .is_ok()
        );

        assert!(load_session(&db_conn, &["resume", "abc"], &mut repl_session_b).is_ok());

        assert_eq!(
            repl_session_b.last_chat_id.unwrap(),
            repl_session.last_chat_id.unwrap()
        );
    }

    fn setup_chat_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                creator_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                snapshot TEXT
            )",
            [],
        )
        .unwrap();
        conn
    }

    fn setup_common_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS atproto_auth_data(
                key TEXT PRIMARY KEY,
                session TEXT,
                state TEXT,
                is_logged_in INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                handle TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        conn
    }

    fn insert_atproto_login(conn: &Connection) {
        conn.execute(
            "INSERT INTO atproto_auth_data(
                key, session, state, is_logged_in, created_at, updated_at, handle
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            ("did:plc:alice", "{}", "", true, 1_i64, 1_i64, "alice.test"),
        )
        .unwrap();
    }
}
