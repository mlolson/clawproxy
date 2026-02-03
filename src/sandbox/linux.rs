//! Linux sandbox implementation using Landlock

use crate::error::{Result, SandboxError};
use crate::sandbox::{Sandbox, SandboxConfig};
use std::convert::Infallible;

/// Linux sandbox implementation using Landlock
pub struct LinuxSandbox;

impl Sandbox for LinuxSandbox {
    fn exec_sandboxed(
        &self,
        config: &SandboxConfig,
        cmd: &str,
        args: &[String],
    ) -> Result<Infallible> {
        tracing::info!(
            cmd = %cmd,
            deny_paths = ?config.deny_read,
            "Applying Linux sandbox (Landlock)"
        );

        // Check if Landlock is available
        if !is_landlock_available() {
            tracing::warn!("Landlock not available (kernel 5.13+ required)");
            tracing::warn!("Secrets directory is NOT protected");
            tracing::warn!("Consider using Docker or upgrading your kernel.");

            // Fall through to exec without sandbox
            return exec_without_sandbox(config, cmd, args);
        }

        // TODO: Implement in Task 3.3
        // - Create Landlock ruleset
        // - Add rules for allowed paths (everything except denied)
        // - Restrict self
        // - Set environment variables
        // - exec() into target command

        Err(SandboxError::Apply("Linux Landlock sandbox not yet implemented (Task 3.3)".to_string()).into())
    }
}

/// Check if Landlock is available on this system
fn is_landlock_available() -> bool {
    // TODO: Implement proper Landlock availability check
    // For now, assume it's available on Linux
    true
}

/// Execute command without sandbox (fallback for older kernels)
fn exec_without_sandbox(
    config: &SandboxConfig,
    _cmd: &str,
    _args: &[String],
) -> Result<Infallible> {
    // Set environment variables
    for (key, value) in &config.env {
        std::env::set_var(key, value);
    }

    // TODO: exec() into target command
    Err(SandboxError::Apply("Exec not yet implemented".to_string()).into())
}
