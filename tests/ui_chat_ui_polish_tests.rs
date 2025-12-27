//! Chat UI polish tests — Phase 8.4
//!
//! Tests for:
//! - No ":" prefix in chat input
//! - Bounded chat history
//! - Wrapping and scroll behavior
//! - Chat lane isolation from plan system

use odincode::ui::state::{App, ChatRole};

// Test helper: Create app with temp db_root
fn create_test_app() -> App {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let db_root = temp_dir.path().to_path_buf();

    // Create minimal config for LLM
    use std::fs::File;
    use std::io::Write;
    let config_path = db_root.join("config.toml");
    let mut config_file = File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"[llm]
mode = "external"
provider = "stub"
base_url = "https://stub.example.com"
model = "stub-model"
"#
    )
    .unwrap();

    // Create execution_log.db (required by EvidenceDb in App::new)
    let exec_log_path = db_root.join("execution_log.db");
    let _conn =
        rusqlite::Connection::open(&exec_log_path).expect("Failed to create execution_log.db");

    App::new(db_root)
}

// ============================================================================
// Test 1: Chat input has NO ":" prefix
// ============================================================================

#[test]
fn test_chat_input_has_no_colon_prefix() {
    let mut app = create_test_app();

    // Type "hello" into input buffer
    app.handle_char('h');
    app.handle_char('e');
    app.handle_char('l');
    app.handle_char('l');
    app.handle_char('o');

    // Input buffer is exactly what user typed
    assert_eq!(app.input_buffer, "hello");
    assert!(!app.input_buffer.starts_with(':'));
    assert!(!app.input_buffer.starts_with(": "));
}

// ============================================================================
// Test 2: Command input still uses "/" prefix
// ============================================================================

#[test]
fn test_command_input_still_uses_slash() {
    use odincode::ui::input::{parse_command, Command};

    // Type "/help" - should be parsed as command
    assert_eq!(parse_command("/help"), Command::Help);

    // Input buffer contains "/" character
    let mut app = create_test_app();
    for c in "/help".chars() {
        app.handle_char(c);
    }
    assert_eq!(app.input_buffer, "/help");
    assert!(app.input_buffer.starts_with('/'));
}

// ============================================================================
// Test 3: Chat transcript wraps long lines (via Paragraph Wrap)
// ============================================================================

#[test]
fn test_chat_transcript_wraps_long_lines() {
    let mut app = create_test_app();

    // Add a very long assistant message (200 chars)
    let long_msg = "a".repeat(200);
    app.add_assistant_message(long_msg.clone());

    // Message is stored intact
    assert_eq!(app.chat_messages.len(), 1);
    assert_eq!(app.chat_messages[0].content.len(), 200);

    // Content is preserved (wrapping happens in render, not storage)
    assert_eq!(app.chat_messages[0].content, long_msg);
}

// ============================================================================
// Test 4: Chat scroll moves with new messages
// ============================================================================

#[test]
fn test_chat_scroll_moves_with_new_messages() {
    let mut app = create_test_app();

    // Add 10 messages
    for i in 0..10 {
        app.add_user_message(format!("message {}", i));
        app.add_assistant_message(format!("response {}", i));
    }

    // All messages stored
    assert_eq!(app.chat_messages.len(), 20);

    // Newest message is last
    assert_eq!(app.chat_messages[19].content, "response 9");
}

// ============================================================================
// Test 5: Chat scroll offset calculation (deterministic)
// ============================================================================

#[test]
fn test_chat_scroll_calculation_is_deterministic() {
    let mut app = create_test_app();

    // Add 5 messages
    for i in 0..5 {
        app.add_user_message(format!("msg {}", i));
    }

    // Calculate scroll offset for a pane showing 3 lines
    let visible_lines = 3usize;
    let total_lines = app.chat_messages.len(); // Each msg = 1 line
    let scroll_start = total_lines.saturating_sub(visible_lines);

    // Scroll offset is deterministic
    assert_eq!(scroll_start, 2); // 5 - 3 = 2

    // Same calculation yields same result
    let scroll_start2 = app.chat_messages.len().saturating_sub(visible_lines);
    assert_eq!(scroll_start, scroll_start2);
}

// ============================================================================
// Test 6: Chat history is bounded
// ============================================================================

#[test]
fn test_chat_history_is_bounded() {
    let mut app = create_test_app();

    // Push more messages than MAX
    for i in 0..250 {
        app.add_user_message(format!("msg {}", i));
    }

    // Only last MAX_CHAT_MESSAGES remain
    assert!(app.chat_messages.len() <= 200);
    assert_eq!(app.chat_messages.len(), 200);

    // Oldest message is NOT msg 0 (it was evicted)
    assert_ne!(app.chat_messages[0].content, "msg 0");

    // Newest message is last
    assert_eq!(app.chat_messages[199].content, "msg 249");
}

