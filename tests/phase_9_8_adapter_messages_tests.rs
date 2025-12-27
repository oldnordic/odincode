//! Phase 9.8: Adapter Multi-Turn Message Tests (TDD)
//!
//! Tests that adapters properly use message arrays instead of
//! collapsing the conversation into a single "user" message.
//!
//! Tests should FAIL initially (implementation doesn't exist yet).

use odincode::llm::adapters::ollama;
use odincode::llm::adapters::openai;
use odincode::llm::frame_stack::FrameStack;
use serde_json::Value as JsonValue;

// =============================================================================
// TEST A: FrameStack.build_messages() role ordering
// =============================================================================

#[test]
fn test_a_build_messages_roles_ordering() {
    // Test that FrameStack builds proper role-ordered messages
    // Expected: User → Assistant → User (ToolResult-prefixed) → User

    let mut stack = FrameStack::new();

    // User: "read file.txt"
    stack.add_user("read file.txt".to_string());

    // Assistant: "I'll read that file."
    stack.add_assistant("I'll read that file.");
    stack.complete_assistant();

    // ToolResult: file_read OK
    stack.add_tool_result("file_read".to_string(), true, "Hello, World!".to_string(), None);

    // User: "what did I just read?"
    stack.add_user("what did I just read?".to_string());

    // Build messages
    let messages = stack.build_messages();

    // Verify we have the expected number of messages:
    // System (from prompt) + User + Assistant + User (ToolResult) + User
    assert_eq!(messages.len(), 5, "Should have 5 messages total");

    // Verify role ordering
    assert_eq!(messages[0].role, odincode::llm::adapters::LlmRole::System);
    assert_eq!(messages[1].role, odincode::llm::adapters::LlmRole::User);
    assert!(messages[1].content.contains("read file.txt"));

    assert_eq!(
        messages[2].role,
        odincode::llm::adapters::LlmRole::Assistant
    );
    assert!(messages[2].content.contains("I'll read that file."));

    assert_eq!(messages[3].role, odincode::llm::adapters::LlmRole::User);
    // ToolResult should be prefixed with "[Tool {name}]: OK|FAILED\nResult: …"
    assert!(messages[3].content.contains("[Tool file_read]: OK"));
    assert!(messages[3].content.contains("Hello, World!"));

    assert_eq!(messages[4].role, odincode::llm::adapters::LlmRole::User);
    assert!(messages[4].content.contains("what did I just read?"));
}

#[test]
fn test_a_build_messages_empty_stack() {
    let mut stack = FrameStack::new();
    let messages = stack.build_messages();

    // Should have at least system message
    assert!(!messages.is_empty(), "Should have at least system message");
    assert_eq!(messages[0].role, odincode::llm::adapters::LlmRole::System);
}

#[test]
fn test_a_build_messages_multiple_tool_results() {
    let mut stack = FrameStack::new();

    stack.add_user("list files".to_string());
    stack.add_assistant("I'll list the files.");
    stack.complete_assistant();
    stack.add_tool_result(
        "file_glob".to_string(),
        true,
        "src/main.rs\nsrc/lib.rs".to_string(),
        None,
    );

    stack.add_user("read src/lib.rs".to_string());
    stack.add_assistant("Reading src/lib.rs...");
    stack.complete_assistant();
    stack.add_tool_result(
        "file_read".to_string(),
        true,
        "pub fn hello() {}".to_string(),
        None,
    );

    let messages = stack.build_messages();

    // Should have: System + User + Assistant + User(Tool) + User + Assistant + User(Tool)
    assert_eq!(messages.len(), 7);

    // Verify both tool results are in user messages
    let tool_contents: Vec<_> = messages
        .iter()
        .filter(|m| m.role == odincode::llm::adapters::LlmRole::User)
        .filter(|m| m.content.contains("[Tool "))
        .collect();

    assert_eq!(tool_contents.len(), 2, "Should have 2 tool result messages");
}

// =============================================================================
// TEST B: OpenAI adapter uses message array (not single blob)
// =============================================================================

