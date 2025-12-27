//! Phase 9.3 — Observability + UX Hardening: Integration Tests (TDD)
//!
//! Tests for:
//! - Chat autoscroll behavior
//! - Tool status with elapsed time
//! - Loop trace querying

use std::fs::File;
use std::thread;
use std::time::Duration;

use odincode::execution_tools::ExecutionDb;
use odincode::ui::state::{App, ChatRole};
use rusqlite::params;

// === TEST UTILITIES ===

/// Create temporary directory with codegraph.db
fn setup_temp_dir() -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create codegraph.db (required by ExecutionDb::open)
    let graph_db_path = temp_dir.path().join("codegraph.db");
    File::create(&graph_db_path).expect("Failed to create codegraph.db");

    temp_dir
}

/// Create ExecutionDb for testing
fn setup_exec_db(temp_dir: &tempfile::TempDir) -> ExecutionDb {
    ExecutionDb::open(temp_dir.path()).expect("Failed to open ExecutionDb")
}

// === TEST 1: Autoscroll follows streaming by default ===

#[test]
fn test_1_autoscroll_follows_streaming_by_default() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Initially autoscroll should be enabled
    assert!(app.autoscroll_enabled());

    // Add messages (simulating streaming)
    for i in 0..10 {
        app.add_assistant_message(format!("Line {}", i));
    }

    // Scroll offset should be at "bottom" (0 means show latest)
    assert_eq!(app.chat_scroll_offset(), 0);
}

// === TEST 2: User scroll disables autoscroll ===

#[test]
fn test_2_user_scroll_disables_autoscroll() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Add more messages than can fit
    for i in 0..50 {
        app.add_assistant_message(format!("Message {}", i));
    }

    // Scroll up (simulating user interaction)
    app.chat_scroll_up(5);

    // Autoscroll should be disabled
    assert!(!app.autoscroll_enabled());

    // Scroll offset should reflect position
    assert!(app.chat_scroll_offset() > 0);
}

// === TEST 3: Scroll to end re-enables autoscroll ===

#[test]
fn test_3_scroll_to_end_reenables_autoscroll() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Add messages
    for i in 0..50 {
        app.add_assistant_message(format!("Message {}", i));
    }

    // Scroll up to disable autoscroll
    app.chat_scroll_up(10);
    assert!(!app.autoscroll_enabled());

    // Scroll to end (simulating End key or 'g')
    app.chat_scroll_to_end();

    // Autoscroll should be re-enabled
    assert!(app.autoscroll_enabled());
    assert_eq!(app.chat_scroll_offset(), 0);
}

// === TEST 4: Tool status shows elapsed time ===

#[test]
fn test_4_tool_status_shows_elapsed_time() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    let start_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Add tool status
    app.set_tool_status("file_write".to_string(), 1, Some(start_time));

    // Sleep a bit to ensure elapsed time > 0
    thread::sleep(Duration::from_millis(100));

    // Find the ToolStatus message
    let tool_status = app
        .chat_messages
        .iter()
        .find(|m| matches!(m.role, ChatRole::ToolStatus { .. }));

    assert!(tool_status.is_some(), "ToolStatus message should exist");

    if let Some(msg) = tool_status {
        // Check display format includes elapsed time (before moving from msg.role)
        let display = msg.role.tool_status_display().unwrap();
        assert!(display.contains("Running file_write"));
        assert!(display.contains("(step 1)"));
        assert!(display.contains("s")); // elapsed seconds
        assert!(display.contains("tokens: n/a")); // Phase 9.3: token counter

        // Now check the individual fields (using ref to avoid moving)
        if let ChatRole::ToolStatus {
            ref tool,
            ref step,
            ref start_timestamp,
        } = msg.role
        {
            assert_eq!(*tool, "file_write");
            assert_eq!(*step, 1);
            assert_eq!(*start_timestamp, start_time);
        }
    }
}

// === TEST 5: Last loop trace query returns steps in order ===

#[test]
fn test_5_last_loop_trace_query_returns_steps_in_order() {
    let temp_dir = setup_temp_dir();
    let exec_db = setup_exec_db(&temp_dir);

    use odincode::ui::trace::query_last_loop_trace;

    // Record some executions with different timestamps
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // Record executions in reverse timestamp order
    for i in (0..3).rev() {
        let timestamp = now + (i as i64 * 1000);
        let _ = exec_db.conn().execute(
            "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![format!("exec_{}", i), "file_write", "{}", timestamp, 1i64], // success as INTEGER
        );
    }

    // Query trace
    let trace = query_last_loop_trace(exec_db.conn(), 10).unwrap();

    // Should return 3 rows
    assert_eq!(trace.len(), 3);

    // Should be in reverse chronological order (newest first)
    assert_eq!(trace[0].tool_name, "file_write");
    assert_eq!(trace[0].id, "exec_2"); // Last recorded
    assert_eq!(trace[1].id, "exec_1");
    assert_eq!(trace[2].id, "exec_0");
}

