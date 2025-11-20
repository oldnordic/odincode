//! LLM Integration Module
//!
//! This module provides integration with various LLM providers (OpenAI, Anthropic, Ollama)
//! for intelligent code analysis and suggestions.

use super::config::{ConfigManager, ProviderConfig};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// LLM Provider enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
    Ollama,
}

/// Analysis type for intelligent model selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AnalysisType {
    CodeCompletion,
    BugDetection,
    Refactoring,
    Documentation,
    PerformanceOptimization,
    SecurityAnalysis,
    GeneralAnalysis,
}

/// Model capability score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapability {
    pub model_name: String,
    pub provider: LLMProvider,
    pub capability_score: f64,
    pub speed_score: f64,
    pub cost_score: f64,
    pub reliability_score: f64,
}

/// Intelligent model selector
pub struct IntelligentModelSelector {
    pub capability_matrix: HashMap<AnalysisType, Vec<ModelCapability>>,
    pub default_models: HashMap<AnalysisType, String>,
}

impl IntelligentModelSelector {
    /// Create a new intelligent model selector
    pub fn new() -> Self {
        let mut capability_matrix = HashMap::new();
        let mut default_models = HashMap::new();

        // Code completion capabilities
        capability_matrix.insert(
            AnalysisType::CodeCompletion,
            vec![
                ModelCapability {
                    model_name: "gpt-4".to_string(),
                    provider: LLMProvider::OpenAI,
                    capability_score: 0.95,
                    speed_score: 0.7,
                    cost_score: 0.6,
                    reliability_score: 0.95,
                },
                ModelCapability {
                    model_name: "claude-3-sonnet-20240229".to_string(),
                    provider: LLMProvider::Anthropic,
                    capability_score: 0.92,
                    speed_score: 0.8,
                    cost_score: 0.7,
                    reliability_score: 0.90,
                },
                ModelCapability {
                    model_name: "llama2".to_string(),
                    provider: LLMProvider::Ollama,
                    capability_score: 0.75,
                    speed_score: 0.9,
                    cost_score: 0.95,
                    reliability_score: 0.8,
                },
            ],
        );
        default_models.insert(AnalysisType::CodeCompletion, "gpt-4".to_string());

        // Bug detection capabilities
        capability_matrix.insert(
            AnalysisType::BugDetection,
            vec![
                ModelCapability {
                    model_name: "gpt-4".to_string(),
                    provider: LLMProvider::OpenAI,
                    capability_score: 0.98,
                    speed_score: 0.6,
                    cost_score: 0.5,
                    reliability_score: 0.98,
                },
                ModelCapability {
                    model_name: "claude-3-sonnet-20240229".to_string(),
                    provider: LLMProvider::Anthropic,
                    capability_score: 0.96,
                    speed_score: 0.7,
                    cost_score: 0.6,
                    reliability_score: 0.95,
                },
                ModelCapability {
                    model_name: "llama2".to_string(),
                    provider: LLMProvider::Ollama,
                    capability_score: 0.70,
                    speed_score: 0.85,
                    cost_score: 0.98,
                    reliability_score: 0.75,
                },
            ],
        );
        default_models.insert(AnalysisType::BugDetection, "gpt-4".to_string());

        // Refactoring capabilities
        capability_matrix.insert(
            AnalysisType::Refactoring,
            vec![
                ModelCapability {
                    model_name: "gpt-4".to_string(),
                    provider: LLMProvider::OpenAI,
                    capability_score: 0.94,
                    speed_score: 0.65,
                    cost_score: 0.55,
                    reliability_score: 0.94,
                },
                ModelCapability {
                    model_name: "claude-3-sonnet-20240229".to_string(),
                    provider: LLMProvider::Anthropic,
                    capability_score: 0.93,
                    speed_score: 0.75,
                    cost_score: 0.65,
                    reliability_score: 0.92,
                },
                ModelCapability {
                    model_name: "llama2".to_string(),
                    provider: LLMProvider::Ollama,
                    capability_score: 0.72,
                    speed_score: 0.88,
                    cost_score: 0.96,
                    reliability_score: 0.78,
                },
            ],
        );
        default_models.insert(AnalysisType::Refactoring, "gpt-4".to_string());

        // Documentation capabilities
        capability_matrix.insert(
            AnalysisType::Documentation,
            vec![
                ModelCapability {
                    model_name: "gpt-4".to_string(),
                    provider: LLMProvider::OpenAI,
                    capability_score: 0.96,
                    speed_score: 0.75,
                    cost_score: 0.65,
                    reliability_score: 0.96,
                },
                ModelCapability {
                    model_name: "claude-3-sonnet-20240229".to_string(),
                    provider: LLMProvider::Anthropic,
                    capability_score: 0.98,
                    speed_score: 0.8,
                    cost_score: 0.7,
                    reliability_score: 0.97,
                },
                ModelCapability {
                    model_name: "llama2".to_string(),
                    provider: LLMProvider::Ollama,
                    capability_score: 0.80,
                    speed_score: 0.9,
                    cost_score: 0.97,
                    reliability_score: 0.82,
                },
            ],
        );
        default_models.insert(
            AnalysisType::Documentation,
            "claude-3-sonnet-20240229".to_string(),
        );

        // Performance optimization capabilities
        capability_matrix.insert(
            AnalysisType::PerformanceOptimization,
            vec![
                ModelCapability {
                    model_name: "gpt-4".to_string(),
                    provider: LLMProvider::OpenAI,
                    capability_score: 0.93,
                    speed_score: 0.6,
                    cost_score: 0.5,
                    reliability_score: 0.93,
                },
                ModelCapability {
                    model_name: "claude-3-sonnet-20240229".to_string(),
                    provider: LLMProvider::Anthropic,
                    capability_score: 0.91,
                    speed_score: 0.7,
                    cost_score: 0.6,
                    reliability_score: 0.90,
                },
                ModelCapability {
                    model_name: "llama2".to_string(),
                    provider: LLMProvider::Ollama,
                    capability_score: 0.68,
                    speed_score: 0.85,
                    cost_score: 0.95,
                    reliability_score: 0.72,
                },
            ],
        );
        default_models.insert(AnalysisType::PerformanceOptimization, "gpt-4".to_string());

        // Security analysis capabilities
        capability_matrix.insert(
            AnalysisType::SecurityAnalysis,
            vec![
                ModelCapability {
                    model_name: "gpt-4".to_string(),
                    provider: LLMProvider::OpenAI,
                    capability_score: 0.97,
                    speed_score: 0.55,
                    cost_score: 0.45,
                    reliability_score: 0.97,
                },
                ModelCapability {
                    model_name: "claude-3-sonnet-20240229".to_string(),
                    provider: LLMProvider::Anthropic,
                    capability_score: 0.95,
                    speed_score: 0.65,
                    cost_score: 0.55,
                    reliability_score: 0.94,
                },
                ModelCapability {
                    model_name: "llama2".to_string(),
                    provider: LLMProvider::Ollama,
                    capability_score: 0.65,
                    speed_score: 0.8,
                    cost_score: 0.94,
                    reliability_score: 0.70,
                },
            ],
        );
        default_models.insert(AnalysisType::SecurityAnalysis, "gpt-4".to_string());

        // General analysis capabilities
        capability_matrix.insert(
            AnalysisType::GeneralAnalysis,
            vec![
                ModelCapability {
                    model_name: "gpt-4".to_string(),
                    provider: LLMProvider::OpenAI,
                    capability_score: 0.94,
                    speed_score: 0.7,
                    cost_score: 0.6,
                    reliability_score: 0.94,
                },
                ModelCapability {
                    model_name: "claude-3-sonnet-20240229".to_string(),
                    provider: LLMProvider::Anthropic,
                    capability_score: 0.93,
                    speed_score: 0.8,
                    cost_score: 0.7,
                    reliability_score: 0.92,
                },
                ModelCapability {
                    model_name: "llama2".to_string(),
                    provider: LLMProvider::Ollama,
                    capability_score: 0.75,
                    speed_score: 0.9,
                    cost_score: 0.96,
                    reliability_score: 0.80,
                },
            ],
        );
        default_models.insert(AnalysisType::GeneralAnalysis, "gpt-4".to_string());

        Self {
            capability_matrix,
            default_models,
        }
    }

