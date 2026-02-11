//! Configuration loading and management

use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use url::Host;

use crate::error::{ConfigError, Result};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub listen: ListenConfig,
    #[serde(default = "default_config_dir")]
    pub location: PathBuf,
    #[serde(default = "default_secrets_dir")]
    pub secrets_dir: PathBuf,
    pub services: HashMap<String, ServiceConfig>,
}

fn default_secrets_dir() -> PathBuf {
    PathBuf::from("secrets")
}

fn default_config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".clawproxy")
}

/// Listen address configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

/// Service configuration for upstream API routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub prefix: String,
    pub upstream: String,
    pub secret: String,
    pub auth_header: String,
    pub auth_format: String,
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        // Validate listen config
        self.validate_listen()?;

        // Validate services
        self.validate_services()?;

        Ok(())
    }

    fn validate_listen(&self) -> Result<()> {
        if !Host::parse(&self.listen.host).is_ok() {
            return Err(ConfigError::Invalid(format!("Invalid host: {}", self.listen.host)).into());
        }

        if self.listen.port < 1024 {
            tracing::warn!(
                port = self.listen.port,
                "Port < 1024 may require root privileges"
            );
        }

        Ok(())
    }

    fn validate_services(&self) -> Result<()> {
        let mut prefixes = HashSet::new();
        for service in self.services.values() {
            if !service.prefix.starts_with("/") {
                return Err(ConfigError::Invalid(format!(
                    "Invalid service prefix. Must begin with /: {}",
                    service.prefix
                ))
                .into());
            }
            if !Url::parse(&service.upstream).is_ok() {
                return Err(ConfigError::Invalid(format!(
                    "Invalid service upstream. Not a valid url: {}",
                    service.upstream
                ))
                .into());
            }
            if !service.auth_format.contains("{secret}") {
                return Err(ConfigError::Invalid(format!(
                    "Invalid service auth_format. Must contain {{secret}}: {}",
                    service.auth_format
                ))
                .into());
            }
            if prefixes.contains(&service.prefix) {
                return Err(ConfigError::Invalid(format!(
                    "Duplicate service prefix: {}",
                    service.prefix
                ))
                .into());
            }
            prefixes.insert(&service.prefix);
        }

        Ok(())
    }

    /// Load configuration from the default location or specified path.
    /// If no path is specified, looks for ~/.config/clawproxy/config.yaml
    /// If the config file doesn't exist, returns default configuration.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let config_path = match path {
            Some(p) => p.to_path_buf(),
            None => Self::default_config_path()?,
        };

        if !config_path.exists() {
            return Err(ConfigError::Invalid(format!(
                "Config file not found at {}",
                config_path.to_string_lossy()
            ))
            .into());
        }

        tracing::debug!(path = %config_path.display(), "Loading config");
        let content: String = fs::read_to_string(&config_path)?;
        let mut config: Config = serde_yaml::from_str(&content)?;
        let config_dir: PathBuf = fs::canonicalize(config_path.parent().unwrap_or(Path::new(".")))?;
        config.location = config_dir;
        config.validate()?;

        Ok(config)
    }

    /// Get the default configuration file path
    pub fn default_config_path() -> Result<PathBuf> {
        Ok(default_config_dir().join("config.yaml"))
    }

    /// Get the default configuration directory path
    pub fn default_config_dir() -> Result<PathBuf> {
        Ok(default_config_dir())
    }

    /// Get the absolute path to the secrets directory.
    /// If secrets_dir is relative, resolves against config directory.
    pub fn secrets_dir(&self) -> PathBuf {
        if self.secrets_dir.is_absolute() {
            self.secrets_dir.clone()
        } else {
            // Resolve relative to config directory
            Self::default_config_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(&self.secrets_dir)
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            location: Self::default_config_dir().unwrap_or_else(|_| PathBuf::from(".")),
            listen: ListenConfig {
                host: default_host(),
                port: default_port(),
            },
            secrets_dir: default_secrets_dir(),
            services: HashMap::new(),
        }
    }
}

