//! Adapter Factory (Phase 5)
//!
//! Creates LLM adapter instances from configuration.

use crate::llm::adapters::glm::GlmAdapter;
use crate::llm::adapters::ollama::OllamaAdapter;
use crate::llm::adapters::openai::OpenAiAdapter;
use crate::llm::adapters::stub::StubAdapter;
use crate::llm::adapters::{Adapter, AdapterError};
use std::path::Path;

/// Create adapter from config file path
pub fn create_adapter_from_config(db_root: &Path) -> Result<Adapter, AdapterError> {
    let config_path = db_root.join("config.toml");
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| AdapterError::Configuration(format!("Failed to read config: {}", e)))?;

    create_adapter_from_config_str(&content)
}

/// Create adapter from config string
///
/// Parses TOML-style config and returns appropriate adapter.
/// Uses simple string matching (no TOML dependency).
pub fn create_adapter_from_config_str(content: &str) -> Result<Adapter, AdapterError> {
    // Check for disabled mode first
    if content.contains("mode = \"disabled\"") {
        return Err(AdapterError::Configuration(
            "LLM mode is disabled".to_string(),
        ));
    }

    // Check for local backend (Ollama)
    if content.contains("backend = \"ollama\"") {
        let host = extract_config_value(content, "host").unwrap_or("127.0.0.1".to_string());
        let port = extract_config_value(content, "port").unwrap_or("11434".to_string());
        let model = extract_config_value(content, "model")
            .ok_or_else(|| AdapterError::Configuration("Missing 'model' in config".to_string()))?;

        return Ok(Adapter::Ollama(OllamaAdapter::new(host, port, model)));
    }

    // External providers (provider field)
    let provider = extract_config_value(content, "provider")
        .ok_or_else(|| AdapterError::Configuration("Missing 'provider' in config".to_string()))?;

    // Handle stub provider for testing
    if provider == "stub" {
        return Ok(Adapter::Stub(StubAdapter::new()));
    }

    let base_url = extract_config_value(content, "base_url")
        .ok_or_else(|| AdapterError::Configuration("Missing 'base_url' in config".to_string()))?;

    let model = extract_config_value(content, "model")
        .ok_or_else(|| AdapterError::Configuration("Missing 'model' in config".to_string()))?;

    let api_key = extract_config_value(content, "api_key")
        .ok_or_else(|| AdapterError::Configuration("Missing 'api_key' in config".to_string()))?;

    // Resolve env:... references
    let api_key = resolve_env_var(&api_key);

    match provider.as_str() {
        "openai" => Ok(Adapter::OpenAi(OpenAiAdapter::new(
            base_url, model, api_key,
        ))),
        "glm" => Ok(Adapter::Glm(GlmAdapter::new(base_url, model, api_key))),
        _ => Err(AdapterError::Configuration(format!(
            "Unknown provider: {}",
            provider
        ))),
    }
}

/// Extract config value from TOML-style string
///
/// Simple parser for "key = \"value\"" pattern.
fn extract_config_value(content: &str, key: &str) -> Option<String> {
    let pattern = format!("{} = \"", key);
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(&pattern) {
            let start = pattern.len();
            if let Some(end) = line[start..].find('"') {
                return Some(line[start..start + end].to_string());
            }
        }
    }
    None
}

/// Resolve environment variable reference
///
/// If value starts with "env:", read from environment.
/// Otherwise return value as-is.
fn resolve_env_var(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("env:") {
        std::env::var(rest).unwrap_or_else(|_| format!("env:{}", rest))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::adapters::LlmAdapter;

    #[test]
    fn test_extract_config_value() {
        let config = r#"[llm]
mode = "external"
provider = "openai"
base_url = "https://api.openai.com/v1"
model = "gpt-4"
"#;

        assert_eq!(
            extract_config_value(config, "provider"),
            Some("openai".to_string())
        );
        assert_eq!(
            extract_config_value(config, "base_url"),
            Some("https://api.openai.com/v1".to_string())
        );
        assert_eq!(
            extract_config_value(config, "model"),
            Some("gpt-4".to_string())
        );
        assert_eq!(extract_config_value(config, "missing"), None);
    }

    #[test]
    fn test_resolve_env_var_direct() {
        assert_eq!(resolve_env_var("direct_value"), "direct_value");
    }

    #[test]
    fn test_resolve_env_var_reference() {
        std::env::set_var("TEST_VAR", "test_value");
        assert_eq!(resolve_env_var("env:TEST_VAR"), "test_value");
    }

    #[test]
    fn test_factory_openai_config() {
        let config = r#"[llm]
mode = "external"
provider = "openai"
base_url = "https://api.openai.com/v1"
api_key = "sk-test"
model = "gpt-4"
"#;

        let result = create_adapter_from_config_str(config);
        assert!(result.is_ok());
        let adapter = result.unwrap();
        assert_eq!(adapter.provider_name(), "openai");
    }

    #[test]
    fn test_factory_glm_config() {
        let config = r#"[llm]
mode = "external"
provider = "glm"
base_url = "https://api.z.ai/api/coding/paas/v4"
api_key = "sk-test"
model = "GLM-4.7"
"#;

        let result = create_adapter_from_config_str(config);
        assert!(result.is_ok());
        let adapter = result.unwrap();
        assert_eq!(adapter.provider_name(), "glm");
    }

    #[test]
    fn test_factory_ollama_config() {
        let config = r#"[llm]
mode = "local"
backend = "ollama"
host = "127.0.0.1"
port = "11434"
model = "codellama"
"#;

        let result = create_adapter_from_config_str(config);
        assert!(result.is_ok());
        let adapter = result.unwrap();
        assert_eq!(adapter.provider_name(), "ollama");
    }

    #[test]
    fn test_factory_disabled_returns_error() {
        let config = r#"[llm]
mode = "disabled"
"#;

        let result = create_adapter_from_config_str(config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AdapterError::Configuration(_)
        ));
    }

    #[test]
    fn test_factory_missing_provider_returns_error() {
        let config = r#"[llm]
mode = "external"
base_url = "http://localhost"
model = "test"
"#;

        let result = create_adapter_from_config_str(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_factory_unknown_provider_returns_error() {
        let config = r#"[llm]
mode = "external"
provider = "unknown"
base_url = "http://localhost"
api_key = "key"
model = "test"
"#;

        let result = create_adapter_from_config_str(config);
        assert!(result.is_err());
    }
}
