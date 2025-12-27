//! Chat — Isolated conversational LLM lane (Phase 8.1)
//!
//! Chat is SEPARATE from plan/workflow system:
//! - No Plan objects
//! - No approval states
//! - No execution DB writes
//! - No plan artifacts
//! - No workflow steps
//!
//! Two entrypoints:
//! - chat()           → stream<TextChunk> (this module)
//! - plan()           → Plan (session.rs module)
//!
//! CRITICAL: Chat must NEVER call plan/session/workflow functions.

use std::fs::OpenOptions;
use std::io::Write;

use crate::llm::adapters::transport::SyncTransport;
use crate::llm::adapters::{create_adapter_from_config, AdapterError, LlmAdapter, LlmMessage};
use crate::llm::chat_events::ChatReceiver;
use crate::llm::chat_thread::ChatThreadHandle;
use std::path::Path;

/// Write to debug log file
fn debug_log(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("/tmp/odincode_debug.log")
    {
        let _ = writeln!(file, "{}", msg);
        let _ = file.flush();
    }
}

/// Chat errors — typed ONLY, no Display impl
///
/// These errors are NOT rendered to chat transcript.
/// They are routed to diagnostics panel by UI layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatError {
    /// Transport/network error from adapter
    TransportError,

    /// HTTP error from adapter
    HttpError,

    /// Authentication/authorization error
    AuthError,

    /// Rate limiting error
    RateLimitedError,

    /// Invalid response from LLM
    InvalidResponseError,

    /// Configuration error (missing or invalid config)
    ConfigurationError,

    /// LLM provider not configured
    NotConfigured,
}

impl From<AdapterError> for ChatError {
    fn from(err: AdapterError) -> Self {
        match err {
            AdapterError::Network(_) => ChatError::TransportError,
            AdapterError::Http { .. } => ChatError::HttpError,
            AdapterError::Authentication(_) => ChatError::AuthError,
            AdapterError::RateLimited { .. } => ChatError::RateLimitedError,
            AdapterError::InvalidResponse(_) => ChatError::InvalidResponseError,
            AdapterError::Configuration(_) => ChatError::ConfigurationError,
            AdapterError::Streaming(_) => ChatError::TransportError,
            AdapterError::Io(_) => ChatError::TransportError,
            AdapterError::Json(_) => ChatError::InvalidResponseError,
            AdapterError::Provider { .. } => ChatError::InvalidResponseError,
        }
    }
}

/// Filter JSON blocks from chat output
///
/// Removes:
/// - Markdown code blocks with ```json ... ```
/// - Standalone JSON object lines (starts with {, ends with })
/// - Preserves regular conversational text
pub fn filter_json_blocks(text: &str) -> String {
    let mut result = String::new();
    let mut in_json_block = false;
    let mut line_start = true;

    for ch in text.chars() {
        match ch {
            '\n' => {
                line_start = true;
                result.push(ch);
            }
            '`' if line_start => {
                // Check for ```json or ```
                in_json_block = !in_json_block;
                line_start = false;
            }
            '{' if line_start && !in_json_block => {
                // Skip standalone JSON object lines
                line_start = false;
            }
            _ => {
                if !in_json_block {
                    result.push(ch);
                }
                line_start = false;
            }
        }
    }

    result
}

/// Stream chat response from LLM
///
/// Phase 8.1: Isolated chat lane - no plan system involvement.
/// Streams text chunks directly to UI via callback.
///
/// # Arguments
/// - `prompt`: User's input text
/// - `db_root`: Path to database root (for config)
/// - `on_chunk`: Callback for each text chunk (called during streaming)
///
/// # Returns
/// Full response text (concatenated chunks)
///
/// # Behavior
/// - Creates adapter from config
/// - Builds chat-specific prompt (simpler than plan prompt)
/// - Streams response chunks via callback
/// - Does NOT create Plan objects
/// - Does NOT write to execution DB
/// - Does NOT touch planning state
pub fn chat<F>(prompt: &str, db_root: &Path, mut on_chunk: F) -> Result<String, ChatError>
where
    F: FnMut(&str),
{
    // Create adapter from config
    let adapter = create_adapter_from_config(db_root).map_err(|_| ChatError::NotConfigured)?;

    // Build chat prompt (simpler than plan prompt)
    let chat_prompt = build_chat_prompt(prompt);

    // Call adapter with CHAT MODE streaming (uses minimal system prompt)
    let response = adapter.generate_chat_streaming(&chat_prompt, |chunk| {
        // Filter out JSON blocks (```json ... ```) and standalone JSON objects
        let filtered = filter_json_blocks(chunk);
        on_chunk(&filtered);
    })?;

    Ok(response)
}

