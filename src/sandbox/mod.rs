//! OS-specific sandbox implementations

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;

use crate::error::{Result, SandboxError};
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;

/// Configuration for the sandbox
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Paths the sandboxed process cannot read
    pub deny_read: Vec<PathBuf>,
    /// Paths the sandboxed process cannot write
    pub deny_write: Vec<PathBuf>,
    /// Environment variables to set
    pub env: HashMap<String, String>,
}

impl SandboxConfig {
    /// Create a new sandbox config for protecting secrets
    pub fn for_secrets(secrets_dir: PathBuf, proxy_url: &str) -> Self {
        let mut env = HashMap::new();
        env.insert("HTTP_PROXY".to_string(), proxy_url.to_string());
        env.insert("HTTPS_PROXY".to_string(), proxy_url.to_string());
        env.insert("http_proxy".to_string(), proxy_url.to_string());
        env.insert("https_proxy".to_string(), proxy_url.to_string());

        Self {
            deny_read: vec![secrets_dir.clone()],
            deny_write: vec![secrets_dir],
            env,
        }
    }
}

/// Trait for platform-specific sandbox implementations
pub trait Sandbox {
    /// Apply sandbox restrictions and exec into the target command.
    /// This function does not return on success (replaces current process).
    fn exec_sandboxed(
        &self,
        config: &SandboxConfig,
        cmd: &str,
        args: &[String],
    ) -> Result<Infallible>;
}

/// Create the appropriate sandbox for the current platform
pub fn create_sandbox() -> Result<Box<dyn Sandbox>> {
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::MacOsSandbox))
    }

    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::LinuxSandbox))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(SandboxError::NotAvailable.into())
    }
}
