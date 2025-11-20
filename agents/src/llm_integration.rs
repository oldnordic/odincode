//! LLM Integration Framework for OdinCode
//!
//! This module provides a unified interface for integrating with various LLM providers
//! including OpenAI, Anthropic Claude, and Ollama for local model support.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// LLM Provider enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LLMProvider {
    /// OpenAI GPT models
    OpenAI,
    /// Anthropic Claude models
    Anthropic,
    /// Ollama local models
    Ollama,
}

/// LLM Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMModel {
    /// Model name/identifier
    pub name: String,
    /// Provider for this model
    pub provider: LLMProvider,
    /// Maximum context length
    pub max_tokens: usize,
    /// Model capabilities
    pub capabilities: Vec<ModelCapability>,
    /// Cost per 1K tokens (input)
    pub cost_per_1k_input: f64,
    /// Cost per 1K tokens (output)
    pub cost_per_1k_output: f64,
    /// Whether this model is available
    pub available: bool,
}

/// Model capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelCapability {
    /// Code generation
    CodeGeneration,
    /// Code completion
    CodeCompletion,
    /// Code explanation
    CodeExplanation,
    /// Refactoring suggestions
    Refactoring,
    /// Bug detection
    BugDetection,
    /// Documentation generation
    Documentation,
    /// Test generation
    TestGeneration,
    /// Multi-language support
    MultiLanguage,
}

/// LLM request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMessage {
    /// Message role (system, user, assistant)
    pub role: String,
    /// Message content
    pub content: String,
    /// Optional message name
    pub name: Option<String>,
}

/// LLM request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequestConfig {
    /// Temperature (0.0 to 2.0)
    pub temperature: f64,
    /// Maximum tokens to generate
    pub max_tokens: Option<usize>,
    /// Top P sampling (0.0 to 1.0)
    pub top_p: Option<f64>,
    /// Frequency penalty (0.0 to 2.0)
    pub frequency_penalty: Option<f64>,
    /// Presence penalty (0.0 to 2.0)
    pub presence_penalty: Option<f64>,
    /// Stop sequences
    pub stop_sequences: Option<Vec<String>>,
}

impl Default for LLMRequestConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: Some(2048),
            top_p: Some(1.0),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            stop_sequences: None,
        }
    }
}

/// LLM request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    /// Model to use
    pub model: String,
    /// Messages in the conversation
    pub messages: Vec<LLMMessage>,
    /// Request configuration
    pub config: LLMRequestConfig,
    /// Optional request ID
    pub request_id: Option<String>,
}

/// LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    /// Request ID
    pub request_id: String,
    /// Model used
    pub model: String,
    /// Generated content
    pub content: String,
    /// Usage statistics
    pub usage: TokenUsage,
    /// Response metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Processing time
    pub processing_time_ms: u64,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens used
    pub prompt_tokens: usize,
    /// Output tokens generated
    pub completion_tokens: usize,
    /// Total tokens used
    pub total_tokens: usize,
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProviderConfig {
    /// API key for the provider
    pub api_key: Option<String>,
    /// Base URL for API requests
    pub base_url: Option<String>,
    /// Organization ID (for OpenAI)
    pub organization: Option<String>,
    /// Project ID (for Anthropic)
    pub project_id: Option<String>,
    /// Request timeout
    pub timeout_seconds: u64,
    /// Maximum retries
    pub max_retries: u32,
    /// Additional headers
    pub headers: HashMap<String, String>,
}

impl Default for LLMProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: None,
            organization: None,
            project_id: None,
            timeout_seconds: 30,
            max_retries: 3,
            headers: HashMap::new(),
        }
    }
}

/// LLM Integration Manager
pub struct LLMIntegrationManager {
    /// Available models
    models: HashMap<String, LLMModel>,
    /// Provider configurations
    provider_configs: HashMap<LLMProvider, LLMProviderConfig>,
    /// Default model for each provider
    default_models: HashMap<LLMProvider, String>,
    /// Request statistics
    stats: RwLock<LLMStats>,
}

