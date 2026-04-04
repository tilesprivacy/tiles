// #![warn(clippy::pedantic)]

use std::error::Error;

use clap::{Args, Parser, Subcommand};
use tiles::{
    core::{
        self,
        network::{link, sync},
    },
    daemon::{start_cmd, start_server, stop_cmd},
    runtime::{RunArgs, build_runtime, mlx::start_pi_rpc},
    utils::installer,
};

use crate::commands::{show_peers, unlink_peer};

mod commands;
#[derive(Debug, Parser)]
#[command(name = "tiles")]
#[command(version, about = "Your private and secure AI assistant for everyday use.", long_about = None, after_help = "Documentation: https://tiles.run/book\nReport issues: https://github.com/tilesprivacy/tiles/issues")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[command(flatten)]
    flags: RunFlags,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Runs the given Modelfile (runs the default model if none passed)
    Run {
        /// Path to the Modelfile (uses default model if not provided)
        modelfile_path: Option<String>,

        #[command(flatten)]
        flags: RunFlags,
    },

    /// Configure your data
    Data(DataArgs),

    /// Checks the status of dependencies
    Health,

    /// start or stop the daemon server
    Server(ServerArgs),

    /// Optimize the SYSTEM prompt in a Modelfile
    Optimize {
        /// Path to the Modelfile to optimize
        modelfile_path: String,

        /// Path to the training data (JSON)
        #[arg(short, long)]
        data: Option<String>,

        /// Model to use for optimization (e.g., openai:gpt-4o-mini, ollama:llama3)
        #[arg(long, default_value = "openai:gpt-4o-mini")]
        model: String,
    },
    /// Manage user account
    Account(AccountArgs),

    /// Update Tiles to latest version
    Update,

    /// Daemon configurations
    Daemon(DaemonArgs),

    /// Link with other devices p2p
    Link(LinkArgs),

    /// Syncs the chats to peers
    Sync {
        /// The DID of the peer you want to sync
        did: Option<String>,
    },
    Pi,
}

#[derive(Debug, Args)]
struct RunFlags {
    /// Max times cli communicates with the model until it gets a proper reply for a user prompt
    #[arg(short = 'r', long, default_value_t = 10)]
    relay_count: u32,

    /// Switches the mode to memory, used for interacting with memory models.
    #[arg(short = 'm', long)]
    memory: bool,
    // Future flags go here:
    // #[arg(long, default_value_t = 6969)]
    // port: u16,

    // Don't go into the repl
    #[arg(short = 'x', long)]
    no_repl: bool,

    // Use PI repl instead of Tiles
    #[arg(short = 'p', long)]
    pi: bool,
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
struct ServerArgs {
    #[command(subcommand)]
    command: Option<ServerCommands>,
}

#[derive(Debug, Subcommand)]
enum ServerCommands {
    /// Start the py server as a daemon
    Start,

    /// Stops the daemon py server
    Stop,
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
struct DataArgs {
    #[command(subcommand)]
    command: DataCommands,
}
#[derive(Debug, Subcommand)]
enum DataCommands {
    /// Set Path for the user data
    SetPath { path: String },
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
struct AccountArgs {
    #[command(subcommand)]
    command: Option<AccountCommands>,
}

#[derive(Debug, Subcommand)]
enum AccountCommands {
    /// Creates a local root account
    Create { nickname: Option<String> },

    /// Sets nickname to local root account
    SetNickname { nickname: String },
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
struct DaemonArgs {
    #[command(subcommand)]
    command: Option<DaemonCommands>,
}

#[derive(Debug, Subcommand)]
enum DaemonCommands {
    /// Start the daemon
    Start { port: Option<u32> },

    /// Stops the daemon
    Stop,
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
struct LinkArgs {
    #[command(subcommand)]
    command: LinkCommands,
}

#[derive(Debug, Subcommand)]
enum LinkCommands {
    /// Produce link ticket and wait or send link request with ticket
    Enable {
        ticket: Option<String>,
    },

    // Unlink give device
    Disable {
        did: String,
    },
    /// Start the daemon
    ListPeers,
}
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    build_logger();
    let cli = Cli::parse();
    let runtime = build_runtime();
    let db_conn = core::init()?;