#[test]
fn test_b_openai_request_uses_messages_array_not_single_user_blob() {
    let adapter = openai::OpenAiAdapter::new(
        "https://api.openai.com/v1".to_string(),
        "gpt-4".to_string(),
        "sk-test".to_string(),
    );

    // Build FrameStack with conversation
    let mut stack = FrameStack::new();
    stack.add_user("read file.txt".to_string());
    stack.add_assistant("I'll read that file.");
    stack.complete_assistant();
    stack.add_tool_result("file_read".to_string(), true, "Hello, World!".to_string(), None);
    stack.add_user("what did I just read?".to_string());

    let messages = stack.build_messages();

    // Convert messages to OpenAI request JSON
    let request_json = adapter
        .build_chat_stream_messages_request(&messages)
        .expect("Should build request from messages");

    let json: JsonValue = serde_json::from_str(&request_json).expect("Should be valid JSON");

    // Verify messages array has more than 2 entries (not just system + single user)
    let messages_array = json["messages"]
        .as_array()
        .expect("Should have messages array");

    assert!(
        messages_array.len() > 2,
        "Should have >2 messages, got {}: {}",
        messages_array.len(),
        request_json
    );

    // Verify we have an assistant message (proves multi-turn)
    let has_assistant = messages_array
        .iter()
        .any(|m| m["role"].as_str() == Some("assistant"));
    assert!(
        has_assistant,
        "Should include assistant role in messages array: {}",
        request_json
    );
}

#[test]
fn test_b_openai_regression_single_turn_unchanged() {
    // Verify the old single-turn path still works (backward compatibility)
    let adapter = openai::OpenAiAdapter::new(
        "https://api.openai.com/v1".to_string(),
        "gpt-4".to_string(),
        "sk-test".to_string(),
    );

    let prompt = "hello world";
    let request_json = adapter
        .build_chat_stream_request(prompt)
        .expect("Should build request from single prompt");

    let json: JsonValue = serde_json::from_str(&request_json).expect("Should be valid JSON");

    let messages_array = json["messages"]
        .as_array()
        .expect("Should have messages array");

    // Old path: exactly 2 messages (system + user)
    assert_eq!(
        messages_array.len(),
        2,
        "Single-turn should have exactly 2 messages"
    );

    assert_eq!(messages_array[0]["role"], "system");
    assert_eq!(messages_array[1]["role"], "user");
    assert_eq!(messages_array[1]["content"], "hello world");
}

// =============================================================================
// TEST C: Ollama adapter follows same contract
// =============================================================================

#[test]
fn test_c_ollama_request_uses_messages_array_not_single_user_blob() {
    let adapter = ollama::OllamaAdapter::new(
        "127.0.0.1".to_string(),
        "11434".to_string(),
        "codellama".to_string(),
    );

    // Build FrameStack with conversation
    let mut stack = FrameStack::new();
    stack.add_user("read file.txt".to_string());
    stack.add_assistant("I'll read that file.");
    stack.complete_assistant();
    stack.add_tool_result("file_read".to_string(), true, "Hello, World!".to_string(), None);
    stack.add_user("what did I just read?".to_string());

    let messages = stack.build_messages();

    // Convert messages to Ollama request JSON
    let request_json = adapter
        .build_chat_stream_messages_request(&messages)
        .expect("Should build request from messages");

    let json: JsonValue = serde_json::from_str(&request_json).expect("Should be valid JSON");

    // Verify messages array has more than 2 entries
    let messages_array = json["messages"]
        .as_array()
        .expect("Should have messages array");

    assert!(
        messages_array.len() > 2,
        "Should have >2 messages, got {}: {}",
        messages_array.len(),
        request_json
    );

    // Verify we have an assistant message
    let has_assistant = messages_array
        .iter()
        .any(|m| m["role"].as_str() == Some("assistant"));
    assert!(
        has_assistant,
        "Should include assistant role in messages array: {}",
        request_json
    );
}

#[test]
fn test_c_ollama_regression_single_turn_unchanged() {
    // Verify the old single-turn path still works (backward compatibility)
    let adapter = ollama::OllamaAdapter::new(
        "127.0.0.1".to_string(),
        "11434".to_string(),
        "codellama".to_string(),
    );

    let prompt = "hello world";
    let request_json = adapter
        .build_chat_stream_request(prompt)
        .expect("Should build request from single prompt");

    let json: JsonValue = serde_json::from_str(&request_json).expect("Should be valid JSON");

    let messages_array = json["messages"]
        .as_array()
        .expect("Should have messages array");

    // Old path: exactly 2 messages (system + user)
    assert_eq!(
        messages_array.len(),
        2,
        "Single-turn should have exactly 2 messages"
    );

    assert_eq!(messages_array[0]["role"], "system");
    assert_eq!(messages_array[1]["role"], "user");
    assert_eq!(messages_array[1]["content"], "hello world");
}

