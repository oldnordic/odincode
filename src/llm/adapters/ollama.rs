//! Ollama Adapter (Phase 5)
//!
//! Ollama local LLM adapter using NDJSON streaming.

use crate::llm::adapters::transport::{SyncTransport, Transport, UreqTransport};
use crate::llm::adapters::{AdapterError, LlmAdapter, LlmMessage, LlmRole};
use serde_json::Value as JsonValue;

// Public parsing module (re-exported for testing)
pub use crate::llm::adapters::ollama_parse::{parse_chat_completion, parse_ndjson_stream};

/// Ollama adapter (local HTTP API)
#[derive(Debug)]
pub struct OllamaAdapter {
    /// Host (e.g., 127.0.0.1)
    host: String,
    /// Port (e.g., 11434)
    port: String,
    /// Model name (e.g., codellama)
    model: String,
    /// HTTP transport
    transport: Transport,
}

impl OllamaAdapter {
    /// Create new Ollama adapter
    pub fn new(host: String, port: String, model: String) -> Self {
        Self {
            host,
            port,
            model,
            transport: Transport::Real(UreqTransport::new()),
        }
    }

    /// Create adapter with custom transport (for testing)
    pub fn with_transport(host: String, port: String, model: String, transport: Transport) -> Self {
        Self {
            host,
            port,
            model,
            transport,
        }
    }

    /// Build base URL
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    /// Get transport (Phase 9.8)
    pub fn transport(&self) -> &Transport {
        &self.transport
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
    /// Converts Vec<LlmMessage> to Ollama messages array format.
    /// This enables multi-turn conversations with proper role-per-message structure.
    pub fn build_chat_stream_messages_request(
        &self,
        messages: &[LlmMessage],
    ) -> Result<String, AdapterError> {
        // Convert LlmMessage to Ollama format
        let ollama_messages: Vec<JsonValue> = messages
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
            "messages": ollama_messages,
            "stream": true
        });

        Ok(request.to_string())
    }

    /// Extract content from JSON response
    fn extract_content(&self, response: &str) -> Result<String, AdapterError> {
        let json: JsonValue = serde_json::from_str(response)?;

        let content = json
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| AdapterError::InvalidResponse("Missing message.content".to_string()))?;

        Ok(content.to_string())
    }
}

impl LlmAdapter for OllamaAdapter {
    fn generate(&self, prompt: &str) -> Result<String, AdapterError> {
        let url = format!("{}/api/chat", self.base_url());
        let body = self.build_request(prompt)?;

        let headers = [("Content-Type", "application/json")];

        let response = self.transport.post_json(&url, &headers, &body)?;
        self.extract_content(&response)
    }

    #[allow(clippy::needless_return)]
    fn generate_streaming<F>(&self, prompt: &str, mut on_chunk: F) -> Result<String, AdapterError>
    where
        F: FnMut(&str),
    {
        let url = format!("{}/api/chat", self.base_url());
        let body = self.build_stream_request(prompt)?;

        let headers = [("Content-Type", "application/json")];

        let mut full_content = String::new();
        let response = self.transport.post_stream(&url, &headers, &body, |line| {
            if let Ok(json) = serde_json::from_str::<JsonValue>(line) {
                // Extract content first (in case this line has done=true)
                if let Some(content) = json
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                {
                    on_chunk(content);
                    full_content.push_str(content);
                }
                // Check if done - closure returns () implicitly
                let _ = json.get("done").and_then(|d| d.as_bool());
            }
        })?;

        // If streaming returned empty or no content, fall back to non-stream
        if full_content.is_empty() && !response.is_empty() {
            // May have gotten single response without done marker
            for line in response.lines() {
                if let Ok(json) = serde_json::from_str::<JsonValue>(line) {
                    if let Some(content) = json
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str())
                    {
                        return Ok(content.to_string());
                    }
                }
            }
        }

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
        let url = format!("{}/api/chat", self.base_url());
        let body = self.build_chat_stream_request(prompt)?;

        let headers = [("Content-Type", "application/json")];

        let mut full_content = String::new();
        let response = self.transport.post_stream(&url, &headers, &body, |line| {
            if let Ok(json) = serde_json::from_str::<JsonValue>(line) {
                // Extract content first (in case this line has done=true)
                if let Some(content) = json
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                {
                    on_chunk(content);
                    full_content.push_str(content);
                }
                // Check if done - closure returns () implicitly
                let _ = json.get("done").and_then(|d| d.as_bool());
            }
        })?;

        // If streaming returned empty or no content, fall back to non-stream
        if full_content.is_empty() && !response.is_empty() {
            // May have gotten single response without done marker
            for line in response.lines() {
                if let Ok(json) = serde_json::from_str::<JsonValue>(line) {
                    if let Some(content) = json
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str())
                    {
                        return Ok(content.to_string());
                    }
                }
            }
        }

        if full_content.is_empty() {
            return self.generate(prompt);
        }

        Ok(full_content)
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn provider_name(&self) -> &str {
        "ollama"
    }
}
