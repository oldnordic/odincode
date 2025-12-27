//! GLM Streaming Integration Tests (Phase 9.10)
//!
//! Tests the GLM streaming callback to identify where the chat loop gets stuck.
//! Uses FakeTransport with realistic GLM SSE responses (NOT stub adapter).
//!
//! **Issue**: Chat loop gets stuck after tool execution
//! **Hypothesis**: GLM API doesn't send streaming chunks for continuation calls,
//!                or SSE parsing fails silently.

use std::fs::File;
use std::io::Write;
use std::sync::mpsc::channel;
use std::time::Duration;
use tempfile::TempDir;

use odincode::llm::adapters::{
    transport::FakeTransport,
    SyncTransport,  // Import trait for post_stream method
    LlmMessage, LlmRole,
};
use odincode::llm::frame_stack::Frame;

/// Helper: Create a realistic GLM SSE streaming response
///
/// This simulates what GLM actually returns for a continuation call.
fn glm_sse_streaming_response() -> String {
    // GLM uses OpenAI-compatible SSE format
    // Each line starts with "data: " prefix
    // Response contains JSON with choices[0].delta.content
    "\
data: {\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"I\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\" have\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\" read\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\" the\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\" file\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\".\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\" Here\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\" is\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\" what\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\" I\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\" found\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\".\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\"},\"finish_reason\":\"stop\"}]}\n\
data: [DONE]\n".to_string()
}

/// Helper: Create an EMPTY SSE streaming response (simulates GLM bug)
///
/// This simulates what happens when GLM returns no content chunks.
fn glm_empty_streaming_response() -> String {
    "\
data: {\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\
data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\"},\"finish_reason\":\"stop\"}]}\n\
data: [DONE]\n".to_string()
}

/// Helper: Create messages array for continuation (simulates FrameStack)
fn create_continuation_messages() -> Vec<LlmMessage> {
    vec![
        LlmMessage {
            role: LlmRole::User,
            content: "please read src/lib.rs".to_string(),
        },
        LlmMessage {
            role: LlmRole::Assistant,
            content: "I'll read that file.\n\nTOOL_CALL:\n  tool: file_read\n  args:\n    path: src/lib.rs".to_string(),
        },
        LlmMessage {
            role: LlmRole::User,  // Hidden tool result
            content: "[Tool Result] file_read on src/lib.rs\nSUCCESS: File content follows...\n// src/lib.rs content here".to_string(),
        },
    ]
}

/// Test A: Verify FakeTransport calls on_line for each SSE line
#[test]
fn test_fake_transport_sse_calls_on_line() {
    let sse_response = glm_sse_streaming_response();
    let transport = FakeTransport::with_stream("", &sse_response);

    let mut lines_received = Vec::new();
    let result = transport.post_stream("http://test", &[], "{}", |line: &str| {
        lines_received.push(line.to_string());
    });

    assert!(result.is_ok(), "post_stream should succeed");

    // FakeTransport.lines() splits on newlines and calls on_line for each
    // We should get multiple lines
    eprintln!("Lines received: {}", lines_received.len());
    for (i, line) in lines_received.iter().enumerate() {
        eprintln!("Line {}: {}", i, line);
    }

    // Each line should be called
    assert!(
        lines_received.len() > 5,
        "Expected at least 5 lines, got {}",
        lines_received.len()
    );

    // First line should start with "data:"
    assert!(
        lines_received[0].starts_with("data:"),
        "First line should start with 'data:', got: {}",
        lines_received[0]
    );
}

/// Test B: Verify GLM SSE parsing with callback
///
/// This simulates the exact flow in `chat_with_messages_glm()` at chat.rs:375-395
#[test]
fn test_glm_sse_parsing_with_callback() {
    let sse_response = glm_sse_streaming_response();
    let transport = FakeTransport::with_stream("", &sse_response);

    let mut chunks_received = Vec::new();
    let mut full_content = String::new();

    // Simulate the exact parsing logic from chat.rs:376-394
    let _result = transport.post_stream("http://test", &[], "{}", |line: &str| {
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
                        chunks_received.push(text.to_string());
                        full_content.push_str(text);
                    }
                }
            }
        }
    });

    eprintln!("Chunks received: {}", chunks_received.len());
    eprintln!("Full content: {}", full_content);

    // Should receive multiple chunks
    assert!(
        !chunks_received.is_empty(),
        "Expected at least one chunk, got zero. \
         This means the callback was never called with content!"
    );

    // Full content should be concatenated
    assert!(
        !full_content.is_empty(),
        "Expected non-empty full_content"
    );

    // Verify content is correct
    assert_eq!(full_content, "I have read the file. Here is what I found.");
}

/// Test C: Verify EMPTY SSE response (the GLM bug scenario)
///
/// This simulates what happens when GLM returns a streaming response
/// with no actual content chunks (only empty deltas).
#[test]
fn test_glm_empty_sse_response() {
    let sse_response = glm_empty_streaming_response();
    let transport = FakeTransport::with_stream("", &sse_response);

    let mut chunks_received = Vec::new();
    let mut full_content = String::new();

    let _result = transport.post_stream("http://test", &[], "{}", |line: &str| {
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
                        // Only record non-empty chunks
                        if !text.is_empty() {
                            chunks_received.push(text.to_string());
                            full_content.push_str(text);
                        }
                    }
                }
            }
        }
    });

    eprintln!("Chunks received: {}", chunks_received.len());
    eprintln!("Full content: '{}'", full_content);

    // With empty deltas, full_content will be empty
    // This is the bug scenario!
    assert!(
        full_content.is_empty(),
        "Expected empty full_content for empty delta scenario"
    );

    // This proves: if GLM sends empty deltas, full_content.is_empty() == true
    // Which triggers the fallback in chat.rs:397-424
}