    match cli.command {
        None => {
            // Running tiles without subcommand - launch default model with flags
            let run_args = RunArgs {
                modelfile_path: None,
                relay_count: cli.flags.relay_count,
                memory: cli.flags.memory,
                pi: cli.flags.pi,
            };

            commands::run_setup_for_ftue(&run_args)
                .inspect_err(|e| eprintln!("Failed to setup Tiles due to {:?}", e))?;
            let _ = commands::try_app_update().await;

            // trying to run the tiles daemon in background concurrently
            // if !cfg!(debug_assertions) {
            let t = tokio::spawn(async move {
                let _ = start_cmd(None).await;
            });
            t.await?;
            // }
            core::init_account(&db_conn)
                .inspect_err(|e| eprintln!("Tiles core init failed due to {:?}", e))?;
            if !cli.flags.no_repl {
                commands::run(&runtime, run_args, &db_conn)
                    .await
                    .inspect_err(|e| eprintln!("Tiles failed to run due to {:?}", e))?;
            }
        }
        Some(Commands::Run {
            modelfile_path,
            flags,
        }) => {
            let run_args = RunArgs {
                modelfile_path,
                relay_count: flags.relay_count,
                memory: flags.memory,
                pi: flags.pi,
            };
            commands::run_setup_for_ftue(&run_args)
                .inspect_err(|e| eprintln!("Failed to setup Tiles due to {:?}", e))?;

            let t = tokio::spawn(async move {
                let _ = start_cmd(None).await;
            });
            t.await?;
            core::init_account(&db_conn)
                .inspect_err(|e| eprintln!("Tiles core init failed due to {:?}", e))?;
            commands::run(&runtime, run_args, &db_conn)
                .await
                .inspect_err(|e| eprintln!("Tiles failed to run due to {:?}", e))?;
        }
        Some(Commands::Health) => {
            commands::check_health().await?;
        }
        Some(Commands::Server(server)) => match server.command {
            Some(ServerCommands::Start) => commands::start_server(&runtime).await,
            Some(ServerCommands::Stop) => commands::stop_server(&runtime).await,
            _ => println!("Expected start or stop"),
        },
        Some(Commands::Data(data)) => match data.command {
            DataCommands::SetPath { path } => commands::set_data(path.as_str()),
        },
        Some(Commands::Optimize {
            modelfile_path,
            data,
            model,
        }) => {
            let modelfile = commands::optimize(&modelfile_path, data, &model).await?;
            std::fs::write(&modelfile_path, modelfile.to_string())?;
            println!("Successfully updated {}", modelfile_path);
        }
        Some(Commands::Account(account_args)) => {
            commands::run_account_commands(account_args)?;
        }
        Some(Commands::Update) => {
            println!("Checking for updates...");
            let res = installer::try_update(None)
                .await
                .inspect_err(|e| eprintln!("Failed in update process due to {:?}", e))?;
            println!("{}", res);
        }
        Some(Commands::Daemon(daemon_args)) => match daemon_args.command {
            Some(DaemonCommands::Start { port }) => start_cmd(port)
                .await
                .inspect_err(|e| eprintln!("Daemon starting failed, reason: {:?}", e))?,
            Some(DaemonCommands::Stop) => stop_cmd()
                .await
                .inspect_err(|e| eprintln!("{:?}", e))
                .inspect(|_| println!("Daemon stopped successfully"))?,
            _ => start_server(None).await?,
        },
        Some(Commands::Link(link_args)) => match link_args.command {
            LinkCommands::Enable { ticket } => link(ticket).await?,
            LinkCommands::Disable { did } => unlink_peer(&db_conn, &did)?,
            LinkCommands::ListPeers => {
                show_peers(&db_conn)?;
            }
        },
        Some(Commands::Sync { did }) => sync(did).await?,
        Some(Commands::Pi) => {
            // blah
            start_pi_rpc()?;
        }
    }
    Ok(())
}

fn build_logger() {
    if cfg!(debug_assertions) {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or("info,iroh=error,tracing=off"),
        )
        .init()
    } else {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or("error,iroh=off,tracing=off"),
        )
        .init()
    }
}
