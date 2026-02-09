//! Linux sandbox implementation using Landlock

use crate::error::{Result, SandboxError};
use crate::sandbox::{Sandbox, SandboxConfig};
use landlock::{
    Access, AccessFs, PathBeneath, PathFd,
    Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetStatus, ABI,
};
use nix::unistd::execvp;
use std::convert::Infallible;
use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};

/// Linux sandbox implementation using Landlock
pub struct LinuxSandbox;

impl Sandbox for LinuxSandbox {
    fn exec_sandboxed(
        &self,
        sandbox_config: &SandboxConfig,
        cmd: &str,
        args: &[String],
    ) -> Result<Infallible> {
        tracing::info!(
            cmd = %cmd,
            "Applying Linux sandbox (Landlock)"
        );

        if !is_landlock_available() {
            return Err(SandboxError::LandlockNotSupported.into());
        }

        apply_landlock(&sandbox_config.config.secrets_dir)?;
        exec(sandbox_config, cmd, args)
    }
}

/// Check if Landlock is available on this system
fn is_landlock_available() -> bool {
    let abi = ABI::V3;
    Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .and_then(|rs| rs.create())
        .is_ok()
}

/// Apply Landlock restrictions to deny access to the secrets directory.
///
/// Landlock is allowlist-based: only paths with explicit rules are accessible.
/// To deny just the secrets directory, we walk the filesystem tree from root
/// to secrets_dir and allow all siblings at each level, skipping the path
/// that leads toward secrets_dir.
fn apply_landlock(secrets_dir: &Path) -> Result<()> {
    let abi = ABI::V3;
    let access = AccessFs::from_all(abi);

    let mut ruleset = Ruleset::default()
        .handle_access(access)
        .map_err(|e| SandboxError::Apply(e.to_string()))?
        .create()
        .map_err(|e| SandboxError::Apply(e.to_string()))?;

    let denied = fs::canonicalize(secrets_dir)
        .map_err(|e| SandboxError::Apply(format!("Cannot resolve secrets dir: {}", e)))?;

    // Build chain from root to denied path: ["/", "/home", "/home/user", ..., denied]
    let mut chain: Vec<PathBuf> = Vec::new();
    let mut current = denied.as_path();
    loop {
        chain.push(current.to_path_buf());
        match current.parent() {
            Some(parent) if parent != current => {
                current = parent;
            }
            _ => break,
        }
    }
    chain.reverse();

    // For each directory in the chain (except the last, which is denied),
    // allow access to all its children EXCEPT the next step toward denied
    for i in 0..chain.len() - 1 {
        let dir = &chain[i];
        let excluded_child = &chain[i + 1];

        let entries = fs::read_dir(dir)
            .map_err(|e| SandboxError::Apply(format!("Cannot read dir {}: {}", dir.display(), e)))?;

        for entry in entries.flatten() {
            let entry_path = match entry.path().canonicalize() {
                Ok(p) => p,
                Err(_) => continue,
            };
            if entry_path != *excluded_child {
                if let Ok(fd) = PathFd::new(&entry_path) {
                    let _ = ruleset.add_rule(PathBeneath::new(fd, access));
                }
            }
        }
    }

    let status = ruleset
        .restrict_self()
        .map_err(|e| SandboxError::Apply(e.to_string()))?;

    match status.ruleset {
        RulesetStatus::FullyEnforced => {
            tracing::info!("Landlock sandbox fully enforced");
        }
        RulesetStatus::PartiallyEnforced => {
            tracing::warn!("Landlock sandbox only partially enforced");
        }
        RulesetStatus::NotEnforced => {
            tracing::warn!("Landlock sandbox not enforced");
        }
    }

    Ok(())
}

fn exec(
    sandbox_config: &SandboxConfig,
    cmd: &str,
    cmd_args: &[String],
) -> Result<Infallible> {
    for (key, value) in &sandbox_config.env {
        std::env::set_var(key, value);
    }

    let cmd_cstr = CString::new(cmd.to_owned())?;
    let mut args = vec![CString::new(cmd.to_owned())?];
    for arg in cmd_args {
        args.push(CString::new(arg.to_owned())?);
    }

    let args_display: Vec<String> = args
        .iter()
        .map(|cs| cs.to_string_lossy().into_owned())
        .collect();
    tracing::debug!("Command: {}", args_display.join(" "));

    execvp(&cmd_cstr, &args).map_err(|e: nix::errno::Errno| {
        crate::error::Error::Sandbox(SandboxError::Exec(e.to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_landlock_available() {
        // Verify the check doesn't panic — result depends on kernel version
        let _ = is_landlock_available();
    }
}