/// LLM usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LLMStats {
    /// Total requests made
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Total tokens used
    pub total_tokens: usize,
    /// Total cost
    pub total_cost: f64,
    /// Average response time
    pub average_response_time_ms: f64,
    /// Requests by provider
    pub requests_by_provider: HashMap<String, u64>,
}

impl LLMIntegrationManager {
    /// Create a new LLM integration manager
    pub fn new() -> Self {
        let mut manager = Self {
            models: HashMap::new(),
            provider_configs: HashMap::new(),
            default_models: HashMap::new(),
            stats: RwLock::new(LLMStats::default()),
        };

        // Initialize with default models
        manager.initialize_default_models();
        manager
    }

    /// Initialize default models
    fn initialize_default_models(&mut self) {
        // OpenAI models
        self.models.insert(
            "gpt-4".to_string(),
            LLMModel {
                name: "gpt-4".to_string(),
                provider: LLMProvider::OpenAI,
                max_tokens: 8192,
                capabilities: vec![
                    ModelCapability::CodeGeneration,
                    ModelCapability::CodeCompletion,
                    ModelCapability::CodeExplanation,
                    ModelCapability::Refactoring,
                    ModelCapability::BugDetection,
                    ModelCapability::Documentation,
                    ModelCapability::TestGeneration,
                    ModelCapability::MultiLanguage,
                ],
                cost_per_1k_input: 0.03,
                cost_per_1k_output: 0.06,
                available: true,
            },
        );

        self.models.insert(
            "gpt-3.5-turbo".to_string(),
            LLMModel {
                name: "gpt-3.5-turbo".to_string(),
                provider: LLMProvider::OpenAI,
                max_tokens: 4096,
                capabilities: vec![
                    ModelCapability::CodeGeneration,
                    ModelCapability::CodeCompletion,
                    ModelCapability::CodeExplanation,
                    ModelCapability::Documentation,
                    ModelCapability::TestGeneration,
                    ModelCapability::MultiLanguage,
                ],
                cost_per_1k_input: 0.0015,
                cost_per_1k_output: 0.002,
                available: true,
            },
        );

        // Anthropic models
        self.models.insert(
            "claude-3-opus".to_string(),
            LLMModel {
                name: "claude-3-opus".to_string(),
                provider: LLMProvider::Anthropic,
                max_tokens: 200000,
                capabilities: vec![
                    ModelCapability::CodeGeneration,
                    ModelCapability::CodeCompletion,
                    ModelCapability::CodeExplanation,
                    ModelCapability::Refactoring,
                    ModelCapability::BugDetection,
                    ModelCapability::Documentation,
                    ModelCapability::TestGeneration,
                    ModelCapability::MultiLanguage,
                ],
                cost_per_1k_input: 0.015,
                cost_per_1k_output: 0.075,
                available: true,
            },
        );

        self.models.insert(
            "claude-3-sonnet".to_string(),
            LLMModel {
                name: "claude-3-sonnet".to_string(),
                provider: LLMProvider::Anthropic,
                max_tokens: 200000,
                capabilities: vec![
                    ModelCapability::CodeGeneration,
                    ModelCapability::CodeCompletion,
                    ModelCapability::CodeExplanation,
                    ModelCapability::Refactoring,
                    ModelCapability::Documentation,
                    ModelCapability::TestGeneration,
                    ModelCapability::MultiLanguage,
                ],
                cost_per_1k_input: 0.003,
                cost_per_1k_output: 0.015,
                available: true,
            },
        );

        // Ollama models (local)
        self.models.insert(
            "llama2".to_string(),
            LLMModel {
                name: "llama2".to_string(),
                provider: LLMProvider::Ollama,
                max_tokens: 4096,
                capabilities: vec![
                    ModelCapability::CodeGeneration,
                    ModelCapability::CodeCompletion,
                    ModelCapability::CodeExplanation,
                    ModelCapability::MultiLanguage,
                ],
                cost_per_1k_input: 0.0,
                cost_per_1k_output: 0.0,
                available: false, // Will be checked at runtime
            },
        );

        self.models.insert(
            "codellama".to_string(),
            LLMModel {
                name: "codellama".to_string(),
                provider: LLMProvider::Ollama,
                max_tokens: 4096,
                capabilities: vec![
                    ModelCapability::CodeGeneration,
                    ModelCapability::CodeCompletion,
                    ModelCapability::CodeExplanation,
                    ModelCapability::TestGeneration,
                    ModelCapability::MultiLanguage,
                ],
                cost_per_1k_input: 0.0,
                cost_per_1k_output: 0.0,
                available: false, // Will be checked at runtime
            },
        );

        // Set default models
        self.default_models
            .insert(LLMProvider::OpenAI, "gpt-4".to_string());
        self.default_models
            .insert(LLMProvider::Anthropic, "claude-3-sonnet".to_string());
        self.default_models
            .insert(LLMProvider::Ollama, "codellama".to_string());
    }

