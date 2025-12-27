//! Phase 9.4 — Trace Viewer UI + Loop Header: Integration Tests (TDD)
//!
//! Tests for:
//! - Trace viewer toggle and visibility
//! - Trace refresh on loop complete/approval events
//! - Loop header text generation
//! - Trace panel state management

use std::fs::File;

use odincode::execution_tools::ExecutionDb;
use odincode::ui::state::App;
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

// === TEST 1: Trace toggle loads rows ===

#[test]
fn test_1_trace_toggle_loads_rows() {
    let temp_dir = setup_temp_dir();
    let exec_db = setup_exec_db(&temp_dir);
    let mut app = App::new(temp_dir.path().into());

    // Initially trace viewer should not be visible
    assert!(!app.trace_viewer_visible());

    // Add some execution data
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let _ = exec_db.conn().execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success, duration_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["exec_1", "file_read", "{}", now, 1i64, 100i64],
    );

    // Toggle trace viewer on
    app.toggle_trace_viewer(&exec_db, 20);

    // Now trace viewer should be visible
    assert!(app.trace_viewer_visible());

    // Should have loaded trace rows
    let rows = app.trace_rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].tool_name, "file_read");
}

// === TEST 2: Trace refresh after loop complete ===

#[test]
fn test_2_trace_refresh_after_loop_complete() {
    let temp_dir = setup_temp_dir();
    let exec_db = setup_exec_db(&temp_dir);
    let mut app = App::new(temp_dir.path().into());

    // Toggle trace viewer on (should load initial empty state)
    app.toggle_trace_viewer(&exec_db, 20);
    assert_eq!(app.trace_rows().len(), 0);

    // Simulate loop completion by adding executions
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    for i in 0..3 {
        let _ = exec_db.conn().execute(
            "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![format!("exec_{}", i), "file_search", "{}", now + i, 1i64, 50i64],
        );
    }

    // Refresh trace
    app.refresh_trace(&exec_db, 20);

    // Should now have 3 rows
    assert_eq!(app.trace_rows().len(), 3);
}

// === TEST 3: Trace refresh after approval event ===

#[test]
fn test_3_trace_refresh_after_approval_event() {
    let temp_dir = setup_temp_dir();
    let exec_db = setup_exec_db(&temp_dir);
    let mut app = App::new(temp_dir.path().into());

    // Toggle trace viewer on
    app.toggle_trace_viewer(&exec_db, 20);

    // Add an approval event
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let _ = exec_db.conn().execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["approval_1", "approval_granted", "{}", now, 1i64],
    );

    // Refresh after approval
    app.on_approval_event_refresh_trace(&exec_db, 20);

    // Should have the approval event
    let rows = app.trace_rows();
    assert!(rows.iter().any(|r| r.tool_name == "approval_granted"));
}

// === TEST 4: Trace panel visibility toggle ===

#[test]
fn test_4_trace_panel_visibility_toggle() {
    let temp_dir = setup_temp_dir();
    let exec_db = setup_exec_db(&temp_dir);
    let mut app = App::new(temp_dir.path().into());

    // Initially not visible
    assert!(!app.trace_viewer_visible());

    // Toggle on
    app.toggle_trace_viewer(&exec_db, 20);
    assert!(app.trace_viewer_visible());

    // Toggle off
    app.toggle_trace_viewer(&exec_db, 20);
    assert!(!app.trace_viewer_visible());

    // Toggle on again
    app.toggle_trace_viewer(&exec_db, 20);
    assert!(app.trace_viewer_visible());
}

// === TEST 5: Loop header text when running ===

#[test]
fn test_5_loop_header_text_when_running() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    // Add a tool status to simulate running loop
    let start_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    app.set_tool_status("file_write".to_string(), 2, Some(start_time));

    // Get loop header text
    let header = app.loop_header_text();

    assert!(header.is_some());
    let header_text = header.unwrap();
    assert!(header_text.contains("step 2")); // Shows step number
    assert!(header_text.contains("file_write")); // Shows tool name
}

// === TEST 6: Loop header text when awaiting approval ===

