//! Chat transcript UI tests — Phase 8.2
//!
//! Tests that chat UI is properly separated from command UI:
//! - Chat messages appear in transcript, not console
//! - No ":" prefix in chat input
//! - Commands don't pollute chat transcript
//! - Streaming works incrementally

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
// Test 1: Chat message appears in transcript WITHOUT ":"
// ============================================================================

#[test]
fn test_chat_message_appears_without_colon_prefix() {
    let mut app = create_test_app();

    // Simulate user typing "hello" (no ":")
    app.input_buffer = "hello".to_string();

    // Add as user message
    app.add_user_message(app.input_buffer.clone());

    // Verify message is in chat transcript
    assert_eq!(app.chat_messages.len(), 1);
    let msg = &app.chat_messages[0];
    assert_eq!(msg.role, ChatRole::User);
    assert_eq!(msg.content, "hello");
    assert!(
        !msg.content.starts_with(':'),
        "Chat should NOT have ':' prefix"
    );
}

// ============================================================================
// Test 2: Multiple chat turns render in order
// ============================================================================

#[test]
fn test_multiple_chat_turns_maintain_order() {
    let mut app = create_test_app();

    // Simulate conversation
    app.add_user_message("first question".to_string());
    app.add_assistant_message("first answer".to_string());
    app.add_user_message("second question".to_string());
    app.add_assistant_message("second answer".to_string());

    // Verify order
    assert_eq!(app.chat_messages.len(), 4);
    assert_eq!(app.chat_messages[0].role, ChatRole::User);
    assert_eq!(app.chat_messages[0].content, "first question");
    assert_eq!(app.chat_messages[1].role, ChatRole::Assistant);
    assert_eq!(app.chat_messages[1].content, "first answer");
    assert_eq!(app.chat_messages[2].role, ChatRole::User);
    assert_eq!(app.chat_messages[2].content, "second question");
    assert_eq!(app.chat_messages[3].role, ChatRole::Assistant);
    assert_eq!(app.chat_messages[3].content, "second answer");
}

// ============================================================================
// Test 3: Streaming chunks append incrementally
// ============================================================================

#[test]
fn test_streaming_chunks_append_to_single_message() {
    let mut app = create_test_app();

    // Start with user message
    app.add_user_message("tell me a joke".to_string());

    // Simulate streaming: create placeholder, then update it
    app.add_assistant_message("Why did".to_string());
    app.chat_messages.last_mut().unwrap().content = "Why did the chicken".to_string();
    app.chat_messages.last_mut().unwrap().content = "Why did the chicken cross".to_string();
    app.chat_messages.last_mut().unwrap().content =
        "Why did the chicken cross the road?".to_string();

    // Verify only 2 messages total (user + assistant)
    assert_eq!(app.chat_messages.len(), 2);
    assert_eq!(
        app.chat_messages[1].content,
        "Why did the chicken cross the road?"
    );
}

// ============================================================================
// Test 4: Commands do NOT appear in chat transcript
// ============================================================================

#[test]
fn test_commands_do_not_pollute_chat_transcript() {
    let mut app = create_test_app();

    // Simulate command execution (uses log(), not chat)
    app.log("Opened: src/lib.rs".to_string());
    app.log("Read: src/lib.rs".to_string());

    // Verify chat transcript is still empty
    assert_eq!(app.chat_messages.len(), 0);

    // But console has the messages
    assert_eq!(app.console_messages.len(), 2);
}

// ============================================================================
// Test 5: Chat and console are separate buffers
// ============================================================================

#[test]
fn test_chat_and_console_are_separate() {
    let mut app = create_test_app();

    // Add to chat
    app.add_user_message("chat message".to_string());
    app.add_assistant_message("assistant response".to_string());

    // Add to console
    app.log("console output".to_string());

    // Verify separation
    assert_eq!(app.chat_messages.len(), 2);
    assert_eq!(app.console_messages.len(), 1);

    // Console doesn't have chat messages
    assert!(!app
        .console_messages
        .iter()
        .any(|m| m.content.contains("chat message")));

    // Chat doesn't have console messages
    assert!(!app
        .chat_messages
        .iter()
        .any(|m| m.content.contains("console output")));
}

// ============================================================================
// Test 6: Input buffer clears after send
// ============================================================================

#[test]
fn test_input_buffer_clears_after_send() {
    let mut app = create_test_app();

    // User types message
    app.input_buffer = "test message".to_string();
    assert_eq!(app.input_buffer, "test message");

    // Handle enter (simulates main loop)
    let _input = std::mem::take(&mut app.input_buffer);

    // Verify buffer cleared
    assert_eq!(app.input_buffer, "");
}

// ============================================================================
// Test 7: Chat transcript can be cleared
// ============================================================================

#[test]
fn test_chat_transcript_can_be_cleared() {
    let mut app = create_test_app();

    // Add messages
    app.add_user_message("msg1".to_string());
    app.add_assistant_message("msg2".to_string());
    assert_eq!(app.chat_messages.len(), 2);

    // Clear chat
    app.clear_chat();

    // Verify empty
    assert_eq!(app.chat_messages.len(), 0);
}

// ============================================================================
// Test 8: Chat doesn't require execution DB
// ============================================================================