    /// Select the best model for a given analysis type
    pub fn select_best_model(
        &self,
        analysis_type: &AnalysisType,
        preferences: &ModelSelectionPreferences,
    ) -> Option<&ModelCapability> {
        let capabilities = self.capability_matrix.get(analysis_type)?;

        if capabilities.is_empty() {
            return None;
        }

        // Calculate weighted scores for each model
        let mut scored_models: Vec<_> = capabilities
            .iter()
            .map(|cap| {
                let weighted_score = cap.capability_score * preferences.capability_weight
                    + cap.speed_score * preferences.speed_weight
                    + cap.cost_score * preferences.cost_weight
                    + cap.reliability_score * preferences.reliability_weight;
                (cap, weighted_score)
            })
            .collect();

        // Sort by weighted score (descending)
        scored_models.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return the best model
        scored_models.first().map(|(cap, _)| *cap)
    }

    /// Get the default model for a given analysis type
    pub fn get_default_model(&self, analysis_type: &AnalysisType) -> Option<&str> {
        self.default_models.get(analysis_type).map(|s| s.as_str())
    }

    /// Get all available models for a given analysis type
    pub fn get_available_models(&self, analysis_type: &AnalysisType) -> Vec<&ModelCapability> {
        self.capability_matrix
            .get(analysis_type)
            .map(|caps| caps.iter().collect())
            .unwrap_or_default()
    }

