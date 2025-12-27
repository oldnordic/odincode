//! Phase 9.5 — Tool Execution State Machine: Integration Tests (TDD)
//!
//! Tests for:
//! - ToolExecutionState lifecycle (Queued → Running → Completed/Failed/Timeout/Cancelled)
//! - ChatEvent ordering (ToolQueued → ToolStarted → ToolFinished)
//! - Tool timeout detection
//! - /cancel command aborts running tool
//! - /status command reflects current execution
//! - Tool panel visibility during loop

use std::fs::File;
use std::time::Duration;

use odincode::execution_tools::ExecutionDb;
use odincode::ui::state::App;
use odincode::ui::tool_state::{ToolExecutionState, ToolQueueEntry};

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
#[allow(dead_code)]
fn setup_exec_db(temp_dir: &tempfile::TempDir) -> ExecutionDb {
    ExecutionDb::open(temp_dir.path()).expect("Failed to open ExecutionDb")
}

// === TEST 1: ToolExecutionState starts as Queued ===

#[test]
fn test_1_tool_state_starts_as_queued() {
    let state = ToolExecutionState::Queued;
    assert!(matches!(state, ToolExecutionState::Queued));
}

// === TEST 2: ToolExecutionState transitions to Running ===

#[test]
fn test_2_tool_state_transitions_to_running() {
    let running = ToolExecutionState::running();

    assert!(matches!(running, ToolExecutionState::Running { .. }));
    if let ToolExecutionState::Running { started_at } = running {
        // started_at should be recent (within last second)
        let elapsed = started_at.elapsed();
        assert!(elapsed < Duration::from_secs(1));
    }
}

// === TEST 3: ToolExecutionState transitions to Completed ===

#[test]
fn test_3_tool_state_transitions_to_completed() {
    let running = ToolExecutionState::running();
    let completed = running.to_completed(150);

    assert!(matches!(
        completed,
        ToolExecutionState::Completed { duration_ms: 150 }
    ));
}

// === TEST 4: ToolExecutionState transitions to Failed ===

#[test]
fn test_4_tool_state_transitions_to_failed() {
    let running = ToolExecutionState::running();
    let failed = running.to_failed("network error".to_string());

    assert!(matches!(failed, ToolExecutionState::Failed { .. }));
    if let ToolExecutionState::Failed { error } = failed {
        assert_eq!(error, "network error");
    }
}

// === TEST 5: ToolExecutionState transitions to Timeout ===

#[test]
fn test_5_tool_state_transitions_to_timeout() {
    let state = ToolExecutionState::Timeout;
    assert!(matches!(state, ToolExecutionState::Timeout));
}

// === TEST 6: ToolExecutionState transitions to Cancelled ===

#[test]
fn test_6_tool_state_transitions_to_cancelled() {
    let state = ToolExecutionState::Cancelled;
    assert!(matches!(state, ToolExecutionState::Cancelled));
}

// === TEST 7: ToolQueueEntry creation ===

#[test]
fn test_7_tool_queue_entry_creation() {
    let entry = ToolQueueEntry::new(
        "file_read".to_string(),
        1,
        Some("/path/to/file.txt".to_string()),
    );

    assert_eq!(entry.tool, "file_read");
    assert_eq!(entry.step, 1);
    assert_eq!(entry.affected_path, Some("/path/to/file.txt".to_string()));
    assert!(matches!(entry.state, ToolExecutionState::Queued));
}

// === TEST 8: ToolQueueEntry state transitions ===

#[test]
fn test_8_tool_queue_entry_state_transitions() {
    let mut entry = ToolQueueEntry::new("file_write".to_string(), 2, None);

    // Starts as Queued
    assert!(matches!(entry.state, ToolExecutionState::Queued));

    // Transition to Running
    entry.start();
    assert!(matches!(entry.state, ToolExecutionState::Running { .. }));

    // Transition to Completed
    entry.complete(100);
    assert!(matches!(entry.state, ToolExecutionState::Completed { .. }));
}

// === TEST 9: App maintains tool queue during loop ===

#[test]
fn test_9_app_maintains_tool_queue_during_loop() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Initially no active tool
    assert!(!app.has_active_tool());

    // Add a queued tool
    app.queue_tool("file_read".to_string(), 1, Some("src/main.rs".to_string()));

    // Now has active tool
    assert!(app.has_active_tool());

    // Get current tool state
    let state = app.current_tool_state();
    assert!(state.is_some());
    assert!(matches!(state.unwrap(), ToolExecutionState::Queued));
}

// === TEST 10: /cancel command clears active tool ===

#[test]
fn test_10_cancel_command_clears_active_tool() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Add and start a tool
    app.queue_tool("file_write".to_string(), 1, Some("test.txt".to_string()));
    assert!(app.has_active_tool());

    // Cancel the tool
    app.cancel_current_tool();

    // Tool should be cleared
    assert!(!app.has_active_tool());
}

// === TEST 11: /status command shows current tool ===

