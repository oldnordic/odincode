//! ML integration configuration management

use serde::{Deserialize, Serialize};

/// ML integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLIntegrationConfig {
    /// Model name to use
    pub model_name: String,
    /// Confidence threshold for suggestions
    pub confidence_threshold: f32,
    /// Maximum number of suggestions to generate
    pub max_suggestions: usize,
    /// Whether to use LTMC for context
    pub use_ltmc_context: bool,
    /// Whether to use LLM for enhanced analysis
    pub use_llm: bool,
    /// Default LLM provider to use
    pub default_llm_provider: String,
    /// Default LLM model to use
    pub default_llm_model: String,
    /// Maximum tokens for LLM requests
    pub llm_max_tokens: usize,
    /// Temperature for LLM requests
    pub llm_temperature: f64,
    /// LLM configuration
    pub llm_config: LLMConfig,
    /// Model registry configuration
    pub model_registry_config: ModelRegistryConfig,
    /// Performance tracking configuration
    pub performance_tracking_config: PerformanceTrackingConfig,
}

/// LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: String,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f64,
}

/// Model registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistryConfig {
    pub max_models: usize,
    pub cache_size_mb: usize,
    pub max_models_in_memory: usize,
    pub persist_to_disk: bool,
    pub persistence_directory: String,
    pub enable_versioning: bool,
    pub max_versions_per_model: usize,
}

/// Performance tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrackingConfig {
    pub enabled: bool,
    pub retention_days: u32,
}

impl Default for MLIntegrationConfig {
    fn default() -> Self {
        Self {
            model_name: "random_forest".to_string(),
            confidence_threshold: 0.7,
            max_suggestions: 5,
            use_ltmc_context: true,
            use_llm: false,
            default_llm_provider: "openai".to_string(),
            default_llm_model: "gpt-3.5-turbo".to_string(),
            llm_max_tokens: 2048,
            llm_temperature: 0.7,
            llm_config: LLMConfig {
                provider: "openai".to_string(),
                model: "gpt-3.5-turbo".to_string(),
                max_tokens: 2048,
                temperature: 0.7,
            },
            model_registry_config: ModelRegistryConfig {
                max_models: 100,
                cache_size_mb: 1024,
                max_models_in_memory: 100,
                persist_to_disk: true,
                persistence_directory: "./models".to_string(),
                enable_versioning: true,
                max_versions_per_model: 5,
            },
            performance_tracking_config: PerformanceTrackingConfig {
                enabled: true,
                retention_days: 30,
            },
        }
    }
}

impl MLIntegrationConfig {
    /// Create new configuration
    pub fn new(model_name: String, confidence_threshold: f32, max_suggestions: usize) -> Self {
        Self {
            model_name,
            confidence_threshold,
            max_suggestions,
            use_ltmc_context: true,
            use_llm: false,
            default_llm_provider: "openai".to_string(),
            default_llm_model: "gpt-3.5-turbo".to_string(),
            llm_max_tokens: 2048,
            llm_temperature: 0.7,
            llm_config: LLMConfig {
                provider: "openai".to_string(),
                model: "gpt-3.5-turbo".to_string(),
                max_tokens: 2048,
                temperature: 0.7,
            },
            model_registry_config: ModelRegistryConfig {
                max_models: 100,
                cache_size_mb: 1024,
                max_models_in_memory: 100,
                persist_to_disk: true,
                persistence_directory: "./models".to_string(),
                enable_versioning: true,
                max_versions_per_model: 5,
            },
            performance_tracking_config: PerformanceTrackingConfig {
                enabled: true,
                retention_days: 30,
            },
        }
    }

    /// Get model name
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Get confidence threshold
    pub fn confidence_threshold(&self) -> f32 {
        self.confidence_threshold
    }

    /// Get max suggestions
    pub fn max_suggestions(&self) -> usize {
        self.max_suggestions
    }

    /// Check if LTMC context is enabled
    pub fn use_ltmc_context(&self) -> bool {
        self.use_ltmc_context
    }

    /// Check if LLM is enabled
    pub fn use_llm(&self) -> bool {
        self.use_llm
    }

    /// Update model name
    pub fn set_model_name(&mut self, model_name: String) {
        self.model_name = model_name;
    }

    /// Update confidence threshold
    pub fn set_confidence_threshold(&mut self, threshold: f32) {
        self.confidence_threshold = threshold;
    }

    /// Update max suggestions
    pub fn set_max_suggestions(&mut self, max_suggestions: usize) {
        self.max_suggestions = max_suggestions;
    }

    /// Enable/disable LTMC context
    pub fn set_use_ltmc_context(&mut self, use_ltmc: bool) {
        self.use_ltmc_context = use_ltmc;
    }

    /// Enable/disable LLM
    pub fn set_use_llm(&mut self, use_llm: bool) {
        self.use_llm = use_llm;
    }
}