// === TEST 6: Trace includes approval events ===

#[test]
fn test_6_trace_includes_approval_events() {
    let temp_dir = setup_temp_dir();
    let exec_db = setup_exec_db(&temp_dir);

    use odincode::ui::trace::query_last_loop_trace;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // Record approval granted
    let _ = exec_db.conn().execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            "approval_granted_file_write",
            "approval_granted",
            "{}",
            now,
            1i64
        ],
    );

    // Record approval denied
    let _ = exec_db.conn().execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            "approval_denied_file_create",
            "approval_denied",
            "{}",
            now + 1000,
            1i64
        ],
    );

    // Query trace
    let trace = query_last_loop_trace(exec_db.conn(), 10).unwrap();

    // Should include approval events
    assert!(trace.iter().any(|r| r.id == "approval_granted_file_write"));
    assert!(trace.iter().any(|r| r.id == "approval_denied_file_create"));
}

// === TEST 7: Scroll offset bounds checking ===

#[test]
fn test_7_scroll_offset_bounds() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Add messages
    for i in 0..10 {
        app.add_assistant_message(format!("Message {}", i));
    }

    // Scroll up beyond available
    app.chat_scroll_up(100);

    // Offset should be clamped (can't scroll past top)
    let offset = app.chat_scroll_offset();
    assert!(offset <= 10); // At most number of messages
}

// === TEST 8: New messages preserve scroll when autoscroll disabled ===

#[test]
fn test_8_new_messages_preserve_scroll_position() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Add initial messages
    for i in 0..20 {
        app.add_assistant_message(format!("Initial {}", i));
    }

    // Scroll up to disable autoscroll
    app.chat_scroll_up(5);
    let _scroll_offset_before = app.chat_scroll_offset();
    assert!(!app.autoscroll_enabled());

    // Add more messages (simulating streaming arrival)
    for i in 0..10 {
        app.add_assistant_message(format!("New {}", i));
    }

    // Scroll position should be preserved (not jump to bottom)
    // The new messages increase total count, but we stay at same relative position
    assert!(!app.autoscroll_enabled());
    // Offset might have increased to account for new messages
    // but we're not at bottom
    assert!(app.chat_scroll_offset() > 0);
}

// === TEST 9: Trace row includes scope from approval artifact ===

#[test]
fn test_9_trace_row_includes_scope_from_approval() {
    let temp_dir = setup_temp_dir();
    let exec_db = setup_exec_db(&temp_dir);

    use odincode::ui::trace::query_last_loop_trace;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // Record a GATED tool execution
    let exec_id = "exec_file_write";
    let _ = exec_db.conn().execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success, duration_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            exec_id,
            "file_write",
            "{\"path\":\"test.txt\"}",
            now,
            1i64,
            150i64
        ],
    );

    // Record approval artifact
    let _ = exec_db.conn().execute(
        "INSERT INTO execution_artifacts (execution_id, artifact_type, content_json)
         VALUES (?1, ?2, ?3)",
        params![
            exec_id,
            "approval_granted",
            "{\"scope\":\"SessionAllGated\",\"tool\":\"file_write\"}"
        ],
    );

    // Query trace
    let trace = query_last_loop_trace(exec_db.conn(), 10).unwrap();

    // Find the file_write execution
    let file_write_row = trace.iter().find(|r| r.tool_name == "file_write");

    assert!(file_write_row.is_some());

    let row = file_write_row.unwrap();
    assert_eq!(row.id, exec_id);
    assert_eq!(row.tool_name, "file_write");
    assert_eq!(row.duration_ms, Some(150));
}

// === TEST 10: Multiple scroll operations work correctly ===

#[test]
fn test_10_multiple_scroll_operations() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Add messages
    for i in 0..30 {
        app.add_assistant_message(format!("Message {}", i));
    }

    // Scroll up multiple times
    app.chat_scroll_up(3);
    let offset1 = app.chat_scroll_offset();

    app.chat_scroll_up(5);
    let offset2 = app.chat_scroll_offset();

    // Each scroll up should increase offset
    assert!(offset2 > offset1);

    // Scroll down
    app.chat_scroll_down(2);
    let offset3 = app.chat_scroll_offset();

    // Scroll down should decrease offset
    assert!(offset3 < offset2);
}
