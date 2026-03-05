use crate::core::accounts::{User, get_current_user};
use crate::core::chats::save_chat;
use crate::core::storage::db::get_db_conn;
use crate::runtime::RunArgs;
use crate::utils::config::{ConfigProvider, DefaultProvider, get_memory_path};
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
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use tilekit::modelfile::Modelfile;
use tokio::time::sleep;
use uuid::Uuid;

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

#[derive(Clone)]
pub struct ChatResponse {
    // think: String,
    pub reply: String,
    pub code: String,
    pub prev_response_id: String,
    pub parent_chat_id: Option<Uuid>,
    pub metrics: Option<BenchmarkMetrics>,
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

    pub async fn run(&self, run_args: super::RunArgs) -> Result<()> {
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

        run_model_with_server(self, modelfile, default_modelfile, &run_args)
            .await
            .inspect_err(|e| eprintln!("Failed to run the model due to {e}"))
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

enum SlashCommand {
    Continue,
    Exit,
    NotACommand,
}

fn handle_slash_command(input: &str, modelname: &str) -> SlashCommand {
    if let Some(cmd) = input.strip_prefix('/') {
        match cmd {
            "help" | "?" => {
                show_help(modelname);
                SlashCommand::Continue
            }
            "bye" => SlashCommand::Exit,
            "" => {
                println!("Empty command. Type /help for available commands.");
                SlashCommand::Continue
            }
            _ => {
                println!(
                    "Unknown command: /{}. Type /help for available commands.",
                    cmd
                );
                SlashCommand::Continue
            }
        }
    } else {
        SlashCommand::NotACommand
    }
}

fn show_help(model_name: &str) {
    let _ = model_name;

    println!("Available Commands:");
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
) -> Result<()> {
    if !cfg!(debug_assertions) {
        let _ = mlx_runtime.start_server_daemon().await.inspect_err(|e| {
            eprintln!("Failed to start daemon server due to {:?}", e);
        });
        let _ = wait_until_server_is_up().await;
    }
    // loading the model from mem-agent via daemon server
    let memory_path = get_memory_path().context("Setting/Retrieving memory_path failed")?;
    let modelname = modelfile.from.as_ref().unwrap();
    match load_model(&modelfile, &default_modelfile, &memory_path).await {
        Ok(_) => start_repl(mlx_runtime, modelname, run_args).await?,
        Err(err) => return Err(anyhow::anyhow!(err)),
    }
    Ok(())
}

async fn start_repl(mlx_runtime: &MLXRuntime, modelname: &str, run_args: &RunArgs) -> Result<()> {
    println!("Running {} in interactive mode", modelname);
    let common_db_conn = get_db_conn(crate::core::storage::db::DBTYPE::COMMON)?;
    let chat_db_conn = get_db_conn(crate::core::storage::db::DBTYPE::CHAT)?;
    let current_user = get_current_user(&common_db_conn)?;

    let config = Config::builder().auto_add_history(true).build();
    let mut editor = Editor::<TilesHinter, DefaultHistory>::with_config(config).unwrap();
    editor.set_helper(Some(TilesHinter));
    let mut g_reply: String = "".to_owned();
    let mut prev_response_id: String = String::from("");

    loop {
        let readline = editor.readline(">>> ");
        let input = match readline {
            Ok(line) => line.trim().to_string(),
            Err(_) => {
                // User pressed Ctrl+C or Ctrl+D
                println!("Exiting interactive mode");
                if !cfg!(debug_assertions) {
                    let _res = mlx_runtime.stop_server_daemon().await;
                }
                break;
            }
        };

        match handle_slash_command(&input, modelname) {
            SlashCommand::Continue => continue,
            SlashCommand::Exit => {
                println!("Exiting interactive mode");
                if !cfg!(debug_assertions) {
                    let _res = mlx_runtime.stop_server_daemon().await;
                }
                break;
            }
            SlashCommand::NotACommand => {}
        }

        if input.is_empty() {
            continue;
        }
        let mut remaining_count = run_args.relay_count;
        let mut python_code: String = "".to_owned();
        let mut bench_metrics: BenchmarkMetrics = BenchmarkMetrics {
            ttft_ms: 0.0,
            total_tokens: 0,
            tokens_per_second: 0.0,
            total_latency_s: 0.0,
        };
        loop {
            if remaining_count > 0 {
                let chat_start = remaining_count == run_args.relay_count;

                match chat(
                    &input,
                    modelname,
                    chat_start,
                    &python_code,
                    &g_reply,
                    run_args,
                    &prev_response_id,
                    &chat_db_conn,
                    &current_user,
                )
                .await
                {
                    Ok(response) => {
                        if response.reply.is_empty() {
                            if !response.code.is_empty() {
                                python_code = response.code;
                            }
                            if let Some(metrics) = response.metrics {
                                bench_metrics.update(metrics);
                            }
                            remaining_count -= 1;
                        } else {
                            g_reply = response.reply.clone();
                            if run_args.memory {
                                println!("\n{}", response.reply.trim());
                            } else {
                                prev_response_id = response.prev_response_id;
                                println!("\n");
                            }
                            // Display benchmark metrics if available
                            if let Some(metrics) = response.metrics {
                                bench_metrics.update(metrics);
                                println!(
                                    "{}",
                                    format!(
                                        "\n{} {:.1} tok/s | {} tokens | {:.0}s TTFT",
                                        "💡".yellow(),
                                        bench_metrics.total_tokens as f64
                                            / bench_metrics.total_latency_s,
                                        bench_metrics.total_tokens,
                                        bench_metrics.ttft_ms / 1000.0
                                    )
                                    .dimmed()
                                );
                            }

                            break;
                        }
                    }
                    Err(err) => {
                        // if out of relay count, then clear the global_reply and ready for next fresh prompt
                        println!("{:?}", err);
                        g_reply.clear();
                        break;
                    }
                }
            }
        }
        if g_reply.is_empty() {
            println!("\nNo reply, try another prompt");
        }
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
) -> Result<()> {
    let client = Client::new();
    let model_name = modelfile.from.clone().unwrap();
    let body = json!({
        "model": model_name,
        "memory_path": memory_path,
        "system_prompt": modelfile.system.clone().unwrap_or(default_modelfile.system.clone().unwrap_or("".to_owned()))
    });

    let res = client
        .post("http://127.0.0.1:6969/start")
        .json(&body)
        .send()
        .await?;
    match res.status() {
        StatusCode::OK => Ok(()),
        StatusCode::NOT_FOUND => {
            println!("Downloading {}\n", model_name);
            match pull_model(&model_name).await {
                Ok(_) => {
                    println!("\nDownloading completed \n");
                    Ok(())
                }
                Err(err) => Err(anyhow::anyhow!(format!("Download failed due to {:?}", err))),
            }
        }
        _ => Err(anyhow::anyhow!(format!(
            "Failed to load model {} due to {:?}",
            model_name, res
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
async fn chat(
    input: &str,
    model_name: &str,
    chat_start: bool,
    python_code: &str,
    g_reply: &str,
    run_args: &RunArgs,
    prev_response_id: &str,
    conn: &Connection,
    user: &User,
) -> Result<ChatResponse> {
    let client = Client::new();
    let body = json!({
        "model": model_name,
        "input": [{
            "type": "message",
            "role": "user",
            "content": input
        },
        {
            "type": "message",
            "role": "developer",
            "content": ""
        }],
        "reasoning": {"effort": "low"},
        "chat_start": chat_start,
        "stream": true,
        // "previous_response_id": prev_response_id,
        "python_code": python_code,
        "messages": [{"role": "assistant", "content": g_reply}, {"role": "user", "content": input}]
    });

    let memory_body = json!({
        "model": model_name,
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
                save_chat(conn, user, &accumulated, Some(&chat_resp))?;
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
