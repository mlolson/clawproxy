//! ClawProxy CLI - Main binary for proxy server and management

use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

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
            cmd_init()?;
            Ok(())
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
                cmd_secret_set(&name)?;
                Ok(())
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

// ============================================================================
// Task 5.1: clawproxy init
// ============================================================================

fn cmd_init() -> anyhow::Result<()> {
    let config_dir = clawproxy::config::Config::default_config_dir()?;

    // Create config directory
    fs::create_dir_all(&config_dir)?;

    // Create secrets directory with mode 700
    let secrets_dir = config_dir.join("secrets");
    fs::create_dir_all(&secrets_dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&secrets_dir, fs::Permissions::from_mode(0o700))?;
    }

    // Write default config (don't overwrite existing)
    let config_path = config_dir.join("config.yaml");
    if config_path.exists() {
        println!("Config file already exists at {}", config_path.display());
    } else {
        let default_config = clawproxy::config::Config::default();
        let yaml = serde_yaml::to_string(&default_config)?;
        fs::write(&config_path, yaml)?;
        println!("Created config file at {}", config_path.display());
    }

    // Create OS-specific service file
    create_service_file(&config_dir)?;

    println!();
    println!("Initialized clawproxy at {}", config_dir.display());
    println!();
    println!("Next steps:");
    println!("  1. Add your API keys:");
    println!("     clawproxy secret set openai");
    println!("     clawproxy secret set anthropic");
    println!();
    println!("  2. Start the proxy:");
    println!("     clawproxy start");

    Ok(())
}

fn create_service_file(config_dir: &Path) -> anyhow::Result<()> {
    // Find clawproxy binary path
    let bin_path = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("/usr/local/bin/clawproxy"));

    if cfg!(target_os = "macos") {
        let plist_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
            .join("Library/LaunchAgents");
        fs::create_dir_all(&plist_dir)?;

        let plist_path = plist_dir.join("ai.clawproxy.plist");
        if plist_path.exists() {
            println!("Service file already exists at {}", plist_path.display());
        } else {
            let plist = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>ai.clawproxy</string>
    <key>ProgramArguments</key>
    <array>
        <string>{bin}</string>
        <string>start</string>
    </array>
    <key>RunAtLoad</key>
    <false/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{config_dir}/clawproxy.log</string>
    <key>StandardErrorPath</key>
    <string>{config_dir}/clawproxy.log</string>
</dict>
</plist>
"#,
                bin = bin_path.display(),
                config_dir = config_dir.display(),
            );
            fs::write(&plist_path, plist)?;
            println!("Created service file at {}", plist_path.display());
        }
    } else if cfg!(target_os = "linux") {
        let systemd_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
            .join(".config/systemd/user");
        fs::create_dir_all(&systemd_dir)?;

        let service_path = systemd_dir.join("clawproxy.service");
        if service_path.exists() {
            println!("Service file already exists at {}", service_path.display());
        } else {
            let service = format!(
                r#"[Unit]
Description=ClawProxy credential injection proxy

[Service]
ExecStart={bin} start
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
"#,
                bin = bin_path.display(),
            );
            fs::write(&service_path, service)?;
            println!("Created service file at {}", service_path.display());
        }
    }

    Ok(())
}

// ============================================================================
// Task 5.2: clawproxy secret set
// ============================================================================

fn cmd_secret_set(name: &str) -> anyhow::Result<()> {
    // Validate secret name
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        anyhow::bail!("Secret name must be alphanumeric (underscores allowed)");
    }

    let config_dir = clawproxy::config::Config::default_config_dir()?;
    let secrets_dir = config_dir.join("secrets");

    if !secrets_dir.exists() {
        anyhow::bail!(
            "Secrets directory not found at {}. Run 'clawproxy init' first.",
            secrets_dir.display()
        );
    }

    // Read the secret value
    let secret = if io::stdin().is_terminal() {
        // Interactive: prompt without echo
        print!("Enter secret for '{}': ", name);
        io::stdout().flush()?;
        rpassword::read_password()?
    } else {
        // Piped: read from stdin
        let mut line = String::new();
        io::stdin().lock().read_line(&mut line)?;
        line.trim().to_string()
    };

    if secret.is_empty() {
        anyhow::bail!("Secret cannot be empty");
    }

    // Write secret file
    let secret_path = secrets_dir.join(name);
    fs::write(&secret_path, &secret)?;

    // Set permissions to 600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&secret_path, fs::Permissions::from_mode(0o600))?;
    }

    let preview = mask_secret(&secret);
    println!("Saved secret '{}' ({})", name, preview);

    Ok(())
}

fn mask_secret(secret: &str) -> String {
    if secret.len() <= 8 {
        "****".to_string()
    } else {
        format!("{}...{}", &secret[..4], &secret[secret.len() - 4..])
    }
}
