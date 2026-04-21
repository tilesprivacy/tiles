use crate::core::accounts::{User, get_current_user};
use crate::core::chats::{Message, save_chat};
use crate::core::storage::db::Dbconn;
use crate::runtime::RunArgs;
use crate::utils::config::{
    ConfigProvider, DefaultProvider, get_memory_path, get_model_cache, update_current_model,
};
use crate::utils::hf_model_downloader::*;
use anyhow::{Context, Result, anyhow};
use futures_util::StreamExt;
use owo_colors::OwoColorize;
use reqwest::{Client, StatusCode};
use rusqlite::Connection;
use rustyline::completion::Completer;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{Config, Editor, Helper};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Command};
use std::process::{ChildStdin, Stdio};
use std::rc::Rc;
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
    // think: String,
    pub reply: String,
    pub code: String,
    pub prev_response_id: String,
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

impl Default for MLXRuntime {
    fn default() -> Self {
        Self::new()
    }
}

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
    #[serde(other)]
    Unknown,
}
fn handle_input(input: &str, modelname: &str) -> InputType {
    if let Some(cmd) = input.strip_prefix('/') {
        match cmd {
            "help" | "?" => {
                show_help(modelname);
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

fn show_help(model_name: &str) {
    let _ = model_name;

    println!("Available Commands:");
    println!("  /state      Show the current session state");
    println!("  /help       Show this help message");
    println!("  /bye        Exit the REPL");
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
    run_args: &RunArgs,
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
    let mut g_reply: String = "".to_owned();
    let mut prev_response_id: String = String::from("");

    let mut conversations: Vec<Message> = vec![];

    let mut pi_process = start_pi_rpc()?;

    let pi_stdin = pi_process.stdin.as_mut().unwrap();
    let mut stdout = pi_process.stdout.take().expect("stdout");
    // let mut stdout: Cell<ChildStdout> = Cell::new();
    loop {
        let readline = editor.readline(">>> ");
        let input = match readline {
            Ok(line) => line.trim().to_string(),
            Err(_) => {
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
        match handle_input(&input, modelname.as_str()) {
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
                let cmd_json = json!(cmd);
                let command: CommandType = serde_json::from_value(cmd_json)?;
                match command {
                    CommandType::Unknown => {
                        println!(
                            "Unknown command: /{}. Type /help for available commands.",
                            cmd
                        );
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

        let mut remaining_count = run_args.relay_count;
        let mut python_code: String = "".to_owned();
        let mut bench_metrics: BenchmarkMetrics = BenchmarkMetrics {
            ttft_ms: 0.0,
            total_tokens: 0,
            tokens_per_second: 0.0,
            total_latency_s: 0.0,
        };
        let mut is_agent_streaming: bool = false;
        let reader = BufReader::new(&mut stdout);

        for line in reader.lines() {
            //TODO: handle the unwrap
            let line = line?;
            let response: PiResponse = serde_json::from_str(&line)?;

            match response {
                PiResponse::AgentStart => {
                    // agent streaming started
                    is_agent_streaming = true
                }
                PiResponse::MessageUpdate(msg_update) => {
                    if msg_update.assistant_message_event.r#type == "text_delta"
                        && msg_update.assistant_message_event.delta.is_some()
                    {
                        print!("{}", msg_update.assistant_message_event.delta.unwrap());
                        // TODO: maybe can optimize check print! doc
                        use std::io::Write;
                        std::io::stdout().flush().ok();
                    }
                }
                PiResponse::AgentEnd => {
                    // agent streaming stopeed
                    is_agent_streaming = false;
                    break;
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
        // loop {
        //     if remaining_count > 0 {
        //         let chat_start = remaining_count == run_args.relay_count;

        //         match chat(
        //             &input,
        //             modelfile,
        //             chat_start,
        //             &python_code,
        //             &g_reply,
        //             run_args,
        //             &prev_response_id,
        //             &db_conn.chat,
        //             &current_user,
        //             &conversations,
        //         )
        //         .await
        //         {
        //             Ok(response) => {
        //                 if response.reply.is_empty() {
        //                     if !response.code.is_empty() {
        //                         python_code = response.code;
        //                     }
        //                     if let Some(metrics) = response.metrics {
        //                         bench_metrics.update(metrics);
        //                     }
        //                     remaining_count -= 1;
        //                 } else {
        //                     g_reply = response.reply.clone();
        //                     if run_args.memory {
        //                         println!("\n{}", response.reply.trim());
        //                     } else {
        //                         prev_response_id = response.prev_response_id.clone();
        //                         println!("\n");
        //                     }
        //                     conversations.push(Message {
        //                         r#type: String::from("message"),
        //                         role: Role::User,
        //                         content: input,
        //                     });
        //                     conversations.push(Message {
        //                         r#type: String::from("message"),
        //                         role: Role::Assistant,
        //                         content: g_reply.clone(),
        //                     });

        //                     save_chat(&db_conn.chat, &current_user, &g_reply, Some(&response))?;
        //                     // Display benchmark metrics if available
        //                     if let Some(metrics) = response.metrics {
        //                         bench_metrics.update(metrics);
        //                         println!(
        //                             "{}",
        //                             format!(
        //                                 "\n{} {:.1} tok/s | {} tokens | {:.0}s TTFT",
        //                                 "💡".yellow(),
        //                                 bench_metrics.total_tokens as f64
        //                                     / bench_metrics.total_latency_s,
        //                                 bench_metrics.total_tokens,
        //                                 bench_metrics.ttft_ms / 1000.0
        //                             )
        //                             .dimmed()
        //                         );
        //                     }

        //                     break;
        //                 }
        //             }
        //             Err(err) => {
        //                 // if out of relay count, then clear the global_reply and ready for next fresh prompt
        //                 println!("{:?}", err);
        //                 g_reply.clear();
        //                 break;
        //             }
        //         }
        //     }
        // }
        // if g_reply.is_empty() {
        //     println!("\nNo reply, try another prompt");
        // }
    }
    Ok(())
}

pub async fn ping() -> Result<()> {
    let client = Client::new();
    let res = client.get("http://127.0.0.1:6969/ping").send().await;

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

//TODO: Have 2 separate chat functions for memory and non-memory
#[allow(clippy::too_many_arguments)]
async fn chat(
    input: &str,
    modelfile: &Modelfile,
    chat_start: bool,
    python_code: &str,
    g_reply: &str,
    run_args: &RunArgs,
    prev_response_id: &str,
    conn: &Connection,
    user: &User,
    conversations: &[Message],
) -> Result<ChatResponse> {
    let client = Client::new();
    let modelname = modelfile
        .from
        .clone()
        .ok_or_else(|| anyhow!("Failed to get model name"))?;
    let prompt = modelfile.system.clone().unwrap_or("".to_owned());
    let convo_input = create_chat_input(input, prompt.as_str(), conversations);
    let body = json!({
        "model": modelname,
        "input": convo_input,
        "reasoning": {"effort": "medium"},
        "chat_start": chat_start,
        "stream": true,
        "previous_response_id": prev_response_id,
        "python_code": python_code,
        "messages": [{"role": "assistant", "content": g_reply}, {"role": "user", "content": input}]
    });

    let memory_body = json!({
        "model": modelname,
        "input": input,
        "chat_start": chat_start,
        "stream": true,
        "python_code": python_code,
        "messages": [{"role": "assistant", "content": g_reply}, {"role": "user", "content": input}]

    });
    let res = if run_args.memory {
        let api_url = "http://127.0.0.1:6969/v1/chat/completions";
        client.post(api_url).json(&memory_body).send().await?
    } else {
        let api_url = "http://127.0.0.1:6969/v1/responses";
        client.post(api_url).json(&body).send().await?
    };

    let chat = save_chat(conn, user, input, None)?;
    let mut stream = res.bytes_stream();
    let mut accumulated = String::new();
    let mut metrics: Option<BenchmarkMetrics> = None;
    let mut is_answer_start = false;
    let mut prev_response_id: String = String::from("");
    let mut output_completed: bool = false;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let s = String::from_utf8_lossy(&chunk);
        for line in s.lines() {
            if !line.starts_with("data: ") {
                continue;
            }

            let data = line.trim_start_matches("data: ");

            if data == "[DONE]" {
                let mut chat_resp = convert_to_chat_response(
                    &accumulated,
                    run_args.memory,
                    prev_response_id,
                    metrics,
                );
                chat_resp.parent_chat_id = Some(chat.id);
                return Ok(chat_resp);
            }

            //TODO: This will break if we ask the model to give an essay and all
            let v: Value = serde_json::from_str(data).unwrap();
            // Check for metrics in the response
            if let Some(metrics_obj) = v.get("metrics") {
                metrics = serde_json::from_value(metrics_obj.clone()).ok();
            }
            let model_text: Option<&str> = if run_args.memory {
                v["choices"][0]["delta"]["content"].as_str()
            } else {
                prev_response_id = serde_json::to_string(&v["id"])?
                    .trim_matches('\"')
                    .to_owned();

                if serde_json::to_string(&v["status"])?.contains("completed") {
                    output_completed = true;
                }

                v["output"][0]["content"][0]["text"].as_str()
            };

            if let Some(delta) = model_text {
                if !run_args.memory {
                    if delta.contains("**[Answer]**") {
                        is_answer_start = true
                    }
                    if !output_completed {
                        accumulated.push_str(delta);
                        if !is_answer_start {
                            print!("{}", delta.dimmed());
                        } else {
                            print!("{}", delta);
                        };
                    }
                } else {
                    accumulated.push_str(delta);
                }
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }
    }

    Err(anyhow!("Result failed"))
}

fn convert_to_chat_response(
    content: &str,
    memory_mode: bool,
    prev_response_id: String,
    metrics: Option<BenchmarkMetrics>,
) -> ChatResponse {
    ChatResponse {
        reply: extract_reply(content, memory_mode),
        code: extract_python(content),
        prev_response_id,
        metrics,
        parent_chat_id: None,
    }
}

fn extract_reply(content: &str, memory_mode: bool) -> String {
    if !memory_mode && content.contains("**[Answer]**") {
        let list_a = content.split("**[Answer]**").collect::<Vec<&str>>();
        list_a[1].to_owned()
    } else if !memory_mode {
        content.to_owned()
    } else if content.contains("<reply>") && content.contains("</reply>") {
        let list_a = content.split("<reply>").collect::<Vec<&str>>();
        let list_b = list_a[1].split("</reply>").collect::<Vec<&str>>();
        list_b[0].to_owned()
    } else {
        "".to_owned()
    }
}

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

pub fn start_pi_rpc() -> Result<Child> {
    let mut pi_dir = DefaultProvider.get_lib_dir()?;
    let user_data_dir = DefaultProvider.get_user_data_dir()?;
    let pi_agent_dir = user_data_dir.join("pi/agent");
    std::fs::create_dir_all(&pi_agent_dir).context("Failed to create pi_agent_dir")?;
    pi_dir = pi_dir.join("pi");
    let pi_exec_path = pi_dir.join("pi");
    let pi_process = Command::new(pi_exec_path)
        .arg("--mode")
        .arg("rpc")
        .arg("--no-session")
        .env("PI_CODING_AGENT_DIR", pi_agent_dir)
        .env("PI_OFFLINE", "true")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to PI");

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
    }
    Ok(())
}
