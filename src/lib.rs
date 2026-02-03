//! ClawProxy - Secure credential injection system for AI agents
//!
//! This library provides the core functionality for:
//! - Loading and managing configuration
//! - Proxying HTTP requests with credential injection
//! - Sandboxing agent processes to prevent secret access

pub mod config;
pub mod error;
pub mod proxy;
pub mod sandbox;

pub use error::{Error, Result};

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize tracing/logging with environment-based filtering.
/// Uses RUST_LOG environment variable for filter configuration.
pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
}
