//! HTTP server implementation

use crate::config::Config;
use crate::error::Result;
use std::collections::HashMap;

/// The proxy server that handles incoming requests
pub struct ProxyServer {
    #[allow(dead_code)]
    config: Config,
    #[allow(dead_code)]
    secrets: HashMap<String, String>,
}

impl ProxyServer {
    /// Create a new proxy server with the given configuration
    pub fn new(config: Config, secrets: HashMap<String, String>) -> Self {
        Self { config, secrets }
    }

    /// Start the proxy server
    pub async fn run(&self) -> Result<()> {
        tracing::info!(
            host = %self.config.listen.host,
            port = %self.config.listen.port,
            "Starting proxy server"
        );
        // TODO: Implement in Task 4.3
        todo!("Implement proxy server")
    }
}