    /// Update capability scores based on performance feedback
    pub fn update_capability_scores(
        &mut self,
        analysis_type: AnalysisType,
        model_name: &str,
        performance_feedback: &PerformanceFeedback,
    ) {
        if let Some(capabilities) = self.capability_matrix.get_mut(&analysis_type) {
            if let Some(capability) = capabilities
                .iter_mut()
                .find(|cap| cap.model_name == model_name)
            {
                // Update scores based on feedback
                capability.capability_score =
                    capability.capability_score * 0.9 + performance_feedback.quality_score * 0.1;
                capability.speed_score =
                    capability.speed_score * 0.9 + performance_feedback.speed_score * 0.1;
                capability.reliability_score = capability.reliability_score * 0.9
                    + performance_feedback.reliability_score * 0.1;

                // Ensure scores stay within bounds
                capability.capability_score = capability.capability_score.clamp(0.0, 1.0);
                capability.speed_score = capability.speed_score.clamp(0.0, 1.0);
                capability.reliability_score = capability.reliability_score.clamp(0.0, 1.0);
            }
        }
    }
}

/// Model selection preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelectionPreferences {
    pub capability_weight: f64,
    pub speed_weight: f64,
    pub cost_weight: f64,
    pub reliability_weight: f64,
}

impl Default for ModelSelectionPreferences {
    fn default() -> Self {
        Self {
            capability_weight: 0.5,
            speed_weight: 0.2,
            cost_weight: 0.2,
            reliability_weight: 0.1,
        }
    }
}

/// Performance feedback for model selection learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceFeedback {
    pub quality_score: f64,
    pub speed_score: f64,
    pub reliability_score: f64,
    pub user_satisfaction: Option<f64>,
}

/// LLM Message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMessage {
    pub role: String,
    pub content: String,
    pub name: Option<String>,
}

/// LLM Request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequestConfig {
    pub temperature: f64,
    pub max_tokens: Option<usize>,
    pub top_p: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
    pub stop_sequences: Option<Vec<String>>,
}

impl Default for LLMRequestConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: Some(1000),
            top_p: Some(1.0),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            stop_sequences: None,
        }
    }
}

/// LLM Request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub model: String,
    pub messages: Vec<LLMMessage>,
    pub config: LLMRequestConfig,
    pub request_id: Option<Uuid>,
}

/// LLM Response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<LLMUsage>,
    pub request_id: Option<Uuid>,
}

/// LLM Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// LLM Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProviderConfig {
    pub provider: LLMProvider,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f64,
}

/// Main LLM Integration Manager
pub struct LLMIntegrationManager {
    /// HTTP client for API requests
    pub client: Client,
    /// Configuration manager
    pub config_manager: ConfigManager,
    /// Configured providers
    pub providers: HashMap<String, LLMProviderConfig>,
    /// Default provider name
    pub default_provider: String,
    /// Intelligent model selector
    pub model_selector: IntelligentModelSelector,
    /// Model selection preferences
    pub selection_preferences: ModelSelectionPreferences,
}

impl LLMIntegrationManager {
    /// Create a new LLM integration manager
    pub fn new() -> Result<Self> {
        Self::new_with_config_manager(ConfigManager::new()?)
    }

    /// Create a new LLM integration manager with custom config manager
    pub fn new_with_config_manager(config_manager: ConfigManager) -> Result<Self> {
        let client = Client::new();
        let config = config_manager.get_config();

        // Convert provider configs
        let providers: HashMap<String, LLMProviderConfig> = config
            .providers
            .iter()
            .filter(|(_, provider_config)| provider_config.enabled)
            .map(|(name, provider_config)| (name.clone(), provider_config.clone().into()))
            .collect();

        // Get default provider
        let default_provider = config
            .default_provider
            .clone()
            .unwrap_or_else(|| "openai".to_string());

        // Create model selection preferences from config
        let selection_preferences = ModelSelectionPreferences {
            capability_weight: config.model_selection.capability_weight,
            speed_weight: config.model_selection.speed_weight,
            cost_weight: config.model_selection.cost_weight,
            reliability_weight: config.model_selection.reliability_weight,
        };

        Ok(Self {
            client,
            config_manager,
            providers,
            default_provider,
            model_selector: IntelligentModelSelector::new(),
            selection_preferences,
        })
    }

    /// Create a new LLM integration manager with custom config path
    pub fn new_with_config_path<P: AsRef<std::path::Path>>(
        path: P,
        format: super::config::ConfigFormat,
    ) -> Result<Self> {
        let config_manager = ConfigManager::with_path(path, format)?;
        Self::new_with_config_manager(config_manager)
    }

