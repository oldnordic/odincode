//! OpenAI-Compatible Message Normalization (Phase 9.8)
//!
//! Normalizes messages for OpenAI-compatible providers (GLM, OpenAI).
//! Handles reasoning_content stripping, tool call tracking, and usage reporting.
//!
//! Mirrors OpenCode behavior:
//! - Strips reasoning_content from assistant-visible text
//! - Tracks tool calls across streaming deltas
//! - Captures usage totals and finish_reason

use crate::llm::adapters::{LlmMessage, LlmRole};
use serde_json::Value as JsonValue;

/// Normalized request with metadata
#[derive(Debug, Clone)]
pub struct NormalizedRequest {
    /// JSON request body
    pub body: String,
    /// Message count (for debugging/validation)
    pub message_count: usize,
    /// Role summary (for debugging)
    pub role_summary: String,
}

/// Streaming event from SSE chunk
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamingEvent {
    /// Assistant text delta
    TextDelta(String),
    /// Tool call started
    ToolCallStart { id: String, name: String },
    /// Tool call argument delta
    ToolCallDelta { id: String, args_delta: String },
    /// Tool call complete
    ToolCallComplete { id: String, args: String },
    /// Stream finished
    Finish { reason: Option<String> },
    /// Usage data (final chunk)
    Usage { prompt_tokens: Option<u64>, completion_tokens: Option<u64> },
}

/// Streaming state for tracking tool calls across chunks
#[derive(Debug, Clone, Default)]
pub struct StreamingState {
    /// Accumulated assistant text (reasoning stripped)
    pub assistant_text: String,
    /// Active tool calls by ID
    pub tool_calls: Vec<ToolCallState>,
    /// Finish reason if received
    pub finish_reason: Option<String>,
    /// Usage data
    pub usage: Option<UsageData>,
}

/// Tool call being constructed from deltas
#[derive(Debug, Clone, Default)]
pub struct ToolCallState {
    /// Call ID (from SSE delta)
    pub id: String,
    /// Function name
    pub name: String,
    /// Accumulated arguments (JSON string being built)
    pub args: String,
    /// Whether complete
    pub complete: bool,
}

