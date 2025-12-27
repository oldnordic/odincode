//! Chat Loop Tool Execution Tests (Phase 9.11)
//!
//! Tests the complete flow: Tool call detection → LoopAction → UI handling.
//! Reproduces the bug where tool is detected but ExecuteTool is not returned.

use std::sync::mpsc::channel;
use tempfile::TempDir;

use odincode::llm::chat_events::ChatEvent;
use odincode::llm::chat_loop::{ChatLoop, LoopAction};
use odincode::execution_engine::ChatToolRunner;

/// Helper: Create a Complete event with matching session_id
fn create_tool_call_event(session_id: &str, tool: &str, args: &[(&str, &str)]) -> ChatEvent {
    let mut tool_call = format!("TOOL_CALL:\n  tool: {}\n  args:", tool);
    for (key, value) in args {
        tool_call.push_str(&format!("\n    {}: {}", key, value));
    }

    ChatEvent::Complete {
        session_id: session_id.to_string(),
        full_response: tool_call,
    }
}

/// Test 1: file_glob tool in "list .rs files in src" should return ExecuteTool
#[test]
fn test_file_glob_in_list_files_returns_execute_tool() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();

    let (tx, _rx) = channel();
    let mut chat_loop = ChatLoop::new(ChatToolRunner::new(None, None));
    chat_loop.set_sender(tx);
    chat_loop.start("list .rs files in src".to_string(), db_root).unwrap();

    // Get the session_id that ChatLoop generated (starts with "loop-")
    let session_id = chat_loop.loop_state.as_ref().unwrap().session_id.clone();

    // The LLM responds with file_glob tool call
    let event = create_tool_call_event(&session_id, "file_glob", &[("pattern", "**/*.rs"), ("root", "src")]);

    // Process the event through ChatLoop
    let action = chat_loop.process_event(&event, db_root);

    // Should return ExecuteTool, not LoopComplete or None
    match action {
        LoopAction::ExecuteTool(tool, args) => {
            assert_eq!(tool, "file_glob");
            assert_eq!(args.get("pattern"), Some(&"**/*.rs".to_string()));
            assert_eq!(args.get("root"), Some(&"src".to_string()));
        }
        other => {
            panic!("Expected ExecuteTool, got {:?}", other);
        }
    }
}

/// Test 2: Test the exact flow from Complete event to LoopAction
#[test]
fn test_complete_event_with_file_glob_returns_execute_tool() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();

    let (tx, _rx) = channel();
    let mut chat_loop = ChatLoop::new(ChatToolRunner::new(None, None));
    chat_loop.set_sender(tx);
    chat_loop.start("list .rs files in src".to_string(), db_root).unwrap();

    // Get the session_id that ChatLoop generated
    let session_id = chat_loop.loop_state.as_ref().unwrap().session_id.clone();

    // Simulate LLM response with TOOL_CALL
    let response = r#"TOOL_CALL:
  tool: file_glob
  args:
    pattern: **/*.rs
    root: src"#;

    let event = ChatEvent::Complete {
        session_id,
        full_response: response.to_string(),
    };

    let action = chat_loop.process_event(&event, db_root);

    // Verify ExecuteTool is returned
    match &action {
        LoopAction::ExecuteTool(tool, _args) => {
            assert_eq!(tool.as_str(), "file_glob");
        }
        _ => {
            panic!("Expected ExecuteTool, got {:?}", action);
        }
    }
}

/// Test 3: Test with FrameStack continuation (multi-turn)
#[test]
fn test_continuation_call_after_tool_result() {
    // Create FrameStack with conversation
    let mut frame_stack = odincode::llm::frame_stack::FrameStack::new();
    frame_stack.add_user("list .rs files in src".to_string());
    frame_stack.add_assistant(
        "I'll list the .rs files.\n\nTOOL_CALL:\n  tool: file_glob\n  args:\n    pattern: **/*.rs\n    root: src"
    );

    let messages = frame_stack.build_messages();

    // Verify we have proper message structure
    assert!(!messages.is_empty(), "Messages should not be empty");

    // First message should be system prompt
    assert_eq!(messages[0].role, odincode::llm::adapters::LlmRole::System);

    // Second message should be user message
    assert_eq!(messages[1].role, odincode::llm::adapters::LlmRole::User);
    assert!(messages[1].content.contains("list .rs files"));

    // Third message should be assistant with TOOL_CALL
    assert_eq!(messages[2].role, odincode::llm::adapters::LlmRole::Assistant);
    assert!(messages[2].content.contains("TOOL_CALL"));
}
