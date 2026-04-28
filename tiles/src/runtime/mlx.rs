use crate::core::account::atproto::share_session;
use crate::core::account::local::get_current_user;
use crate::core::chats::{
    Message, create_session, fetch_chats_by_session_id, fetch_sessions, save_chat,
};
use crate::core::storage::db::Dbconn;
use crate::runtime::RunArgs;
use crate::utils::config::{
    ConfigProvider, DefaultProvider, create_pi_provider_config, get_memory_path, get_model_cache,
};
use crate::utils::hf_model_downloader::*;
use anyhow::{Context, Result, anyhow};
use atrium_api::types::string::Datetime;
use log::info;
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
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command};
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
pub struct MLXRuntime {}

impl MLXRuntime {}

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
    r#type: String,
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
    text: String,
}

impl Default for MLXRuntime {
    fn default() -> Self {
        Self::new()
    }
}

const PY_PORT: u32 = 6969;

impl MLXRuntime {
    pub fn new() -> Self {
        MLXRuntime {}
    }

    pub async fn run(&self, run_args: super::RunArgs, db_conn: &Dbconn) -> Result<()> {
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

        run_model_with_server(self, modelfile, default_modelfile, &run_args, db_conn).await
    }

    #[allow(clippy::zombie_processes)]
    pub async fn start_server_daemon(&self) -> Result<()> {
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

        std::fs::write(pid_file, child.id().to_string()).unwrap();
        println!("Server started with PID {}", child.id());
        Ok(())
    }

    pub async fn stop_server_daemon(&self) -> Result<()> {
        if (ping().await).is_err() {
            println!("Server is not running");
            return Ok(());
        }
        let pid_file = DefaultProvider.get_config_dir()?.join("server.pid");

        if !pid_file.exists() {
            eprintln!("server pid doesnt exist");
            return Ok(());
        }

        let pid = std::fs::read_to_string(&pid_file).unwrap();
        Command::new("kill").arg(pid.trim()).status().unwrap();
        std::fs::remove_file(pid_file).unwrap();
        println!("Server stopped.");
        Ok(())
    }
}

struct TilesHinter;

impl Hinter for TilesHinter {
    type Hint = String;

