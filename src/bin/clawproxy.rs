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

    /// Start the clawproxy daemon
    Start,

    /// Stop the clawproxy daemon
    Stop,

    /// Run the proxy server in the foreground (used by daemon)
    Serve {
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
        Commands::Start => {
            cmd_daemon_start()
        }
        Commands::Stop => {
            cmd_daemon_stop()
        }
        Commands::Serve { config } => {
            cmd_serve(config).await
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
        <string>serve</string>
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
ExecStart={bin} serve
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

    // Auto-configure known service if not already in config
    if let Some(service_config) = clawproxy::config::known_service_config(name) {
        let config_path = config_dir.join("config.yaml");
        if config_path.exists() {
            let mut config: Config = serde_yaml::from_str(&fs::read_to_string(&config_path)?)?;
            if !config.services.contains_key(name) {
                config.services.insert(name.to_string(), service_config);
                let yaml = serde_yaml::to_string(&config)?;
                fs::write(&config_path, yaml)?;
                println!("Added '{}' service to config", name);
            }
        }
    }

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
// Daemon management: start / stop
// ============================================================================

fn plist_path() -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home.join("Library/LaunchAgents/ai.clawproxy.plist"))
}

fn cmd_daemon_start() -> anyhow::Result<()> {
    if cfg!(target_os = "macos") {
        let plist = plist_path()?;
        if !plist.exists() {
            anyhow::bail!(
                "Service file not found at {}\nRun 'clawproxy init' first",
                plist.display()
            );
        }
        let status = std::process::Command::new("launchctl")
            .args(["load", "-w"])
            .arg(&plist)
            .status()?;
        if !status.success() {
            anyhow::bail!("launchctl load failed");
        }
        println!("ClawProxy daemon started");
    } else if cfg!(target_os = "linux") {
        let status = std::process::Command::new("systemctl")
            .args(["--user", "start", "clawproxy.service"])
            .status()?;
        if !status.success() {
            anyhow::bail!("systemctl start failed");
        }
        println!("ClawProxy daemon started");
    } else {
        anyhow::bail!("Unsupported platform for daemon management");
    }
    Ok(())
}

fn cmd_daemon_stop() -> anyhow::Result<()> {
    if cfg!(target_os = "macos") {
        let plist = plist_path()?;
        if !plist.exists() {
            anyhow::bail!(
                "Service file not found at {}\nRun 'clawproxy init' first",
                plist.display()
            );
        }
        let status = std::process::Command::new("launchctl")
            .args(["unload"])
            .arg(&plist)
            .status()?;
        if !status.success() {
            anyhow::bail!("launchctl unload failed");
        }
        println!("ClawProxy daemon stopped");
    } else if cfg!(target_os = "linux") {
        let status = std::process::Command::new("systemctl")
            .args(["--user", "stop", "clawproxy.service"])
            .status()?;
        if !status.success() {
            anyhow::bail!("systemctl stop failed");
        }
        println!("ClawProxy daemon stopped");
    } else {
        anyhow::bail!("Unsupported platform for daemon management");
    }
    Ok(())
}

// ============================================================================
// Task 5.5: clawproxy serve (foreground, used by daemon)
// ============================================================================