    /// Reload configuration
    pub fn reload_config(&mut self) -> Result<()> {
        self.config_manager.load_config()?;
        self.config_manager.apply_env_overrides()?;

        // Validate configuration
        self.config_manager.validate_config()?;

        // Update providers and settings
        let config = self.config_manager.get_config();

        // Convert provider configs
        self.providers = config
            .providers
            .iter()
            .filter(|(_, provider_config)| provider_config.enabled)
            .map(|(name, provider_config)| (name.clone(), provider_config.clone().into()))
            .collect();

        // Update default provider
        self.default_provider = config
            .default_provider
            .clone()
            .unwrap_or_else(|| "openai".to_string());

        // Update model selection preferences
        self.selection_preferences = ModelSelectionPreferences {
            capability_weight: config.model_selection.capability_weight,
            speed_weight: config.model_selection.speed_weight,
            cost_weight: config.model_selection.cost_weight,
            reliability_weight: config.model_selection.reliability_weight,
        };

        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Save current configuration
    pub fn save_config(&self) -> Result<()> {
        self.config_manager.save_config()
    }

    /// Get configuration manager reference
    pub fn get_config_manager(&self) -> &ConfigManager {
        &self.config_manager
    }

    /// Get mutable configuration manager reference
    pub fn get_config_manager_mut(&mut self) -> &mut ConfigManager {
        &mut self.config_manager
    }

    /// Add or update a provider configuration
    pub fn add_provider(&mut self, name: String, config: LLMProviderConfig) {
        // Convert to provider config and add to configuration manager
        let provider_config = ProviderConfig {
            provider: config.provider.clone(),
            api_key: config.api_key.clone(),
            base_url: config.base_url.clone(),
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            enabled: true,
            timeout_seconds: 30,
            retry_attempts: 3,
        };

        self.config_manager
            .add_provider(name.clone(), provider_config);
        self.providers.insert(name, config);
        info!("Added/updated LLM provider configuration");
    }

    /// Set the default provider
    pub fn set_default_provider(&mut self, provider_name: String) -> Result<()> {
        if self.providers.contains_key(&provider_name) {
            let provider_name_clone = provider_name.clone();
            self.default_provider = provider_name;
            info!("Set default LLM provider to: {}", provider_name_clone);
            Ok(())
        } else {
            Err(anyhow!("Provider '{}' not found", provider_name))
        }
    }

    /// Enable or disable a provider
    pub fn set_provider_enabled(&mut self, provider_name: &str, enabled: bool) -> Result<()> {
        self.config_manager
            .set_provider_enabled(provider_name, enabled)?;

        if enabled {
            // If enabling, add the provider back to the active providers
            if let Some(config) = self
                .config_manager
                .get_config()
                .providers
                .get(provider_name)
            {
                self.providers
                    .insert(provider_name.to_string(), config.clone().into());
            }
        } else {
            // If disabling, remove from active providers
            self.providers.remove(provider_name);
        }

        info!(
            "{} provider: {}",
            if enabled { "Enabled" } else { "Disabled" },
            provider_name
        );
        Ok(())
    }

    /// Remove a provider configuration
    pub fn remove_provider(&mut self, provider_name: &str) -> Result<()> {
        self.config_manager.remove_provider(provider_name)?;
        self.providers.remove(provider_name);
        info!("Removed LLM provider: {}", provider_name);
        Ok(())
    }

    /// Get a provider configuration by name
    pub fn get_provider(&self, name: &str) -> Option<&LLMProviderConfig> {
        self.providers.get(name)
    }

    /// Send a request to the specified LLM provider
    pub async fn send_request(&self, request: LLMRequest) -> Result<LLMResponse> {
        let provider_name =
            if request.model.contains("claude") || request.model.contains("anthropic") {
                "anthropic"
            } else if request.model.contains("llama") || request.model.contains("mistral") {
                "ollama"
            } else {
                &self.default_provider
            };

        let provider_config = self
            .providers
            .get(provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_name))?;

        debug!("Sending request to LLM provider: {}", provider_name);

        match provider_config.provider {
            LLMProvider::OpenAI => self.send_openai_request(request, provider_config).await,
            LLMProvider::Anthropic => self.send_anthropic_request(request, provider_config).await,
            LLMProvider::Ollama => self.send_ollama_request(request, provider_config).await,
        }
    }

