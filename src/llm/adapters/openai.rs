//! OpenAI Adapter (Phase 5)
//!
//! OpenAI-compatible HTTP API adapter.
//! Used by both OpenAI and GLM providers.

use crate::llm::adapters::transport::{SyncTransport, Transport, UreqTransport};
use crate::llm::adapters::{AdapterError, LlmAdapter, LlmMessage, LlmRole};
use serde_json::Value as JsonValue;

// Public parsing module (re-exported for testing)
pub use crate::llm::adapters::openai_parse::{parse_chat_completion, parse_sse_stream};

/// OpenAI-compatible adapter
#[derive(Debug)]
pub struct OpenAiAdapter {
    /// Base URL (e.g., https://api.openai.com/v1)
    base_url: String,
    /// Model name (e.g., gpt-4)
    model: String,
    /// API key
    api_key: String,
    /// HTTP transport
    transport: Transport,
}

impl OpenAiAdapter {
    /// Create new OpenAI adapter
    pub fn new(base_url: String, model: String, api_key: String) -> Self {
        Self {
            base_url,
            model,
            api_key,
            transport: Transport::Real(UreqTransport::new()),
        }
    }

    /// Create adapter with custom transport (for testing)
    pub fn with_transport(
        base_url: String,
        model: String,
        api_key: String,
        transport: Transport,
    ) -> Self {
        Self {
            base_url,
            model,
            api_key,
            transport,
        }
    }

    /// Get base URL (Phase 9.8)
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get API key (Phase 9.8)
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Get transport (Phase 9.8)
    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    /// Get model name (Phase 9.8)
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Build chat request body
    fn build_request(&self, prompt: &str) -> Result<String, AdapterError> {
        let system = crate::llm::contracts::system_prompt();
        let messages = serde_json::json!([
            {"role": "system", "content": system},
            {"role": "user", "content": prompt}
        ]);

        let request = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false
        });

        Ok(request.to_string())
    }

    /// Build streaming chat request body
    fn build_stream_request(&self, prompt: &str) -> Result<String, AdapterError> {
        let system = crate::llm::contracts::system_prompt();
        let messages = serde_json::json!([
            {"role": "system", "content": system},
            {"role": "user", "content": prompt}
        ]);

        let request = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true
        });

        Ok(request.to_string())
    }

    /// Build streaming chat request body for CHAT mode
    ///
    /// Uses minimal, conversational system prompt instead of full planning prompt.
    pub fn build_chat_stream_request(&self, prompt: &str) -> Result<String, AdapterError> {
        let system = crate::llm::contracts::chat_system_prompt();
        let messages = serde_json::json!([
            {"role": "system", "content": system},
            {"role": "user", "content": prompt}
        ]);

        let request = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true
        });

        Ok(request.to_string())
    }

    /// Build streaming chat request body from message array (Phase 9.8)
    ///
    /// Converts Vec<LlmMessage> to OpenAI messages array format.
    /// This enables multi-turn conversations with proper role-per-message structure.
    pub fn build_chat_stream_messages_request(
        &self,
        messages: &[LlmMessage],
    ) -> Result<String, AdapterError> {
        // Convert LlmMessage to OpenAI format
        let openai_messages: Vec<JsonValue> = messages
            .iter()
            .map(|msg| {
                let role_str = match msg.role {
                    LlmRole::System => "system",
                    LlmRole::User => "user",
                    LlmRole::Assistant => "assistant",
                };
                serde_json::json!({
                    "role": role_str,
                    "content": msg.content
                })
            })
            .collect();

        let request = serde_json::json!({
            "model": self.model,
            "messages": openai_messages,
            "stream": true
        });

        Ok(request.to_string())
    }

    /// Extract content from JSON response
    fn extract_content(&self, response: &str) -> Result<String, AdapterError> {
        let json: JsonValue = serde_json::from_str(response)?;

        let content = json["choices"]
            .get(0)
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| {
                AdapterError::InvalidResponse("Missing choices[0].message.content".to_string())
            })?;

        Ok(content.to_string())
    }
}

impl LlmAdapter for OpenAiAdapter {
    fn generate(&self, prompt: &str) -> Result<String, AdapterError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = self.build_request(prompt)?;

        let auth_header = format!("Bearer {}", self.api_key);
        let headers = [
            ("Authorization", auth_header.as_str()),
            ("Content-Type", "application/json"),
        ];

        let response = self.transport.post_json(&url, &headers, &body)?;
        self.extract_content(&response)
    }

    fn generate_streaming<F>(&self, prompt: &str, mut on_chunk: F) -> Result<String, AdapterError>
    where
        F: FnMut(&str),
    {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = self.build_stream_request(prompt)?;

        let auth_header = format!("Bearer {}", self.api_key);
        let headers = [
            ("Authorization", auth_header.as_str()),
            ("Content-Type", "application/json"),
        ];

        let mut full_content = String::new();
        let _response = self.transport.post_stream(&url, &headers, &body, |line| {
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    return;
                }
                if let Ok(json) = serde_json::from_str::<JsonValue>(data) {
                    if let Some(delta) = json["choices"]
                        .get(0)
                        .and_then(|c| c.get("delta"))
                        .and_then(|d| d.get("content"))
                    {
                        if let Some(text) = delta.as_str() {
                            on_chunk(text);
                            full_content.push_str(text);
                        }
                    }
                }
            }
        })?;

        // If streaming returned empty or no content, fall back to non-stream
        if full_content.is_empty() {
            return self.generate(prompt);
        }

        Ok(full_content)
    }

    fn generate_chat_streaming<F>(
        &self,
        prompt: &str,
        mut on_chunk: F,
    ) -> Result<String, AdapterError>
    where
        F: FnMut(&str),
    {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = self.build_chat_stream_request(prompt)?;

        let auth_header = format!("Bearer {}", self.api_key);
        let headers = [
            ("Authorization", auth_header.as_str()),
            ("Content-Type", "application/json"),
        ];

        let mut full_content = String::new();
        let _response = self.transport.post_stream(&url, &headers, &body, |line| {
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    return;
                }
                if let Ok(json) = serde_json::from_str::<JsonValue>(data) {
                    if let Some(delta) = json["choices"]
                        .get(0)
                        .and_then(|c| c.get("delta"))
                        .and_then(|d| d.get("content"))
                    {
                        if let Some(text) = delta.as_str() {
                            on_chunk(text);
                            full_content.push_str(text);
                        }
                    }
                }
            }
        })?;

        // If streaming returned empty or no content, fall back to non-stream
        if full_content.is_empty() {
            return self.generate(prompt);
        }

        Ok(full_content)
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}
