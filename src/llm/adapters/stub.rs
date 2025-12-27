//! Stub Adapter (Phase 5)
//!
//! Testing adapter that returns fake responses without network calls.
//! Used for integration tests and when LLM is not available.

use crate::llm::adapters::{AdapterError, LlmAdapter, LlmMessage, LlmRole};
use serde_json::Value as JsonValue;

/// Stub adapter for testing (returns fake responses)
#[derive(Debug)]
pub struct StubAdapter {
    /// Fake response to return
    response: String,
}

impl StubAdapter {
    /// Create new stub adapter with default fake response
    pub fn new() -> Self {
        Self {
            response: Self::default_response(),
        }
    }

    /// Create stub adapter with custom response
    pub fn with_response(response: String) -> Self {
        Self { response }
    }

    /// Build streaming chat request body from message array (Phase 9.8)
    ///
    /// Stub implementation that returns a fake request JSON for testing.
    pub fn build_chat_stream_messages_request(
        &self,
        messages: &[LlmMessage],
    ) -> Result<String, AdapterError> {
        // Convert LlmMessage to generic format (same as OpenAI)
        let formatted_messages: Vec<JsonValue> = messages
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
            "model": "stub-model",
            "messages": formatted_messages,
            "stream": true
        });

        Ok(request.to_string())
    }

    /// Build streaming chat request body for single prompt (Phase 9.8 regression test)
    ///
    /// Stub implementation that returns a fake request JSON for testing.
    pub fn build_chat_stream_request(&self, prompt: &str) -> Result<String, AdapterError> {
        let system = crate::llm::contracts::chat_system_prompt();
        let messages = serde_json::json!([
            {"role": "system", "content": system},
            {"role": "user", "content": prompt}
        ]);

        let request = serde_json::json!({
            "model": "stub-model",
            "messages": messages,
            "stream": true
        });

        Ok(request.to_string())
    }

    /// Default fake plan response
    fn default_response() -> String {
        r#"{"plan_id":"plan_stub_001","intent":"READ","steps":[{"step_id":"step_1","tool":"file_read","arguments":{"file_path":"src/lib.rs"},"precondition":"file exists","requires_confirmation":false}],"evidence_referenced":[]}"#.to_string()
    }
}

impl Default for StubAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmAdapter for StubAdapter {
    fn generate(&self, _prompt: &str) -> Result<String, AdapterError> {
        Ok(self.response.clone())
    }

    fn generate_streaming<F>(&self, _prompt: &str, mut on_chunk: F) -> Result<String, AdapterError>
    where
        F: FnMut(&str),
    {
        // Emit response in chunks for realism
        let chunk_size = 20;
        let mut chars = self.response.chars().peekable();

        while chars.peek().is_some() {
            let chunk: String = chars.by_ref().take(chunk_size).collect();
            on_chunk(&chunk);
        }

        Ok(self.response.clone())
    }

    fn generate_chat_streaming<F>(
        &self,
        _prompt: &str,
        mut on_chunk: F,
    ) -> Result<String, AdapterError>
    where
        F: FnMut(&str),
    {
        // For chat mode, return a conversational test response
        let chat_response =
            "Hello! I'm OdinCode, your AI programming assistant. How can I help you today?";
        let chunk_size = 20;
        let mut chars = chat_response.chars().peekable();

        while chars.peek().is_some() {
            let chunk: String = chars.by_ref().take(chunk_size).collect();
            on_chunk(&chunk);
        }

        Ok(chat_response.to_string())
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn provider_name(&self) -> &str {
        "stub"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_adapter_returns_default_response() {
        let adapter = StubAdapter::new();
        let result = adapter.generate("test prompt");
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("plan_id"));
        assert!(content.contains("plan_stub_001"));
    }

    #[test]
    fn test_stub_adapter_with_custom_response() {
        let adapter = StubAdapter::with_response("custom response".to_string());
        let result = adapter.generate("test");
        assert_eq!(result.unwrap(), "custom response");
    }

    #[test]
    fn test_stub_adapter_streaming() {
        let adapter = StubAdapter::new();
        let mut chunks = Vec::new();
        let result = adapter.generate_streaming("test", |c| chunks.push(c.to_string()));
        assert!(result.is_ok());
        assert!(!chunks.is_empty(), "Should emit at least one chunk");
        let full = result.unwrap();
        assert!(full.contains("plan_id"));
    }

    #[test]
    fn test_stub_adapter_provider_name() {
        let adapter = StubAdapter::new();
        assert_eq!(adapter.provider_name(), "stub");
        assert!(adapter.supports_streaming());
    }
}