    /// Send request to OpenAI
    async fn send_openai_request(
        &self,
        request: LLMRequest,
        config: &LLMProviderConfig,
    ) -> Result<LLMResponse> {
        let base_url = config
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        let url = format!("{}/chat/completions", base_url);

        let openai_request = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "temperature": request.config.temperature,
            "max_tokens": request.config.max_tokens,
            "top_p": request.config.top_p,
            "frequency_penalty": request.config.frequency_penalty,
            "presence_penalty": request.config.presence_penalty,
        });

        let mut req_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(api_key) = &config.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = req_builder.json(&openai_request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenAI API error: {}", error_text));
        }

        let response_json: serde_json::Value = response.json().await?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = response_json.get("usage").map(|usage| LLMUsage {
            prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as usize,
            completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as usize,
            total_tokens: usage["total_tokens"].as_u64().unwrap_or(0) as usize,
        });

        Ok(LLMResponse {
            content,
            model: request.model,
            usage,
            request_id: request.request_id,
        })
    }

    /// Send request to Anthropic
    async fn send_anthropic_request(
        &self,
        request: LLMRequest,
        config: &LLMProviderConfig,
    ) -> Result<LLMResponse> {
        let base_url = config
            .base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com");
        let url = format!("{}/v1/messages", base_url);

        // Convert messages to Anthropic format
        let mut system_message = String::new();
        let mut user_messages = Vec::new();

        for message in &request.messages {
            match message.role.as_str() {
                "system" => system_message = message.content.clone(),
                "user" | "assistant" => user_messages.push(message.clone()),
                _ => {}
            }
        }

        let anthropic_request = serde_json::json!({
            "model": request.model,
            "max_tokens": request.config.max_tokens.unwrap_or(1000),
            "temperature": request.config.temperature,
            "system": system_message,
            "messages": user_messages,
        });

        let mut req_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-version", "2023-06-01");

        if let Some(api_key) = &config.api_key {
            req_builder = req_builder.header("x-api-key", api_key);
        }

        let response = req_builder.json(&anthropic_request).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Anthropic API error: {}", error_text));
        }

        let response_json: serde_json::Value = response.json().await?;

        let content = response_json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = response_json.get("usage").map(|usage| LLMUsage {
            prompt_tokens: usage["input_tokens"].as_u64().unwrap_or(0) as usize,
            completion_tokens: usage["output_tokens"].as_u64().unwrap_or(0) as usize,
            total_tokens: usage["input_tokens"].as_u64().unwrap_or(0) as usize
                + usage["output_tokens"].as_u64().unwrap_or(0) as usize,
        });

        Ok(LLMResponse {
            content,
            model: request.model,
            usage,
            request_id: request.request_id,
        })
    }

    /// Send request to Ollama
    async fn send_ollama_request(
        &self,
        request: LLMRequest,
        config: &LLMProviderConfig,
    ) -> Result<LLMResponse> {
        let base_url = config
            .base_url
            .as_deref()
            .unwrap_or("http://localhost:11434");
        let url = format!("{}/api/generate", base_url);

        let ollama_request = serde_json::json!({
            "model": request.model,
            "prompt": request.messages.first().map(|m| &m.content).unwrap_or(&"".to_string()),
            "stream": false,
            "options": {
                "temperature": request.config.temperature,
                "top_p": request.config.top_p,
                "max_tokens": request.config.max_tokens,
            }
        });

        let response = self
            .client
            .post(&url)
            .json(&ollama_request)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if response.status().is_success() {
            let response_text = response.text().await?;
            let ollama_response: serde_json::Value = serde_json::from_str(&response_text)?;

            Ok(LLMResponse {
                content: ollama_response["response"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                model: request.model,
                usage: None,
                request_id: request.request_id,
            })
        } else {
            Err(anyhow::anyhow!(
                "Ollama request failed: {}",
                response.status()
            ))
        }
    }

    /// Generate a response from the LLM (convenience method)
    pub async fn generate_response(&self, prompt: &str) -> Result<String> {
        let request = LLMRequest {
            model: "gpt-4".to_string(), // Default model
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
                name: None,
            }],
            config: LLMRequestConfig::default(),
            request_id: None,
        };

        let response = self.send_request(request).await?;
        Ok(response.content)
    }

    /// Test connection to a provider
    pub async fn test_connection(&self, provider_name: &str) -> Result<bool> {
        let config = self
            .providers
            .get(provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_name))?;

        let test_request = LLMRequest {
            model: config.model.clone(),
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: "Hello, this is a test message.".to_string(),
                name: None,
            }],
            config: LLMRequestConfig {
                max_tokens: Some(10),
                temperature: 0.7,
                ..Default::default()
            },
            request_id: None,
        };

        match self.send_request(test_request).await {
            Ok(_) => {
                info!("Connection test successful for provider: {}", provider_name);
                Ok(true)
            }
            Err(e) => {
                warn!(
                    "Connection test failed for provider {}: {}",
                    provider_name, e
                );
                Ok(false)
            }
        }
    }

    /// Get available providers
    pub fn get_available_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// Get provider statistics
    pub fn get_provider_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();

        for (name, config) in &self.providers {
            let provider_stats = serde_json::json!({
                "provider": format!("{:?}", config.provider),
                "model": config.model,
                "max_tokens": config.max_tokens,
                "temperature": config.temperature,
                "has_api_key": config.api_key.is_some(),
                "base_url": config.base_url,
            });
            stats.insert(name.clone(), provider_stats);
        }

        stats
    }

    /// Select the best model for a given analysis type
    pub fn select_best_model_for_analysis(&self, analysis_type: AnalysisType) -> Option<String> {
        self.model_selector
            .select_best_model(&analysis_type, &self.selection_preferences)
            .map(|cap| cap.model_name.clone())
    }

    /// Get intelligent model recommendation with fallback
    pub fn get_intelligent_model_recommendation(
        &self,
        analysis_type: AnalysisType,
        preferred_model: Option<String>,
    ) -> String {
        // First try to use the preferred model if it's available and suitable
        if let Some(preferred) = preferred_model {
            if self.is_model_suitable_for_analysis(&preferred, &analysis_type) {
                return preferred;
            }
        }

        // Fall back to intelligent selection
        self.select_best_model_for_analysis(analysis_type.clone())
            .or_else(|| {
                self.model_selector
                    .get_default_model(&analysis_type)
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "gpt-4".to_string()) // Ultimate fallback
    }

    /// Check if a model is suitable for a given analysis type
    pub fn is_model_suitable_for_analysis(
        &self,
        model_name: &str,
        analysis_type: &AnalysisType,
    ) -> bool {
        self.model_selector
            .get_available_models(analysis_type)
            .iter()
            .any(|cap| cap.model_name == model_name)
    }

    /// Update model selection preferences
    pub fn update_selection_preferences(&mut self, preferences: ModelSelectionPreferences) {
        self.selection_preferences = preferences;
        info!("Updated model selection preferences");
    }

    /// Provide performance feedback to improve model selection
    pub fn provide_performance_feedback(
        &mut self,
        analysis_type: AnalysisType,
        model_name: &str,
        feedback: PerformanceFeedback,
    ) {
        self.model_selector
            .update_capability_scores(analysis_type.clone(), model_name, &feedback);
        info!(
            "Updated capability scores for model: {} on analysis: {:?}",
            model_name, analysis_type
        );
    }

    /// Get model selection explanation
    pub fn get_model_selection_explanation(
        &self,
        analysis_type: AnalysisType,
        selected_model: &str,
    ) -> Option<String> {
        let capabilities = self.model_selector.get_available_models(&analysis_type);
        let selected_cap = capabilities
            .iter()
            .find(|cap| cap.model_name == selected_model)?;

        let explanation = format!(
            "Selected model '{}' for {:?} analysis based on:\n\
            - Capability score: {:.2}\n\
            - Speed score: {:.2}\n\
            - Cost score: {:.2}\n\
            - Reliability score: {:.2}\n\
            - Weighted score: {:.2}",
            selected_model,
            analysis_type,
            selected_cap.capability_score,
            selected_cap.speed_score,
            selected_cap.cost_score,
            selected_cap.reliability_score,
            selected_cap.capability_score * self.selection_preferences.capability_weight
                + selected_cap.speed_score * self.selection_preferences.speed_weight
                + selected_cap.cost_score * self.selection_preferences.cost_weight
                + selected_cap.reliability_score * self.selection_preferences.reliability_weight
        );

        Some(explanation)
    }

    /// Auto-detect analysis type from code context
    pub fn detect_analysis_type_from_context(
        &self,
        code_context: &str,
        user_intent: Option<&str>,
    ) -> AnalysisType {
        let code_lower = code_context.to_lowercase();
        let intent_lower = user_intent.unwrap_or("").to_lowercase();

        // Check for security-related patterns
        if code_lower.contains("password")
            || code_lower.contains("auth")
            || code_lower.contains("encrypt")
            || code_lower.contains("decrypt")
            || code_lower.contains("token")
            || code_lower.contains("session")
            || intent_lower.contains("security")
            || intent_lower.contains("vulnerability")
        {
            return AnalysisType::SecurityAnalysis;
        }

        // Check for performance-related patterns
        if code_lower.contains("performance")
            || code_lower.contains("optimize")
            || code_lower.contains("slow")
            || code_lower.contains("fast")
            || code_lower.contains("memory")
            || code_lower.contains("cpu")
            || intent_lower.contains("performance")
            || intent_lower.contains("optimize")
        {
            return AnalysisType::PerformanceOptimization;
        }

        // Check for documentation-related patterns
        if code_lower.contains("todo")
            || code_lower.contains("fixme")
            || code_lower.contains("doc")
            || intent_lower.contains("document")
            || intent_lower.contains("explain")
            || intent_lower.contains("comment")
        {
            return AnalysisType::Documentation;
        }

        // Check for refactoring-related patterns
        if code_lower.contains("refactor")
            || code_lower.contains("clean")
            || code_lower.contains("simplify")
            || intent_lower.contains("refactor")
            || intent_lower.contains("improve")
            || intent_lower.contains("clean")
        {
            return AnalysisType::Refactoring;
        }

        // Check for bug detection patterns
        if code_lower.contains("bug")
            || code_lower.contains("error")
            || code_lower.contains("fix")
            || intent_lower.contains("bug")
            || intent_lower.contains("error")
            || intent_lower.contains("debug")
        {
            return AnalysisType::BugDetection;
        }

        // Check for code completion patterns
        if code_lower.ends_with(".")
            || code_lower.ends_with("::")
            || code_lower.contains("autocomplete")
            || intent_lower.contains("complete")
            || intent_lower.contains("finish")
            || intent_lower.contains("autocomplete")
        {
            return AnalysisType::CodeCompletion;
        }

        // Default to general analysis
        AnalysisType::GeneralAnalysis
    }

    /// Generate intelligent response with automatic model selection
    pub async fn generate_intelligent_response(
        &self,
        prompt: &str,
        code_context: Option<&str>,
        user_intent: Option<&str>,
    ) -> Result<(String, String)> {
        // Detect analysis type
        let analysis_type = if let Some(context) = code_context {
            self.detect_analysis_type_from_context(context, user_intent)
        } else {
            AnalysisType::GeneralAnalysis
        };

        // Select best model
        let selected_model = self.get_intelligent_model_recommendation(analysis_type.clone(), None);

        // Create request with selected model
        let request = LLMRequest {
            model: selected_model.clone(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: format!("You are an expert code analysis assistant specializing in {:?} analysis. Provide helpful, accurate, and concise responses.", analysis_type),
                    name: None,
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                    name: None,
                }
            ],
            config: LLMRequestConfig::default(),
            request_id: None,
        };

        // Send request
        let response = self.send_request(request).await?;

        // Get explanation for model selection
        let explanation = self
            .get_model_selection_explanation(analysis_type.clone(), &selected_model)
            .unwrap_or_else(|| {
                format!(
                    "Selected model: {} for {:?} analysis",
                    selected_model, analysis_type
                )
            });

        Ok((response.content, explanation))
    }

    /// Generate analysis response with enriched semantic context
    pub async fn generate_analysis_with_semantic_context(
        &self,
        code_file: &crate::CodeFile,
        semantic_analysis: &crate::semantic_analysis::SemanticAnalysis,
        analysis_request: &str,
    ) -> Result<String> {
        // Generate a rich context from semantic analysis
        let context_description = self.format_semantic_context(semantic_analysis, code_file)?;

        // Create a detailed prompt combining the code, semantic analysis results, and user request
        let detailed_prompt = format!(
            "Code Analysis with Semantic Context:\n\n{}\n\nSemantic Analysis Results:\n{}\n\nUser Request: {}\n\nProvide a detailed analysis based on both the code and the semantic analysis results.",
            code_file.content,
            context_description,
            analysis_request
        );

        // Detect analysis type from request
        let analysis_type =
            self.detect_analysis_type_from_context(&detailed_prompt, Some(analysis_request));

        // Select the most appropriate model for this type of analysis
        let selected_model = self.get_intelligent_model_recommendation(analysis_type.clone(), None);

        // Create the request
        let request = LLMRequest {
            model: selected_model.clone(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: "You are an expert code analysis assistant. Analyze the provided code with its semantic analysis results and respond to the user's request. Consider complexity metrics, identified patterns, and dependency structures in your analysis. Provide specific, actionable recommendations.".to_string(),
                    name: None,
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: detailed_prompt,
                    name: None,
                }
            ],
            config: LLMRequestConfig {
                temperature: 0.3,  // Lower temperature for more consistent, analytical responses
                max_tokens: Some(2000),
                top_p: Some(0.9),
                frequency_penalty: Some(0.2),
                presence_penalty: Some(0.2),
                stop_sequences: None,
            },
            request_id: None,
        };

        // Send the request
        let response = self.send_request(request).await?;

        Ok(response.content)
    }

    /// Format semantic context for LLM consumption
    fn format_semantic_context(
        &self,
        semantic_analysis: &crate::semantic_analysis::SemanticAnalysis,
        code_file: &crate::CodeFile,
    ) -> Result<String> {
        let mut context = String::new();

        // Add complexity metrics
        context.push_str(&format!(
            "Complexity Metrics:\n- Cyclomatic Complexity: {:.2}\n- Cognitive Complexity: {:.2}\n- Halstead Volume: {:.2}\n- Lines of Code: {}\n- Comment Ratio: {:.2}\n- Function Count: {}\n- Class Count: {}\n- Parameter Count: {}\n\n",
            semantic_analysis.complexity_metrics.cyclomatic_complexity,
            semantic_analysis.complexity_metrics.cognitive_complexity,
            semantic_analysis.complexity_metrics.halstead_volume,
            semantic_analysis.complexity_metrics.loc,
            semantic_analysis.complexity_metrics.comment_ratio,
            semantic_analysis.complexity_metrics.function_count,
            semantic_analysis.complexity_metrics.class_count,
            semantic_analysis.complexity_metrics.parameter_count
        ));

        // Add identified patterns
        if !semantic_analysis.identified_patterns.is_empty() {
            context.push_str("Identified Patterns:\n");
            for pattern in &semantic_analysis.identified_patterns {
                context.push_str(&format!(
                    "- {}: {} (Confidence: {:.2}, Lines {}-{})\n  Description: {}\n  Suggestions: {}\n\n",
                    match &pattern.pattern_type {
                        crate::semantic_analysis::PatternType::AntiPattern => "Anti-Pattern",
                        crate::semantic_analysis::PatternType::DesignPattern => "Design Pattern", 
                        crate::semantic_analysis::PatternType::CodeSmell => "Code Smell",
                        crate::semantic_analysis::PatternType::PerformanceIssue => "Performance Issue",
                        crate::semantic_analysis::PatternType::SecurityVulnerability => "Security Vulnerability",
                        crate::semantic_analysis::PatternType::BestPractice => "Best Practice",
                        crate::semantic_analysis::PatternType::ArchitecturalPattern => "Architectural Pattern",
                        crate::semantic_analysis::PatternType::RefactoringOpportunity => "Refactoring Opportunity",
                    },
                    pattern.name,
                    pattern.confidence,
                    pattern.start_line,
                    pattern.end_line,
                    pattern.description,
                    pattern.suggestions.join(", ")
                ));
            }
            context.push('\n');
        }

        // Add dependency information
        context.push_str(&format!(
            "Dependency Graph: {} nodes, {} edges\n",
            semantic_analysis.dependency_graph.nodes.len(),
            semantic_analysis.dependency_graph.edges.len()
        ));

        Ok(context)
    }

    /// Generate code suggestions using both semantic analysis and LLM intelligence
    pub async fn generate_enhanced_suggestions(
        &self,
        code_file: &crate::CodeFile,
        semantic_analysis: &crate::semantic_analysis::SemanticAnalysis,
    ) -> Result<Vec<crate::CodeSuggestion>> {
        let mut suggestions = Vec::new();

        // Start with the semantic analysis suggestions
        for pattern in &semantic_analysis.identified_patterns {
            let suggestion_type = match &pattern.pattern_type {
                crate::semantic_analysis::PatternType::AntiPattern
                | crate::semantic_analysis::PatternType::CodeSmell
                | crate::semantic_analysis::PatternType::RefactoringOpportunity => {
                    crate::SuggestionType::Refactor
                }
                crate::semantic_analysis::PatternType::PerformanceIssue
                | crate::semantic_analysis::PatternType::SecurityVulnerability => {
                    crate::SuggestionType::Optimize
                }
                crate::semantic_analysis::PatternType::BestPractice
                | crate::semantic_analysis::PatternType::DesignPattern => {
                    crate::SuggestionType::Document
                }
                _ => crate::SuggestionType::Refactor,
            };

            suggestions.push(crate::CodeSuggestion::complete(
                pattern.id,
                suggestion_type,
                pattern.description.clone(),
                pattern.description.clone(),
                pattern.suggestions.first().cloned(),
                (pattern.confidence * 0.7) as f32, // Base confidence from semantic analysis
                code_file.path.clone(),
                None,
                crate::Severity::Info,
                false,
            ));
        }

        // Enhance with LLM analysis
        let llm_prompt = format!(
            "Analyze this code and provide specific, actionable suggestions for improvement based on the semantic analysis results:\n\nCode:\n{}\n\nSemantic Analysis:\n{}",
            code_file.content,
            self.format_semantic_context(semantic_analysis, code_file)?
        );

        let llm_response = self
            .generate_analysis_with_semantic_context(code_file, semantic_analysis, &llm_prompt)
            .await?;

        // Parse the LLM response for additional suggestions
        let llm_suggestions = self.parse_llm_suggestions(&llm_response, code_file)?;
        suggestions.extend(llm_suggestions);

        Ok(suggestions)
    }

    /// Parse LLM response to extract structured suggestions
    fn parse_llm_suggestions(
        &self,
        llm_response: &str,
        code_file: &crate::CodeFile,
    ) -> Result<Vec<crate::CodeSuggestion>> {
        let mut suggestions = Vec::new();

        // This is a simplified parser - in a real implementation, you would want more robust parsing
        // Look for patterns like "Suggestion: [description]" or "Recommendation: [description]"
        let lines: Vec<&str> = llm_response.lines().collect();

        for line in lines {
            if line.to_lowercase().contains("suggestion:")
                || line.to_lowercase().contains("recommendation:")
            {
                let description = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
                if !description.is_empty() {
                    suggestions.push(crate::CodeSuggestion::complete(
                        uuid::Uuid::new_v4(),
                        crate::SuggestionType::Refactor, // Default type
                        description.clone(),
                        description,
                        Some("// Implementation needed".to_string()),
                        0.6, // Default confidence for LLM suggestions
                        code_file.path.clone(),
                        None,
                        crate::Severity::Info,
                        false,
                    ));
                }
            }
        }

        Ok(suggestions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_llm_manager_creation() {
        let manager = LLMIntegrationManager::new().unwrap();
        assert!(!manager.providers.is_empty());
        assert_eq!(manager.default_provider, "openai");
    }

    #[tokio::test]
    async fn test_add_provider() {
        let mut manager = LLMIntegrationManager::new().unwrap();

        let config = LLMProviderConfig {
            provider: LLMProvider::OpenAI,
            api_key: Some("test_key".to_string()),
            base_url: Some("https://api.test.com".to_string()),
            model: "test-model".to_string(),
            max_tokens: 500,
            temperature: 0.5,
        };

        manager.add_provider("test".to_string(), config);
        assert!(manager.providers.contains_key("test"));
    }

    #[tokio::test]
    async fn test_set_default_provider() {
        let mut manager = LLMIntegrationManager::new().unwrap();

        let result = manager.set_default_provider("anthropic".to_string());
        assert!(result.is_ok());
        assert_eq!(manager.default_provider, "anthropic");

        let result = manager.set_default_provider("nonexistent".to_string());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_available_providers() {
        let manager = LLMIntegrationManager::new().unwrap();
        let providers = manager.get_available_providers();

        assert!(providers.contains(&"openai".to_string()));
        assert!(providers.contains(&"anthropic".to_string()));
        assert!(providers.contains(&"ollama".to_string()));
    }

    #[tokio::test]
    async fn test_get_provider_stats() {
        let manager = LLMIntegrationManager::new().unwrap();
        let stats = manager.get_provider_stats();

        assert!(!stats.is_empty());
        assert!(stats.contains_key("openai"));
    }
}
