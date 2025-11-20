//! Configuration Management Module
//!
//! This module provides comprehensive configuration management for LLM providers,
//! including file-based configuration, environment variable integration,
//! and validation.

use super::llm_integration::{LLMProvider, LLMProviderConfig};
use anyhow::{anyhow, Result};
use dirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Configuration file format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigFormat {
    Json,
    Toml,
    Yaml,
}

impl Default for ConfigFormat {
    fn default() -> Self {
        ConfigFormat::Toml
    }
}

impl std::str::FromStr for ConfigFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(ConfigFormat::Json),
            "toml" => Ok(ConfigFormat::Toml),
            "yaml" | "yml" => Ok(ConfigFormat::Yaml),
            _ => Err(anyhow!("Unsupported config format: {}", s)),
        }
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// Default provider name
    pub default_provider: Option<String>,
    /// Provider configurations
    pub providers: HashMap<String, ProviderConfig>,
    /// Global settings
    pub settings: ConfigSettings,
    /// Model selection preferences
    pub model_selection: ModelSelectionConfig,
}

impl Default for LLMConfig {
    fn default() -> Self {
        let mut providers = HashMap::new();

        // Add default provider configurations
        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                provider: LLMProvider::OpenAI,
                api_key: None,
                base_url: Some("https://api.openai.com/v1".to_string()),
                model: "gpt-3.5-turbo".to_string(),
                max_tokens: 1000,
                temperature: 0.7,
                enabled: true,
                timeout_seconds: 30,
                retry_attempts: 3,
            },
        );

        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                provider: LLMProvider::Anthropic,
                api_key: None,
                base_url: Some("https://api.anthropic.com".to_string()),
                model: "claude-3-sonnet-20240229".to_string(),
                max_tokens: 1000,
                temperature: 0.7,
                enabled: true,
                timeout_seconds: 30,
                retry_attempts: 3,
            },
        );

        providers.insert(
            "ollama".to_string(),
            ProviderConfig {
                provider: LLMProvider::Ollama,
                api_key: None,
                base_url: Some("http://localhost:11434".to_string()),
                model: "llama2".to_string(),
                max_tokens: 1000,
                temperature: 0.7,
                enabled: true,
                timeout_seconds: 60,
                retry_attempts: 5,
            },
        );

        Self {
            default_provider: Some("openai".to_string()),
            providers,
            settings: ConfigSettings::default(),
            model_selection: ModelSelectionConfig::default(),
        }
    }
}

/// Extended provider configuration with additional settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: LLMProvider,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f64,
    pub enabled: bool,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
}

impl From<ProviderConfig> for LLMProviderConfig {
    fn from(config: ProviderConfig) -> Self {
        LLMProviderConfig {
            provider: config.provider,
            api_key: config.api_key,
            base_url: config.base_url,
            model: config.model,
            max_tokens: config.max_tokens,
            temperature: config.temperature,
        }
    }
}

/// Global configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSettings {
    pub cache_enabled: bool,
    pub cache_ttl_seconds: u64,
    pub max_cache_size: usize,
    pub log_level: String,
    pub telemetry_enabled: bool,
    pub request_timeout_seconds: u64,
}

impl Default for ConfigSettings {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            cache_ttl_seconds: 3600, // 1 hour
            max_cache_size: 1000,
            log_level: "info".to_string(),
            telemetry_enabled: false,
            request_timeout_seconds: 30,
        }
    }
}

/// Model selection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelectionConfig {
    pub capability_weight: f64,
    pub speed_weight: f64,
    pub cost_weight: f64,
    pub reliability_weight: f64,
    pub auto_detection_enabled: bool,
    pub learning_enabled: bool,
}

impl Default for ModelSelectionConfig {
    fn default() -> Self {
        Self {
            capability_weight: 0.5,
            speed_weight: 0.2,
            cost_weight: 0.2,
            reliability_weight: 0.1,
            auto_detection_enabled: true,
            learning_enabled: true,
        }
    }
}

/// Configuration manager
pub struct ConfigManager {
    config_path: PathBuf,
    config_format: ConfigFormat,
    config: LLMConfig,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_config_dir()?;
        let config_path = config_dir.join("odincode").join("llm_config");

        // Determine config format based on available files
        let config_format = Self::detect_config_format(&config_path)?;

