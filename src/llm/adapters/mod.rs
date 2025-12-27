//! LLM Adapters (Phase 5)
//!
//! Provider-agnostic interface for LLM HTTP APIs.
//! Supports GLM, OpenAI, and Ollama.

pub mod factory;

/// LLM message role (universal subset across providers)
///
/// All providers support these three roles. Some providers have additional
/// roles (e.g., OpenAI's "tool"), but we restrict to this universal set
/// for provider-agnostic compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmRole {
    /// System message (sets behavior/context)
    System,
    /// User message (human input)
    User,
    /// Assistant message (LLM response)
    Assistant,
}

/// Single LLM message (provider-agnostic)
///
/// Represents one turn in a conversation. Each message has a role and content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmMessage {
    /// Message role
    pub role: LlmRole,
    /// Message content
    pub content: String,
}
pub mod glm;
pub mod normalize;
pub mod ollama;
pub mod ollama_parse;
pub mod openai;
pub mod openai_parse;
pub mod stub;
pub mod transport;
pub mod transport_fake;
pub mod transport_types;
pub mod transport_ureq;

// Re-export common types
pub use factory::{create_adapter_from_config, create_adapter_from_config_str};
pub use transport::{AdapterError, SyncTransport};

/// LLM adapter trait (Phase 5)
///
/// All providers implement this trait.
/// UI layer calls adapters through this uniform interface.
pub trait LlmAdapter: Send + Sync {
    /// Generate completion from prompt (non-streaming)
    ///
    /// Returns full response text.
    fn generate(&self, prompt: &str) -> Result<String, AdapterError>;

    /// Generate completion with streaming callback
    ///
    /// - Uses PLANNING system prompt (with evidence requirements)
    /// - Calls `on_chunk` for each piece of response
    /// - Returns full response text (concatenated chunks)
    /// - If provider doesn't support streaming, calls once with full response
    fn generate_streaming<F>(&self, prompt: &str, on_chunk: F) -> Result<String, AdapterError>
    where
        F: FnMut(&str);

    /// Generate completion with streaming callback using CHAT mode
    ///
    /// - Uses CHAT system prompt (minimal, conversational)
    /// - Does NOT require evidence or structured plans
    /// - Calls `on_chunk` for each piece of response
    /// - Returns full response text (concatenated chunks)
    fn generate_chat_streaming<F>(&self, prompt: &str, on_chunk: F) -> Result<String, AdapterError>
    where
        F: FnMut(&str);

    /// Check if adapter supports native streaming
    fn supports_streaming(&self) -> bool;

    /// Get provider name for logging
    fn provider_name(&self) -> &str;
}

/// Adapter enum — concrete type for all providers (Phase 5)
///
/// Rust traits with generic methods aren't dyn-compatible.
/// This enum wraps all adapter types, implementing LlmAdapter via delegation.
#[derive(Debug)]
pub enum Adapter {
    OpenAi(openai::OpenAiAdapter),
    Glm(glm::GlmAdapter),
    Ollama(ollama::OllamaAdapter),
    Stub(stub::StubAdapter),
}

impl LlmAdapter for Adapter {
    fn generate(&self, prompt: &str) -> Result<String, AdapterError> {
        match self {
            Adapter::OpenAi(a) => a.generate(prompt),
            Adapter::Glm(a) => a.generate(prompt),
            Adapter::Ollama(a) => a.generate(prompt),
            Adapter::Stub(a) => a.generate(prompt),
        }
    }

    fn generate_streaming<F>(&self, prompt: &str, on_chunk: F) -> Result<String, AdapterError>
    where
        F: FnMut(&str),
    {
        match self {
            Adapter::OpenAi(a) => a.generate_streaming(prompt, on_chunk),
            Adapter::Glm(a) => a.generate_streaming(prompt, on_chunk),
            Adapter::Ollama(a) => a.generate_streaming(prompt, on_chunk),
            Adapter::Stub(a) => a.generate_streaming(prompt, on_chunk),
        }
    }

    fn generate_chat_streaming<F>(&self, prompt: &str, on_chunk: F) -> Result<String, AdapterError>
    where
        F: FnMut(&str),
    {
        match self {
            Adapter::OpenAi(a) => a.generate_chat_streaming(prompt, on_chunk),
            Adapter::Glm(a) => a.generate_chat_streaming(prompt, on_chunk),
            Adapter::Ollama(a) => a.generate_chat_streaming(prompt, on_chunk),
            Adapter::Stub(a) => a.generate_chat_streaming(prompt, on_chunk),
        }
    }

    fn supports_streaming(&self) -> bool {
        match self {
            Adapter::OpenAi(a) => a.supports_streaming(),
            Adapter::Glm(a) => a.supports_streaming(),
            Adapter::Ollama(a) => a.supports_streaming(),
            Adapter::Stub(a) => a.supports_streaming(),
        }
    }

    fn provider_name(&self) -> &str {
        match self {
            Adapter::OpenAi(a) => a.provider_name(),
            Adapter::Glm(a) => a.provider_name(),
            Adapter::Ollama(a) => a.provider_name(),
            Adapter::Stub(a) => a.provider_name(),
        }
    }
}
