//! HTTP proxy server with credential injection and response streaming

use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode},
    Router,
};
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::signal;
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::error::{ProxyError, Result};
use crate::proxy::{router, substitution};

/// Shared application state passed to handlers via Axum's State extractor.
#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    secrets: Arc<HashMap<String, String>>,
    client: reqwest::Client,
}

/// The proxy server that handles incoming requests.
pub struct ProxyServer {
    config: Config,
    secrets: HashMap<String, String>,
}

impl ProxyServer {
    /// Create a new proxy server with the given configuration and pre-loaded secrets.
    pub fn new(config: Config, secrets: HashMap<String, String>) -> Self {
        Self { config, secrets }
    }

    /// Start the proxy server, binding to the configured address.
    /// Blocks until a shutdown signal (SIGINT/SIGTERM) is received.
    pub async fn run(self) -> Result<()> {
        let state = AppState {
            config: Arc::new(self.config.clone()),
            secrets: Arc::new(self.secrets),
            client: reqwest::Client::new(),
        };

        let app = Router::new()
            .fallback(proxy_handler)
            .layer(TraceLayer::new_for_http())
            .with_state(state);

        let addr = format!("{}:{}", self.config.listen.host, self.config.listen.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        tracing::info!(addr = %addr, "Proxy server listening");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    }
}

/// Catch-all handler that routes, rewrites, injects credentials, and forwards requests.
async fn proxy_handler(
    State(state): State<AppState>,
    request: Request<Body>,
) -> std::result::Result<Response<Body>, StatusCode> {
    match forward_request(&state, request).await {
        Ok(response) => Ok(response),
        Err(e) => {
            tracing::error!(error = %e, "Proxy error");
            match e {
                ProxyError::UnknownService(_) => Err(StatusCode::NOT_FOUND),
                ProxyError::InvalidToken(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                ProxyError::UpstreamRequest(_) => Err(StatusCode::BAD_GATEWAY),
                _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}

/// Forward a request to the matched upstream service with credential injection.
async fn forward_request(
    state: &AppState,
    request: Request<Body>,
) -> std::result::Result<Response<Body>, ProxyError> {
    let path = request.uri().path().to_string();
    let query = request.uri().query().map(|q| q.to_string());

    // Match the request path to a configured service
    let (service_name, service) = router::match_service(&path, &state.config.services)
        .ok_or_else(|| ProxyError::UnknownService(path.clone()))?;

    tracing::debug!(service = service_name, path = %path, "Matched service");

    // Build the upstream URL with rewritten path
    let upstream_url = router::build_upstream_url(service, &path, query.as_deref());

    // Look up the secret for this service
    let secret = state
        .secrets
        .get(&service.secret)
        .ok_or_else(|| ProxyError::InvalidToken(service.secret.clone()))?;

    // Format the auth header value
    let auth_value = substitution::format_auth_header(&service.auth_format, secret);

    // Build the upstream request
    let method = request.method().clone();
    let mut req_builder = state.client.request(method, &upstream_url);

    // Copy headers, skipping Host and the service's auth header
    let auth_header_lower = service.auth_header.to_lowercase();
    for (name, value) in request.headers() {
        if name == "host" || name.as_str().to_lowercase() == auth_header_lower {
            continue;
        }
        req_builder = req_builder.header(name, value);
    }

    // Inject the auth header
    req_builder = req_builder.header(&service.auth_header, &auth_value);

    // Forward the request body
    let body_bytes = axum::body::to_bytes(request.into_body(), 10 * 1024 * 1024)
        .await
        .map_err(|e| ProxyError::UpstreamRequest(e.to_string()))?;
    if !body_bytes.is_empty() {
        req_builder = req_builder.body(body_bytes);
    }

    tracing::debug!(upstream = %upstream_url, "Forwarding request");

    // Send the request upstream
    let upstream_response = req_builder
        .send()
        .await
        .map_err(|e| ProxyError::UpstreamRequest(e.to_string()))?;

    // Convert and return the response
    convert_response(upstream_response).await
}

/// Convert a reqwest response into an axum response, streaming when appropriate.
async fn convert_response(
    upstream_response: reqwest::Response,
) -> std::result::Result<Response<Body>, ProxyError> {
    let status = upstream_response.status();
    let headers = upstream_response.headers().clone();

    // Detect SSE streaming responses
    let is_streaming = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/event-stream"))
        .unwrap_or(false);

    let body = if is_streaming {
        // Stream SSE responses chunk by chunk
        let stream = upstream_response
            .bytes_stream()
            .map(|result| result.map_err(|e| axum::Error::new(e)));
        Body::from_stream(stream)
    } else {
        // Buffer non-streaming responses
        let bytes = upstream_response
            .bytes()
            .await
            .map_err(|e| ProxyError::UpstreamRequest(e.to_string()))?;
        Body::from(bytes)
    };

    let mut builder = Response::builder().status(
        StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
    );

    // Copy response headers, skipping hop-by-hop headers
    for (name, value) in headers.iter() {
        if !is_hop_by_hop(name.as_str()) {
            builder = builder.header(name, value);
        }
    }

    builder
        .body(body)
        .map_err(|e| ProxyError::UpstreamRequest(e.to_string()))
}

/// Returns true for hop-by-hop headers that should not be forwarded.
fn is_hop_by_hop(header: &str) -> bool {
    matches!(
        header.to_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}

/// Wait for SIGINT (Ctrl+C) or SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(unix)]
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    #[cfg(not(unix))]
    ctrl_c.await;

    tracing::info!("Shutdown signal received");
}
