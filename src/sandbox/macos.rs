//! macOS sandbox implementation using sandbox-exec

use crate::error::{Result, SandboxError};
use crate::sandbox::{Sandbox, SandboxConfig};
use nix::unistd::execvp;
use std::convert::Infallible;
use std::ffi::CString;
use std::path::{Path, PathBuf};

/// macOS sandbox implementation
pub struct MacOsSandbox;

impl Sandbox for MacOsSandbox {
    fn exec_sandboxed(
        &self,
        sandbox_config: &SandboxConfig,
        cmd: &str,
        args: &[String],
    ) -> Result<Infallible> {
        tracing::info!(
            cmd = %cmd,
            "Applying macOS sandbox"
        );

        let profile = generate_profile(sandbox_config)?;

        exec(&profile, &sandbox_config, &cmd, &args)
    }
}

/// Generate a sandbox-exec profile from config
fn generate_profile(sandbox_config: &SandboxConfig) -> Result<String> {
    let config_dir: PathBuf = sandbox_config.config.location.clone();
    let profile_path: PathBuf = config_dir.join("macos/sandbox.sb.template");

    let path: &Path = Path::new(&profile_path);
    if !path.exists() {
        let str_path: std::borrow::Cow<'_, str> = path.to_string_lossy();
        return Err(SandboxError::Apply(format!("Profile does not exists at {str_path}")).into());
    }

    let secrets_dir_str = sandbox_config.config.secrets_dir.to_string_lossy();
    let content = std::fs::read_to_string(&path)?;
    let profile = content.replace("{secrets_dir}", &secrets_dir_str);
    Ok(profile)
}

fn exec(
    profile: &String,
    sandbox_config: &SandboxConfig,
    cmd: &str,
    cmd_args: &[String],
) -> Result<Infallible> {
    tracing::debug!("Using sandbox profile: {}", profile);

    for (key, value) in &sandbox_config.env {
        std::env::set_var(key, value);
    }

    // Build args: sandbox-exec -f <profile> <cmd> <args...>
    let sandbox_exec = CString::new("sandbox-exec")?;
    let mut args = vec![
        CString::new("sandbox-exec")?,
        CString::new("-p")?,
        CString::new(profile.to_owned())?,
        CString::new(cmd.to_owned())?,
    ];

    for arg in cmd_args {
        args.push(CString::new(arg.to_owned())?);
    }
    let cmd_args: Vec<String> = args
        .iter()
        .map(|cs| cs.to_string_lossy().into_owned())
        .collect();

    tracing::debug!("Command {}", cmd_args.join(" "));

    execvp(&sandbox_exec, &args).map_err(|e: nix::errno::Errno| {
        crate::error::Error::Sandbox(SandboxError::Exec(e.to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::Config;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_generate_profile() {
        let dir: tempfile::TempDir = tempdir().unwrap();
        let macos_dir = dir.path().join("macos");
        fs::create_dir_all(&macos_dir).unwrap();
        fs::write(
            macos_dir.join("sandbox.sb.template"),
            "(version 1)\n(allow default)\n",
        )
        .unwrap();

        let mut config = Config::default();
        config.location = dir.path().to_path_buf();
        let sandbox_config = SandboxConfig {
            env: Default::default(),
            config: config,
        };

        let profile = generate_profile(&sandbox_config).expect("generate_profile failed");
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(allow default)"));
    }
}
