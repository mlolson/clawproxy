//! Token substitution for credential injection

use crate::error::{ProxyError, Result};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Pattern to match PROXY:xxx tokens
static PROXY_TOKEN_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"PROXY:(\w+)").expect("Invalid regex pattern"));

/// Substitute PROXY:xxx tokens with actual secrets
pub fn substitute_tokens(input: &str, secrets: &HashMap<String, String>) -> Result<String> {
    let mut result = input.to_string();

    for cap in PROXY_TOKEN_PATTERN.captures_iter(input) {
        let full_match = cap.get(0).unwrap().as_str();
        let secret_name = cap.get(1).unwrap().as_str();

        let secret_value = secrets
            .get(secret_name)
            .ok_or_else(|| ProxyError::InvalidToken(format!("Unknown secret: {}", secret_name)))?;

        result = result.replace(full_match, secret_value);
    }

    Ok(result)
}

/// Format an auth header value using the configured format and secret
pub fn format_auth_header(format: &str, secret: &str) -> String {
    format.replace("{secret}", secret)
}

/// Check if a string contains PROXY:xxx tokens
pub fn contains_proxy_token(input: &str) -> bool {
    PROXY_TOKEN_PATTERN.is_match(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_secrets() -> HashMap<String, String> {
        let mut secrets = HashMap::new();
        secrets.insert("openai".to_string(), "sk-12345".to_string());
        secrets.insert("anthropic".to_string(), "sk-ant-67890".to_string());
        secrets
    }

    #[test]
    fn test_substitute_tokens() {
        let secrets = test_secrets();
        let result = substitute_tokens("Bearer PROXY:openai", &secrets).unwrap();
        assert_eq!(result, "Bearer sk-12345");
    }

    #[test]
    fn test_substitute_multiple_tokens() {
        let secrets = test_secrets();
        let result =
            substitute_tokens("PROXY:openai and PROXY:anthropic", &secrets).unwrap();
        assert_eq!(result, "sk-12345 and sk-ant-67890");
    }

    #[test]
    fn test_unknown_token_error() {
        let secrets = test_secrets();
        let result = substitute_tokens("Bearer PROXY:unknown", &secrets);
        assert!(result.is_err());
    }

    #[test]
    fn test_no_tokens() {
        let secrets = test_secrets();
        let result = substitute_tokens("Bearer sk-real-key", &secrets).unwrap();
        assert_eq!(result, "Bearer sk-real-key");
    }

    #[test]
    fn test_format_auth_header() {
        assert_eq!(
            format_auth_header("Bearer {secret}", "sk-12345"),
            "Bearer sk-12345"
        );
        assert_eq!(format_auth_header("{secret}", "sk-ant-67890"), "sk-ant-67890");
    }

    #[test]
    fn test_contains_proxy_token() {
        assert!(contains_proxy_token("Bearer PROXY:openai"));
        assert!(contains_proxy_token("PROXY:test"));
        assert!(!contains_proxy_token("Bearer sk-12345"));
        assert!(!contains_proxy_token("no token here"));
    }
}
