// #![warn(clippy::pedantic)]

use std::error::Error;

use clap::{Args, CommandFactory, Parser, Subcommand};
use tiles::{
    core::{
        self,
        account::atproto::{login, logout},
        network::sync,
        plugin::{self, install, uninstall},
        service,
    },
    daemon::{
        remote_status, share_remote_link, start_cmd, start_server, stop_cmd, unshare_remote_link,
    },
    repl::{self, RunArgs},
    utils::{config::LlamaConfig, installer},
};

use crate::commands::{
    add_link, create_link, set_inference_config_to_daemon, show_peers, unlink_peer,
};

mod commands;

const CLI_HELP_TEMPLATE: &str = concat!(
    r#"
                               .:-::.
                       .:-+*#%@@@@@@%#**+=
               .:-=*#%@@@@@%%#***#%%@@@%=.
       .:-=+#%@@@@@%%##****#%@@@@@@@%*=
   -#%@@@@@@%#*****#%%@@@@@@%#*=-:.
 -#@%%#####%%%%@@@%%#@@@@@#.
-###%@@@@@@@@@%%%%%%@@@@@*
  ..:--=+##*+==@@@@@@@@@=
             .#@@@@@@@@:
            -@@@@@@@@#.
           +@@@@@%@@*
          #@@@@#+@@=
        .%@@@@#+@@:
       .@@@@@##@#.
        %@@@#%@*
        :@@*@@=
         -+@@:
          .-.
"#,
    "\nTiles ",
    env!("CARGO_PKG_VERSION"),
    "\nA private, collaborative AI assistant that works for you. Built with local models and AT Protocol\n\n",
    "Usage: {usage}\n\n",
    "Commands:\n\n",
    "  Getting Started\n",
    "    run       Run a Modelfile (uses the default model if none is provided)\n",
    "    help      Show this message or help for a specific command\n\n",
    "  Accounts\n",
    "    account   Manage your user account\n",
    "    at        ATProto-related commands\n",
    "    data      Configure your data and storage\n\n",
    "  Sync\n",
    "    link      Link devices via peer-to-peer\n",
    "    sync      Sync chats with peers\n",
    "    remote    Remote inference commands\n\n",
    "  System\n",
    "    update    Update Tiles to the latest version\n",
    "    uninstall Uninstall Tiles from this machine\n",
    "    health    Check the status of dependencies\n",
    "    server    Configure the inference server\n",
    "    daemon    Configure daemon behavior\n\n",
    "  Tools\n",
    "    plugin    Manage plugins such as skills, extensions etc\n\n",
    "Options:\n",
    "  -h, --help       Show help\n",
    "  -V, --version    Show version\n\n",
    "Documentation: https://tiles.run/book\n",
    "Report issues: https://github.com/tilesprivacy/tiles/issues\n"
);

#[derive(Debug, Parser)]
#[command(name = "tiles")]
#[command(
    version,
    about = "A private, collaborative AI assistant that works for you. Built with local models and AT Protocol",
    disable_help_subcommand = true,
    long_about = None,
    override_usage = "tiles [OPTIONS] [COMMAND]",
    help_template = CLI_HELP_TEMPLATE
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[command(flatten)]
    flags: RunFlags,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(flatten, next_help_heading = "Getting Started")]
    GettingStarted(GettingStartedCommands),

    #[command(flatten, next_help_heading = "Accounts")]
    Accounts(AccountCommandsGroup),

    #[command(flatten, next_help_heading = "Sync")]
    Sync(SyncCommands),

    #[command(flatten, next_help_heading = "System")]
    System(SystemCommands),

    #[command(flatten, next_help_heading = "Plugins")]
    Tools(ToolsCommandsGroup),

    Uninstall {
        #[arg(long)]
        all: bool,
    },
}