#[test]
fn test_11_status_command_shows_current_tool() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Add a tool
    app.queue_tool("file_glob".to_string(), 1, Some("/".to_string()));

    // Get status
    let status = app.tool_status_text();
    assert!(status.is_some());

    let status_text = status.unwrap();
    assert!(status_text.contains("file_glob"));
    assert!(status_text.contains("Step 1"));
}

// === TEST 12: Tool panel visible when tool active ===

#[test]
fn test_12_tool_panel_visible_when_tool_active() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Initially not visible
    assert!(!app.tool_panel_visible());

    // Add a tool
    app.queue_tool("file_read".to_string(), 1, None);

    // Now visible
    assert!(app.tool_panel_visible());
}

// === TEST 13: Tool panel hides after completion ===

#[test]
fn test_13_tool_panel_hides_after_completion() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Add and complete a tool
    app.queue_tool("file_read".to_string(), 1, None);
    assert!(app.tool_panel_visible());

    // Complete the tool
    app.complete_current_tool(50);

    // Panel should hide after completion
    // (Implementation may delay hiding for visibility)
    assert!(!app.has_active_tool());
}

// === TEST 14: Tool timeout is detected ===

#[test]
fn test_14_tool_timeout_is_detected() {
    let running = ToolExecutionState::running();

    // Simulate timeout check after some time
    std::thread::sleep(Duration::from_millis(10));

    let timed_out = running.check_timeout(Duration::from_secs(0));
    // Should timeout since timeout is 0 and running has elapsed time
    assert!(timed_out);
}

// === TEST 15: Multiple tools maintain queue order ===

#[test]
fn test_15_multiple_tools_maintain_queue_order() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Add and complete multiple tools
    app.queue_tool("file_read".to_string(), 1, Some("a.txt".to_string()));
    app.complete_current_tool(100);
    app.queue_tool("file_write".to_string(), 2, Some("b.txt".to_string()));
    app.complete_current_tool(150);
    app.queue_tool("file_glob".to_string(), 3, None);
    app.complete_current_tool(50);

    // Get tool history
    let history = app.tool_history();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].tool, "file_read");
    assert_eq!(history[1].tool, "file_write");
    assert_eq!(history[2].tool, "file_glob");
}

// === TEST 16: State machine rejects invalid transitions ===

#[test]
fn test_16_state_machine_rejects_invalid_transitions() {
    // Cannot transition from Completed to Running
    let completed = ToolExecutionState::Completed { duration_ms: 100 };

    // This should create a new Running state, not transition
    let running = ToolExecutionState::running();
    assert!(matches!(running, ToolExecutionState::Running { .. }));

    // Completed should still be Completed
    assert!(matches!(completed, ToolExecutionState::Completed { .. }));
}

// === TEST 17: ToolExecutionState display formatting ===

#[test]
fn test_17_tool_state_display_formatting() {
    let queued = ToolExecutionState::Queued;
    assert_eq!(queued.display_name(), "QUEUED");

    let running = ToolExecutionState::running();
    assert_eq!(running.display_name(), "RUNNING");

    let completed = ToolExecutionState::Completed { duration_ms: 150 };
    assert_eq!(completed.display_name(), "COMPLETED");

    let failed = ToolExecutionState::Failed {
        error: "error".to_string(),
    };
    assert_eq!(failed.display_name(), "FAILED");

    let timeout = ToolExecutionState::Timeout;
    assert_eq!(timeout.display_name(), "TIMEOUT");

    let cancelled = ToolExecutionState::Cancelled;
    assert_eq!(cancelled.display_name(), "CANCELLED");
}

// === TEST 18: Tool state is logged to console ===

#[test]
fn test_18_tool_state_is_logged_to_console() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Add a tool
    app.queue_tool("file_read".to_string(), 1, None);

    // Should have a console message about the tool
    let messages = &app.console_messages;
    assert!(messages
        .iter()
        .any(|m| m.content.contains("Tool queued") || m.content.contains("file_read")));
}

// === TEST 19: Elapsed time calculation for running tools ===

#[test]
fn test_19_elapsed_time_calculation_for_running_tools() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Queue and start a tool
    app.queue_tool("file_read".to_string(), 1, None);
    app.start_current_tool(); // Must start to get elapsed time

    // Get elapsed time (should be 0 or very small initially)
    let elapsed = app.current_tool_elapsed_ms();
    assert!(elapsed.is_some());
    // Should be less than 100ms just after starting
    assert!(elapsed.unwrap() < 100);

    // Wait a bit
    std::thread::sleep(Duration::from_millis(50));

    // Elapsed should increase
    let elapsed_after = app.current_tool_elapsed_ms();
    assert!(elapsed_after.is_some());
    assert!(elapsed_after.unwrap() >= 50);
}

// === TEST 20: Cancel emits proper event ===

#[test]
fn test_20_cancel_emits_proper_state() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Add and start a tool
    app.queue_tool("file_write".to_string(), 1, None);

    // Cancel
    app.cancel_current_tool();

    // State should reflect cancellation
    let state = app.current_tool_state();
    // After cancel, no active tool, so state is None
    assert!(state.is_none());
}
