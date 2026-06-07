use crate::core::account::atproto::{fetch_logged_in_data, login, share_session};
use crate::core::account::local::get_current_user;
use crate::core::chats::{
    Session, create_session, fetch_chats_by_session_id, fetch_models_used_by_session,
    fetch_session, fetch_sessions, save_chat,
};
use crate::core::storage::db::Dbconn;
use crate::utils::config::{
    ConfigProvider, DefaultProvider, create_pi_provider_config, get_memory_path, get_model_cache,
};
use crate::utils::hf_model_downloader::*;
use anyhow::{Context, Result, anyhow};
use atrium_api::types::string::Datetime;
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
use std::fs::{self, OpenOptions};
use std::io::{self};
use std::path::PathBuf;
use std::process::Stdio;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tilekit::modelfile::Modelfile;
use tilekit::modelfile::Role;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
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
    pub memory: bool, // Future flags go here
    pub pi: bool,
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum PiResponse {
    #[serde(rename = "response")]
    Response(PiResponseMessage),
    #[serde(rename = "agent_start")]
    AgentStart,
    #[serde(rename = "message_update")]
    MessageUpdate(PiMessageUpdate),
    #[serde(rename = "agent_end")]
    AgentEnd(PiAgentEndEvent),
    #[serde(rename = "turn_end")]
    TurnEnd(PiTurnEndEvent),
    #[serde[other]]
    Unknown,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetStateData {
    model: PiModelInfo,
    #[serde(rename = "thinkingLevel")]
    pub thinking_level: String,
    #[serde(rename = "isStreaming")]
    is_streaming: bool,
    #[serde(rename = "sessionId")]
    session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PiModelInfo {
    id: String,
    name: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct PiMessageUpdate {
    #[serde(rename = "assistantMessageEvent")]
    assistant_message_event: PiAsstTextMsg,
}

#[derive(Serialize, Deserialize, Debug)]
struct PiAsstTextMsg {
    r#type: AsstMsgEventType,
    delta: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PiResponseMessage {
    command: CommandType,
    success: bool,
    data: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PiTurnEndEvent {
    message: PiTurnEndEventMsg,
}

#[derive(Serialize, Deserialize, Debug)]
struct PiTurnEndEventMsg {
    role: String,
    content: Vec<PiMsgContent>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PiMsgEvent {
    role: Role,
    content: Vec<PiMsgContent>,
    #[serde(rename = "stopReason")]
    stop_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PiMsgContent {
    r#type: String,
    text: Option<String>,
    thinking: Option<String>,
    arguments: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PiAgentEndEvent {
    messages: Vec<PiMsgEvent>,
}

#[derive(Serialize, Deserialize, Debug)]
enum AsstMsgEventType {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "text_start")]
    TextStart,
    #[serde(rename = "text_delta")]
    TextDelta,
    #[serde(rename = "text_end")]
    TextEnd,
    #[serde(rename = "thinking_start")]
    ThinkingStart,
    #[serde(rename = "thinking_delta")]
    ThinkingDelta,
    #[serde(rename = "thinking_end")]
    ThinkingEnd,
    #[serde(rename = "toolcall_start")]
    ToolcallStart,
    #[serde(rename = "toolcall_delta")]
    ToolcallDelta,
    #[serde(rename = "toolcall_end")]
    ToolcallEnd,
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "error")]
    Error,
}

#[derive(Clone, Copy)]
enum ReasoningEffort {
    High,
    Medium,
    Low,
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
const PY_PORT: u32 = 6969;

pub async fn run(run_args: RunArgs, db_conn: &Dbconn) -> Result<()> {
    let default_modelfile_path = get_default_modelfile(run_args.memory)?;
    let default_modelfile =
        tilekit::modelfile::parse_from_file(default_modelfile_path.to_str().unwrap()).unwrap();
    let modelfile_parse_result = if let Some(modelfile_str) = &run_args.modelfile_path {
        tilekit::modelfile::parse_from_file(modelfile_str.as_str())
    } else {
        Err("NOT PROVIDED".to_string())
    };

    let modelfile = match modelfile_parse_result {
        Ok(mf) => mf,
        Err(err) if err == "NOT PROVIDED" => default_modelfile.clone(),
        Err(_err) => {
            println!("Invalid Modelfile");
            return Ok(());
        }
    };

    run_model_with_server(modelfile, default_modelfile, &run_args, db_conn).await
}

#[allow(clippy::zombie_processes)]
pub async fn start_server_daemon() -> Result<()> {
    // check if the server is running
    // start server as a child process
    // save the pid in a file under ~/.config/tiles/server_pid

    if (ping().await).is_ok() {
        println!("server is already up");
        return Ok(());
    }

    let config_dir = DefaultProvider.get_config_dir()?;
    let data_dir = DefaultProvider.get_data_dir()?;
    let mut server_dir = DefaultProvider.get_lib_dir()?;
    let pid_file = config_dir.join("server.pid");
    server_dir = server_dir.join("server");
    let stdout_log = OpenOptions::new()
        .append(true)
        .open(data_dir.join("logs/server.out.log"))?;
    let stderr_log = OpenOptions::new()
        .append(true)
        .open(data_dir.join("logs/server.err.log"))?;
    let server_path = server_dir.join("stack_export_prod/app-server/bin/python");
    server_dir.pop();
    let child = unsafe {
        Command::new(server_path)
            .args(["-m", "server.main"])
            .current_dir(server_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout_log))
            .stderr(Stdio::from(stderr_log))
            .pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            })
            .spawn()
            .expect("failed to start server")
    };

    std::fs::write(pid_file, child.id().expect("Not child Id").to_string())
        .expect("Failed to write to pid file");
    println!(
        "Server started with PID {}",
        child.id().expect("No child Id")
    );
    Ok(())
}

pub async fn stop_server_daemon() -> Result<()> {
    if (ping().await).is_err() {
        println!("Server is not running");
        return Ok(());
    }
    let pid_file = DefaultProvider.get_config_dir()?.join("server.pid");

    if !pid_file.exists() {
        eprintln!("server pid doesnt exist");
        return Ok(());
    }

    let pid = std::fs::read_to_string(&pid_file).context("Failed to read the string")?;
    Command::new("kill")
        .arg(pid.trim())
        .status()
        .await
        .context("Failed to initiate kill commad")?;

    std::fs::remove_file(pid_file).context("Failed to removed pid file")?;
    println!("Server stopped.");
    Ok(())
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
}

#[derive(Deserialize, Serialize, Debug)]
enum CommandType {
    #[serde(rename = "status")]
    Status,
    #[serde(rename = "share")]
    Share,
    #[serde(rename = "sessions")]
    Sessions,
    #[serde(rename = "resume")]
    Resume,
    #[serde(rename = "set_thinking_level")]
    Reasoning,
    #[serde(rename = "abort")]
    Abort,
    #[serde(other)]
    Unknown,
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
}

impl ReplSession {
    pub fn new(state: &GetStateData) -> Self {
        ReplSession {
            session_id: state.session_id.clone(),
            resume_session_pending: false,
            resumed_session: String::from(""),
            current_modelname: state.model.name.to_owned(),
            last_chat_id: None,
            session_started: false,
            reasoning: state.thinking_level.parse().unwrap_or(ReasoningEffort::Low),
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
        match cmd {
            "help" | "?" => {
                show_help();
                InputType::Skip
            }
            "bye" => InputType::Exit,
            "" => {
                println!("Empty command. Type /help for available commands.");
                InputType::Skip
            }
            cmd => InputType::Command(cmd.to_owned()),
        }
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
                    "/set_thinking_level <reasoning_value>",
                    "Set the reasoning effort of current model (high, medium, low)",
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
    if !cfg!(debug_assertions) {
        let _ = start_server_daemon().await.inspect_err(|e| {
            eprintln!("Failed to start daemon server due to {:?}", e);
        });
        let _ = wait_until_server_is_up().await;
    }
    // loading the model from mem-agent via daemon server
    let memory_path = get_memory_path().context("Setting/Retrieving memory_path failed")?;
    match load_model(&modelfile, &default_modelfile, &memory_path, 0).await {
        Ok(_) => start_repl(&modelfile, run_args, db_conn)
            .await
            .map_err(|e| anyhow!(e)),
        Err(err) => Err(anyhow!(err)),
    }
}

#[allow(unused_assignments)]
async fn start_repl(modelfile: &Modelfile, _run_args: &RunArgs, db_conn: &Dbconn) -> Result<()> {
    let modelname = modelfile
        .from
        .clone()
        .ok_or_else(|| anyhow!("Error getting FROM from modelfile due to"))?;

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

    // Setting up Pi rpc process handles
    let mut pi_process = start_pi_rpc(&modelname, &system_prompt)?;
    let pi_stdin = pi_process.stdin.as_mut().unwrap();
    let mut pi_stdout = pi_process.stdout.take().expect("stdout");

    let pi_session_state = get_pi_state(pi_stdin, &mut pi_stdout).await?;
    let mut repl_session = ReplSession::new(&pi_session_state);

    // The great REPL loop
    loop {
        // Reads the user input
        let readline = editor.readline(">>> ");
        let input = match readline {
            Ok(line) => line.trim().to_string().to_lowercase(),
            Err(_) => {
                // FIXME: Panic when entering another prompt after ctr-l C
                // called `Result::unwrap()` on an `Err` value: Os { code: 32, kind: BrokenPipe, message: "Broken pipe" }
                //
                // User pressed Ctrl+C or Ctrl+D
                handle_repl_exit(pi_stdin).await?;
                break;
            }
        };

        if input.is_empty() {
            continue;
        }

        // Process the user input in the repl
        match handle_input(&input) {
            InputType::Skip => continue,
            InputType::Exit => {
                handle_repl_exit(pi_stdin).await?;
                break;
            }
            InputType::Prompt => {
                handle_input_prompt(pi_stdin, &mut repl_session, &input).await?;
            }
            InputType::Command(cmd) => {
                let res = handle_input_commands(cmd, &mut repl_session, db_conn, pi_stdin).await?;

                if let InputCommandResponse::ProcessNextInput = res {
                    continue;
                }
            }
        }

        // Reads the output from Pi and process and responds to the repl
        let mut reader = BufReader::new(&mut pi_stdout).lines();

        while let Some(line) = reader.next_line().await? {
            if !running.load(std::sync::atomic::Ordering::SeqCst) {
                info!("Ctrlc detected, aborting Pi ops");
                let end_payload = json!({
                    "type": "abort",
                });
                send_to_pi(pi_stdin, end_payload).await?;
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
                        pi_stdin,
                        db_conn,
                        &current_user,
                    )
                    .await?;
                    break;
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
                                process_command(
                                    response_msg,
                                    &mut repl_session,
                                    pi_stdin,
                                    &mut pi_stdout,
                                )
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
            }
        }
    }
    Ok(())
}

pub async fn ping() -> Result<()> {
    let client = Client::new();
    let url = format!("http://127.0.0.1:{}/ping", PY_PORT);
    let res = client.get(url).send().await;

    match res {
        Err(err) => Err(anyhow!("Server down due to {:?}", err)),
        _ => Ok(()),
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
    let model_name = modelfile.from.clone().unwrap();
    let model_cache_res = get_model_cache(&model_name);

    if model_cache_res.is_err() {
        download_model(&model_name).await?;
        return Box::pin(load_model(modelfile, default_modelfile, memory_path, 0)).await;
    }

    // If loading fails it most probably a partial downloaded
    // model present, so we try to resume the download
    if load_model_in_py(
        modelfile,
        default_modelfile,
        memory_path,
        &model_cache_res.unwrap(),
    )
    .await
    .is_err()
    {
        log::warn!("Load model failed, resuming the partial download");
        download_model(&model_name).await?;
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
            Ok(()) => {
                break;
            }
            Err(_err) => {
                println!("### tiling ###...");
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

fn get_default_modelfile(memory_mode: bool) -> Result<PathBuf> {
    if memory_mode {
        let path = DefaultProvider.get_lib_dir()?.join("modelfiles/mem-agent");
        Ok(path)
    } else {
        let path = DefaultProvider.get_lib_dir()?.join("modelfiles/gpt-oss");
        Ok(path)
    }
}

async fn load_model_in_py(
    modelfile: &Modelfile,
    default_modelfile: &Modelfile,
    memory_path: &str,
    model_cache_path: &PathBuf,
) -> Result<()> {
    let client = Client::new();
    let model_name = modelfile
        .from
        .clone()
        .expect("Failed to get `FROM` of modelfile");
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
        StatusCode::OK => Ok(()),
        _ => Err(anyhow::anyhow!(format!(
            "Failed to load model {} due to {:?}",
            model_name, res
        ))),
    }
}

async fn download_model(model_name: &str) -> Result<()> {
    match pull_model(model_name).await {
        Ok(_) => {
            println!("\nDownloading completed \n");
            Ok(())
        }
        Err(err) => Err(anyhow::anyhow!(format!("Download failed due to {:?}", err))),
    }
}

// Need to create models.json for the provider
fn start_pi_rpc(model_name: &str, system_prompt: &str) -> Result<Child> {
    let tiles_lib_dir = DefaultProvider.get_lib_dir()?;
    let user_data_dir = DefaultProvider.get_user_data_dir()?;
    let pi_agent_dir = user_data_dir.join("pi/agent/");
    std::fs::create_dir_all(&pi_agent_dir).context("Failed to create Pi agent directory")?;

    let provider_config_file_path = pi_agent_dir.join("models.json");
    let endpoint_url = format!("http://127.0.0.1:{}/v1", PY_PORT);
    let model_config = create_pi_provider_config(model_name, &endpoint_url)?;

    fs::write(provider_config_file_path, model_config)?;

    // For easy debugging Pi, when developing when needed we can directly call the
    // local on-demand build pi binary and point local path
    // assuming `tiles-pi` is cloned as a sibling in the same dir
    // let pi_exec_path =
    //  PathBuf::from("~/tiles-pi/packages/coding-agent/binaries/darwin-arm64/pi");
    // For example:
    // let pi_exec_path =
    //     PathBuf::from("/Users/tiles/tiles-pi/packages/coding-agent/binaries/darwin-arm64/pi");
    // On building binary locally, from tiles-pi root dir run
    // `./scripts/build-binaries.sh --platform darwin-arm64`
    // More platform flags can be seen in the `build-binaries.sh`

    let pi_exec_path = tiles_lib_dir.join("pi/pi");

    let pi_process = unsafe {
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
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            })
            .spawn()
            .expect("failed to run Pi")
    };
    Ok(pi_process)
}

async fn send_to_pi(pi_child_stdin: &mut ChildStdin, payload_json: Value) -> Result<()> {
    let payload_str = format!("{}\n", serde_json::to_string(&payload_json)?);
    pi_child_stdin.write_all(payload_str.as_bytes()).await?;
    pi_child_stdin.flush().await?;
    Ok(())
}

async fn process_command(
    response_msg: PiResponseMessage,
    repl_session: &mut ReplSession,
    pi_stdin: &mut ChildStdin,
    pi_stdout: &mut ChildStdout,
) -> Result<()> {
    if let CommandType::Reasoning = response_msg.command {
        let state = get_pi_state(pi_stdin, pi_stdout).await?;
        repl_session.reasoning = state
            .thinking_level
            .parse::<ReasoningEffort>()
            .context("Failed to parse reasoning effort")?;
        println!("Reasoning settings updated successfully")
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

    let mut shared_contents: Vec<SharedContent> = vec![];
    for chat in delta_chats.chats {
        shared_contents.push(SharedContent {
            role: chat.role,
            content: chat.content,
        });
    }

    let models_used = fetch_models_used_by_session(&conn.chat, session_id)?;
    let shared_sessions = SharedSession {
        r#type: "run.tiles.session".to_string(),
        session_id: session_id.to_string(),
        name: session.name.clone(),
        contents: shared_contents,
        created_at: Datetime::now().as_str().to_string(),
        models_used,
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

    match share_session(&conn.common, shared_sessions.clone(), is_private).await {
        Err(err) if &err.to_string() == "NOT_LOGGED_IN" => {
            let login_prompt = format!("{}", "Sharing a chat session requires logging in, as the data is stored on your Bluesky-based ATProto PDS.\nDo you want to proceed with the login flow? (Y/n)".yellow());

            println!("{}", login_prompt);

            let stdin = io::stdin();
            let mut input = String::new();
            stdin.read_line(&mut input)?;
            let clean_input = input.trim();
            if clean_input.to_lowercase() == "y" {
                input.clear();
                println!("Please enter your Bluesky handle (ex: john.bsky.team)");
                stdin.read_line(&mut input)?;
                login(conn, input.trim()).await?;
                share_session(&conn.common, shared_sessions, is_private).await?;
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
    let sessions = fetch_sessions(&db_conn.chat)?;

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

    repl_session.session_id = session_id.to_string();
    repl_session.set_pending_resume_session(true);
    repl_session.set_resumed_session(chat_history);
    repl_session.last_chat_id = last_chat_id;
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

fn handle_pi_message_update(msg_update: PiMessageUpdate) {
    match msg_update.assistant_message_event.r#type {
        AsstMsgEventType::TextStart => {
            println!();
            info!("msg text_start")
        }
        AsstMsgEventType::TextDelta => {
            if let Some(delta) = msg_update.assistant_message_event.delta {
                print!("{}", delta);
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }
        AsstMsgEventType::TextEnd => {
            info!("msg text_end")
        }
        AsstMsgEventType::ThinkingStart => {
            println!();
        }
        AsstMsgEventType::ThinkingDelta => {
            if let Some(delta) = msg_update.assistant_message_event.delta {
                print!("{}", delta.dimmed());
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }
        AsstMsgEventType::ThinkingEnd => {}
        AsstMsgEventType::ToolcallStart => {
            info!("Selecting tool to execute");
            println!();
            let delta = "**[Tool Calling]**";
            println!("{}", delta.dimmed());
        }
        AsstMsgEventType::ToolcallDelta => {
            if let Some(delta) = msg_update.assistant_message_event.delta {
                print!("{}", delta.dimmed());
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }
        AsstMsgEventType::ToolcallEnd => {
            info!("Tool call selected");
        }
        AsstMsgEventType::Done => {
            info!("msg done event")
        }
        AsstMsgEventType::Error => {
            warn!("msg error event")
        }
        _ => (),
    }
}

async fn get_pi_state(
    pi_stdin: &mut ChildStdin,
    pi_stdout: &mut ChildStdout,
) -> Result<GetStateData> {
    let init_cmd_payload = json!({
        "type": "get_state",
    });
    send_to_pi(pi_stdin, init_cmd_payload)
        .await
        .inspect_err(|_e| eprintln!("sending command to  pi failed"))?;

    let reader = BufReader::new(pi_stdout);

    if let Some(line) = reader.lines().next_line().await? {
        let response: PiResponse = serde_json::from_str(&line)?;
        if let PiResponse::Response(msg) = response {
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

async fn handle_repl_exit(pi_stdin: &mut ChildStdin) -> Result<()> {
    let end_payload = json!({
        "type": "abort",
    });
    let payload_str = format!("{}\n", serde_json::to_string(&end_payload)?);
    pi_stdin.write_all(payload_str.as_bytes()).await?;
    pi_stdin.flush().await?;
    println!("Exiting interactive mode");
    if !cfg!(debug_assertions) {
        let _res = stop_server_daemon().await;
    }
    Ok(())
}

async fn handle_input_prompt(
    pi_stdin: &mut ChildStdin,
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
    send_to_pi(pi_stdin, payload).await
}

async fn handle_input_commands(
    cmd: String,
    repl_session: &mut ReplSession,
    db_conn: &Dbconn,
    pi_stdin: &mut ChildStdin,
) -> Result<InputCommandResponse> {
    let args: Vec<&str> = cmd.split(" ").collect();
    let main_cmd = args.first().expect("Main command should be there");

    let cmd_json = json!(main_cmd);

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
            if let Err(err) = set_reasoning_effort(pi_stdin, &args).await {
                println!("Failed to set reasoning effort due to {}", err);
                InputCommandResponse::ProcessNextInput
            } else {
                InputCommandResponse::WaitForNextLine
            }
        }
        _ => InputCommandResponse::ProcessNextInput,
    };
    Ok(res)
}

async fn process_pi_agent_end_event(
    repl_session: &mut ReplSession,
    agent_end_event: PiAgentEndEvent,
    pi_stdin: &mut ChildStdin,
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
        send_to_pi(pi_stdin, payload).await?;
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
    let mut full_response: String = String::from("");
    for msg in agent_end_event.messages {
        match msg.role {
            Role::User => {
                let input = get_pi_msg_content(msg.content);
                let parent_chat_id = if !repl_session.session_started {
                    create_session(
                        &db_conn.chat,
                        &repl_session.session_id,
                        &input,
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
                let response = get_pi_msg_content(msg.content);
                full_response.push_str(&response);
            }
            _ => continue,
        }
    }
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
    Ok(())
}

fn get_pi_msg_content(msgs: Vec<PiMsgContent>) -> String {
    let mut content: Vec<String> = vec![];
    for msg in msgs {
        if msg.r#type == "text" {
            content.push(msg.text.unwrap_or(String::from("")));
        } else if msg.r#type == "thinking" {
            content.push(msg.thinking.unwrap_or(String::from("")));
        } else if msg.r#type == "toolCall"
            && let Some(args) = msg.arguments
        {
            content.push("\n**[ToolCall]**\n".to_string());
            let arguments = serde_json::to_string(&args).unwrap_or("{}".to_string());
            content.push(arguments);
        }
    }
    content.join("\n")
}

async fn set_reasoning_effort(pi_stdin: &mut ChildStdin, args: &[&str]) -> Result<()> {
    let args = if let Some((_main_command, sub_commands)) = args.split_first() {
        sub_commands
    } else {
        return Err(anyhow!("Not a valid command"));
    };

    let reasoning_effort: ReasoningEffort = if args.is_empty() {
        return Err(anyhow!(
            "Please provide Reasoning effort (low, medium, high)"
        ));
    } else {
        args[0].parse()?
    };

    let effort_str = String::from(reasoning_effort);

    let pi_cmd = json!({
        "type": "set_thinking_level",
        "level": &effort_str
    });

    send_to_pi(pi_stdin, pi_cmd).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chats::create_session;
    use crate::core::chats::tests::create_user;
    use rusqlite::Connection;
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
                        arguments: None
                    }],
                    stop_reason: None,
                },
                PiMsgEvent {
                    role: Role::Assistant,
                    content: vec![
                        PiMsgContent {
                        r#type: String::from("thinking"),
                        text: None,
                        thinking: Some(
                            "**[Reasoning]**\n\nUser asks: \"what is capital of sweden\". Likely they mean Sweden. Answer: Stockholm.".to_string()),
                        arguments: None
                    },
                    PiMsgContent {
                        r#type: String::from("toolCall"),
                        text: None,
                        thinking: None,
                        arguments: None
                     },
                    PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("\n---\n**[Answer]**\n\nThe capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).".to_string()),
                        thinking: None,
                        arguments: None
                    },
                 ],
                 stop_reason: None,
                },
                PiMsgEvent {
                    role: Role::ToolResult,
                    content: vec![PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("Validation failed for tool \"read\":\n  - path: must have required property 'path'\n\nReceived arguments:\n{}".to_string()),
                        thinking: None,
                        arguments: None
                    }],
                    stop_reason: None,
                },
                PiMsgEvent {
                    role: Role::Assistant,
                    content: vec![PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("\n---\n**[Answer]**\n\nThe capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).".to_string()),
                        thinking: None,
                        arguments: None
                    }],
                    stop_reason: Some("stop".to_string()),
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
        assert_eq!(last_session.content, "**[Reasoning]**\n\nUser asks: \"what is capital of sweden\". Likely they mean Sweden. Answer: Stockholm.\n\n---\n**[Answer]**\n\nThe capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).\n---\n**[Answer]**\n\nThe capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).".to_string());

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
                        arguments: None
                    }],
                    stop_reason: None,
                },
                PiMsgEvent {
                    role: Role::Assistant,
                    content: vec![
                        PiMsgContent {
                        r#type: String::from("thinking"),
                        text: None,
                        thinking: Some(
                            "**[Reasoning]**\n\nUser asks: \"what is capital of sweden\". Likely they mean Sweden. Answer: Stockholm.".to_string()),
                        arguments: None
                    },
                    PiMsgContent {
                        r#type: String::from("toolCall"),
                        text: None,
                        thinking: None,
                        arguments: None
                     },
                    PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("\n---\n**[Answer]**\n\nThe capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).".to_string()),
                        thinking: None,
                        arguments: None
                    },
                 ],
                 stop_reason: None,
                },
                PiMsgEvent {
                    role: Role::ToolResult,
                    content: vec![PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("Validation failed for tool \"read\":\n  - path: must have required property 'path'\n\nReceived arguments:\n{}".to_string()),
                        thinking: None,
                        arguments: None
                    }],
                    stop_reason: None,
                },
                PiMsgEvent {
                    role: Role::Assistant,
                    content: vec![PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("\n---\n**[Answer]**\n\nThe capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).".to_string()),
                        thinking: None,
                        arguments: None
                    }],
                    stop_reason: Some("stop".to_string()),
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
        assert_eq!(last_session.content, "**[Reasoning]**\n\nUser asks: \"what is capital of sweden\". Likely they mean Sweden. Answer: Stockholm.\n\n---\n**[Answer]**\n\nThe capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).\n---\n**[Answer]**\n\nThe capital of Sweden is **Stockholm** (often spelled \"Stockholm\" in English).".to_string());

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

        let agent_end_event = PiAgentEndEvent {
            messages: vec![
                PiMsgEvent {
                    role: Role::User,
                    content: vec![PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("what is capital of India".to_string()),
                        thinking: None,
                        arguments: None
                    }],
                    stop_reason: None,
                },
                PiMsgEvent {
                    role: Role::Assistant,
                    content: vec![PiMsgContent {
                        r#type: String::from("text"),
                        text: Some("\n---\n**[Answer]**\n\nThe capital of India is **Delhi** (often spelled \"Delhi\" in English).".to_string()),
                        thinking: None,
                        arguments: None
                    }],
                    stop_reason: Some("stop".to_string()),
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
                created_at INTEGER NOT NULL
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
