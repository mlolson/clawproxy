//! Request routing based on URL prefixes

use crate::config::ServiceConfig;
use std::collections::HashMap;

/// Match a request path to a service configuration
pub fn match_service<'a>(
    path: &str,
    services: &'a HashMap<String, ServiceConfig>,
) -> Option<(&'a str, &'a ServiceConfig)> {
    for (name, config) in services {
        if path.starts_with(&config.prefix) {
            return Some((name.as_str(), config));
        }
    }
    None
}

/// Rewrite a request path by removing the service prefix
pub fn rewrite_path(path: &str, prefix: &str) -> String {
    path.strip_prefix(prefix).unwrap_or(path).to_string()
}

/// Build the upstream URL from service config and request path
pub fn build_upstream_url(service: &ServiceConfig, path: &str, query: Option<&str>) -> String {
    let rewritten = rewrite_path(path, &service.prefix);
    match query {
        Some(q) => format!("{}{}?{}", service.upstream, rewritten, q),
        None => format!("{}{}", service.upstream, rewritten),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_services() -> HashMap<String, ServiceConfig> {
        let mut services = HashMap::new();
        services.insert(
            "openai".to_string(),
            ServiceConfig {
                prefix: "/openai".to_string(),
                upstream: "https://api.openai.com".to_string(),
                secret: "openai".to_string(),
                auth_header: "Authorization".to_string(),
                auth_format: "Bearer {secret}".to_string(),
            },
        );
        services
    }

    #[test]
    fn test_rewrite_path() {
        assert_eq!(rewrite_path("/openai/v1/chat", "/openai"), "/v1/chat");
        assert_eq!(
            rewrite_path("/anthropic/v1/messages", "/anthropic"),
            "/v1/messages"
        );
    }

    #[test]
    fn test_match_service() {
        let services = test_services();
        let result = match_service("/openai/v1/chat/completions", &services);
        assert!(result.is_some());
        let (name, _) = result.unwrap();
        assert_eq!(name, "openai");
    }

    #[test]
    fn test_match_service_no_match() {
        let services = test_services();
        let result = match_service("/unknown/path", &services);
        assert!(result.is_none());
    }

    #[test]
    fn test_build_upstream_url() {
        let services = test_services();
        let service = services.get("openai").unwrap();

        let url = build_upstream_url(service, "/openai/v1/chat", None);
        assert_eq!(url, "https://api.openai.com/v1/chat");

        let url = build_upstream_url(service, "/openai/v1/chat", Some("stream=true"));
        assert_eq!(url, "https://api.openai.com/v1/chat?stream=true");
    }
}
