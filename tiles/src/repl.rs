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
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Command};
use std::process::{ChildStdin, Stdio};
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
    pub memory: bool, // Future flags go here
    pub pi: bool,
}
#[derive(Clone, Debug)]
pub struct ChatResponse {
    // text content
    pub input: String,
    pub session_id: String,
    pub role: Role,
    pub code: Option<String>,
    // deprecated, will remove soon
    pub prev_response_id: Option<String>,
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
    AgentEnd,
    #[serde(rename = "turn_end")]
    TurnEnd(PiTurnEndEvent),
    // #[serde(rename = "tool_execution_start")]
    // ToolExecutionStart,
    // #[serde(rename = "tool_execution_update")]
    // ToolExecutionUpdate,
    // #[serde(rename = "tool_execution_end")]
    // ToolExecutionEnd,
    #[serde[other]]
    Unknown,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetStateData {
    model: Value,
    #[serde(rename = "thinkingLevel")]
    thinking_level: String,
    #[serde(rename = "isStreaming")]
    is_streaming: bool,
    #[serde(rename = "sessionId")]
    session_id: String,
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
struct PiMsgContent {
    r#type: String,
    text: Option<String>,
    thinking: Option<String>,
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
    let child = Command::new(server_path)
        .args(["-m", "server.main"])
        .current_dir(server_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_log))
        .stderr(Stdio::from(stderr_log))
        .spawn()
        .expect("failed to start server");

    std::fs::write(pid_file, child.id().to_string()).expect("Failed to write to pid file");
    println!("Server started with PID {}", child.id());
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
    turn_count: usize,
    // if true, we will prepend the resumed session history to the input
    resume_session_pending: bool,
    resumed_session: String,
    pub current_modelname: String,
    pub last_chat_id: String,
}

impl ReplSession {
    pub fn new(session_id: &str, model_name: &str) -> Self {
        ReplSession {
            session_id: session_id.to_owned(),
            turn_count: 0,
            resume_session_pending: false,
            resumed_session: String::from(""),
            current_modelname: model_name.to_owned(),
            last_chat_id: String::from(""),
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

    pub fn get_turn_count(&self) -> usize {
        self.turn_count
    }

    pub fn set_turn_count(&mut self, count: usize) {
        self.turn_count = count
    }

    pub fn inc_turn_count(&mut self) {
        self.turn_count = self.turn_count + 1
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

    // Setting up Pi rpc process handles
    let mut pi_process = start_pi_rpc(&modelname, &system_prompt)?;
    let mut pi_stdin = pi_process.stdin.as_mut().unwrap();
    let mut pi_stdout = pi_process.stdout.take().expect("stdout");
    let inti_cmd_payload = get_command_payload(CommandType::Status);
    send_to_pi(pi_stdin, inti_cmd_payload)
        .inspect_err(|_e| eprintln!("sending command to  pi failed"))?;

    let pi_session_id = get_initial_session_id(&mut pi_stdin, &mut pi_stdout)?;
    let mut repl_session = ReplSession::new(&pi_session_id, &modelname);

    // The great REPL loop
    loop {
        repl_session.set_pending_resume_session(false);

        // Reads the user input
        let readline = editor.readline(">>> ");
        let input = match readline {
            Ok(line) => line.trim().to_string().to_lowercase(),
            Err(_) => {
                //TODO: Panic when entering another prompt after ctr-l C
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
                handle_input_prompt(pi_stdin, &repl_session, &input)?;
            }
            InputType::Command(cmd) => {
                handle_input_commands(cmd, &mut repl_session, db_conn, &modelname).await?;
                continue;
            }
        }

        // Reads the output from Pi and process and responds to the repl
        let reader = BufReader::new(&mut pi_stdout);
        for line in reader.lines() {
            let line = line?;
            let response: PiResponse =
                serde_json::from_str(&line).context("Failed to parse Pi response")?;
            match response {
                PiResponse::AgentStart => {
                    // info!("agent start")
                }
                PiResponse::MessageUpdate(msg_update) => {
                    handle_pi_message_update(msg_update);
                }
                // PiResponse::ToolExecutionStart => {
                //     info!("tool exec start")
                // }
                // PiResponse::ToolExecutionUpdate => {
                //     info!("tool exec update")
                // }
                // PiResponse::ToolExecutionEnd => {
                //     info!("tool exec end")
                // }
                PiResponse::AgentEnd => {
                    // info!("agent end");
                    break;
                }
                // TODO: We should think about process in the agent end instead of
                // turn end, since multi-turn, means each entry in db as of now
                PiResponse::TurnEnd(turn_event) => {
                    process_pi_turn_event(
                        turn_event,
                        &mut repl_session,
                        db_conn,
                        &current_user,
                        &input,
                    )?;
                }
                PiResponse::Response(response_msg) => {
                    if response_msg.success {
                        match response_msg.command {
                            CommandType::Unknown => {
                                continue;
                            }
                            cmd => process_command(cmd, response_msg.data)?,
                        }
                    } else {
                        println!("Command failed")
                    }
                    break;
                }
                PiResponse::Unknown => {
                    // info!("Unsupported response {}", &line);
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
    // PathBuf::from("/Users/tiles/tiles-pi/packages/coding-agent/binaries/darwin-arm64/pi");
    // On building binary locally, from tiles-pi root dir run
    // `./scripts/build-binaries.sh --platform darwin-arm64`
    // More platform flags can be seen in the `build-binaries.sh`

    let pi_exec_path = tiles_lib_dir.join("pi/pi");

    let pi_process = Command::new(pi_exec_path)
        .arg("--mode")
        .arg("rpc")
        .arg("--append-system-prompt")
        .arg(system_prompt)
        .arg("--no-session")
        .env("PI_CODING_AGENT_DIR", pi_agent_dir)
        .env("PI_OFFLINE", "true")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to run Pi");

    Ok(pi_process)
}

fn send_to_pi(pi_child_stdin: &mut ChildStdin, payload_json: Value) -> Result<()> {
    let payload_str = format!("{}\n", serde_json::to_string(&payload_json)?);

    pi_child_stdin.write_all(payload_str.as_bytes()).unwrap();
    pi_child_stdin.flush()?;
    Ok(())
}

fn get_command_payload(cmd: CommandType) -> Value {
    match cmd {
        CommandType::Unknown => {
            json!({
                "type": "none"
            })
        }
        CommandType::Status => {
            json!({
                "type": "get_state",
            })
        }
        // catch-all cases are where prolly its not a Pi command
        _ => json!([]),
    }
}

fn process_command(_cmd: CommandType, _data: Option<Value>) -> Result<()> {
    // if let CommandType::Unknown = cmd {
    //     ()
    // }
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
            eprintln!("Failed to share session due to {:?}\nTry re-login", err)
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

//TODO: load the session via prompt into the model too
fn load_session(db_conn: &Dbconn, args: &[&str]) -> Result<(String, usize, String)> {
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

    Ok((
        session_id.to_string(),
        delta_chats.chats.len(),
        chat_history,
    ))
}

fn show_status(session_id: &str, modelname: &str, db_conn: &Dbconn) -> Result<()> {
    let cwd_pathbuf = std::env::current_dir()?;
    let cwd = cwd_pathbuf.to_string_lossy();
    let logged_in_atproto = fetch_logged_in_data(&db_conn.common)?;
    let session_data: Option<Session> = fetch_session(&db_conn.chat, session_id).ok();
    let status_lines = build_status_lines(&cwd, session_data, modelname, logged_in_atproto);

    println!("\n");
    for line in status_lines {
        println!("{}", line);
    }
    Ok(())
}

fn build_status_lines(
    cwd: &str,
    session_data: Option<Session>,
    modelname: &str,
    logged_in_atproto: Option<crate::core::account::atproto::AtprotoAuthData>,
) -> Vec<String> {
    let mut status_map: Vec<(&str, String)> = vec![];
    let session_status = if let Some(session) = session_data {
        format!("{} ({})", session.name.yellow(), session.id.dimmed())
    } else {
        "Session not started yet".to_owned()
    };
    status_map.push(("Session", session_status));
    status_map.push(("Model", modelname.to_owned()));
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
        AsstMsgEventType::ThinkingStart => {}
        AsstMsgEventType::ThinkingDelta => {
            if let Some(delta) = msg_update.assistant_message_event.delta {
                print!("{}", delta.dimmed());
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }
        AsstMsgEventType::ThinkingEnd => {
            // info!("msg thinking_end")
        }
        AsstMsgEventType::ToolcallStart => {
            println!("Selecting tool to execute")
            // info!("toolcall msg_start")
        }
        AsstMsgEventType::ToolcallDelta => {
            // info!("toolcall msg_delta")
            if let Some(delta) = msg_update.assistant_message_event.delta {
                print!("{}", delta.dimmed());
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }
        AsstMsgEventType::ToolcallEnd => {
            // info!("toolcall msg_end")
            println!("Tool call selected")
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

fn get_initial_session_id(
    pi_stdin: &mut ChildStdin,
    pi_stdout: &mut ChildStdout,
) -> Result<String> {
    let inti_cmd_payload = get_command_payload(CommandType::Status);
    send_to_pi(pi_stdin, inti_cmd_payload)
        .inspect_err(|_e| eprintln!("sending command to  pi failed"))?;

    let mut pi_session_state = String::new();
    let mut reader = BufReader::new(pi_stdout);
    let _ = reader
        .read_line(&mut pi_session_state)
        .context("Failed reading pi session state")?;
    let response: PiResponse = serde_json::from_str(&pi_session_state)?;
    if let PiResponse::Response(msg) = response {
        let state: GetStateData =
            serde_json::from_value(msg.data.expect("get state parsing failed"))?;
        Ok(state.session_id)
    } else {
        Err(anyhow!("Failed to fetch session_id from Pi"))
    }
}

async fn handle_repl_exit(pi_stdin: &mut ChildStdin) -> Result<()> {
    let end_payload = json!({
        "type": "abort",
    });
    let payload_str = format!("{}\n", serde_json::to_string(&end_payload)?);
    pi_stdin.write_all(payload_str.as_bytes())?;
    pi_stdin.flush()?;
    println!("Exiting interactive mode");
    if !cfg!(debug_assertions) {
        let _res = stop_server_daemon().await;
    }
    Ok(())
}

fn handle_input_prompt(
    pi_stdin: &mut ChildStdin,
    repl_session: &ReplSession,
    input: &str,
) -> Result<()> {
    let final_input = if repl_session.get_pending_resume_session() {
        info!("Pending resumed session, prepend the history");
        format!(
            "user_chat_history - {}\nUser question- {}",
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
    send_to_pi(pi_stdin, payload)
}

async fn handle_input_commands(
    cmd: String,
    repl_session: &mut ReplSession,
    db_conn: &Dbconn,
    modelname: &str,
) -> Result<()> {
    let args: Vec<&str> = cmd.split(" ").collect();
    let main_cmd = args.first().expect("Main command should be there");

    let cmd_json = json!(main_cmd);

    let command: CommandType = serde_json::from_value(cmd_json)?;
    match command {
        CommandType::Unknown => {
            println!(
                "Unknown command: /{}. Type /help for available commands.",
                cmd
            );
        }
        CommandType::Share => {
            process_share_session(db_conn, &repl_session.session_id, &args).await?;
        }
        CommandType::Sessions => {
            show_session_info(db_conn)?;
        }
        CommandType::Resume => match load_session(db_conn, &args) {
            Ok((sesh_id, turn_count, history)) => {
                repl_session.session_id = sesh_id;
                repl_session.set_turn_count(turn_count);
                repl_session.set_pending_resume_session(true);
                repl_session.set_resumed_session(history);
            }

            Err(err) => {
                println!("{}", err)
            }
        },
        CommandType::Status => {
            if let Err(err) = show_status(&repl_session.session_id, &modelname, db_conn) {
                println!("Failed to display status due to {}", err);
            }
        }
    }
    Ok(())
}

fn process_pi_turn_event(
    turn_event: PiTurnEndEvent,
    repl_session: &mut ReplSession,
    db_conn: &Dbconn,
    current_user: &crate::core::account::local::User,
    input: &str,
) -> Result<()> {
    info!("Turn end");
    // will just toggle off if was toggledon, since we dont need to compact
    // every time
    repl_session.set_pending_resume_session(false);
    repl_session.inc_turn_count();
    // on agent end create a new session entry, only for the
    // first time
    if repl_session.get_turn_count() == 1 {
        create_session(
            &db_conn.chat,
            &repl_session.session_id,
            input,
            &current_user.user_id,
        )?;
    }
    let parent_chat_id = if repl_session.get_turn_count() == 1 {
        None
    } else {
        Some(repl_session.last_chat_id.clone())
    };
    let chat_response = ChatResponse {
        input: input.to_owned(),
        session_id: repl_session.session_id.clone(),
        role: Role::User,
        code: None,
        prev_response_id: None,
        parent_chat_id,
        metrics: None,
        model_used: repl_session.current_modelname.clone(),
    };
    let prompt_chat = save_chat(&db_conn.chat, &current_user, chat_response)?;
    repl_session.last_chat_id = prompt_chat.id;
    //TODO: Refactor this..
    if turn_event.message.role == "assistant" {
        let content = turn_event.message.content;
        let full_content = content.iter().fold(String::new(), |mut acc, x| {
            if x.r#type == "thinking" {
                acc.push_str(&x.thinking.clone().unwrap_or("".to_owned()));
                acc
            } else if x.r#type == "text" {
                acc.push_str(&x.text.clone().unwrap_or("".to_owned()));
                acc
            } else {
                acc.push_str("");
                acc
            }
        });
        let chat_response = ChatResponse {
            input: full_content,
            session_id: repl_session.session_id.clone(),
            role: Role::Assistant,
            code: None,
            prev_response_id: None,
            parent_chat_id: Some(repl_session.last_chat_id.clone()),
            metrics: None,
            model_used: repl_session.current_modelname.clone(),
        };
        let chat = save_chat(&db_conn.chat, &current_user, chat_response)?;
        repl_session.last_chat_id = chat.id;
    } else {
        info!("Not handling {} role now", turn_event.message.role);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chats::create_session;
    use rusqlite::Connection;

    #[test]
    fn status_lines_show_defaults_without_session_or_atproto_login() {
        let lines = build_status_lines("/tmp/tiles", None, "test-model", None);

        assert_eq!(
            lines,
            vec![
                "Session:          \tSession not started yet",
                "Model:            \ttest-model",
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
        let lines = build_status_lines("/work/tiles", session, "llama", atproto_login);

        assert!(lines[0].starts_with("Session:          \t"));
        assert!(lines[0].contains("First chat"));
        assert!(lines[0].contains("session-1"));
        assert_eq!(lines[1], "Model:            \tllama");
        assert_eq!(lines[2], "Working Directory:\t/work/tiles");
        assert!(lines[3].starts_with("ATProto:          \t"));
        assert!(lines[3].contains("@"));
        assert!(lines[3].contains("alice.test"));
        assert!(lines[3].contains("did:plc:alice"));
    }

    fn setup_db_conn() -> Dbconn {
        Dbconn {
            chat: setup_chat_db(),
            common: setup_common_db(),
        }
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