    fn hint(&self, line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<Self::Hint> {
        if line.is_empty() {
            Some("Send a message (/help for help)".to_string())
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
    #[serde(rename = "get_state")]
    State,
    #[serde(rename = "share")]
    Share,
    #[serde(rename = "list-sessions")]
    ListSessions,
    #[serde(rename = "load-session")]
    LoadSession,
    #[serde(other)]
    Unknown,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SharedSession {
    #[serde(rename = "$type")]
    r#type: String,
    session_id: String,
    name: String,
    contents: Vec<SharedContent>,
    created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SharedContent {
    role: Role,
    content: String,
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
    let help_list = vec![
        ("status", "Show the current session state"),
        ("list-sessions", "List available sessions"),
        (
            "share",
            "Create a shareable link for currently running session",
        ),
        (
            "load-session <sessionId>",
            "Loads and resume the given session",
        ),
        (
            "share",
            "Create a shareable link for currently running session",
        ),
        (
            "share <sessionId>",
            "Create a shareable link for given sessionId",
        ),
        ("help", "Show this help message"),
        ("bye", "Exit the REPL"),
    ];

    // finding the length of the longest command, for padding purposes
    let max_length = help_list
        .iter()
        .fold(0, |acc, x| if x.0.len() > acc { x.0.len() } else { acc });

    println!("Available Commands:");

    for help in help_list {
        let final_str = format!(
            " /{}{}\t\t{}",
            help.0,
            " ".repeat(max_length - help.0.len()),
            help.1
        );

        println!("{}", final_str);
    }

    println!();

    println!("\nDocumentation: https://tiles.run/book");

    println!("Report issues: https://github.com/tilesprivacy/tiles/issues");
    println!();
}

async fn run_model_with_server(
    mlx_runtime: &MLXRuntime,
    modelfile: Modelfile,
    default_modelfile: Modelfile,
    run_args: &RunArgs,
    db_conn: &Dbconn,
) -> Result<()> {
    if !cfg!(debug_assertions) {
        let _ = mlx_runtime.start_server_daemon().await.inspect_err(|e| {
            eprintln!("Failed to start daemon server due to {:?}", e);
        });
        let _ = wait_until_server_is_up().await;
    }
    // loading the model from mem-agent via daemon server
    let memory_path = get_memory_path().context("Setting/Retrieving memory_path failed")?;
    match load_model(&modelfile, &default_modelfile, &memory_path, 0).await {
        Ok(_) => start_repl(mlx_runtime, &modelfile, run_args, db_conn).await?,
        Err(err) => return Err(anyhow::anyhow!(err)),
    }
    Ok(())
}

async fn start_repl(
    mlx_runtime: &MLXRuntime,
    modelfile: &Modelfile,
    _run_args: &RunArgs,
    db_conn: &Dbconn,
) -> Result<()> {
    let modelname = modelfile
        .from
        .clone()
        .ok_or_else(|| anyhow!("Error getting FROM from modelfile due to"))?;

    println!("Running {} in interactive mode", modelname);
    let current_user = get_current_user(&db_conn.common)?;

    let config = Config::builder().auto_add_history(true).build();
    let mut editor = Editor::<TilesHinter, DefaultHistory>::with_config(config).unwrap();
    editor.set_helper(Some(TilesHinter));

    let mut pi_process = start_pi_rpc(&modelname)?;
    let mut session_id = String::new();
    let pi_stdin = pi_process.stdin.as_mut().unwrap();
    let mut stdout = pi_process.stdout.take().expect("stdout");
    let inti_cmd_payload = get_command_payload(CommandType::State);
    send_to_pi(pi_stdin, inti_cmd_payload).inspect_err(|_e| eprintln!("send pi failed"))?;

    //TODO: Refactor session_id fetching
    let mut pi_session_state = String::new();
    let mut reader = BufReader::new(&mut stdout);
    let _ = reader
        .read_line(&mut pi_session_state)
        .context("Failed reading pi session state")?;
    let response: PiResponse = serde_json::from_str(&pi_session_state)?;
    if let PiResponse::Response(msg) = response {
        let state: GetStateData =
            serde_json::from_value(msg.data.expect("get state parsing failed"))?;
        session_id = state.session_id;
    }
    let mut session_turn_count = 0;
    loop {
        let readline = editor.readline(">>> ");
        let input = match readline {
            Ok(line) => line.trim().to_string(),
            Err(_) => {
                //TODO: Panic when entering another prompt after ctr-l C
                // called `Result::unwrap()` on an `Err` value: Os { code: 32, kind: BrokenPipe, message: "Broken pipe" }
                //
                // User pressed Ctrl+C or Ctrl+D
                let end_payload = json!({
                    "type": "abort",
                });
                let payload_str = format!("{}\n", serde_json::to_string(&end_payload)?);
                pi_stdin.write_all(payload_str.as_bytes())?;
                pi_stdin.flush()?;
                println!("Exiting interactive mode");
                if !cfg!(debug_assertions) {
                    let _res = mlx_runtime.stop_server_daemon().await;
                }
                break;
            }
        };

        if input.is_empty() {
            continue;
        }
        match handle_input(&input) {
            InputType::Skip => continue,
            InputType::Exit => {
                let end_payload = json!({
                    "type": "abort",
                });
                let payload_str = format!("{}\n", serde_json::to_string(&end_payload)?);
                pi_stdin.write_all(payload_str.as_bytes())?;
                pi_stdin.flush()?;
                println!("Exiting interactive mode");
                if !cfg!(debug_assertions) {
                    let _res = mlx_runtime.stop_server_daemon().await;
                }
                break;
            }
            InputType::Prompt => {
                let payload = json!({
                    "type": "prompt",
                    "message": input
                });
                send_to_pi(pi_stdin, payload)?;
            }
            InputType::Command(cmd) => {
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
                        continue;
                    }
                    CommandType::Share => {
                        process_share_session(db_conn, &session_id, &args).await?;
                        continue;
                    }
                    CommandType::ListSessions => {
                        show_session_info(db_conn)?;
                        continue;
                    }
                    CommandType::LoadSession => {
                        match load_session(db_conn, &args) {
                            Ok((sesh_id, turn_count)) => {
                                session_id = sesh_id;
                                session_turn_count = turn_count;
                            }
                            Err(err) => {
                                println!("{}", err)
                            }
                        }
                        continue;
                    }
                    cmd_type => {
                        let payload = get_command_payload(cmd_type);
                        send_to_pi(pi_stdin, payload)
                            .inspect_err(|_e| eprintln!("send pi failed"))?;
                    }
                }
            }
        }

        let reader = BufReader::new(&mut stdout);
        let mut last_chat_id: String = "".to_owned();
        for line in reader.lines() {
            //TODO: handle the unwrap
            let line = line?;
            let response: PiResponse = serde_json::from_str(&line)?;

            match response {
                PiResponse::AgentStart => {}
                PiResponse::MessageUpdate(msg_update) => {
                    if msg_update.assistant_message_event.r#type == "text_delta"
                        && msg_update.assistant_message_event.delta.is_some()
                    {
                        // TODO: Can we remove the unwrap
                        print!("{}", msg_update.assistant_message_event.delta.unwrap());
                        // TODO: maybe can optimize check print! doc
                        use std::io::Write;
                        std::io::stdout().flush().ok();
                    }
                }
                PiResponse::AgentEnd => {
                    break;
                }
                PiResponse::TurnEnd(turn_event) => {
                    session_turn_count += 1;

                    // on agent end create a new session entry, only for the
                    // first time
                    if session_turn_count == 1 {
                        info!("Created session {}", session_id);
                        create_session(&db_conn.chat, &session_id, &input, &current_user.user_id)?;
                    }
                    let parent_chat_id = if session_turn_count == 1 {
                        None
                    } else {
                        Some(last_chat_id.clone())
                    };
                    let chat_response = ChatResponse {
                        input: input.clone(),
                        session_id: session_id.clone(),
                        role: Role::User,
                        code: None,
                        prev_response_id: None,
                        parent_chat_id,
                        metrics: None,
                    };
                    let prompt_chat = save_chat(&db_conn.chat, &current_user, chat_response)?;
                    last_chat_id = prompt_chat.id;
                    if turn_event.message.role == "assistant" {
                        let mut content = turn_event.message.content;
                        if let Some(msg) = content.pop() {
                            let chat_response = ChatResponse {
                                input: msg.text.clone(),
                                session_id: session_id.clone(),
                                role: Role::Assistant,
                                code: None,
                                prev_response_id: None,
                                parent_chat_id: Some(last_chat_id.clone()),
                                metrics: None,
                            };
                            let chat = save_chat(&db_conn.chat, &current_user, chat_response)?;
                            last_chat_id = chat.id;
                        }
                    } else {
                        info!("Not handling {} role now", turn_event.message.role);
                    }
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
                    // Not handling now
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

#[allow(dead_code)]
fn extract_python(content: &str) -> String {
    if content.contains("<python>") && content.contains("</python>") {
        let list_a = content.split("<python>").collect::<Vec<&str>>();
        let list_b = list_a[1].split("</python>").collect::<Vec<&str>>();
        list_b[0].to_owned()
    } else {
        "".to_owned()
    }
}

async fn wait_until_server_is_up() {
    loop {
        match ping().await {
            Ok(()) => {
                break;
            }
            Err(_err) => {
                println!("tiling...");
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

//TODO: Deprecated if not needed
#[allow(dead_code)]
fn create_chat_input(input: &str, prompt: &str, conversations: &[Message]) -> Vec<Message> {
    let dev_msg = Message {
        r#type: "message".to_owned(),
        role: Role::Developer,
        content: String::from(prompt),
    };

    let input = Message {
        r#type: "message".to_owned(),
        role: Role::User,
        content: String::from(input),
    };

    let last_n = if conversations.len() < 10 {
        conversations
    } else {
        &conversations[conversations.len() - 10..]
    };

    if !conversations.is_empty() {
        let mut convo: Vec<Message> = vec![];
        convo.push(dev_msg);
        convo.append(&mut last_n.to_vec());
        convo.push(input);
        convo
    } else {
        vec![dev_msg, input]
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
fn start_pi_rpc(model_name: &str) -> Result<Child> {
    let tiles_lib_dir = DefaultProvider.get_lib_dir()?;
    let user_data_dir = DefaultProvider.get_user_data_dir()?;
    let pi_agent_dir = user_data_dir.join("pi/agent/");
    std::fs::create_dir_all(&pi_agent_dir).context("Failed to create Pi agent directory")?;

    let provider_config_file_path = pi_agent_dir.join("models.json");
    let endpoint_url = format!("http://127.0.0.1:{}/v1", PY_PORT);
    let model_config = create_pi_provider_config(model_name, &endpoint_url)?;

    fs::write(provider_config_file_path, model_config)?;
    let pi_exec_path = tiles_lib_dir.join("pi/pi");

    let pi_process = Command::new(pi_exec_path)
        .arg("--mode")
        .arg("rpc")
        // .arg("--no-session")
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
        CommandType::State => {
            json!({
                "type": "get_state",
            })
        }
        // catch-all cases are where prolly its not a Pi command
        _ => json!([]),
    }
}

fn process_command(cmd: CommandType, data: Option<Value>) -> Result<()> {
    match cmd {
        CommandType::Unknown => (),
        CommandType::State => {
            let state: GetStateData = serde_json::from_value(data.unwrap())?;
            println!("{:?}", state);
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
        // catch-all cases are non-Pi commands
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

    let session = &delta_chats.sessions[0];

    let mut shared_contents: Vec<SharedContent> = vec![];
    for chat in delta_chats.chats {
        shared_contents.push(SharedContent {
            role: chat.role,
            content: chat.content,
        });
    }

    let shared_sessions = SharedSession {
        r#type: "run.tiles.session".to_string(),
        session_id: session_id.to_string(),
        name: session.name.clone(),
        contents: shared_contents,
        created_at: Datetime::now().as_str().to_string(),
    };

    share_session(&conn.common, shared_sessions).await?;
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
fn load_session(db_conn: &Dbconn, args: &[&str]) -> Result<(String, usize)> {
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

    for chat in &delta_chats.chats {
        println!("{}", chat.content);
    }

    Ok((session_id.to_string(), delta_chats.chats.len()))
}