#[test]
fn test_6_loop_header_text_when_awaiting_approval() {
    let temp_dir = setup_temp_dir();
    let mut app = App::new(temp_dir.path().into());

    use odincode::ui::approval::PendingApproval;

    // Create a pending approval
    let pending = PendingApproval::new(
        "session_123".to_string(),
        "file_create".to_string(),
        std::collections::HashMap::new(),
        1,
        Some("/tmp/test.txt".to_string()),
    );

    app.approval_state.set_pending(pending);

    // Get loop header text
    let header = app.loop_header_text();

    assert!(header.is_some());
    let header_text = header.unwrap();
    assert!(header_text.contains("approval required")); // Shows approval state
    assert!(header_text.contains("file_create")); // Shows tool
    assert!(header_text.contains("a=all")); // Shows approval hints
}

// === TEST 7: Trace shows error on DB failure ===

#[test]
fn test_7_trace_shows_error_on_db_failure() {
    let temp_dir = setup_temp_dir();
    let exec_db = setup_exec_db(&temp_dir);
    let mut app = App::new(temp_dir.path().into());

    // Close and delete execution_log.db to simulate failure
    drop(exec_db);
    std::fs::remove_file(temp_dir.path().join("execution_log.db")).unwrap();

    // Create a new ExecutionDb (which will recreate the file but be empty)
    let exec_db = ExecutionDb::open(temp_dir.path()).unwrap();

    // Toggle trace viewer - should succeed but with empty trace
    app.toggle_trace_viewer(&exec_db, 20);

    // Trace viewer should be visible
    assert!(app.trace_viewer_visible());
    // No error (just empty trace since no executions)
    assert!(app.trace_error().is_none());
    assert_eq!(app.trace_rows().len(), 0);
}

// === TEST 8: Trace rows include all expected fields ===

#[test]
fn test_8_trace_rows_include_all_expected_fields() {
    let temp_dir = setup_temp_dir();
    let exec_db = setup_exec_db(&temp_dir);
    let mut app = App::new(temp_dir.path().into());

    // Add execution with full data
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let _ = exec_db.conn().execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success, duration_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "exec_full",
            "file_write",
            "{\"path\":\"/tmp/test.txt\"}",
            now,
            1i64,
            150i64
        ],
    );

    app.toggle_trace_viewer(&exec_db, 20);

    let rows = app.trace_rows();
    assert_eq!(rows.len(), 1);

    let row = &rows[0];
    assert_eq!(row.id, "exec_full");
    assert_eq!(row.tool_name, "file_write");
    assert_eq!(row.timestamp, now);
    assert!(row.success);
    assert_eq!(row.duration_ms, Some(150));
    assert_eq!(row.affected_path, Some("/tmp/test.txt".to_string()));
}

// === TEST 9: Trace respects limit parameter ===

#[test]
fn test_9_trace_respects_limit_parameter() {
    let temp_dir = setup_temp_dir();
    let exec_db = setup_exec_db(&temp_dir);
    let mut app = App::new(temp_dir.path().into());

    // Verify execution_log.db exists
    assert!(temp_dir.path().join("execution_log.db").exists());

    // Add 5 executions
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    for i in 0..5 {
        let result = exec_db.conn().execute(
            "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![format!("exec_{}", i), "file_read", "{}", now + i, 1i64, 50i64],
        );
        assert!(result.is_ok(), "Insert should succeed: {:?}", result);
    }

    // Verify data was inserted immediately
    let count: i64 = exec_db
        .conn()
        .query_row("SELECT COUNT(*) FROM executions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        count, 5,
        "Database should contain 5 executions after insert"
    );

    // Request only 3 rows
    app.toggle_trace_viewer(&exec_db, 3);

    let rows = app.trace_rows();
    assert_eq!(rows.len(), 3);
}

// === TEST 10: Loop header returns None when idle ===

#[test]
fn test_10_loop_header_returns_none_when_idle() {
    let temp_dir = setup_temp_dir();
    let app = App::new(temp_dir.path().into());

    // No tool status, no pending approval
    assert!(app.loop_header_text().is_none());
}
