//! OS-specific sandbox implementations

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;

use crate::config::Config;
use crate::error::{Result};
use std::collections::HashMap;
use std::convert::Infallible;

/// Configuration for the sandbox
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub config: Config,
    /// Environment variables to set
    pub env: HashMap<String, String>,
}

impl SandboxConfig {
    /// Create a new sandbox config for protecting secrets
    pub fn for_secrets(config: Config, proxy_url: &str) -> Self {
        let mut env = HashMap::new();
        env.insert("HTTP_PROXY".to_string(), proxy_url.to_string());
        env.insert("HTTPS_PROXY".to_string(), proxy_url.to_string());
        env.insert("http_proxy".to_string(), proxy_url.to_string());
        env.insert("https_proxy".to_string(), proxy_url.to_string());
        Self {
            config,
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
        sandbox_config: &SandboxConfig,
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