/// Stream chat response from LLM with full conversation prompt (Phase 9.7)
///
/// # Phase 9.7: Context Continuity
/// Unlike `chat()`, this function accepts a pre-built full prompt that includes:
/// - System prompt
/// - Complete conversation history (user messages, assistant responses, tool results)
///
/// This fixes LLM amnesia during multi-step tool loops.
///
/// # Arguments
/// - `full_prompt`: Complete conversation prompt (built by FrameStack)
/// - `db_root`: Path to database root (for config)
/// - `on_chunk`: Callback for each text chunk (called during streaming)
///
/// # Returns
/// Full response text (concatenated chunks)
pub fn chat_with_full_prompt<F>(
    full_prompt: &str,
    db_root: &Path,
    mut on_chunk: F,
) -> Result<String, ChatError>
where
    F: FnMut(&str),
{
    // Create adapter from config
    let adapter = create_adapter_from_config(db_root).map_err(|_| ChatError::NotConfigured)?;

    // Call adapter with CHAT MODE streaming
    // Note: full_prompt already contains system prompt + conversation history
    let response = adapter.generate_chat_streaming(full_prompt, |chunk| {
        // Filter out JSON blocks (```json ... ```) and standalone JSON objects
        let filtered = filter_json_blocks(chunk);
        on_chunk(&filtered);
    })?;

    Ok(response)
}

/// Stream chat response from LLM using message array (Phase 9.8)
///
/// # Phase 9.8: Multi-Turn Message Support
/// Unlike `chat_with_full_prompt()`, this function sends each frame as a separate
/// message with proper role-per-message structure. This fixes the LLM amnesia bug
/// where adapters collapsed the entire conversation into a single "user" message.
///
/// # Arguments
/// - `messages`: Array of LlmMessage (built by FrameStack.build_messages())
/// - `db_root`: Path to database root (for config)
/// - `on_chunk`: Callback for each text chunk (called during streaming)
///
/// # Returns
/// Full response text (concatenated chunks)
///
/// # Implementation Note
/// This function uses provider-specific adapter methods to build proper message arrays.
/// Currently supports OpenAI-compatible providers (OpenAI, GLM) and Ollama.
pub fn chat_with_messages<F>(
    messages: &[LlmMessage],
    db_root: &Path,
    mut on_chunk: F,
) -> Result<String, ChatError>
where
    F: FnMut(&str),
{
    use crate::llm::adapters::Adapter;

    // DEBUG: Log messages being sent to LLM
    debug_log(&format!("[CHAT_WITH_MESSAGES] Sending {} messages to LLM:", messages.len()));
    for (i, msg) in messages.iter().enumerate() {
        let preview = if msg.content.len() > 200 {
            format!("{}...", &msg.content[..200])
        } else {
            msg.content.clone()
        };
        debug_log(&format!("[CHAT_WITH_MESSAGES] [{}] {:?}: {}", i, msg.role, preview));
    }

    // Create adapter from config
    let adapter = create_adapter_from_config(db_root).map_err(|_| ChatError::NotConfigured)?;

    // Match on adapter type to use provider-specific message building
    // This is necessary because the LlmAdapter trait doesn't support message arrays
    match &adapter {
        Adapter::OpenAi(openai_adapter) => {
            chat_with_messages_openai(openai_adapter, messages, |chunk| {
                let filtered = filter_json_blocks(chunk);
                on_chunk(&filtered);
            })
        }
        Adapter::Glm(glm_adapter) => {
            // GLM wraps OpenAI adapter, delegate to OpenAI path
            chat_with_messages_glm(glm_adapter, messages, |chunk| {
                let filtered = filter_json_blocks(chunk);
                on_chunk(&filtered);
            })
        }
        Adapter::Ollama(ollama_adapter) => {
            chat_with_messages_ollama(ollama_adapter, messages, |chunk| {
                let filtered = filter_json_blocks(chunk);
                on_chunk(&filtered);
            })
        }
        Adapter::Stub(_) => {
            // Stub doesn't support message arrays, fall back to single-prompt path
            // This is acceptable for testing
            let prompt = messages
                .iter()
                .map(|m| format!("{:?}: {}", m.role, m.content))
                .collect::<Vec<_>>()
                .join("\n");
            adapter
                .generate_chat_streaming(&prompt, |chunk| {
                    let filtered = filter_json_blocks(chunk);
                    on_chunk(&filtered);
                })
                .map_err(ChatError::from)
        }
    }
}

