//! ClawProxy Run - Sandboxed process launcher

use clap::Parser;
use clawproxy::{config::Config, sandbox};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "clawproxy-run")]
#[command(about = "Run a command in a sandbox without access to API secrets")]
#[command(version)]
struct Cli {
    /// Command to run
    #[arg(required = true)]
    command: String,

    /// Arguments to pass to command
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,

    /// Config file path
    #[arg(short, long)]
    config_location: Option<PathBuf>,

    /// Proxy URL (default: http://127.0.0.1:8080)
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    proxy: String,

    /// Skip sandbox (dangerous, for debugging)
    #[arg(long, hide = true)]
    no_sandbox: bool,
}

fn main() -> anyhow::Result<()> {
    clawproxy::init_tracing();

    let cli = Cli::parse();

    // Load config to find secrets directory
    let config = Config::load(cli.config_location.as_deref())?;
    let secrets_dir = config.secrets_dir();

    // Determine sandbox type for logging
    let sandbox_type = if cli.no_sandbox {
        "disabled"
    } else if cfg!(target_os = "macos") {
        "macOS (sandbox-exec)"
    } else if cfg!(target_os = "linux") {
        "Linux (Landlock)"
    } else {
        "unavailable"
    };

    tracing::info!(
        sandbox = sandbox_type,
        secrets_dir = %secrets_dir.display(),
        proxy = %cli.proxy,
        command = %cli.command,
        args = ?cli.args,
        "Launching sandboxed process"
    );

    if cli.no_sandbox {
        tracing::warn!("Running without sandbox protection!");
        return exec_without_sandbox(&cli.command, &cli.args);
    }

    // Build sandbox config
    let sandbox_config = sandbox::SandboxConfig::for_secrets(config, &cli.proxy);

    // Create and apply sandbox
    let sandbox = sandbox::create_sandbox()?;
    let _ = sandbox.exec_sandboxed(&sandbox_config, &cli.command, &cli.args)?;

    // exec_sandboxed doesn't return on success, so we only get here on error
    unreachable!()
}

fn exec_without_sandbox(cmd: &str, args: &[String]) -> anyhow::Result<()> {
    use std::os::unix::process::CommandExt;

    let err = std::process::Command::new(cmd).args(args).exec();

    // exec() only returns on error
    Err(err.into())
}
