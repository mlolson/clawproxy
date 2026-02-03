//! ClawProxy CLI - Main binary for proxy server and management

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "clawproxy")]
#[command(about = "Secure credential injection proxy for AI agents")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize clawproxy configuration
    Init,

    /// Start the proxy server
    Start {
        /// Path to config file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Show proxy status
    Status,

    /// Manage secrets
    #[command(subcommand)]
    Secret(SecretCommands),

    /// Configure OpenClaw integration
    ConfigureOpenclaw {
        /// Show what would be changed without modifying files
        #[arg(long)]
        dry_run: bool,

        /// Restore original files from backups
        #[arg(long)]
        revert: bool,
    },
}

#[derive(Subcommand)]
enum SecretCommands {
    /// Set a secret
    Set {
        /// Name of the secret
        name: String,
    },
    /// List all secrets
    List,
    /// Delete a secret
    Delete {
        /// Name of the secret
        name: String,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    clawproxy::init_tracing();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            tracing::info!("Initializing clawproxy...");
            // TODO: Implement in Task 5.1
            todo!("Implement init command (Task 5.1)")
        }
        Commands::Start { config } => {
            tracing::info!("Starting proxy server...");
            let cfg = clawproxy::config::Config::load(config.as_deref())?;
            let secrets_dir = cfg.secrets_dir();
            let secrets = clawproxy::config::load_all_secrets(&secrets_dir, &cfg)?;

            let server = clawproxy::proxy::ProxyServer::new(cfg, secrets);
            server.run().await?;
            Ok(())
        }
        Commands::Status => {
            tracing::info!("Checking proxy status...");
            // TODO: Implement in Task 5.6 (Human task)
            todo!("Implement status command (Task 5.6)")
        }
        Commands::Secret(cmd) => match cmd {
            SecretCommands::Set { name } => {
                tracing::info!(name = %name, "Setting secret");
                // TODO: Implement in Task 5.2
                todo!("Implement secret set command (Task 5.2)")
            }
            SecretCommands::List => {
                tracing::info!("Listing secrets...");
                // TODO: Implement in Task 5.3 (Human task)
                todo!("Implement secret list command (Task 5.3)")
            }
            SecretCommands::Delete { name, force } => {
                tracing::info!(name = %name, force = force, "Deleting secret");
                // TODO: Implement in Task 5.4
                todo!("Implement secret delete command (Task 5.4)")
            }
        },
        Commands::ConfigureOpenclaw { dry_run, revert } => {
            if dry_run {
                tracing::info!("Dry run: showing what would be changed...");
            } else if revert {
                tracing::info!("Reverting OpenClaw configuration...");
            } else {
                tracing::info!("Configuring OpenClaw integration...");
            }
            // TODO: Implement in Task 5.7
            todo!("Implement configure-openclaw command (Task 5.7)")
        }
    }
}
