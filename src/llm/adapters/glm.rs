//! GLM Adapter (Phase 9.8)
//!
//! GLM (Zhipu AI) adapter using OpenAI-compatible API.
//!
//! Base URL: https://api.z.ai/api/coding/paas/v4
//! Endpoint: /chat/completions
//!
//! Wraps OpenAI adapter with GLM-specific base_url and model.
//! Adds message array support for multi-turn conversations.

use crate::llm::adapters::openai::OpenAiAdapter;
use crate::llm::adapters::transport::Transport;
use crate::llm::adapters::{AdapterError, LlmAdapter, LlmMessage, LlmRole};
use serde_json::Value as JsonValue;

/// GLM adapter (OpenAI-compatible API)
#[derive(Debug)]
pub struct GlmAdapter {
    /// Inner OpenAI-compatible adapter
    inner: OpenAiAdapter,
}

impl GlmAdapter {
    /// Create new GLM adapter
    pub fn new(base_url: String, model: String, api_key: String) -> Self {
        Self {
            inner: OpenAiAdapter::new(base_url, model, api_key),
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
            inner: OpenAiAdapter::with_transport(base_url, model, api_key, transport),
        }
    }

    /// Get base URL (Phase 9.8)
    pub fn base_url(&self) -> &str {
        self.inner.base_url()
    }

    /// Get API key (Phase 9.8)
    pub fn api_key(&self) -> &str {
        self.inner.api_key()
    }

    /// Get transport (Phase 9.8)
    pub fn transport(&self) -> &Transport {
        self.inner.transport()
    }

    /// Build streaming chat request body from message array (Phase 9.8)
    ///
    /// Converts Vec<LlmMessage> to GLM/OpenAI-compatible messages array format.
    /// This enables multi-turn conversations with proper role-per-message structure.
    pub fn build_chat_stream_messages_request(
        &self,
        messages: &[LlmMessage],
    ) -> Result<String, AdapterError> {
        // Convert LlmMessage to GLM/OpenAI format
        let glm_messages: Vec<JsonValue> = messages
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
            "model": self.inner.model(),
            "messages": glm_messages,
            "stream": true
        });

        Ok(request.to_string())
    }
}

impl LlmAdapter for GlmAdapter {
    fn generate(&self, prompt: &str) -> Result<String, AdapterError> {
        self.inner.generate(prompt)
    }

    fn generate_streaming<F>(&self, prompt: &str, on_chunk: F) -> Result<String, AdapterError>
    where
        F: FnMut(&str),
    {
        self.inner.generate_streaming(prompt, on_chunk)
    }

    fn generate_chat_streaming<F>(&self, prompt: &str, on_chunk: F) -> Result<String, AdapterError>
    where
        F: FnMut(&str),
    {
        self.inner.generate_chat_streaming(prompt, on_chunk)
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn provider_name(&self) -> &str {
        "glm"
    }
}

/// Parse GLM SSE stream (alias to OpenAI parsing)
///
/// GLM uses the same SSE format as OpenAI.
pub fn parse_sse_stream<F>(body: &str, on_chunk: F) -> Result<String, AdapterError>
where
    F: FnMut(&str),
{
    crate::llm::adapters::openai::parse_sse_stream(body, on_chunk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glm_adapter_provider_name() {
        let adapter = GlmAdapter::new(
            "https://api.z.ai/api/coding/paas/v4".to_string(),
            "GLM-4.7".to_string(),
            "test-key".to_string(),
        );
        assert_eq!(adapter.provider_name(), "glm");
        assert!(adapter.supports_streaming());
    }

    #[test]
    fn test_glm_parse_sse_stream() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"GLM test\"}}]}\n\
                  data: [DONE]\n";

        let mut chunks = Vec::new();
        let result = parse_sse_stream(sse, |c| chunks.push(c.to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "GLM test");
    }
}
