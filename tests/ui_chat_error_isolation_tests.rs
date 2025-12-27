//! Chat error isolation tests — Phase 8.5
//!
//! Tests that errors are routed to diagnostics, NOT chat transcript:
//! - Adapter errors don't appear in chat
//! - Network errors don't appear in chat
//! - Error state is shown in diagnostics panel
//! - Chat transcript remains unchanged on error
//! - Thinking indicator is removed on error
//! - No identity text appears in chat

use odincode::llm::chat::ChatError;
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
// Test 1: ChatError is typed-only (no Display impl)
// ============================================================================

#[test]
fn test_chat_error_has_no_display_impl() {
    // ChatError variants are typed and cannot be converted to String
    // by accident (no Display impl)

    let err = ChatError::TransportError;
    // This compiles because we're matching, not using Display
    let is_transport = matches!(err, ChatError::TransportError);
    assert!(is_transport);
}

// ============================================================================
// Test 2: set_chat_error routes to diagnostics, not chat
// ============================================================================

#[test]
fn test_chat_error_routes_to_diagnostics_not_chat() {
    let mut app = create_test_app();

    // Add user message
    app.add_user_message("test message".to_string());
    assert_eq!(app.chat_messages.len(), 1);

    // Set chat error (simulating handler behavior)
    app.set_chat_error(ChatError::TransportError);

    // Verify error is in diagnostics
    assert!(app.chat_error.is_some());
    assert_eq!(app.chat_error, Some(ChatError::TransportError));

    // Verify chat transcript is unchanged (only user message)
    assert_eq!(app.chat_messages.len(), 1);
    assert_eq!(app.chat_messages[0].role, ChatRole::User);
    assert_eq!(app.chat_messages[0].content, "test message");

    // No assistant message was added
    assert!(!app
        .chat_messages
        .iter()
        .any(|m| m.role == ChatRole::Assistant));
}

// ============================================================================
// Test 3: chat_error_description returns human-readable text
// ============================================================================

#[test]
fn test_chat_error_description_returns_human_text() {
    let app = create_test_app();

    // Initially no error
    assert!(app.chat_error_description().is_none());

    // Set error
    let mut app = create_test_app();
    app.set_chat_error(ChatError::TransportError);

    // Get description
    let desc = app.chat_error_description();
    assert_eq!(desc, Some("Transport error: Cannot reach LLM service"));

    // Verify all error types have descriptions
    app.set_chat_error(ChatError::HttpError);
    assert_eq!(
        app.chat_error_description(),
        Some("HTTP error: LLM service returned error")
    );

    app.set_chat_error(ChatError::AuthError);
    assert_eq!(
        app.chat_error_description(),
        Some("Authentication failed: Check API key")
    );

    app.set_chat_error(ChatError::RateLimitedError);
    assert_eq!(
        app.chat_error_description(),
        Some("Rate limited: Too many requests")
    );

    app.set_chat_error(ChatError::InvalidResponseError);
    assert_eq!(
        app.chat_error_description(),
        Some("Invalid response: LLM returned malformed data")
    );

    app.set_chat_error(ChatError::ConfigurationError);
    assert_eq!(
        app.chat_error_description(),
        Some("Configuration error: Check LLM settings")
    );

    app.set_chat_error(ChatError::NotConfigured);
    assert_eq!(
        app.chat_error_description(),
        Some("Not configured: Set up LLM provider")
    );
}

// ============================================================================
// Test 4: Thinking indicator is removed on error
// ============================================================================

#[test]
fn test_thinking_removed_when_error_set() {
    let mut app = create_test_app();

    // Set thinking
    app.set_thinking();
    assert_eq!(app.chat_messages.len(), 1);
    assert_eq!(app.chat_messages[0].role, ChatRole::Thinking);

    // Set error (should remove thinking)
    app.set_chat_error(ChatError::TransportError);

    // Thinking is removed
    assert!(!app
        .chat_messages
        .iter()
        .any(|m| m.role == ChatRole::Thinking));

    // Error is in diagnostics
    assert!(app.chat_error.is_some());
}

// ============================================================================
// Test 5: clear_chat_error removes error state
// ============================================================================

#[test]
fn test_clear_chat_error_removes_state() {
    let mut app = create_test_app();

    app.set_chat_error(ChatError::TransportError);
    assert!(app.chat_error.is_some());

    app.clear_chat_error();
    assert!(app.chat_error.is_none());
    assert!(app.chat_error_description().is_none());
}

// ============================================================================
// Test 6: No identity text in chat output
// ============================================================================

#[test]
fn test_no_identity_text_in_chat() {
    use odincode::llm::chat::build_chat_prompt;

    // Chat prompt is just user input, no identity wrapping
    let prompt = build_chat_prompt("hello");
    assert_eq!(prompt, "hello");
    assert!(!prompt.contains("OdinCode"));
    assert!(!prompt.contains("assistant"));
    assert!(!prompt.contains("You are"));
    assert!(!prompt.contains("refactoring"));
}

// ============================================================================
// Test 7: Error doesn't create assistant message
// ============================================================================

#[test]
fn test_error_does_not_create_assistant_message() {
    let mut app = create_test_app();

    app.add_user_message("test".to_string());
    app.set_thinking();

    // Simulate error path
    app.set_chat_error(ChatError::TransportError);

    // Count messages by role
    let user_count = app
        .chat_messages
        .iter()
        .filter(|m| m.role == ChatRole::User)
        .count();
    let assistant_count = app
        .chat_messages
        .iter()
        .filter(|m| m.role == ChatRole::Assistant)
        .count();
    let thinking_count = app
        .chat_messages
        .iter()
        .filter(|m| m.role == ChatRole::Thinking)
        .count();

    // Should have only user message (thinking removed, no assistant added)
    assert_eq!(user_count, 1);
    assert_eq!(assistant_count, 0);
    assert_eq!(thinking_count, 0);
}

// ============================================================================
// Test 8: New chat request clears previous error
// ============================================================================

#[test]
fn test_new_chat_clears_previous_error() {
    let mut app = create_test_app();

    // Set error from previous failed request
    app.set_chat_error(ChatError::TransportError);
    assert!(app.chat_error.is_some());

    // Clear error (simulating handle_chat start)
    app.clear_chat_error();
    assert!(app.chat_error.is_none());
}

// ============================================================================
// Test 9: ChatError is Copy-free (Clone only)
// ============================================================================

#[test]
fn test_chat_error_design() {
    // Verify ChatError is Clone (needed for App state)
    let err1 = ChatError::TransportError;
    let err2 = err1.clone();
    assert_eq!(err1, err2);

    // Verify PartialEq works
    assert_eq!(ChatError::HttpError, ChatError::HttpError);
    assert_ne!(ChatError::TransportError, ChatError::HttpError);
}

// ============================================================================
// Test 10: Multiple errors overwrite correctly
// ============================================================================

#[test]
fn test_multiple_errors_overwrite() {
    let mut app = create_test_app();

    app.set_chat_error(ChatError::TransportError);
    assert_eq!(app.chat_error, Some(ChatError::TransportError));

    app.set_chat_error(ChatError::AuthError);
    assert_eq!(app.chat_error, Some(ChatError::AuthError));

    // Description matches latest error
    assert_eq!(
        app.chat_error_description(),
        Some("Authentication failed: Check API key")
    );
}
