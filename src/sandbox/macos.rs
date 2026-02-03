//! macOS sandbox implementation using sandbox-exec

use crate::error::{Result, SandboxError};
use crate::sandbox::{Sandbox, SandboxConfig};
use std::convert::Infallible;

/// macOS sandbox implementation
pub struct MacOsSandbox;

impl Sandbox for MacOsSandbox {
    fn exec_sandboxed(
        &self,
        config: &SandboxConfig,
        cmd: &str,
        args: &[String],
    ) -> Result<Infallible> {
        tracing::info!(
            cmd = %cmd,
            deny_paths = ?config.deny_read,
            "Applying macOS sandbox"
        );

        // Generate sandbox profile
        let profile = generate_profile(config);
        tracing::debug!(profile = %profile, "Generated sandbox profile");

        // TODO: Implement in Task 3.2 (Human task)
        // - Write profile to temp file
        // - Build sandbox-exec command
        // - Set environment variables
        // - exec() into sandbox-exec

        let _ = args; // Suppress unused warning
        Err(SandboxError::Apply("macOS sandbox not yet implemented (Task 3.2)".to_string()).into())
    }
}

/// Generate a sandbox-exec profile from config
fn generate_profile(config: &SandboxConfig) -> String {
    let mut profile = String::from(
        r#"(version 1)
(allow default)
"#,
    );

    // Add deny rules for read paths
    for path in &config.deny_read {
        profile.push_str(&format!(
            r#"(deny file-read* (subpath "{}"))
"#,
            path.display()
        ));
    }

    // Add deny rules for write paths
    for path in &config.deny_write {
        profile.push_str(&format!(
            r#"(deny file-write* (subpath "{}"))
"#,
            path.display()
        ));
    }

    profile
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_generate_profile() {
        let config = SandboxConfig {
            deny_read: vec![PathBuf::from("/secrets")],
            deny_write: vec![PathBuf::from("/secrets")],
            env: Default::default(),
        };

        let profile = generate_profile(&config);
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(allow default)"));
        assert!(profile.contains(r#"(deny file-read* (subpath "/secrets"))"#));
        assert!(profile.contains(r#"(deny file-write* (subpath "/secrets"))"#));
    }
}