#[derive(Debug, Subcommand)]
enum GettingStartedCommands {
    /// Run a Modelfile (uses the default model if none is provided)
    Run {
        /// Path to the Modelfile (uses default model if not provided)
        modelfile_path: Option<String>,

        #[command(flatten)]
        flags: RunFlags,
    },

    /// Show this message or help for a specific command
    Help {
        /// Command to show help for
        #[arg(value_name = "COMMAND")]
        command: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum AccountCommandsGroup {
    /// Manage your user account
    Account(AccountArgs),

    /// ATProto-related commands
    At(AtArgs),

    /// Configure your data and storage
    Data(DataArgs),
}

#[derive(Debug, Subcommand)]
enum SyncCommands {
    /// Link devices via peer-to-peer
    Link(LinkArgs),

    /// Sync chats with peers
    Sync {
        /// The DID of the peer you want to sync
        did: Option<String>,
    },

    /// Remote stuff
    Remote(RemoteArgs),
}

#[derive(Debug, Subcommand)]
enum SystemCommands {
    /// Update Tiles to the latest version
    Update,

    /// Check the status of dependencies
    Health,

    /// Configure the inference server
    Server(ServerArgs),

    /// Configure daemon behavior
    Daemon(DaemonArgs),

    /// Run Tiles in the background from login
    Service(ServiceArgs),
}

#[derive(Debug, Subcommand)]
enum TilekitCommands {
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
}

#[derive(Debug, Args)]
struct RunFlags {
    /// Max times cli communicates with the model until it gets a proper reply for a user prompt
    #[arg(short = 'r', long, default_value_t = 10, hide = true)]
    relay_count: u32,

    // Future flags go here:
    // #[arg(long, default_value_t = 6969)]
    // port: u16,

    // Don't go into the repl
    #[arg(short = 'x', long, hide = true)]
    no_repl: bool,

    /// Context window for local llama.cpp inference
    #[arg(long)]
    context_length: Option<u32>,

    /// Number of model layers to offload to GPU for llama.cpp
    #[arg(long)]
    gpu_layers: Option<i32>,

    /// Offload K/Q/V attention operations for llama.cpp
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    offload_kqv: Option<bool>,

    /// Prompt processing batch size for llama.cpp
    #[arg(long)]
    batch_size: Option<u32>,

    /// n_cpu_more, default 12 for macos
    #[arg(long)]
    n_cpu_moe: Option<u32>,

    /// flash, not the DC one
    #[arg(long, default_missing_value = "true")]
    flash_attn: Option<bool>,

    /// no_mmap, default true
    #[arg(long)]
    no_mmap: Option<bool>,

    /// Enable MTP speculative decoding (requires an MTP head GGUF next to the model)
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    mtp: Option<bool>,

    /// Connect to remote inference with the ticket
    #[arg(long)]
    remote: Option<String>,
}

fn llama_config_from_flags(flags: &RunFlags) -> Option<LlamaConfig> {
    let config = LlamaConfig {
        context_length: flags.context_length,
        gpu_layers: flags.gpu_layers,
        offload_kqv: flags.offload_kqv,
        batch_size: flags.batch_size,
        mtp: flags.mtp,
        n_cpu_moe: flags.n_cpu_moe,
        flash_attn: flags.flash_attn,
        no_mmap: flags.no_mmap,
    };

    Some(config)
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
struct ServerArgs {
    #[command(subcommand)]
    command: ServerCommands,
}

#[derive(Debug, Subcommand)]
enum ServerCommands {
    /// Start the inference
    Start,

    /// Stops the inference
    Stop,

    /// configure the inference to run in background
    Daemon { flag: Option<bool> },
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

    /// Serve without launching the menu bar app
    #[arg(long)]
    no_ui: bool,
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
struct ServiceArgs {
    #[command(subcommand)]
    command: ServiceCommands,
}

#[derive(Debug, Subcommand)]
enum ServiceCommands {
    /// Start Tiles automatically at login
    Install,

    /// Stop starting Tiles at login
    Uninstall,

    /// Show whether the service is installed and running
    Status,

    /// Start the service now
    Start,

    /// Stop the service now
    Stop,
}

#[derive(Debug, Subcommand)]
enum ToolsCommandsGroup {
    /// Manage Plugins
    Plugin(PluginArgs),
}
#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
struct PluginArgs {
    #[command(subcommand)]
    command: PluginCommands,
}

#[derive(Debug, Subcommand)]
enum PluginCommands {
    List,
    /// Install a plugin
    Install {
        path: String,
    },

    /// Uninstall a plugin
    Uninstall {
        name: String,
    },
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
    /// Creates an authorization token for linking
    Create {
        peer_did: Option<String>,
    },

    /// Adds the sync authorization token from peer
    Add {
        token: String,
    },
    /// List the linked peers
    ListPeers,

    // Revokes the given peer
    Revoke {
        peer_did: String,
    },
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
struct RemoteArgs {
    #[command(subcommand)]
    command: RemoteCommands,
}

#[derive(Debug, Subcommand)]
enum RemoteCommands {
    /// Share your inference to the world
    Share,

    /// Unshare the your remote inference
    Unshare,

    // Shows the info of your inference status
    Status,
}
#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
struct AtArgs {
    #[command(subcommand)]
    command: AtCommands,
}

#[derive(Debug, Subcommand)]
enum AtCommands {
    #[command(about = "LogIn to Atproto account using handle (ex: john.bsky.team)")]
    Login { handle: String },
    #[command(about = "Log out of your Atproto account.")]
    Logout,
}
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    build_logger();
    let cli = Cli::parse();

    if let Some(Commands::Uninstall { all }) = &cli.command {
        commands::uninstall_tiles(*all).await?;
        return Ok(());
    }

    let db_conn = core::init()?;

    match cli.command {
        None => {
            // Running tiles without subcommand - launch default model with flags
            let run_args = RunArgs {
                modelfile_path: None,
                relay_count: cli.flags.relay_count,
                llama_config: llama_config_from_flags(&cli.flags),
                remote: cli.flags.remote,
            };

            commands::run_setup_for_ftue(&run_args)
                .await
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
                repl::run(run_args, &db_conn)
                    .await
                    .inspect_err(|e| eprintln!("Tiles failed to run due to {:?}", e))?;
            }
        }
        Some(Commands::GettingStarted(GettingStartedCommands::Run {
            modelfile_path,
            flags,
        })) => {
            let run_args = RunArgs {
                modelfile_path,
                relay_count: flags.relay_count,
                llama_config: llama_config_from_flags(&flags),
                remote: flags.remote,
            };
            commands::run_setup_for_ftue(&run_args)
                .await
                .inspect_err(|e| eprintln!("Failed to setup Tiles due to {:?}", e))?;

            let t = tokio::spawn(async move {
                let _ = start_cmd(None).await;
            });
            t.await?;
            core::init_account(&db_conn)
                .inspect_err(|e| eprintln!("Tiles core init failed due to {:?}", e))?;
            repl::run(run_args, &db_conn)
                .await
                .inspect_err(|e| eprintln!("Tiles failed to run due to {:?}", e))?;
        }
        Some(Commands::GettingStarted(GettingStartedCommands::Help { command })) => {
            print_help_for_command(&command)?;
        }
        Some(Commands::System(SystemCommands::Health)) => {
            commands::check_health().await?;
        }
        Some(Commands::System(SystemCommands::Server(server))) => match server.command {
            ServerCommands::Start => commands::start_server().await,
            ServerCommands::Stop => commands::stop_server().await,
            ServerCommands::Daemon { flag } => {
                set_inference_config_to_daemon(flag.unwrap_or(false))?
            }
        },
        Some(Commands::Accounts(AccountCommandsGroup::Data(data))) => match data.command {
            DataCommands::SetPath { path } => commands::set_data(path.as_str()),
        },
        Some(Commands::Accounts(AccountCommandsGroup::Account(account_args))) => {
            commands::run_account_commands(account_args).await?;
        }
        Some(Commands::System(SystemCommands::Update)) => {
            println!("Checking for updates...");
            let res = installer::try_update(None)
                .await
                .inspect_err(|e| eprintln!("Failed in update process due to {:?}", e))?;
            println!("{}", res);
        }
        Some(Commands::System(SystemCommands::Daemon(daemon_args))) => match daemon_args.command {
            Some(DaemonCommands::Start { port }) => start_cmd(port)
                .await
                .inspect_err(|e| eprintln!("Daemon starting failed, reason: {:?}", e))?,
            Some(DaemonCommands::Stop) => stop_cmd()
                .await
                .inspect_err(|e| eprintln!("{:?}", e))
                .inspect(|_| println!("Daemon stopped successfully"))?,
            _ => start_server(None, !daemon_args.no_ui).await?,
        },
        Some(Commands::System(SystemCommands::Service(service_args))) => {
            match service_args.command {
                ServiceCommands::Install => service::install()?,
                ServiceCommands::Uninstall => service::uninstall()?,
                ServiceCommands::Status => service::status().await?,
                ServiceCommands::Start => {
                    service::start()?;
                    println!("Service started");
                }
                ServiceCommands::Stop => {
                    service::stop()?;
                    println!("Service stopped");
                }
            }
        }
        Some(Commands::Tools(ToolsCommandsGroup::Plugin(plugin_args))) => {
            match plugin_args.command {
                PluginCommands::List => {
                    plugin::list()?;
                }
                PluginCommands::Install { path } => match install(path).await {
                    Ok(resp) => println!("{}", resp),
                    Err(err) => eprintln!("Plugin failed to install due to {:?}", err),
                },
                PluginCommands::Uninstall { name } => {
                    // handle uninstall
                    match uninstall(&name) {
                        Ok(resp) => println!("{}", resp),
                        Err(_err) => eprintln!(
                            "Plugin failed to uninstall, please check if the name is correct and try again"
                        ),
                    }
                }
            }
        }
        Some(Commands::Sync(SyncCommands::Link(link_args))) => match link_args.command {
            LinkCommands::Revoke { peer_did } => unlink_peer(&db_conn, &peer_did)?,

            LinkCommands::ListPeers => {
                show_peers(&db_conn)?;
            }
            LinkCommands::Create { peer_did } => {
                create_link(peer_did, &db_conn).await?;
            }
            LinkCommands::Add { token } => {
                add_link(token, &db_conn).await?;
            }
        },
        Some(Commands::Sync(SyncCommands::Sync { did })) => sync(did).await?,
        Some(Commands::Sync(SyncCommands::Remote(remote_args))) => match remote_args.command {
            RemoteCommands::Share => {
                println!("{}", share_remote_link().await?);
            }
            RemoteCommands::Unshare => {
                unshare_remote_link().await?;
            }
            RemoteCommands::Status => {
                println!("{}", remote_status().await?);
            }
        },
        Some(Commands::Accounts(AccountCommandsGroup::At(at_args))) => match at_args.command {
            AtCommands::Login { handle } => {
                login(&db_conn, &handle).await?;
            }
            AtCommands::Logout => logout(&db_conn)?,
        },
        Some(Commands::Uninstall { all }) => commands::uninstall_tiles(all).await?,
    }
    Ok(())
}

fn print_help_for_command(command_path: &[String]) -> Result<(), Box<dyn Error>> {
    let mut argv = vec!["tiles".to_owned()];
    argv.extend(command_path.iter().cloned());
    argv.push("--help".to_owned());

    if let Err(err) = Cli::command().try_get_matches_from(argv) {
        err.print()?;
    }

    Ok(())
}

fn build_logger() {
    if cfg!(debug_assertions) {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or("warn,iroh=error,tracing=off"),
        )
        .init()
    } else {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or("error,iroh=off,tracing=off"),
        )
        .init()
    }
}