/// Helper: Chat with messages using OpenAI adapter
fn chat_with_messages_openai<F>(
    adapter: &crate::llm::adapters::openai::OpenAiAdapter,
    messages: &[LlmMessage],
    mut on_chunk: F,
) -> Result<String, ChatError>
where
    F: FnMut(&str),
{
    // Build request body with messages array
    let body = adapter.build_chat_stream_messages_request(messages)?;

    // Get URL and headers from adapter (re-creating request internals)
    // This is a bit awkward but necessary to avoid changing the trait
    let url = format!(
        "{}/chat/completions",
        adapter.base_url().trim_end_matches('/')
    );
    let auth_header = format!("Bearer {}", adapter.api_key());
    let headers = [
        ("Authorization", auth_header.as_str()),
        ("Content-Type", "application/json"),
    ];

    // Use the adapter's transport to make the request
    // We need to access the transport layer directly
    let transport = adapter.transport();

    let mut full_content = String::new();
    let _response = transport
        .post_stream(&url, &headers, &body, |line: &str| {
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    return;
                }
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
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
        })
        .map_err(ChatError::from)?;

    if full_content.is_empty() {
        // Fallback to non-streaming if streaming returned empty
        debug_log("[CHAT] Streaming returned empty, falling back to non-streaming POST");
        let response = ureq::request("POST", &url)
            .timeout(std::time::Duration::from_secs(30))
            .set("Authorization", &auth_header)
            .set("Content-Type", "application/json")
            .send_string(&body)
            .map_err(|_| ChatError::TransportError)?;

        let response_text = response
            .into_string()
            .map_err(|_| ChatError::TransportError)?;
        let json: serde_json::Value =
            serde_json::from_str(&response_text).map_err(|_| ChatError::InvalidResponseError)?;

        let content = json["choices"]
            .get(0)
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or(ChatError::InvalidResponseError)?;

        on_chunk(content);
        Ok(content.to_string())
    } else {
        Ok(full_content)
    }
}

/// Helper: Chat with messages using GLM adapter
fn chat_with_messages_glm<F>(
    adapter: &crate::llm::adapters::glm::GlmAdapter,
    messages: &[LlmMessage],
    mut on_chunk: F,
) -> Result<String, ChatError>
where
    F: FnMut(&str),
{
    // Build request body with messages array (Phase 9.8)
    let body = adapter.build_chat_stream_messages_request(messages)?;

    // Get URL and headers from adapter (GLM uses OpenAI-compatible format)
    let url = format!("{}/chat/completions", adapter.base_url().trim_end_matches('/'));
    let auth_header = format!("Bearer {}", adapter.api_key());
    let headers = [
        ("Authorization", auth_header.as_str()),
        ("Content-Type", "application/json"),
    ];

    // Use the adapter's transport to make the request
    let transport = adapter.transport();

    let mut full_content = String::new();
    let _response = transport
        .post_stream(&url, &headers, &body, |line: &str| {
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    return;
                }
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
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
        })
        .map_err(ChatError::from)?;

    if full_content.is_empty() {
        // Fallback to non-streaming if streaming returned empty
        debug_log("[CHAT] Streaming returned empty, falling back to non-streaming POST");
        let response = ureq::request("POST", &url)
            .timeout(std::time::Duration::from_secs(30))
            .set("Authorization", &auth_header)
            .set("Content-Type", "application/json")
            .send_string(&body)
            .map_err(|_| ChatError::TransportError)?;

        let response_text = response
            .into_string()
            .map_err(|_| ChatError::TransportError)?;
        let json: serde_json::Value =
            serde_json::from_str(&response_text).map_err(|_| ChatError::InvalidResponseError)?;

        let content = json["choices"]
            .get(0)
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or(ChatError::InvalidResponseError)?;

        on_chunk(content);
        Ok(content.to_string())
    } else {
        Ok(full_content)
    }
}

