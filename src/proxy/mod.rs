//! HTTP proxy server with credential injection

pub mod router;
pub mod server;
pub mod substitution;

pub use server::ProxyServer;