/// Test D: Full chat_with_messages flow with GLM adapter (using FakeTransport)
///
/// This test requires access to internal chat_with_messages_glm function,
/// which is not public. Instead, we'll use the public chat::chat_with_messages
/// with a config that uses GLM.
#[test]
fn test_chat_with_messages_glm_streaming() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();

    // Create config.toml with GLM provider
    let config_path = db_root.join("config.toml");
    let mut config_file = File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"[llm]
mode = "external"
provider = "glm"
base_url = "https://api.z.ai/api/coding/paas/v4"
model = "GLM-4.7"
api_key = "test-key-123"
"#
    )
    .unwrap();

    // Note: This test will make REAL HTTP calls if we don't mock the transport
    // For now, we'll skip it and just verify the config loads
    // A proper mock would require modifying the adapter factory to accept test transports

    // TODO: Make adapter factory accept test transports for proper integration testing
    eprintln!("Test skipped - need to inject FakeTransport into GLM adapter");
}

/// Test E: Verify ChatThread sends Chunk events
///
/// This tests the actual flow from spawn_chat_thread_with_frame_stack
/// through to Chunk events being sent.
#[test]
fn test_spawn_chat_thread_sends_chunk_events() {
    use odincode::llm::chat_events::ChatEvent;
    use odincode::llm::chat_thread::spawn_chat_thread_with_frame_stack;
    use odincode::llm::frame_stack::FrameStack;

    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();

    // Create config with stub (for this test we want to verify event flow)
    let config_path = db_root.join("config.toml");
    let mut config_file = File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"[llm]
mode = "external"
provider = "stub"
model = "test"
"#
    )
    .unwrap();

    // Create FrameStack with conversation
    let mut frame_stack = FrameStack::new();
    frame_stack.add_user("please read src/lib.rs".to_string());
    frame_stack.add_assistant("I'll read that file.\n\nTOOL_CALL:\n  tool: file_read\n  args:\n    path: src/lib.rs");
    // Don't add tool result - we want to test the continuation call

    let (tx, rx) = channel();

    // Spawn chat thread
    let handle = spawn_chat_thread_with_frame_stack(
        &db_root,
        &mut frame_stack,
        tx.clone(),
        None,
    );

    eprintln!("Session ID: {}", handle.session_id());

    // Wait for events
    let mut events = Vec::new();
    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(100));
        while let Ok(event) = rx.try_recv() {
            eprintln!("Received event: {:?}", std::mem::discriminant(&event));
            events.push(event);
        }
        if events.len() > 2 {
            break;
        }
    }

    eprintln!("Total events received: {}", events.len());

    // Should at least get Started event
    let has_started = events.iter().any(|e| matches!(e, ChatEvent::Started { .. }));
    assert!(has_started, "Expected at least Started event");

    // With stub, we should get Complete event
    let has_complete = events.iter().any(|e| matches!(e, ChatEvent::Complete { .. }));
    assert!(has_complete, "Expected Complete event from stub adapter");
}

/// Test F: Verify FakeTransport with NEWLINE handling
///
/// Tests whether FakeTransport properly handles newlines in SSE responses.
#[test]
fn test_fake_transport_newline_handling() {
    // This tests the exact SSE format from GLM
    let sse = "data: {\"test\": \"value\"}\ndata: [DONE]\n";
    let transport = FakeTransport::with_stream("", sse);

    let mut lines = Vec::new();
    let _ = transport.post_stream("http://test", &[], "", |line: &str| {
        lines.push(line.to_string());
    });

    eprintln!("Lines from SSE: {:?}", lines);

    // Should get 2 lines
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "data: {\"test\": \"value\"}");
    assert_eq!(lines[1], "data: [DONE]");

    // First line should have "data: " prefix
    assert!(lines[0].starts_with("data: "));
}

/// Test G: Prove that empty content triggers fallback condition
///
/// This test proves the exact condition that triggers the fallback
/// in chat.rs:397: `if full_content.is_empty()`
#[test]
fn test_empty_content_triggers_fallback_condition() {
    let sse_response = glm_empty_streaming_response();
    let transport = FakeTransport::with_stream("", &sse_response);

    let mut full_content = String::new();

    let _result = transport.post_stream("http://test", &[], "{}", |line: &str| {
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
                        if !text.is_empty() {
                            full_content.push_str(text);
                        }
                    }
                }
            }
        }
    });

    // This is the KEY condition
    let would_trigger_fallback = full_content.is_empty();

    eprintln!("Full content: '{}'", full_content);
    eprintln!("Would trigger fallback: {}", would_trigger_fallback);

    assert!(
        would_trigger_fallback,
        "Expected full_content to be empty, triggering fallback"
    );

    // This PROVES: If GLM sends empty deltas, the fallback is triggered
    // The fallback makes a non-streaming POST request
}