/// Returns the known service config for well-known providers.
/// Used by `secret set` to auto-configure services.
pub fn known_service_config(name: &str) -> Option<ServiceConfig> {
    match name {
        "anthropic" => Some(ServiceConfig {
            prefix: "/anthropic".to_string(),
            upstream: "https://api.anthropic.com".to_string(),
            secret: "anthropic".to_string(),
            auth_header: "x-api-key".to_string(),
            auth_format: "{secret}".to_string(),
        }),
        "openai" => Some(ServiceConfig {
            prefix: "/openai".to_string(),
            upstream: "https://api.openai.com".to_string(),
            secret: "openai".to_string(),
            auth_header: "Authorization".to_string(),
            auth_format: "Bearer {secret}".to_string(),
        }),
        _ => None,
    }
}

// ============================================================================
// Secrets Loading (Task 2.2)
// ============================================================================

/// Load a single secret from the secrets directory
pub fn load_secret(secrets_dir: &Path, name: &str) -> Result<String> {
    let secret_path = secrets_dir.join(name);

    if !secret_path.exists() {
        return Err(ConfigError::SecretNotFound(name.to_string()).into());
    }

    let secret = fs::read_to_string(&secret_path)?;
    Ok(secret.trim().to_string())
}

/// Load all secrets required by the configured services
pub fn load_all_secrets(secrets_dir: &Path, config: &Config) -> Result<HashMap<String, String>> {
    if !secrets_dir.exists() {
        return Err(ConfigError::SecretsDirectoryNotFound(secrets_dir.to_path_buf()).into());
    }

    // // Check permissions on secrets directory
    check_secrets_dir_permissions(secrets_dir);

    let mut secrets = HashMap::new();

    for service in config.services.values() {
        if secrets.contains_key(&service.secret) {
            continue; // Already loaded this secret
        }

        let secret = load_secret(secrets_dir, &service.secret)?;
        secrets.insert(service.secret.clone(), secret);
    }

    Ok(secrets)
}

/// Check if secrets directory has appropriate permissions (mode 700)
fn check_secrets_dir_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if let Ok(metadata) = fs::metadata(path) {
            let mode = metadata.permissions().mode();
            // Check if group or others have any permissions
            if mode & 0o077 != 0 {
                tracing::warn!(
                    path = %path.display(),
                    mode = format!("{:o}", mode & 0o777),
                    "Secrets directory has permissive permissions, should be 700"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.listen.host, "127.0.0.1");
        assert_eq!(config.listen.port, 8080);
        assert!(config.services.is_empty());
    }

    #[test]
    fn test_load_config_from_file() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.yaml");

        fs::write(
            &config_path,
            r#"
listen:
  host: "0.0.0.0"
  port: 9000
secrets_dir: "/custom/secrets"
services:
  test:
    prefix: "/test"
    upstream: "https://test.example.com"
    secret: "test_key"
    auth_header: "Authorization"
    auth_format: "Bearer {secret}"
"#,
        )
        .unwrap();

        let config = Config::load(Some(&config_path)).unwrap();
        assert_eq!(config.listen.host, "0.0.0.0");
        assert_eq!(config.listen.port, 9000);
        assert_eq!(config.services.len(), 1);
        assert!(config.services.contains_key("test"));
    }

    #[test]
    fn test_load_secret() {
        let dir = TempDir::new().unwrap();
        let secret_path = dir.path().join("test_secret");
        fs::write(&secret_path, "my-secret-value\n").unwrap();

        let secret = load_secret(dir.path(), "test_secret").unwrap();
        assert_eq!(secret, "my-secret-value"); // Trimmed
    }

    #[test]
    fn test_load_secret_not_found() {
        let dir = TempDir::new().unwrap();
        let result = load_secret(dir.path(), "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_secrets_dir_relative_path() {
        let config = Config::default();
        let secrets_dir = config.secrets_dir();
        assert!(secrets_dir.is_absolute());
    }

    #[test]
    fn test_secrets_dir_absolute_path() {
        let mut config = Config::default();
        config.secrets_dir = PathBuf::from("/absolute/path/secrets");
        let secrets_dir = config.secrets_dir();
        assert_eq!(secrets_dir, PathBuf::from("/absolute/path/secrets"));
    }
}
