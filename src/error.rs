//! Error types for ClawProxy

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias using ClawProxy's Error type
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for ClawProxy
#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Proxy error: {0}")]
    Proxy(#[from] ProxyError),

    #[error("Sandbox error: {0}")]
    Sandbox(#[from] SandboxError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(PathBuf),

    #[error("Failed to parse config: {0}")]
    Parse(String),

    #[error("Invalid config: {0}")]
    Invalid(String),

    #[error("Secret not found: {0}")]
    SecretNotFound(String),

    #[error("Secrets directory not found: {0}")]
    SecretsDirectoryNotFound(PathBuf),
}

/// Proxy-related errors
#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Failed to start server: {0}")]
    ServerStart(String),

    #[error("Unknown service prefix: {0}")]
    UnknownService(String),

    #[error("Upstream request failed: {0}")]
    UpstreamRequest(String),

    #[error("Invalid token format: {0}")]
    InvalidToken(String),
}

/// Sandbox-related errors
#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("Sandbox not available on this platform")]
    NotAvailable,

    #[error("Failed to create sandbox profile: {0}")]
    ProfileCreation(String),

    #[error("Failed to apply sandbox: {0}")]
    Apply(String),

    #[error("Failed to execute command: {0}")]
    Exec(String),

    #[error("Landlock not supported (kernel 5.13+ required)")]
    LandlockNotSupported,
}