// =============================================================================
// TEST D: ToolResult message format verification
// =============================================================================

#[test]
fn test_d_tool_result_message_format_success() {
    let mut stack = FrameStack::new();
    stack.add_user("read file.txt".to_string());
    stack.add_assistant("Reading file.");
    stack.complete_assistant();
    stack.add_tool_result("file_read".to_string(), true, "Hello, World!".to_string(), None);

    let messages = stack.build_messages();

    // Find the ToolResult-prefixed user message
    let tool_msg = messages
        .iter()
        .find(|m| m.content.contains("[Tool "))
        .expect("Should have ToolResult message");

    // Verify format: "[Tool {name}]: OK|FAILED\nResult: …"
    assert!(tool_msg.content.contains("[Tool file_read]: OK"));
    assert!(tool_msg.content.contains("Result:"));
    assert!(tool_msg.content.contains("Hello, World!"));
}

#[test]
fn test_d_tool_result_message_format_failure() {
    let mut stack = FrameStack::new();
    stack.add_user("read file.txt".to_string());
    stack.add_assistant("Reading file.");
    stack.complete_assistant();
    stack.add_tool_result("file_read".to_string(), false, "File not found".to_string(), None);

    let messages = stack.build_messages();

    // Find the ToolResult-prefixed user message
    let tool_msg = messages
        .iter()
        .find(|m| m.content.contains("[Tool "))
        .expect("Should have ToolResult message");

    // Verify FAILED status
    assert!(tool_msg.content.contains("[Tool file_read]: FAILED"));
    assert!(tool_msg.content.contains("Result:"));
    assert!(tool_msg.content.contains("File not found"));
}

// =============================================================================
// TEST E: LlmRole and LlmMessage type verification
// =============================================================================

#[test]
fn test_e_llm_role_has_system_user_assistant() {
    // Verify LlmRole enum has the three universal roles
    use odincode::llm::adapters::LlmRole;

    let _ = LlmRole::System;
    let _ = LlmRole::User;
    let _ = LlmRole::Assistant;
}

#[test]
fn test_e_llm_message_has_role_and_content() {
    use odincode::llm::adapters::{LlmMessage, LlmRole};

    let msg = LlmMessage {
        role: LlmRole::User,
        content: "test".to_string(),
    };

    assert_eq!(msg.role, LlmRole::User);
    assert_eq!(msg.content, "test");
}

// =============================================================================
// TEST F: Message serialization to provider formats
// =============================================================================

#[test]
fn test_f_openai_message_serialization() {
    use odincode::llm::adapters::{LlmMessage, LlmRole};

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

    let adapter = openai::OpenAiAdapter::new(
        "https://api.openai.com/v1".to_string(),
        "gpt-4".to_string(),
        "sk-test".to_string(),
    );

    let request_json = adapter
        .build_chat_stream_messages_request(&messages)
        .expect("Should serialize messages");

    let json: JsonValue = serde_json::from_str(&request_json).expect("Should be valid JSON");

    assert_eq!(json["messages"][0]["role"], "system");
    assert_eq!(json["messages"][0]["content"], "You are helpful.");
    assert_eq!(json["messages"][1]["role"], "user");
    assert_eq!(json["messages"][1]["content"], "hello");
}

#[test]
fn test_f_ollama_message_serialization() {
    use odincode::llm::adapters::{LlmMessage, LlmRole};

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

    let adapter = ollama::OllamaAdapter::new(
        "127.0.0.1".to_string(),
        "11434".to_string(),
        "codellama".to_string(),
    );

    let request_json = adapter
        .build_chat_stream_messages_request(&messages)
        .expect("Should serialize messages");

    let json: JsonValue = serde_json::from_str(&request_json).expect("Should be valid JSON");

    assert_eq!(json["messages"][0]["role"], "system");
    assert_eq!(json["messages"][0]["content"], "You are helpful.");
    assert_eq!(json["messages"][1]["role"], "user");
    assert_eq!(json["messages"][1]["content"], "hello");
}
