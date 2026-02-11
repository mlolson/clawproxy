//! ClawProxy CLI - Main binary for proxy server and management

use clap::{Parser, Subcommand};
use clawproxy::config::Config;
use clawproxy::error::ConfigError;
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

struct SecretInfo {
    name: String,
    used_by: Vec<String>,
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

fn list_secrets(config: &Config) -> anyhow::Result<()> {
    let secrets_dir = config.secrets_dir();
    if !secrets_dir.exists() {
        return Err(ConfigError::SecretsDirectoryNotFound(secrets_dir).into());
    }
    println!("Secrets:");
    let secrets: Vec<_> = fs::read_dir(&secrets_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            let used_by: Vec<String> = config
                .services
                .iter()
                .filter(|(_, s)| s.secret == name)
                .map(|(n, _)| n.clone())
                .collect();
            return SecretInfo { name, used_by };
        })
        .collect();

    println!("{:<12} {:<16}", "NAME", "USED BY");
    for info in &secrets {
        println!("{:<12} {:<16}", info.name, info.used_by.join(","));
    }

    Ok(())
}

fn delete_secret(config: &Config, name: &str, force: bool) -> anyhow::Result<()> {
    let secrets_dir = config.secrets_dir();
    let secret_path = secrets_dir.join(name);

    if !secret_path.exists() {
        anyhow::bail!("Secret '{}' not found", name);
    }

    // Confirm unless forced
    if !force {
        print!("Delete secret '{}'? [y/N] ", name);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    // Check if any services use this secret
    let config = Config::load(None)?;
    let used_by: Vec<_> = config.services
        .iter()
        .filter(|(_, s)| s.secret == name)
        .map(|(n, _)| n.as_str())
        .collect();

    if !used_by.is_empty() && !force {
        println!("Warning: Secret '{}' is used by services: {}",
            name, used_by.join(", "));
        print!("Delete anyway? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    fs::remove_file(&secret_path)?;
    println!("Deleted secret '{}'", name);

    Ok(())
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
            cmd_start(config).await
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
                let cfg: Config = clawproxy::config::Config::load(None)?;
                return list_secrets(&cfg);
            }
            SecretCommands::Delete { name, force } => {
                tracing::info!(name = %name, force = force, "Deleting secret");
                // TODO: Implement in Task 5.4
                let cfg: Config = clawproxy::config::Config::load(None)?;
                return delete_secret(&cfg, &name, force);
            }
        },
        Commands::ConfigureOpenclaw { dry_run, revert } => {
            cmd_configure_openclaw(dry_run, revert)
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
    let bin_path =
        std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/usr/local/bin/clawproxy"));

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

// ============================================================================
// Task 5.5: clawproxy start
// ============================================================================

async fn cmd_start(config_path: Option<PathBuf>) -> anyhow::Result<()> {
    let config = Config::load(config_path.as_deref())?;

    let secrets_dir = config.secrets_dir();
    if !secrets_dir.exists() {
        anyhow::bail!(
            "Secrets directory not found: {}\nRun 'clawproxy init' first",
            secrets_dir.display()
        );
    }

    let secrets = clawproxy::config::load_all_secrets(&secrets_dir, &config)?;

    // Verify all required secrets are present
    for (service_name, service) in &config.services {
        if !secrets.contains_key(&service.secret) {
            anyhow::bail!(
                "Secret '{}' not found (required by service '{}')\n\
                 Run: clawproxy secret set {}",
                service.secret,
                service_name,
                service.secret
            );
        }
    }

    println!(
        "ClawProxy listening on {}:{}",
        config.listen.host, config.listen.port
    );
    println!(
        "Services: {}",
        config
            .services
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();
    println!("Press Ctrl+C to stop");

    let server = clawproxy::proxy::ProxyServer::new(config, secrets);
    server.run().await?;

    tracing::info!("Proxy server stopped");
    Ok(())
}

// ============================================================================
// Task 5.7: clawproxy configure-openclaw
// ============================================================================

fn cmd_configure_openclaw(dry_run: bool, revert: bool) -> anyhow::Result<()> {
    let openclaw_config_path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
        .join(".openclaw/openclaw.json");

    if revert {
        return revert_openclaw_config(&openclaw_config_path);
    }

    if !openclaw_config_path.exists() {
        anyhow::bail!(
            "OpenClaw config not found at {}\n\
             Run 'openclaw onboard' first",
            openclaw_config_path.display()
        );
    }

    // Ensure clawproxy is initialized
    let clawproxy_config = Config::load(None)?;
    let secrets_dir = clawproxy_config.secrets_dir();
    let proxy_url = format!(
        "http://{}:{}",
        clawproxy_config.listen.host, clawproxy_config.listen.port
    );

    // Read and parse OpenClaw config
    let config_content = fs::read_to_string(&openclaw_config_path)?;
    let mut config: serde_json::Value = serde_json::from_str(&config_content)?;

    let mut migrated_keys: Vec<(String, String)> = Vec::new();

    // Process each provider
    if let Some(providers) = config
        .get_mut("models")
        .and_then(|m| m.get_mut("providers"))
        .and_then(|p| p.as_object_mut())
    {
        for (provider_name, provider_config) in providers.iter_mut() {
            if let Some(obj) = provider_config.as_object_mut() {
                // Extract existing API key if present
                let existing_key = obj
                    .get("apiKey")
                    .and_then(|v| v.as_str())
                    .filter(|k| !k.is_empty())
                    .map(|k| k.to_string());

                // Map provider to clawproxy service prefix
                let service_prefix = format!("/{}", provider_name);

                // Check if we have a matching service in clawproxy config
                let matching_service = clawproxy_config
                    .services
                    .values()
                    .find(|s| s.prefix == service_prefix);

                if let Some(_service) = matching_service {
                    // Update baseUrl to go through proxy
                    let new_base_url = format!("{}{}", proxy_url, service_prefix);

                    if dry_run {
                        println!("[Dry run] {} provider:", provider_name);
                        if let Some(old_url) = obj.get("baseUrl").and_then(|v| v.as_str()) {
                            println!("  baseUrl: {} -> {}", old_url, new_base_url);
                        }
                        if existing_key.is_some() {
                            println!(
                                "  apiKey: would migrate to clawproxy secret '{}'",
                                provider_name
                            );
                        }
                        println!("  apiKey: would set to 'PROXY' (placeholder)");
                    } else {
                        obj.insert(
                            "baseUrl".to_string(),
                            serde_json::Value::String(new_base_url),
                        );
                        obj.insert(
                            "apiKey".to_string(),
                            serde_json::Value::String("PROXY".to_string()),
                        );

                        if let Some(key) = existing_key {
                            migrated_keys.push((provider_name.clone(), key));
                        }
                    }
                }
            }
        }
    }

    if dry_run {
        println!();
        println!("[Dry run] Would modify:");
        println!("  - {}", openclaw_config_path.display());
        if !migrated_keys.is_empty() {
            println!("  - Would migrate {} API keys to clawproxy secrets", migrated_keys.len());
        }
        return Ok(());
    }

    // Migrate API keys to clawproxy secrets
    for (provider_name, key) in &migrated_keys {
        let secret_path = secrets_dir.join(provider_name);
        if secret_path.exists() {
            println!(
                "Secret '{}' already exists, skipping key migration",
                provider_name
            );
        } else {
            fs::write(&secret_path, key)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&secret_path, fs::Permissions::from_mode(0o600))?;
            }
            println!(
                "Migrated API key for '{}' to clawproxy secret ({})",
                provider_name,
                mask_secret(key)
            );
        }
    }

    // Backup and write config
    backup_file(&openclaw_config_path)?;
    fs::write(
        &openclaw_config_path,
        serde_json::to_string_pretty(&config)?,
    )?;

    // Modify daemon service file
    modify_daemon_service(dry_run)?;

    println!();
    println!("OpenClaw configured successfully!");
    println!();
    println!("Backups created:");
    println!(
        "  - {}.pre-clawproxy",
        openclaw_config_path.display()
    );
    println!();
    println!("Next steps:");
    println!("  1. Start clawproxy: clawproxy start");
    println!("  2. Restart OpenClaw");

    Ok(())
}

fn backup_file(path: &Path) -> anyhow::Result<()> {
    let backup_path = path.with_extension(
        format!(
            "{}.pre-clawproxy",
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
        ),
    );
    if !backup_path.exists() {
        fs::copy(path, &backup_path)?;
        println!("Backed up {} to {}", path.display(), backup_path.display());
    }
    Ok(())
}

fn revert_openclaw_config(openclaw_config_path: &Path) -> anyhow::Result<()> {
    let backup_path = openclaw_config_path.with_extension(
        format!(
            "{}.pre-clawproxy",
            openclaw_config_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
        ),
    );

    if !backup_path.exists() {
        anyhow::bail!(
            "No backup found at {}\nNothing to revert",
            backup_path.display()
        );
    }

    fs::copy(&backup_path, openclaw_config_path)?;
    fs::remove_file(&backup_path)?;
    println!("Reverted {}", openclaw_config_path.display());

    // Revert daemon service
    revert_daemon_service()?;

    println!("OpenClaw configuration reverted successfully");
    Ok(())
}

fn modify_daemon_service(dry_run: bool) -> anyhow::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let clawproxy_run_path = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("/usr/local/bin/clawproxy"))
        .parent()
        .map(|p| p.join("clawproxy-run"))
        .unwrap_or_else(|| PathBuf::from("/usr/local/bin/clawproxy-run"));

    if cfg!(target_os = "macos") {
        let plist_path = home.join("Library/LaunchAgents/ai.openclaw.gateway.plist");
        if !plist_path.exists() {
            println!("OpenClaw daemon plist not found at {}, skipping", plist_path.display());
            return Ok(());
        }

        if dry_run {
            println!("[Dry run] Would modify daemon: {}", plist_path.display());
            println!("  Would wrap ProgramArguments with clawproxy-run");
            return Ok(());
        }

        let content = fs::read_to_string(&plist_path)?;
        backup_file(&plist_path)?;

        // Simple approach: replace the ProgramArguments to wrap with clawproxy-run
        // Look for the existing command and wrap it
        if content.contains("clawproxy-run") {
            println!("Daemon already configured for clawproxy-run, skipping");
            return Ok(());
        }

        // Replace the executable path in the plist to use clawproxy-run
        // This is a simplified approach — we wrap the existing command
        let modified = content.replace(
            "<string>openclaw</string>",
            &format!(
                "<string>{}</string>\n        <string>-c</string>\n        <string>openclaw</string>",
                clawproxy_run_path.display()
            ),
        );

        if modified == content {
            println!(
                "Could not find openclaw command in plist to wrap. \
                 You may need to manually update {}",
                plist_path.display()
            );
        } else {
            fs::write(&plist_path, modified)?;
            println!("Updated daemon plist to use clawproxy-run");
        }
    } else if cfg!(target_os = "linux") {
        let service_path = home.join(".config/systemd/user/openclaw-gateway.service");
        if !service_path.exists() {
            println!(
                "OpenClaw daemon service not found at {}, skipping",
                service_path.display()
            );
            return Ok(());
        }

        if dry_run {
            println!("[Dry run] Would modify daemon: {}", service_path.display());
            return Ok(());
        }

        let content = fs::read_to_string(&service_path)?;
        backup_file(&service_path)?;

        if content.contains("clawproxy-run") {
            println!("Daemon already configured for clawproxy-run, skipping");
            return Ok(());
        }

        // Wrap ExecStart with clawproxy-run -c "original command"
        let mut modified = String::new();
        for line in content.lines() {
            if line.starts_with("ExecStart=") {
                let original_cmd = line.trim_start_matches("ExecStart=");
                modified.push_str(&format!(
                    "ExecStart={} -c \"{}\"",
                    clawproxy_run_path.display(),
                    original_cmd
                ));
            } else {
                modified.push_str(line);
            }
            modified.push('\n');
        }

        fs::write(&service_path, modified)?;
        println!("Updated systemd service to use clawproxy-run");
    }

    Ok(())
}

fn revert_daemon_service() -> anyhow::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    if cfg!(target_os = "macos") {
        let plist_path = home.join("Library/LaunchAgents/ai.openclaw.gateway.plist");
        let backup = plist_path.with_extension("plist.pre-clawproxy");
        if backup.exists() {
            fs::copy(&backup, &plist_path)?;
            fs::remove_file(&backup)?;
            println!("Reverted daemon plist");
        }
    } else if cfg!(target_os = "linux") {
        let service_path = home.join(".config/systemd/user/openclaw-gateway.service");
        let backup = service_path.with_extension("service.pre-clawproxy");
        if backup.exists() {
            fs::copy(&backup, &service_path)?;
            fs::remove_file(&backup)?;
            println!("Reverted systemd service");
        }
    }

    Ok(())
}