async fn cmd_serve(config_path: Option<PathBuf>) -> anyhow::Result<()> {
    let config = Config::load(config_path.as_deref())?;

    let secrets_dir = config.secrets_dir();
    if !secrets_dir.exists() {
        anyhow::bail!(
            "Secrets directory not found: {}\nRun 'clawproxy init' first",
            secrets_dir.display()
        );
    }

    if config.services.is_empty() {
        anyhow::bail!(
            "No services configured.\n\
             Add a service by setting a secret: clawproxy secret set anthropic"
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
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let openclaw_config_path = home.join(".openclaw/openclaw.json");
    let auth_profiles_path = home.join(".openclaw/agents/default/agent/auth-profiles.json");

    if revert {
        return revert_openclaw_config(&openclaw_config_path, &auth_profiles_path);
    }

    if !openclaw_config_path.exists() {
        anyhow::bail!(
            "OpenClaw config not found at {}\n\
             Run 'openclaw onboard' first",
            openclaw_config_path.display()
        );
    }

    let clawproxy_config = Config::load(None)?;
    let secrets_dir = clawproxy_config.secrets_dir();
    let proxy_url = format!(
        "http://{}:{}",
        clawproxy_config.listen.host, clawproxy_config.listen.port
    );

    if clawproxy_config.services.is_empty() {
        anyhow::bail!(
            "No clawproxy services configured.\n\
             Add one with: clawproxy secret set anthropic"
        );
    }

    let mut redirected_providers: Vec<String> = Vec::new();
    let mut migrated_keys: Vec<(String, String)> = Vec::new();

    // --- 1. Update openclaw.json: add models.providers.<name>.baseUrl ---
    let config_content = fs::read_to_string(&openclaw_config_path)?;
    let mut config: serde_json::Value = serde_json::from_str(&config_content)?;

    // Ensure models.providers exists
    if config.get("models").is_none() {
        config.as_object_mut().unwrap().insert(
            "models".to_string(),
            serde_json::json!({}),
        );
    }
    let models = config.get_mut("models").unwrap().as_object_mut().unwrap();
    if models.get("providers").is_none() {
        models.insert("providers".to_string(), serde_json::json!({}));
    }
    let providers = models
        .get_mut("providers")
        .unwrap()
        .as_object_mut()
        .unwrap();

    for (service_name, service) in &clawproxy_config.services {
        let provider_name = service.prefix.trim_start_matches('/');
        let new_base_url = format!("{}{}", proxy_url, service.prefix);

        // Create or update the provider entry with baseUrl
        if let Some(existing) = providers.get_mut(provider_name) {
            if let Some(obj) = existing.as_object_mut() {
                obj.insert(
                    "baseUrl".to_string(),
                    serde_json::Value::String(new_base_url),
                );
            }
        } else {
            providers.insert(
                provider_name.to_string(),
                serde_json::json!({ "baseUrl": new_base_url }),
            );
        }

        redirected_providers.push(service_name.clone());
    }

    let new_content = serde_json::to_string_pretty(&config)?;

    // --- 2. Scan auth-profiles.json for tokens to migrate ---
    let mut new_auth_content: Option<String> = None;
    if auth_profiles_path.exists() {
        let auth_content = fs::read_to_string(&auth_profiles_path)?;
        let mut auth_config: serde_json::Value = serde_json::from_str(&auth_content)?;

        if let Some(profiles) = auth_config
            .get_mut("profiles")
            .and_then(|p| p.as_object_mut())
        {
            for (profile_key, profile_value) in profiles.iter_mut() {
                if let Some(obj) = profile_value.as_object_mut() {
                    // Check for token or key fields
                    let token_field = if obj.get("token").and_then(|v| v.as_str()).is_some() {
                        Some("token")
                    } else if obj.get("key").and_then(|v| v.as_str()).is_some() {
                        Some("key")
                    } else {
                        None
                    };

                    let Some(field) = token_field else {
                        continue;
                    };

                    let existing_value = obj
                        .get(field)
                        .and_then(|v| v.as_str())
                        .filter(|t| !t.is_empty() && *t != "PROXY")
                        .map(|t| t.to_string());

                    if existing_value.is_none() {
                        continue;
                    }

                    let provider_name =
                        profile_key.split(':').next().unwrap_or(profile_key);

                    obj.insert(
                        field.to_string(),
                        serde_json::Value::String("PROXY".to_string()),
                    );

                    if let Some(value) = existing_value {
                        if !migrated_keys.iter().any(|(n, _)| n == provider_name) {
                            migrated_keys.push((provider_name.to_string(), value));
                        }
                    }
                }
            }
        }

        new_auth_content = Some(serde_json::to_string_pretty(&auth_config)?);
    }

    // --- Summary ---
    if dry_run {
        for name in &redirected_providers {
            println!("Redirect {} -> {}/{}", name, proxy_url, name);
        }
        for (name, _) in &migrated_keys {
            println!("Migrate {} token to clawproxy secret", name);
        }
        return Ok(());
    }

    // --- 3. Migrate tokens to clawproxy secrets ---
    for (provider_name, key) in &migrated_keys {
        let secret_path = secrets_dir.join(provider_name);
        if secret_path.exists() {
            println!(
                "Secret '{}' already exists, skipping token migration",
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
                "Migrated token for '{}' to clawproxy secret ({})",
                provider_name,
                mask_secret(key)
            );
        }
    }

    // --- 4. Write modified files ---
    backup_file(&openclaw_config_path)?;
    fs::write(&openclaw_config_path, &new_content)?;

    if let Some(auth_content) = &new_auth_content {
        backup_file(&auth_profiles_path)?;
        fs::write(&auth_profiles_path, auth_content)?;
    }

    println!();
    println!("OpenClaw configured for clawproxy.");
    println!("Restart OpenClaw to apply changes.");

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

fn revert_openclaw_config(
    openclaw_config_path: &Path,
    auth_profiles_path: &Path,
) -> anyhow::Result<()> {
    let mut reverted_any = false;

    if revert_from_backup(openclaw_config_path)? {
        reverted_any = true;
    }

    if revert_from_backup(auth_profiles_path)? {
        reverted_any = true;
    }

    if !reverted_any {
        anyhow::bail!("No backups found. Nothing to revert.");
    }

    println!("OpenClaw configuration reverted. Restart OpenClaw to apply.");
    Ok(())
}

/// Restores a file from its .pre-clawproxy backup. Returns true if a backup was found.
fn revert_from_backup(path: &Path) -> anyhow::Result<bool> {
    let backup_path = path.with_extension(format!(
        "{}.pre-clawproxy",
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
    ));

    if !backup_path.exists() {
        return Ok(false);
    }

    fs::copy(&backup_path, path)?;
    fs::remove_file(&backup_path)?;
    println!("Reverted {}", path.display());
    Ok(true)
}