/// Usage data from final chunk
#[derive(Debug, Clone, Default)]
pub struct UsageData {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// Normalize messages for OpenAI-compatible request
///
/// Converts Vec<LlmMessage> to JSON, ensuring:
/// - Proper role-per-message structure
/// - No interleaved reasoning_content in assistant text
/// - Stable format for provider consumption
pub fn normalize_for_openai_compatible(
    messages: &[LlmMessage],
    model: &str,
    stream: bool,
) -> Result<NormalizedRequest, String> {
    // Convert messages to JSON array
    let json_messages: Vec<JsonValue> = messages
        .iter()
        .map(|msg| {
            let role_str = match msg.role {
                LlmRole::System => "system",
                LlmRole::User => "user",
                LlmRole::Assistant => "assistant",
            };
            // Strip any reasoning_content prefixes from assistant messages
            let content = strip_reasoning_content(&msg.content);
            serde_json::json!({
                "role": role_str,
                "content": content
            })
        })
        .collect();

    // Build role summary for debugging
    let role_summary = messages
        .iter()
        .map(|m| match m.role {
            LlmRole::System => "S",
            LlmRole::User => "U",
            LlmRole::Assistant => "A",
        })
        .collect::<Vec<_>>()
        .join("");

    // Build full request JSON
    let request = serde_json::json!({
        "model": model,
        "messages": json_messages,
        "stream": stream
    });

    Ok(NormalizedRequest {
        body: request.to_string(),
        message_count: messages.len(),
        role_summary,
    })
}

/// Strip reasoning_content from assistant text
///
/// Some providers (like GLM with extended reasoning) embed reasoning chunks
/// in assistant responses. OpenCode strips these and moves them to
/// providerOptions.openaiCompatible.reasoning_content.
///
/// For now, we detect and strip common patterns:
/// - "<reasoning>...</reasoning>" tags
/// - "[REASONING]..." prefixes
/// - Lines starting with "Thinking:" or similar
fn strip_reasoning_content(content: &str) -> String {
    let mut result = String::new();
    let mut pos = 0;

    // Handle <reasoning>...</reasoning> tags (inline or multi-line)
    while let Some(start_pos) = content[pos..].find("<reasoning>") {
        // Keep content before reasoning tag
        result.push_str(&content[pos..pos + start_pos]);

        // Find end tag
        let tag_start = pos + start_pos;
        if let Some(end_relative) = content[tag_start..].find("</reasoning>") {
            pos = tag_start + end_relative + "</reasoning>".len();
        } else {
            // No closing tag, skip rest
            break;
        }
    }

    // Append remaining content after last reasoning block
    if pos < content.len() {
        result.push_str(&content[pos..]);
    }

    // Now handle [REASONING]...[/REASONING] blocks
    let mut result2 = String::new();
    let mut pos = 0;
    while let Some(start_pos) = result[pos..].find("[REASONING]") {
        // Keep content before reasoning tag
        result2.push_str(&result[pos..pos + start_pos]);

        // Find end tag
        let tag_start = pos + start_pos;
        if let Some(end_relative) = result[tag_start..].find("[/REASONING]") {
            pos = tag_start + end_relative + "[/REASONING]".len();
        } else {
            // No closing tag, skip rest
            break;
        }
    }

    // Append remaining content
    if pos < result.len() {
        result2.push_str(&result[pos..]);
    }

    // Finally, remove lines that start with Thinking:/Thought:/Reasoning:
    result2
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("Thinking:")
                && !trimmed.starts_with("Thought:")
                && !trimmed.starts_with("Reasoning:")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse SSE chunk and extract streaming events
///
/// OpenAI-compatible SSE format:
/// data: {"choices":[{"delta":{"content":"..."}}]}
/// data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"name":"...","arguments":"..."}}]}}]}
/// data: [DONE]
pub fn parse_sse_chunk(line: &str, state: &mut StreamingState) -> Vec<StreamingEvent> {
    let mut events = Vec::new();

    // Strip "data: " prefix
    let data = match line.strip_prefix("data: ") {
        Some(d) => d,
        None => return events,
    };

    // Check for DONE
    if data == "[DONE]" {
        events.push(StreamingEvent::Finish {
            reason: state.finish_reason.clone(),
        });
        return events;
    }

    // Parse JSON
    let json: JsonValue = match serde_json::from_str(data) {
        Ok(j) => j,
        Err(_) => return events,
    };

    // Extract finish_reason if present
    if let Some(finish) = json["choices"][0]["finish_reason"].as_str() {
        state.finish_reason = Some(finish.to_string());
    }

    // Extract usage if present
    if let Some(usage) = json.get("usage") {
        let prompt = usage["prompt_tokens"].as_u64();
        let completion = usage["completion_tokens"].as_u64();
        events.push(StreamingEvent::Usage {
            prompt_tokens: prompt,
            completion_tokens: completion,
        });
        state.usage = Some(UsageData {
            prompt_tokens: prompt.unwrap_or(0),
            completion_tokens: completion.unwrap_or(0),
            total_tokens: usage["total_tokens"].as_u64().unwrap_or(0),
        });
    }

    // Extract content delta
    if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
        if !content.is_empty() {
            // Strip reasoning from content chunks
            let stripped = strip_reasoning_content(content);
            if !stripped.is_empty() {
                state.assistant_text.push_str(&stripped);
                events.push(StreamingEvent::TextDelta(stripped));
            }
        }
    }

    // Extract tool call deltas
    if let Some(calls) = json["choices"][0]["delta"].get("tool_calls") {
        if let Some(calls_array) = calls.as_array() {
            for call_json in calls_array {
                let index = call_json["index"].as_u64().unwrap_or(0) as usize;

                // Ensure we have enough tool call slots
                while state.tool_calls.len() <= index {
                    state.tool_calls.push(ToolCallState::default());
                }

                let tool_call = &mut state.tool_calls[index];

                // Extract call ID
                if let Some(id) = call_json["id"].as_str() {
                    if tool_call.id.is_empty() {
                        tool_call.id = id.to_string();
                        events.push(StreamingEvent::ToolCallStart {
                            id: id.to_string(),
                            name: String::new(),
                        });
                    }
                }

                // Extract function name
                if let Some(name) = call_json["function"]["name"].as_str() {
                    if !name.is_empty() && tool_call.name.is_empty() {
                        tool_call.name = name.to_string();
                        events.push(StreamingEvent::ToolCallStart {
                            id: tool_call.id.clone(),
                            name: name.to_string(),
                        });
                    }
                }

                // Extract arguments delta
                if let Some(args) = call_json["function"]["arguments"].as_str() {
                    if !args.is_empty() {
                        tool_call.args.push_str(args);
                        events.push(StreamingEvent::ToolCallDelta {
                            id: tool_call.id.clone(),
                            args_delta: args.to_string(),
                        });
                    }
                }
            }
        }
    }

    events
}

/// Get final tool calls from streaming state
///
/// Returns complete tool calls with parsed arguments.
pub fn get_final_tool_calls(state: &StreamingState) -> Vec<(String, String)> {
    state
        .tool_calls
        .iter()
        .filter(|t| !t.name.is_empty() && !t.args.is_empty())
        .map(|t| (t.name.clone(), t.args.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_basic_messages() {
        let messages = vec![
            LlmMessage {
                role: LlmRole::System,
                content: "You are helpful.".to_string(),
            },
            LlmMessage {
                role: LlmRole::User,
                content: "hello".to_string(),
            },
        ];

        let result = normalize_for_openai_compatible(&messages, "gpt-4", true).unwrap();
        assert_eq!(result.message_count, 2);
        assert_eq!(result.role_summary, "SU");

        // Verify JSON is valid
        let json: JsonValue = serde_json::from_str(&result.body).unwrap();
        assert_eq!(json["messages"][0]["role"], "system");
        assert_eq!(json["messages"][1]["role"], "user");
    }

    #[test]
    fn test_normalize_multi_turn() {
        let messages = vec![
            LlmMessage {
                role: LlmRole::System,
                content: "System".to_string(),
            },
            LlmMessage {
                role: LlmRole::User,
                content: "First message".to_string(),
            },
            LlmMessage {
                role: LlmRole::Assistant,
                content: "First response".to_string(),
            },
            LlmMessage {
                role: LlmRole::User,
                content: "[Tool file_read]: OK\nResult: content".to_string(),
            },
        ];

        let result = normalize_for_openai_compatible(&messages, "gpt-4", true).unwrap();
        assert_eq!(result.message_count, 4);
        assert_eq!(result.role_summary, "SUAU");

        // Verify assistant message exists
        let json: JsonValue = serde_json::from_str(&result.body).unwrap();
        assert_eq!(json["messages"][2]["role"], "assistant");
        assert_eq!(json["messages"][2]["content"], "First response");
    }

    #[test]
    fn test_strip_reasoning_content_tags() {
        let input = "Hello\n<reasoning>This is hidden</reasoning>\nWorld";
        let output = strip_reasoning_content(input);
        assert!(!output.contains("reasoning"));
        assert!(!output.contains("hidden"));
        assert!(output.contains("Hello"));
        assert!(output.contains("World"));
    }

    #[test]
    fn test_strip_reasoning_brackets() {
        let input = "Response\n[REASONING]Hidden thought[/REASONING]\nMore text";
        let output = strip_reasoning_content(input);
        assert!(!output.contains("REASONING"));
        assert!(!output.contains("Hidden"));
        assert!(output.contains("Response"));
        assert!(output.contains("More text"));
    }

    #[test]
    fn test_strip_thinking_prefix() {
        let input = "Text\nThinking: I should consider...\nMore text";
        let output = strip_reasoning_content(input);
        assert!(!output.contains("Thinking"));
        assert!(output.contains("Text"));
        assert!(output.contains("More text"));
    }

    #[test]
    fn test_parse_sse_content_delta() {
        let line = "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}";

        let mut state = StreamingState::default();
        let events = parse_sse_chunk(line, &mut state);

        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamingEvent::TextDelta(text) => assert_eq!(text, "Hello"),
            _ => panic!("Expected TextDelta event"),
        }

        assert_eq!(state.assistant_text, "Hello");
    }

    #[test]
    fn test_parse_sse_done() {
        let line = "data: [DONE]";

        let mut state = StreamingState {
            finish_reason: Some("stop".to_string()),
            ..Default::default()
        };
        let events = parse_sse_chunk(line, &mut state);

        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamingEvent::Finish { reason } => assert_eq!(reason.as_ref().unwrap(), "stop"),
            _ => panic!("Expected Finish event"),
        }
    }

    // TODO: Tool call parsing tests - to be completed with real GLM API examples
    // The parse_sse_chunk function supports tool call deltas, but we need real SSE
    // examples to write accurate tests. For now, the function handles:
    // - Content deltas (text_delta)
    // - Usage data (usage)
    // - Finish reason (finish)
    // - Tool call tracking (tool_calls state)
    #[test]
    fn test_streaming_state_tracks_tool_calls() {
        // Test that StreamingState can track tool calls
        let mut state = StreamingState::default();
        state.tool_calls.push(ToolCallState {
            id: "call_123".to_string(),
            name: "file_read".to_string(),
            args: r#"{"path":"src/lib.rs"}"#.to_string(),
            complete: false,
        });

        let calls = get_final_tool_calls(&state);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "file_read");
    }