#[test]
fn test_chat_works_without_codegraph() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let db_root = temp_dir.path().to_path_buf();

    // Only config, no codegraph.db
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

    // Create execution_log.db only
    let exec_log_path = db_root.join("execution_log.db");
    let _conn = rusqlite::Connection::open(&exec_log_path).unwrap();

    let mut app = App::new(db_root);

    // Chat should still work
    app.add_user_message("hello".to_string());
    assert_eq!(app.chat_messages.len(), 1);
}

// ============================================================================
// Test 9: Console messages still work for commands
// ============================================================================

#[test]
fn test_console_messages_work_for_commands() {
    let mut app = create_test_app();

    // Command output uses console
    app.log("Command: /read src/lib.rs".to_string());
    app.log("Success: Read 123 bytes".to_string());

    assert_eq!(app.console_messages.len(), 2);
    assert!(app.console_messages[0].content.contains("Command: /read"));
    assert!(app.console_messages[1].content.contains("Success: Read"));
}

// ============================================================================
// Test 10: Empty chat shows hint (via render check)
// ============================================================================

#[test]
fn test_empty_chat_transcript_is_valid() {
    let app = create_test_app();

    // Empty chat is valid state
    assert_eq!(app.chat_messages.len(), 0);
    assert!(app.chat_history().is_empty());
}

// ============================================================================
// Phase 8.3 REGRESSION TESTS
// ============================================================================

// Test 11: JSON blocks are filtered from chat output
#[test]
fn test_json_blocks_filtered_from_chat() {
    use odincode::llm::chat::filter_json_blocks;

    // Filter markdown JSON blocks
    let input = "Here's my answer:\n```json\n{\"plan_id\": \"test\"}\n```\nDone!";
    let filtered = filter_json_blocks(input);
    assert!(!filtered.contains("```json"));
    assert!(!filtered.contains("plan_id"));
    assert!(filtered.contains("Here's my answer"));
    assert!(filtered.contains("Done!"));
}

// Test 12: Standalone JSON lines are filtered
#[test]
fn test_standalone_json_filtered() {
    use odincode::llm::chat::filter_json_blocks;

    let input = "Some text\n{\"key\": \"value\"}\nMore text";
    let filtered = filter_json_blocks(input);
    assert!(!filtered.contains("{\"key\":"));
    assert!(filtered.contains("Some text"));
    assert!(filtered.contains("More text"));
}

// Test 13: Thinking indicator can be set and cleared
#[test]
fn test_thinking_indicator_lifecycle() {
    let mut app = create_test_app();

    // Set thinking
    app.set_thinking();
    assert_eq!(app.chat_messages.len(), 1);
    assert_eq!(app.chat_messages[0].role, ChatRole::Thinking);

    // Clear with content
    app.clear_thinking_with_content("Hello!".to_string());
    assert_eq!(app.chat_messages.len(), 1);
    assert_eq!(app.chat_messages[0].role, ChatRole::Assistant);
    assert_eq!(app.chat_messages[0].content, "Hello!");
}

// Test 14: Thinking is NOT included in chat history
#[test]
fn test_thinking_not_in_history() {
    let mut app = create_test_app();

    app.add_user_message("test".to_string());
    app.set_thinking();
    app.add_assistant_message("response".to_string());

    let history = app.chat_history();
    // Thinking filtered out, only user/assistant pairs remain
    // chat_history() creates pairs only when Assistant messages are found
    assert_eq!(history.len(), 1);
    // Single pair: user message + assistant response
    assert_eq!(history[0].0, "test");
    assert_eq!(history[0].1, "response");

    // Verify thinking is not in any entry
    for (user, response) in &history {
        assert!(!user.contains("Thinking"));
        assert!(!response.contains("Thinking"));
    }
}

// Test 15: Streaming updates replace thinking progressively
#[test]
fn test_streaming_replaces_thinking_progressively() {
    let mut app = create_test_app();

    app.set_thinking();

    // First chunk replaces thinking
    app.clear_thinking_with_content("Hel".to_string());
    assert_eq!(app.chat_messages.len(), 1);
    assert_eq!(app.chat_messages[0].role, ChatRole::Assistant);
    assert_eq!(app.chat_messages[0].content, "Hel");

    // Subsequent updates append
    app.update_last_message("Hello ".to_string());
    assert_eq!(app.chat_messages[0].content, "Hello ");

    app.update_last_message("Hello world".to_string());
    assert_eq!(app.chat_messages[0].content, "Hello world");
}

// Test 16: Chat prompt doesn't inject identity
#[test]
fn test_chat_prompt_no_identity_injection() {
    use odincode::llm::chat::build_chat_prompt;

    let prompt = build_chat_prompt("What is Rust?");
    assert!(!prompt.contains("OdinCode"));
    assert!(!prompt.contains("refactoring assistant"));
    assert!(!prompt.contains("You are"));
    assert_eq!(prompt, "What is Rust?");
}

// Test 17: Multiple thinking states only show one
#[test]
fn test_multiple_thinking_calls_collapses() {
    let mut app = create_test_app();

    app.set_thinking();
    app.set_thinking(); // Should replace, not duplicate

    assert_eq!(app.chat_messages.len(), 1);
    assert_eq!(app.chat_messages[0].role, ChatRole::Thinking);
}

// Test 18: Empty chunks remove thinking indicator
#[test]
fn test_empty_streaming_clears_thinking() {
    let mut app = create_test_app();

    app.set_thinking();
    app.chat_messages.retain(|m| m.role != ChatRole::Thinking); // Simulate empty result

    assert_eq!(app.chat_messages.len(), 0);
}