        let mut manager = Self {
            config_path,
            config_format,
            config: LLMConfig::default(),
        };

        // Load configuration if it exists
        if manager.config_exists() {
            manager.load_config()?;
        } else {
            // Create default configuration
            manager.save_config()?;
        }

        // Override with environment variables
        manager.apply_env_overrides()?;

        Ok(manager)
    }

    /// Create configuration manager with custom path
    pub fn with_path<P: AsRef<Path>>(path: P, format: ConfigFormat) -> Result<Self> {
        let config_path = path.as_ref().to_path_buf();

        let mut manager = Self {
            config_path,
            config_format: format,
            config: LLMConfig::default(),
        };

        if manager.config_exists() {
            manager.load_config()?;
        } else {
            manager.save_config()?;
        }

        manager.apply_env_overrides()?;

        Ok(manager)
    }

    /// Get the configuration directory
    fn get_config_dir() -> Result<PathBuf> {
        dirs::config_dir().ok_or_else(|| anyhow!("Could not determine config directory"))
    }

    /// Detect configuration format based on file extension
    fn detect_config_format(base_path: &Path) -> Result<ConfigFormat> {
        let extensions = ["toml", "json", "yaml", "yml"];

        for ext in &extensions {
            let file_path = base_path.with_extension(ext);
            if file_path.exists() {
                return match ext {
                    &"toml" => Ok(ConfigFormat::Toml),
                    &"json" => Ok(ConfigFormat::Json),
                    &"yaml" | &"yml" => Ok(ConfigFormat::Yaml),
                    _ => Err(anyhow!("Unsupported config format: {}", ext)),
                };
            }
        }

        // Default to TOML if no file exists
        Ok(ConfigFormat::Toml)
    }

    /// Check if configuration file exists
    pub fn config_exists(&self) -> bool {
        self.get_config_file_path().exists()
    }

    /// Get the full configuration file path
    pub fn get_config_file_path(&self) -> PathBuf {
        match self.config_format {
            ConfigFormat::Toml => self.config_path.with_extension("toml"),
            ConfigFormat::Json => self.config_path.with_extension("json"),
            ConfigFormat::Yaml => self.config_path.with_extension("yaml"),
        }
    }

    /// Load configuration from file
    pub fn load_config(&mut self) -> Result<()> {
        let config_file = self.get_config_file_path();

        if !config_file.exists() {
            return Err(anyhow!("Configuration file not found: {:?}", config_file));
        }

        let content = fs::read_to_string(&config_file)
            .map_err(|e| anyhow!("Failed to read config file {:?}: {}", config_file, e))?;

        self.config = match self.config_format {
            ConfigFormat::Toml => toml::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse TOML config: {}", e))?,
            ConfigFormat::Json => serde_json::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse JSON config: {}", e))?,
            ConfigFormat::Yaml => serde_yaml::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse YAML config: {}", e))?,
        };

        info!("Loaded configuration from {:?}", config_file);
        Ok(())
    }

    /// Save configuration to file
    pub fn save_config(&self) -> Result<()> {
        let config_file = self.get_config_file_path();

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed to create config directory {:?}: {}", parent, e))?;
        }

        let content = match self.config_format {
            ConfigFormat::Toml => toml::to_string_pretty(&self.config)
                .map_err(|e| anyhow!("Failed to serialize TOML config: {}", e))?,
            ConfigFormat::Json => serde_json::to_string_pretty(&self.config)
                .map_err(|e| anyhow!("Failed to serialize JSON config: {}", e))?,
            ConfigFormat::Yaml => serde_yaml::to_string(&self.config)
                .map_err(|e| anyhow!("Failed to serialize YAML config: {}", e))?,
        };

        fs::write(&config_file, content)
            .map_err(|e| anyhow!("Failed to write config file {:?}: {}", config_file, e))?;

        info!("Saved configuration to {:?}", config_file);
        Ok(())
    }

    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) -> Result<()> {
        // Override default provider
        if let Ok(default_provider) = std::env::var("ODINCODE_DEFAULT_PROVIDER") {
            self.config.default_provider = Some(default_provider);
            debug!("Applied env override for default provider");
        }

        // Override API keys
        for (provider_name, provider_config) in &mut self.config.providers {
            let env_var_name = format!("ODINCODE_{}_API_KEY", provider_name.to_uppercase());
            if let Ok(api_key) = std::env::var(&env_var_name) {
                provider_config.api_key = Some(api_key);
                debug!("Applied env override for {} API key", provider_name);
            }
        }

        // Override base URLs
        for (provider_name, provider_config) in &mut self.config.providers {
            let env_var_name = format!("ODINCODE_{}_BASE_URL", provider_name.to_uppercase());
            if let Ok(base_url) = std::env::var(&env_var_name) {
                provider_config.base_url = Some(base_url);
                debug!("Applied env override for {} base URL", provider_name);
            }
        }

        // Override models
        for (provider_name, provider_config) in &mut self.config.providers {
            let env_var_name = format!("ODINCODE_{}_MODEL", provider_name.to_uppercase());
            if let Ok(model) = std::env::var(&env_var_name) {
                provider_config.model = model;
                debug!("Applied env override for {} model", provider_name);
            }
        }

        // Override max_tokens
        for (provider_name, provider_config) in &mut self.config.providers {
            let env_var_name = format!("ODINCODE_{}_MAX_TOKENS", provider_name.to_uppercase());
            if let Ok(max_tokens_str) = std::env::var(&env_var_name) {
                if let Ok(max_tokens) = max_tokens_str.parse::<usize>() {
                    provider_config.max_tokens = max_tokens;
                    debug!("Applied env override for {} max tokens", provider_name);
                }
            }
        }

        // Override temperature
        for (provider_name, provider_config) in &mut self.config.providers {
            let env_var_name = format!("ODINCODE_{}_TEMPERATURE", provider_name.to_uppercase());
            if let Ok(temperature_str) = std::env::var(&env_var_name) {
                if let Ok(temperature) = temperature_str.parse::<f64>() {
                    provider_config.temperature = temperature;
                    debug!("Applied env override for {} temperature", provider_name);
                }
            }
        }

        // Override global max_tokens for all providers
        if let Ok(max_tokens_str) = std::env::var("ODINCODE_MAX_TOKENS") {
            if let Ok(max_tokens) = max_tokens_str.parse::<usize>() {
                for provider_config in self.config.providers.values_mut() {
                    provider_config.max_tokens = max_tokens;
                }
                debug!("Applied env override for global max tokens");
            }
        }

        // Override global temperature for all providers
        if let Ok(temperature_str) = std::env::var("ODINCODE_TEMPERATURE") {
            if let Ok(temperature) = temperature_str.parse::<f64>() {
                for provider_config in self.config.providers.values_mut() {
                    provider_config.temperature = temperature;
                }
                debug!("Applied env override for global temperature");
            }
        }

        // Override settings
        if let Ok(log_level) = std::env::var("ODINCODE_LOG_LEVEL") {
            self.config.settings.log_level = log_level;
            debug!("Applied env override for log level");
        }

        if let Ok(telemetry_enabled) = std::env::var("ODINCODE_TELEMETRY_ENABLED") {
            self.config.settings.telemetry_enabled = telemetry_enabled.to_lowercase() == "true";
            debug!("Applied env override for telemetry");
        }

        Ok(())
    }

    /// Get the current configuration
    pub fn get_config(&self) -> &LLMConfig {
        &self.config
    }

    /// Get mutable configuration
    pub fn get_config_mut(&mut self) -> &mut LLMConfig {
        &mut self.config
    }

    /// Get provider configurations as LLMProviderConfig
    pub fn get_llm_provider_configs(&self) -> HashMap<String, LLMProviderConfig> {
        self.config
            .providers
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(name, config)| (name.clone(), config.clone().into()))
            .collect()
    }

    /// Get default provider name
    pub fn get_default_provider(&self) -> Option<&str> {
        self.config.default_provider.as_deref()
    }

    /// Set default provider
    pub fn set_default_provider(&mut self, provider: String) -> Result<()> {
        if !self.config.providers.contains_key(&provider) {
            return Err(anyhow!("Provider not found: {}", provider));
        }
        self.config.default_provider = Some(provider);
        Ok(())
    }

    /// Add or update a provider
    pub fn add_provider(&mut self, name: String, config: ProviderConfig) {
        self.config.providers.insert(name, config);
    }

    /// Remove a provider
    pub fn remove_provider(&mut self, name: &str) -> Result<()> {
        if self.config.providers.remove(name).is_none() {
            return Err(anyhow!("Provider not found: {}", name));
        }
        Ok(())
    }

    /// Enable/disable a provider
    pub fn set_provider_enabled(&mut self, name: &str, enabled: bool) -> Result<()> {
        let provider = self
            .config
            .providers
            .get_mut(name)
            .ok_or_else(|| anyhow!("Provider not found: {}", name))?;
        provider.enabled = enabled;
        Ok(())
    }

    /// Validate configuration
    pub fn validate_config(&self) -> Result<()> {
        // Validate default provider exists
        if let Some(ref default_provider) = self.config.default_provider {
            if !self.config.providers.contains_key(default_provider) {
                return Err(anyhow!(
                    "Default provider '{}' not found in providers",
                    default_provider
                ));
            }
        }

        // Validate each provider
        for (name, provider) in &self.config.providers {
            if !provider.enabled {
                continue;
            }

            // Validate required fields
            if provider.model.is_empty() {
                return Err(anyhow!("Provider '{}' has empty model name", name));
            }

            // Validate API key for cloud providers
            match provider.provider {
                LLMProvider::OpenAI | LLMProvider::Anthropic => {
                    if provider.api_key.is_none() || provider.api_key.as_ref().unwrap().is_empty() {
                        warn!(
                            "Provider '{}' requires API key but none is configured",
                            name
                        );
                    }
                }
                LLMProvider::Ollama => {
                    // Ollama doesn't require API key
                }
            }

            // Validate timeout
            if provider.timeout_seconds == 0 {
                return Err(anyhow!(
                    "Provider '{}' has invalid timeout: {} seconds",
                    name,
                    provider.timeout_seconds
                ));
            }

            // Validate retry attempts
            if provider.retry_attempts == 0 {
                return Err(anyhow!(
                    "Provider '{}' has invalid retry attempts: {}",
                    name,
                    provider.retry_attempts
                ));
            }

            // Validate temperature
            if !(0.0..=2.0).contains(&provider.temperature) {
                return Err(anyhow!(
                    "Provider '{}' has invalid temperature: {} (must be between 0.0 and 2.0)",
                    name,
                    provider.temperature
                ));
            }

            // Validate max tokens
            if provider.max_tokens == 0 {
                return Err(anyhow!(
                    "Provider '{}' has invalid max tokens: {}",
                    name,
                    provider.max_tokens
                ));
            }
        }

        // Validate model selection weights
        let weights = [
            self.config.model_selection.capability_weight,
            self.config.model_selection.speed_weight,
            self.config.model_selection.cost_weight,
            self.config.model_selection.reliability_weight,
        ];

        let total_weight: f64 = weights.iter().sum();
        if (total_weight - 1.0).abs() > 0.01 {
            return Err(anyhow!(
                "Model selection weights must sum to 1.0, got: {}",
                total_weight
            ));
        }

        for (i, &weight) in weights.iter().enumerate() {
            if weight < 0.0 || weight > 1.0 {
                let names = ["capability", "speed", "cost", "reliability"];
                return Err(anyhow!(
                    "Model selection {} weight must be between 0.0 and 1.0, got: {}",
                    names[i],
                    weight
                ));
            }
        }

        // Validate settings
        if self.config.settings.cache_ttl_seconds == 0 {
            return Err(anyhow!("Cache TTL must be greater than 0"));
        }

        if self.config.settings.max_cache_size == 0 {
            return Err(anyhow!("Max cache size must be greater than 0"));
        }

        if self.config.settings.request_timeout_seconds == 0 {
            return Err(anyhow!("Request timeout must be greater than 0"));
        }

        Ok(())
    }

    /// Reset to default configuration
    pub fn reset_to_defaults(&mut self) {
        self.config = LLMConfig::default();
    }

    /// Get configuration file path
    pub fn get_config_path(&self) -> &Path {
        &self.config_path
    }

    /// Get configuration format
    pub fn get_config_format(&self) -> &ConfigFormat {
        &self.config_format
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_default_creation() {
        let config = LLMConfig::default();
        assert!(config.providers.contains_key("openai"));
        assert!(config.providers.contains_key("anthropic"));
        assert!(config.providers.contains_key("ollama"));
        assert_eq!(config.default_provider, Some("openai".to_string()));
    }

    #[test]
    fn test_provider_config_conversion() {
        let provider_config = ProviderConfig {
            provider: LLMProvider::OpenAI,
            api_key: Some("test_key".to_string()),
            base_url: Some("https://api.test.com".to_string()),
            model: "test-model".to_string(),
            max_tokens: 500,
            temperature: 0.5,
            enabled: true,
            timeout_seconds: 30,
            retry_attempts: 3,
        };

        let llm_config: LLMProviderConfig = provider_config.clone().into();

        assert_eq!(llm_config.provider, provider_config.provider);
        assert_eq!(llm_config.api_key, provider_config.api_key);
        assert_eq!(llm_config.base_url, provider_config.base_url);
        assert_eq!(llm_config.model, provider_config.model);
        assert_eq!(llm_config.max_tokens, provider_config.max_tokens);
        assert_eq!(llm_config.temperature, provider_config.temperature);
    }

    #[test]
    fn test_config_format_from_str() {
        assert_eq!("json".parse::<ConfigFormat>().unwrap(), ConfigFormat::Json);
        assert_eq!("toml".parse::<ConfigFormat>().unwrap(), ConfigFormat::Toml);
        assert_eq!("yaml".parse::<ConfigFormat>().unwrap(), ConfigFormat::Yaml);
        assert_eq!("yml".parse::<ConfigFormat>().unwrap(), ConfigFormat::Yaml);
        assert!("invalid".parse::<ConfigFormat>().is_err());
    }

    #[test]
    fn test_config_validation() {
        let mut config = LLMConfig::default();

        // Valid config should pass
        assert!(ConfigManager::validate_config_static(&config).is_ok());

        // Invalid default provider should fail
        config.default_provider = Some("nonexistent".to_string());
        assert!(ConfigManager::validate_config_static(&config).is_err());

        // Fix default provider
        config.default_provider = Some("openai".to_string());

        // Invalid model name should fail
        config.providers.get_mut("openai").unwrap().model = "".to_string();
        assert!(ConfigManager::validate_config_static(&config).is_err());

        // Fix model name
        config.providers.get_mut("openai").unwrap().model = "gpt-4".to_string();

        // Invalid temperature should fail
        config.providers.get_mut("openai").unwrap().temperature = 3.0;
        assert!(ConfigManager::validate_config_static(&config).is_err());
    }

    #[test]
    fn test_config_save_load() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config");

        let mut manager = ConfigManager::with_path(&config_path, ConfigFormat::Toml).unwrap();

        // Modify config
        manager.config.default_provider = Some("anthropic".to_string());
        manager.config.providers.get_mut("openai").unwrap().model = "gpt-4".to_string();

        // Save config
        manager.save_config().unwrap();

        // Create new manager and load config
        let manager2 = ConfigManager::with_path(&config_path, ConfigFormat::Toml).unwrap();

        assert_eq!(
            manager2.config.default_provider,
            Some("anthropic".to_string())
        );
        assert_eq!(
            manager2.config.providers.get("openai").unwrap().model,
            "gpt-4".to_string()
        );
    }

    // Static validation method for testing
    impl ConfigManager {
        fn validate_config_static(config: &LLMConfig) -> Result<()> {
            // Validate default provider exists
            if let Some(ref default_provider) = config.default_provider {
                if !config.providers.contains_key(default_provider) {
                    return Err(anyhow!(
                        "Default provider '{}' not found in providers",
                        default_provider
                    ));
                }
            }

            // Validate each provider
            for (name, provider) in &config.providers {
                if !provider.enabled {
                    continue;
                }

                // Validate required fields
                if provider.model.is_empty() {
                    return Err(anyhow!("Provider '{}' has empty model name", name));
                }

                // Validate temperature
                if !(0.0..=2.0).contains(&provider.temperature) {
                    return Err(anyhow!(
                        "Provider '{}' has invalid temperature: {} (must be between 0.0 and 2.0)",
                        name,
                        provider.temperature
                    ));
                }

                // Validate max tokens
                if provider.max_tokens == 0 {
                    return Err(anyhow!(
                        "Provider '{}' has invalid max tokens: {}",
                        name,
                        provider.max_tokens
                    ));
                }
            }

            Ok(())
        }
    }
}
