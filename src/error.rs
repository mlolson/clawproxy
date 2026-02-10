//! Error types for ClawProxy

use std::path::PathBuf;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
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

    #[error("Null byte error: {0}")]
    Nul(#[from] std::ffi::NulError),

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

    #[error("Upstream unavailable: {0}")]
    UpstreamUnavailable(String),

    #[error("Upstream timeout: {0}")]
    UpstreamTimeout(String),

    #[error("Upstream request failed: {0}")]
    UpstreamRequest(String),

    #[error("Invalid token format: {0}")]
    InvalidToken(String),

    #[error("Bad request: {0}")]
    BadRequest(String),
}

impl ProxyError {
    /// Classify a reqwest error into the appropriate ProxyError variant.
    pub fn from_reqwest(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ProxyError::UpstreamTimeout(err.to_string())
        } else if err.is_connect() {
            ProxyError::UpstreamUnavailable(err.to_string())
        } else {
            ProxyError::UpstreamRequest(err.to_string())
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            ProxyError::UnknownService(_) => StatusCode::NOT_FOUND,
            ProxyError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ProxyError::UpstreamUnavailable(_) => StatusCode::BAD_GATEWAY,
            ProxyError::UpstreamTimeout(_) => StatusCode::GATEWAY_TIMEOUT,
            ProxyError::UpstreamRequest(_) => StatusCode::BAD_GATEWAY,
            ProxyError::InvalidToken(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ProxyError::ServerStart(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Client-safe error message (never exposes internal details or secrets).
    fn client_message(&self) -> &str {
        match self {
            ProxyError::UnknownService(_) => "Unknown service",
            ProxyError::BadRequest(_) => "Invalid request",
            ProxyError::UpstreamUnavailable(_) => "Upstream unavailable",
            ProxyError::UpstreamTimeout(_) => "Upstream timeout",
            ProxyError::UpstreamRequest(_) => "Upstream error",
            ProxyError::InvalidToken(_) => "Configuration error",
            ProxyError::ServerStart(_) => "Internal server error",
        }
    }
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let client_message = self.client_message();

        // Log full details internally — the Display impl includes context
        // but client_message() is sanitized
        tracing::error!(
            status = %status,
            error = %self,
            "Proxy error"
        );

        let body = axum::Json(json!({ "error": client_message }));
        (status, body).into_response()
    }
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