    #[test]
    fn test_parse_sse_usage() {
        let line = "data: {\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}";

        let mut state = StreamingState::default();
        let events = parse_sse_chunk(line, &mut state);

        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamingEvent::Usage {
                prompt_tokens,
                completion_tokens,
            } => {
                assert_eq!(prompt_tokens, &Some(10u64));
                assert_eq!(completion_tokens, &Some(5u64));
            }
            _ => panic!("Expected Usage event"),
        }

        assert!(state.usage.is_some());
        let usage = state.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
    }

    #[test]
    fn test_get_final_tool_calls() {
        let mut state = StreamingState::default();
        state.tool_calls.push(ToolCallState {
            id: "call_1".to_string(),
            name: "file_read".to_string(),
            args: r#"{"path":"src/lib.rs"}"#.to_string(),
            complete: false,
        });

        let calls = get_final_tool_calls(&state);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "file_read");
        assert_eq!(calls[0].1, r#"{"path":"src/lib.rs"}"#);
    }

    #[test]
    fn test_regression_multi_turn_message_count() {
        // Regression test: ensure >2 messages after tool loop
        let messages = vec![
            LlmMessage {
                role: LlmRole::System,
                content: "System".to_string(),
            },
            LlmMessage {
                role: LlmRole::User,
                content: "read file.txt".to_string(),
            },
            LlmMessage {
                role: LlmRole::Assistant,
                content: "I'll read that file.".to_string(),
            },
            LlmMessage {
                role: LlmRole::User,
                content: "[Tool file_read]: OK\nResult: content".to_string(),
            },
            LlmMessage {
                role: LlmRole::User,
                content: "what did I just read?".to_string(),
            },
        ];

        let result = normalize_for_openai_compatible(&messages, "glm", true).unwrap();
        assert!(result.message_count > 2, "Should have >2 messages for multi-turn");
    }
}