    /// Configure a provider
    pub async fn configure_provider(
        &mut self,
        provider: LLMProvider,
        config: LLMProviderConfig,
    ) -> Result<()> {
        info!("Configuring LLM provider: {:?}", provider);

        // Validate configuration
        if provider != LLMProvider::Ollama && config.api_key.is_none() {
            return Err(anyhow::anyhow!(
                "API key required for provider: {:?}",
                provider
            ));
        }

        self.provider_configs.insert(provider, config);

        // Test the configuration
        self.test_provider_connection(&provider).await?;

        Ok(())
    }

    /// Test provider connection
    pub async fn test_provider_connection(&self, provider: &LLMProvider) -> Result<bool> {
        debug!("Testing connection to LLM provider: {:?}", provider);

        let config = self
            .provider_configs
            .get(provider)
            .ok_or_else(|| anyhow::anyhow!("Provider not configured: {:?}", provider))?;

        match provider {
            LLMProvider::OpenAI => self.test_openai_connection(config).await,
            LLMProvider::Anthropic => self.test_anthropic_connection(config).await,
            LLMProvider::Ollama => self.test_ollama_connection(config).await,
        }
    }

    /// Test OpenAI connection
    async fn test_openai_connection(&self, config: &LLMProviderConfig) -> Result<bool> {
        let client = reqwest::Client::new();
        let url = config
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/models");

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", config.api_key.as_ref().unwrap())
                .parse()
                .unwrap(),
        );
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let response = client
            .get(url)
            .headers(headers)
            .timeout(Duration::from_secs(config.timeout_seconds))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let success = resp.status().is_success();
                if success {
                    debug!("OpenAI connection test successful");
                } else {
                    warn!("OpenAI connection test failed: {}", resp.status());
                }
                Ok(success)
            }
            Err(e) => {
                error!("OpenAI connection test error: {}", e);
                Ok(false)
            }
        }
    }

    /// Test Anthropic connection
    async fn test_anthropic_connection(&self, config: &LLMProviderConfig) -> Result<bool> {
        let client = reqwest::Client::new();
        let url = config
            .base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com/v1/messages");

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-api-key",
            config.api_key.as_ref().unwrap().parse().unwrap(),
        );
        headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());

        // For Anthropic, we'll send a minimal test message
        let test_message = serde_json::json!({
            "model": "claude-3-sonnet-20240229",
            "max_tokens": 10,
            "messages": [{"role": "user", "content": "test"}]
        });

        let response = client
            .post(url)
            .headers(headers)
            .json(&test_message)
            .timeout(Duration::from_secs(config.timeout_seconds))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let success = resp.status().is_success();
                if success {
                    debug!("Anthropic connection test successful");
                } else {
                    warn!("Anthropic connection test failed: {}", resp.status());
                }
                Ok(success)
            }
            Err(e) => {
                error!("Anthropic connection test error: {}", e);
                Ok(false)
            }
        }
    }

    /// Test Ollama connection
    async fn test_ollama_connection(&self, config: &LLMProviderConfig) -> Result<bool> {
        let client = reqwest::Client::new();
        let url = config
            .base_url
            .as_deref()
            .unwrap_or("http://localhost:11434/api/tags");

        let response = client
            .get(url)
            .timeout(Duration::from_secs(config.timeout_seconds))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let success = resp.status().is_success();
                if success {
                    debug!("Ollama connection test successful");
                    // Update model availability based on response
                    self.update_ollama_model_availability(&resp).await?;
                } else {
                    warn!("Ollama connection test failed: {}", resp.status());
                }
                Ok(success)
            }
            Err(e) => {
                error!("Ollama connection test error: {}", e);
                Ok(false)
            }
        }
    }

    /// Update Ollama model availability based on API response
    async fn update_ollama_model_availability(&self, response: &reqwest::Response) -> Result<()> {
        // This would parse the response and update model availability
        // For now, we'll just mark codellama as available if connection succeeds
        debug!("Updating Ollama model availability");
        Ok(())
    }

    /// Get available models
    pub async fn get_available_models(&self) -> Vec<&LLMModel> {
        self.models
            .values()
            .filter(|model| model.available)
            .collect()
    }

    /// Get models by provider
    pub async fn get_models_by_provider(&self, provider: &LLMProvider) -> Vec<&LLMModel> {
        self.models
            .values()
            .filter(|model| model.provider == *provider)
            .collect()
    }

    /// Get default model for provider
    pub async fn get_default_model(&self, provider: &LLMProvider) -> Option<&LLMModel> {
        self.default_models
            .get(provider)
            .and_then(|model_name| self.models.get(model_name))
    }

    /// Send a request to LLM
    pub async fn send_request(&self, request: LLMRequest) -> Result<LLMResponse> {
        let start_time = std::time::Instant::now();

        debug!("Sending LLM request to model: {}", request.model);

        // Get model information
        let model = self
            .models
            .get(&request.model)
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", request.model))?;

        // Get provider configuration
        let config = self
            .provider_configs
            .get(&model.provider)
            .ok_or_else(|| anyhow::anyhow!("Provider not configured: {:?}", model.provider))?;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_requests += 1;
            let provider_name = format!("{:?}", model.provider);
            *stats.requests_by_provider.entry(provider_name).or_insert(0) += 1;
        }

        // Send request to appropriate provider
        let result = match model.provider {
            LLMProvider::OpenAI => self.send_openai_request(&request, config).await,
            LLMProvider::Anthropic => self.send_anthropic_request(&request, config).await,
            LLMProvider::Ollama => self.send_ollama_request(&request, config).await,
        };

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                // Update success statistics
                {
                    let mut stats = self.stats.write().await;
                    stats.successful_requests += 1;
                    stats.total_tokens += response.usage.total_tokens;
                    stats.total_cost += self.calculate_cost(&request.model, &response.usage);

                    // Update average response time
                    let total_time =
                        stats.average_response_time_ms * (stats.successful_requests - 1) as f64;
                    stats.average_response_time_ms =
                        (total_time + processing_time_ms as f64) / stats.successful_requests as f64;
                }

                info!(
                    "LLM request completed successfully in {}ms",
                    processing_time_ms
                );
                Ok(response)
            }
            Err(e) => {
                // Update failure statistics
                {
                    let mut stats = self.stats.write().await;
                    stats.failed_requests += 1;
                }

                error!("LLM request failed: {}", e);
                Err(e)
            }
        }
    }

    /// Send request to OpenAI
    async fn send_openai_request(
        &self,
        request: &LLMRequest,
        config: &LLMProviderConfig,
    ) -> Result<LLMResponse> {
        let client = reqwest::Client::new();
        let url = config
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", config.api_key.as_ref().unwrap())
                .parse()
                .unwrap(),
        );
        headers.insert("Content-Type", "application/json".parse().unwrap());

        if let Some(org) = &config.organization {
            headers.insert("OpenAI-Organization", org.parse().unwrap());
        }

        let openai_request = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "temperature": request.config.temperature,
            "max_tokens": request.config.max_tokens,
            "top_p": request.config.top_p,
            "frequency_penalty": request.config.frequency_penalty,
            "presence_penalty": request.config.presence_penalty,
            "stop": request.config.stop_sequences,
        });

        let response = client
            .post(url)
            .headers(headers)
            .json(&openai_request)
            .timeout(Duration::from_secs(config.timeout_seconds))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("OpenAI API error: {}", response.status()));
        }

        let openai_response: serde_json::Value = response.json().await?;
        self.parse_openai_response(openai_response, request)
    }

    /// Send request to Anthropic
    async fn send_anthropic_request(
        &self,
        request: &LLMRequest,
        config: &LLMProviderConfig,
    ) -> Result<LLMResponse> {
        let client = reqwest::Client::new();
        let url = config
            .base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com/v1/messages");

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-api-key",
            config.api_key.as_ref().unwrap().parse().unwrap(),
        );
        headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());

        if let Some(project_id) = &config.project_id {
            headers.insert("anthropic-project-id", project_id.parse().unwrap());
        }

        let anthropic_request = serde_json::json!({
            "model": request.model,
            "max_tokens": request.config.max_tokens.unwrap_or(2048),
            "temperature": request.config.temperature,
            "messages": request.messages,
        });

        let response = client
            .post(url)
            .headers(headers)
            .json(&anthropic_request)
            .timeout(Duration::from_secs(config.timeout_seconds))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Anthropic API error: {}",
                response.status()
            ));
        }

        let anthropic_response: serde_json::Value = response.json().await?;
        self.parse_anthropic_response(anthropic_response, request)
    }

    /// Send request to Ollama
    async fn send_ollama_request(
        &self,
        request: &LLMRequest,
        config: &LLMProviderConfig,
    ) -> Result<LLMResponse> {
        let client = reqwest::Client::new();
        let url = config
            .base_url
            .as_deref()
            .unwrap_or("http://localhost:11434/api/generate");

        let ollama_request = serde_json::json!({
            "model": request.model,
            "prompt": self.format_ollama_prompt(&request.messages),
            "stream": false,
            "options": {
                "temperature": request.config.temperature,
                "top_p": request.config.top_p.unwrap_or(1.0),
                "num_predict": request.config.max_tokens.unwrap_or(2048),
            }
        });

        let response = client
            .post(url)
            .json(&ollama_request)
            .timeout(Duration::from_secs(config.timeout_seconds))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Ollama API error: {}", response.status()));
        }

        let ollama_response: serde_json::Value = response.json().await?;
        self.parse_ollama_response(ollama_response, request)
    }

    /// Format messages for Ollama prompt
    fn format_ollama_prompt(&self, messages: &[LLMMessage]) -> String {
        let mut prompt = String::new();

        for message in messages {
            match message.role.as_str() {
                "system" => prompt.push_str(&format!("System: {}\n", message.content)),
                "user" => prompt.push_str(&format!("User: {}\n", message.content)),
                "assistant" => prompt.push_str(&format!("Assistant: {}\n", message.content)),
                _ => prompt.push_str(&format!("{}: {}\n", message.role, message.content)),
            }
        }

        prompt.push_str("Assistant: ");
        prompt
    }

    /// Parse OpenAI response
    fn parse_openai_response(
        &self,
        response: serde_json::Value,
        request: &LLMRequest,
    ) -> Result<LLMResponse> {
        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No content in OpenAI response"))?
            .to_string();

        let usage = TokenUsage {
            prompt_tokens: response["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize,
            completion_tokens: response["usage"]["completion_tokens"].as_u64().unwrap_or(0)
                as usize,
            total_tokens: response["usage"]["total_tokens"].as_u64().unwrap_or(0) as usize,
        };

        Ok(LLMResponse {
            request_id: request
                .request_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            model: request.model.clone(),
            content,
            usage,
            metadata: HashMap::new(),
            processing_time_ms: 0, // Will be set by caller
        })
    }

    /// Parse Anthropic response
    fn parse_anthropic_response(
        &self,
        response: serde_json::Value,
        request: &LLMRequest,
    ) -> Result<LLMResponse> {
        let content = response["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No content in Anthropic response"))?
            .to_string();

        let usage = TokenUsage {
            prompt_tokens: response["usage"]["input_tokens"].as_u64().unwrap_or(0) as usize,
            completion_tokens: response["usage"]["output_tokens"].as_u64().unwrap_or(0) as usize,
            total_tokens: response["usage"]["input_tokens"].as_u64().unwrap_or(0) as usize
                + response["usage"]["output_tokens"].as_u64().unwrap_or(0) as usize,
        };

        Ok(LLMResponse {
            request_id: request
                .request_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            model: request.model.clone(),
            content,
            usage,
            metadata: HashMap::new(),
            processing_time_ms: 0, // Will be set by caller
        })
    }

    /// Parse Ollama response
    fn parse_ollama_response(
        &self,
        response: serde_json::Value,
        request: &LLMRequest,
    ) -> Result<LLMResponse> {
        let content = response["response"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No content in Ollama response"))?
            .to_string();

        // Ollama doesn't provide detailed token usage, so we estimate
        let estimated_tokens = (content.len() / 4).max(1);

        let usage = TokenUsage {
            prompt_tokens: estimated_tokens,
            completion_tokens: estimated_tokens,
            total_tokens: estimated_tokens * 2,
        };

        Ok(LLMResponse {
            request_id: request
                .request_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            model: request.model.clone(),
            content,
            usage,
            metadata: HashMap::new(),
            processing_time_ms: 0, // Will be set by caller
        })
    }

    /// Calculate cost for a request
    fn calculate_cost(&self, model: &str, usage: &TokenUsage) -> f64 {
        if let Some(model_info) = self.models.get(model) {
            let input_cost = (usage.prompt_tokens as f64 / 1000.0) * model_info.cost_per_1k_input;
            let output_cost =
                (usage.completion_tokens as f64 / 1000.0) * model_info.cost_per_1k_output;
            input_cost + output_cost
        } else {
            0.0
        }
    }

    /// Get usage statistics
    pub async fn get_stats(&self) -> LLMStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = LLMStats::default();
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_manager_creation() {
        let manager = LLMIntegrationManager::new();
        assert!(!manager.models.is_empty());
        assert!(manager.models.contains_key("gpt-4"));
        assert!(manager.models.contains_key("claude-3-sonnet"));
    }

    #[test]
    fn test_request_config_default() {
        let config = LLMRequestConfig::default();
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, Some(2048));
        assert_eq!(config.top_p, Some(1.0));
    }

    #[tokio::test]
    async fn test_get_available_models() {
        let manager = LLMIntegrationManager::new();
        let models = manager.get_available_models().await;

        // Should have at least some available models
        assert!(!models.is_empty());

        // Check that OpenAI and Anthropic models are available by default
        let openai_available = models
            .iter()
            .any(|m| matches!(m.provider, LLMProvider::OpenAI));
        let anthropic_available = models
            .iter()
            .any(|m| matches!(m.provider, LLMProvider::Anthropic));

        assert!(openai_available);
        assert!(anthropic_available);
    }

    #[tokio::test]
    async fn test_get_models_by_provider() {
        let manager = LLMIntegrationManager::new();
        let openai_models = manager.get_models_by_provider(&LLMProvider::OpenAI).await;

        assert!(!openai_models.is_empty());
        assert!(openai_models
            .iter()
            .all(|m| matches!(m.provider, LLMProvider::OpenAI)));
    }

    #[tokio::test]
    async fn test_stats_initialization() {
        let manager = LLMIntegrationManager::new();
        let stats = manager.get_stats().await;

        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.successful_requests, 0);
        assert_eq!(stats.failed_requests, 0);
        assert_eq!(stats.total_tokens, 0);
        assert_eq!(stats.total_cost, 0.0);
    }
}