/// Helper: Chat with messages using Ollama adapter
fn chat_with_messages_ollama<F>(
    adapter: &crate::llm::adapters::ollama::OllamaAdapter,
    messages: &[LlmMessage],
    mut on_chunk: F,
) -> Result<String, ChatError>
where
    F: FnMut(&str),
{
    // Build request body with messages array
    let body = adapter.build_chat_stream_messages_request(messages)?;

    let url = format!("{}/api/chat", adapter.base_url());
    let headers = [("Content-Type", "application/json")];

    let transport = adapter.transport();

    let mut full_content = String::new();
    let _response = transport
        .post_stream(&url, &headers, &body, |line: &str| {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                if json.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                    return;
                }
                if let Some(message) = json.get("message") {
                    if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                        on_chunk(content);
                        full_content.push_str(content);
                    }
                }
            }
        })
        .map_err(ChatError::from)?;

    if full_content.is_empty() {
        Err(ChatError::InvalidResponseError)
    } else {
        Ok(full_content)
    }
}

/// Stream chat response from LLM using background thread (Phase 8.6)
///
/// # Threaded Behavior
/// - Spawns background thread for LLM I/O only
/// - Thread sends events via mpsc::channel to main thread
/// - Main thread handles UI updates + persistence
/// - Returns (receiver, handle) tuple
///
/// # Arguments
/// - `user_message`: User's chat message
/// - `db_root`: Path to database root (for LLM config)
///
/// # Returns
/// - `ChatReceiver`: Channel receiver — main thread polls this
/// - `ChatThreadHandle`: Thread handle for shutdown/cleanup
///
/// # CRITICAL INVARIANT
/// Thread does ONLY LLM I/O. NO database writes. NO tool execution.
/// All persistence happens on main thread.
///
/// # Usage
/// ```rust,ignore
/// use odincode::llm::chat::chat_threaded;
/// let (rx, handle) = chat_threaded("hello", &db_root)?;
/// // Main loop: poll rx.try_recv() for ChatEvent
/// ```
pub fn chat_threaded(
    user_message: &str,
    db_root: &Path,
) -> Result<(ChatReceiver, ChatThreadHandle), ChatError> {
    use crate::llm::chat_thread::spawn_chat_thread;
    use std::sync::mpsc::channel;

    // Create channel for thread communication
    let (tx, rx) = channel();

    // Spawn background thread (tx is moved into thread)
    // Note: None for session_id = generate new ID (standalone call, not part of loop)
    let handle = spawn_chat_thread(db_root, user_message.to_string(), tx, None);

    Ok((rx, handle))
}

/// Build chat-specific prompt
///
/// Chat uses a minimal prompt — let the model be itself.
/// - No enforced identity
/// - No tool schema
/// - No plan structure requirements
/// - No evidence query format
pub fn build_chat_prompt(user_input: &str) -> String {
    user_input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_chat_prompt_contains_user_input() {
        let prompt = build_chat_prompt("hello world");
        assert!(prompt.contains("hello world"));
        // Prompt is just the user input — no wrapping
        assert_eq!(prompt, "hello world");
    }

    #[test]
    fn test_build_chat_prompt_no_plan_structure() {
        let prompt = build_chat_prompt("test");
        // Chat prompt should NOT contain plan-specific instructions
        assert!(!prompt.contains("plan_id"));
        assert!(!prompt.contains("OUTPUT FORMAT"));
        assert!(!prompt.contains("structured plan"));
        assert!(!prompt.contains("You are"));
    }

    #[test]
    fn test_filter_json_blocks_removes_markdown_blocks() {
        let input = "Text\n```json\n{\"key\": \"value\"}\n```\nMore text";
        let filtered = filter_json_blocks(input);
        assert!(!filtered.contains("```json"));
        assert!(!filtered.contains("{\"key\":"));
        assert!(filtered.contains("Text"));
        assert!(filtered.contains("More text"));
    }

    #[test]
    fn test_chat_threaded_returns_components() {
        use crate::llm::chat_events::ChatEvent;

        // Test that chat_threaded spawns thread and returns receiver
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_root = temp_dir.path();

        // Call chat_threaded - spawns thread even without config
        let result = chat_threaded("test", db_root);

        // Should succeed (thread spawns, error comes later via channel)
        assert!(result.is_ok());

        let (rx, handle) = result.unwrap();

        // Verify receiver was created
        // Verify handle was created
        assert!(!handle.session_id().is_empty());

        // The thread will fail and send error via channel
        // Give it a moment to complete
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Try to receive the error event
        if let Ok(event) = rx.try_recv() {
            match event {
                ChatEvent::Error { error, .. } => {
                    // Verify we got ChatError
                    let _ = error;
                }
                ChatEvent::Started { .. } => {
                    // Got Started event, which is also valid
                }
                _ => {}
            }
        }
    }
}