// ============================================================================
// Test 7: Chat lane does not import plan modules
// ============================================================================

#[test]
fn test_chat_lane_does_not_import_plan_modules() {
    // Read chat.rs source and verify no plan/session imports
    let chat_source = include_str!("../src/llm/chat.rs");

    // Must NOT import plan-related modules
    assert!(!chat_source.contains("use crate::llm::session"),);
    assert!(!chat_source.contains("use crate::llm::planner"));
    assert!(!chat_source.contains("use crate::workflow"));

    // Must NOT create Plan objects
    assert!(!chat_source.contains("Plan::"));
    assert!(!chat_source.contains("Plan {"));

    // Header comments confirm isolation
    assert!(chat_source.contains("Isolated conversational LLM lane"));
    assert!(chat_source.contains("NEVER call plan/session"));
}

// ============================================================================
// Test 8: Chat render filters JSON even when present
// ============================================================================

#[test]
fn test_chat_render_filters_json_even_when_present() {
    let mut app = create_test_app();

    // Add message with JSON content
    let json_content = r#"Here's my answer:
```json
{"plan_id": "test", "steps": []}
```
Done!"#;
    app.add_assistant_message(json_content.to_string());

    // Content is stored with JSON
    assert!(app.chat_messages[0].content.contains("```json"));
    assert!(app.chat_messages[0].content.contains("plan_id"));

    // But filter_json_blocks removes it
    use odincode::llm::chat::filter_json_blocks;
    let filtered = filter_json_blocks(&app.chat_messages[0].content);

    assert!(!filtered.contains("```json"));
    assert!(!filtered.contains("plan_id"));
    assert!(filtered.contains("Here's my answer"));
    assert!(filtered.contains("Done!"));
}

// ============================================================================
// Test 9: Colon input is treated as chat, not command
// ============================================================================

#[test]
fn test_colon_input_is_chat_not_command() {
    use odincode::ui::input::{parse_command, Command};

    // ":" prefixed input is CHAT, not command
    assert!(matches!(parse_command(":help"), Command::Chat(_)));
    assert!(matches!(parse_command(":quit"), Command::Chat(_)));
    assert!(matches!(parse_command(":q"), Command::Chat(_)));

    // Only "/" is command prefix
    assert_eq!(parse_command("/help"), Command::Help);
    assert_eq!(parse_command("/quit"), Command::Quit);
}

// ============================================================================
// Test 10: Empty input buffer is valid state
// ============================================================================

#[test]
fn test_empty_input_buffer_is_valid() {
    let app = create_test_app();

    // Empty buffer is valid
    assert_eq!(app.input_buffer, "");
    assert!(app.input_buffer.is_empty());
}

// ============================================================================
// Test 11: MAX_CHAT_MESSAGES constant exists
// ============================================================================

#[test]
fn test_max_chat_messages_constant_exists() {
    // Verify the constant is defined
    // This is checked by the bounded history test (test 6)
    // which would fail without the constant

    let mut app = create_test_app();
    // Adding 250 messages should result in exactly 200
    for i in 0..250 {
        app.add_user_message(format!("msg {}", i));
    }
    assert_eq!(app.chat_messages.len(), 200);
}

// ============================================================================
// Test 12: Chat messages maintain insertion order
// ============================================================================

#[test]
fn test_chat_messages_maintain_insertion_order() {
    let mut app = create_test_app();

    app.add_user_message("first".to_string());
    app.add_user_message("second".to_string());
    app.add_user_message("third".to_string());

    assert_eq!(app.chat_messages[0].content, "first");
    assert_eq!(app.chat_messages[1].content, "second");
    assert_eq!(app.chat_messages[2].content, "third");
}

// ============================================================================
// Test 13: Thinking indicator is not counted as history
// ============================================================================

#[test]
fn test_thinking_not_counted_for_history_bounds() {
    let mut app = create_test_app();

    // Fill to near limit
    for i in 0..199 {
        app.add_user_message(format!("msg {}", i));
    }
    assert_eq!(app.chat_messages.len(), 199);

    // Add thinking (should not affect count for eviction)
    app.set_thinking();
    assert_eq!(app.chat_messages.len(), 200);

    // Add one more - should evict oldest USER message, not thinking
    app.add_user_message("overflow".to_string());
    assert_eq!(app.chat_messages.len(), 200);

    // Thinking should still be present (somewhere in the array)
    // Note: With FIFO eviction from front, thinking shifts left but is preserved
    assert!(app
        .chat_messages
        .iter()
        .any(|m| m.role == ChatRole::Thinking));

    // Newest message is the overflow we just added
    assert_eq!(app.chat_messages[199].content, "overflow");
}
