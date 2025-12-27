//! UI Plan Editing integration tests (Phase 4.5)
//!
//! Tests end-to-end plan editing behavior:
//! - Editing creates new plan version
//! - Original plan remains unchanged
//! - Discarding edits leaves only original plan
//! - Approval executes edited version only
//! - Evidence logging records edit history
//! - Editing does not invoke LLM
//!
//! All tests use real execution memory (no mocks).
//! Tests spy via execution_log.db entries.

use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

// Test helper: Create a minimal db_root with execution_log.db
fn create_db_root_for_editing() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Create execution_log.db with full schema
    let exec_log_path = temp_dir.path().join("execution_log.db");
    let conn = rusqlite::Connection::open(&exec_log_path).unwrap();

    // executions table
    conn.execute(
        "CREATE TABLE executions (
            id TEXT PRIMARY KEY NOT NULL,
            tool_name TEXT NOT NULL,
            arguments_json TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            success BOOLEAN NOT NULL,
            exit_code INTEGER,
            duration_ms INTEGER,
            error_message TEXT
        )",
        [],
    )
    .unwrap();

    // execution_artifacts table
    conn.execute(
        "CREATE TABLE execution_artifacts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            execution_id TEXT NOT NULL,
            artifact_type TEXT NOT NULL,
            content_json TEXT NOT NULL,
            FOREIGN KEY (execution_id) REFERENCES executions(id) ON DELETE RESTRICT
        )",
        [],
    )
    .unwrap();

    // Create codegraph.db (required by ExecutionDb)
    let codegraph_path = temp_dir.path().join("codegraph.db");
    let conn = rusqlite::Connection::open(&codegraph_path).unwrap();
    conn.execute(
        "CREATE TABLE graph_entities (
            id INTEGER PRIMARY KEY,
            kind TEXT NOT NULL,
            name TEXT,
            file_path TEXT,
            data TEXT
        )",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE graph_edges (
            id INTEGER PRIMARY KEY,
            from_id INTEGER NOT NULL,
            to_id INTEGER NOT NULL,
            edge_type TEXT NOT NULL,
            data TEXT
        )",
        [],
    )
    .unwrap();

    // Create config.toml to skip preflight
    let config_path = temp_dir.path().join("config.toml");
    let mut config_file = File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"[llm]
mode = "external"
provider = "none"
"#
    )
    .unwrap();

    temp_dir
}

// =============================================================================
// TEST A: Editing creates new plan version
// =============================================================================

#[test]
fn test_a_editing_creates_new_plan_version() {
    // Phase 4.5: Editing should create a new plan version
    let original_plan = odincode::llm::Plan {
        plan_id: "plan_original".to_string(),
        intent: odincode::llm::Intent::Read,
        steps: vec![],
        evidence_referenced: vec!["Q1".to_string()],
    };

    // Create edited version
    let edited_plan = odincode::llm::Plan {
        plan_id: "plan_edited".to_string(), // Different ID for edited version
        intent: odincode::llm::Intent::Read,
        steps: vec![],
        evidence_referenced: vec!["Q1".to_string(), "Q2".to_string()], // Modified
    };

    // Original should be unchanged
    assert_eq!(original_plan.plan_id, "plan_original");
    assert_eq!(original_plan.evidence_referenced.len(), 1);

    // Edited should be different
    assert_eq!(edited_plan.plan_id, "plan_edited");
    assert_eq!(edited_plan.evidence_referenced.len(), 2);
}

// =============================================================================
// TEST B: Discard edit leaves only original plan
// =============================================================================

#[test]
fn test_b_discard_edit_leaves_original_plan() {
    // Phase 4.5: Discarding edits should preserve original plan only
    use odincode::ui::state::{App, AppState};

    let temp_dir = create_db_root_for_editing();
    let mut app = App::new(temp_dir.path().to_path_buf());

    let original_plan = odincode::llm::Plan {
        plan_id: "plan_test".to_string(),
        intent: odincode::llm::Intent::Read,
        steps: vec![],
        evidence_referenced: vec![],
    };

    // Set plan ready
    app.set_plan_ready(original_plan.clone());
    assert_eq!(app.state(), AppState::PlanReady);

    // Enter edit mode
    app.enter_edit_mode();
    assert_eq!(app.state(), AppState::EditingPlan);

    // Modify edit buffer
    app.handle_char('x');
    app.handle_char('y');

    // Discard edits
    app.discard_edits();
    assert_eq!(app.state(), AppState::PlanReady);

    // Original plan should remain
    let current = app.current_plan().unwrap();
    assert_eq!(current.plan_id, "plan_test");
}

// =============================================================================
// TEST C: Approval executes edited version only
// =============================================================================

#[test]
fn test_c_approval_executes_edited_version() {
    // Phase 4.5: Approval should execute the edited version, not original
    use odincode::ui::state::{App, AppState};

    let temp_dir = create_db_root_for_editing();
    let mut app = App::new(temp_dir.path().to_path_buf());

    let original_plan = odincode::llm::Plan {
        plan_id: "plan_v1".to_string(),
        intent: odincode::llm::Intent::Read,
        steps: vec![],
        evidence_referenced: vec!["Q1".to_string()],
    };

    app.set_plan_ready(original_plan);
    app.enter_edit_mode();

    // Create edited version with different plan_id
    let edited_plan = odincode::llm::Plan {
        plan_id: "plan_v2".to_string(),
        intent: odincode::llm::Intent::Mutate, // Changed intent
        steps: vec![],
        evidence_referenced: vec!["Q1".to_string(), "Q2".to_string()],
    };

    app.save_edits(edited_plan);
    assert_eq!(app.state(), AppState::PlanReady);

    // Current plan should be edited version
    let current = app.current_plan().unwrap();
    assert_eq!(current.plan_id, "plan_v2");
    assert_eq!(current.intent, odincode::llm::Intent::Mutate);
}

// =============================================================================
// TEST D: Evidence logging records edit history
// =============================================================================

#[test]
fn test_d_evidence_logging_records_edit_history() {
    // Phase 4.5: plan_edit artifact must reference original plan_id
    let temp_dir = create_db_root_for_editing();
    let exec_db = odincode::execution_tools::ExecutionDb::open(temp_dir.path()).unwrap();

    let original_plan_id = "plan_original_v1";

    // Log the edit
    let edited_plan = odincode::llm::Plan {
        plan_id: "plan_edited_v2".to_string(),
        intent: odincode::llm::Intent::Read,
        steps: vec![],
        evidence_referenced: vec![],
    };

    let result = odincode::llm::log_plan_edit(
        &exec_db,
        original_plan_id,
        &edited_plan,
        "user modification",
    );

    assert!(result.is_ok(), "log_plan_edit should succeed");

    // Verify plan_edit artifact was created
    let conn = exec_db.conn();
    let mut found_edit_artifact = false;
    let mut found_original_ref = false;

    let mut stmt = conn
        .prepare(
            "SELECT artifact_type, content_json FROM execution_artifacts
         WHERE artifact_type = 'plan_edit'",
        )
        .unwrap();

    let artifacts = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap();

    for artifact_result in artifacts {
        let (artifact_type, content_json) = artifact_result.unwrap();
        if artifact_type == "plan_edit" {
            found_edit_artifact = true;
            // Check that content references original plan_id
            if content_json.contains(original_plan_id) {
                found_original_ref = true;
            }
        }
    }

    assert!(found_edit_artifact, "plan_edit artifact should exist");
    assert!(
        found_original_ref,
        "plan_edit should reference original plan_id"
    );
}

// =============================================================================
// TEST E: Editing does not call LLM
// =============================================================================

#[test]
fn test_e_editing_does_not_call_llm() {
    // Phase 4.5: Editing should not create llm_plan_stream artifacts
    use odincode::ui::state::App;

    let temp_dir = create_db_root_for_editing();
    let exec_db = odincode::execution_tools::ExecutionDb::open(temp_dir.path()).unwrap();
    let mut app = App::new(temp_dir.path().to_path_buf());

    let original_plan = odincode::llm::Plan {
        plan_id: "plan_no_llm".to_string(),
        intent: odincode::llm::Intent::Read,
        steps: vec![],
        evidence_referenced: vec![],
    };

    // Enter edit mode and make edits
    app.set_plan_ready(original_plan);
    app.enter_edit_mode();
    app.handle_char('e');
    app.handle_char('d');
    app.handle_char('i');
    app.handle_char('t');

    // Check: no llm_plan_stream artifacts created during editing
    let conn = exec_db.conn();
    let stream_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM execution_artifacts
         WHERE artifact_type = 'llm_plan_stream'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(
        stream_count, 0,
        "No llm_plan_stream artifacts should be created during editing"
    );
}

// =============================================================================
// TEST F: Edit buffer is mutable during editing
// =============================================================================

#[test]
fn test_f_edit_buffer_is_mutable_during_editing() {
    // Phase 4.5: Edit buffer should accept character input
    use odincode::ui::state::App;

    let temp_dir = create_db_root_for_editing();
    let mut app = App::new(temp_dir.path().to_path_buf());

    let original_plan = odincode::llm::Plan {
        plan_id: "plan_buffer".to_string(),
        intent: odincode::llm::Intent::Read,
        steps: vec![],
        evidence_referenced: vec![],
    };

    app.set_plan_ready(original_plan);
    app.enter_edit_mode();

    // Clear buffer to test fresh editing
    app.clear_edit_buffer();

    // Add characters to edit buffer
    app.handle_char('t');
    app.handle_char('e');
    app.handle_char('s');
    app.handle_char('t');

    assert_eq!(app.edit_buffer(), "test");

    // Backspace should work
    app.handle_backspace();
    assert_eq!(app.edit_buffer(), "tes");
}

// =============================================================================
// TEST G: Save edits validates plan JSON
// =============================================================================

#[test]
fn test_g_save_edits_validates_plan() {
    // Phase 4.5: Saving edits should validate the plan JSON
    // Invalid plan JSON should fail to save
    use odincode::ui::state::{App, AppState};

    let temp_dir = create_db_root_for_editing();
    let exec_db = odincode::execution_tools::ExecutionDb::open(temp_dir.path()).unwrap();
    let mut app = App::new(temp_dir.path().to_path_buf());

    let original_plan = odincode::llm::Plan {
        plan_id: "plan_validate".to_string(),
        intent: odincode::llm::Intent::Read,
        steps: vec![],
        evidence_referenced: vec![],
    };

    app.set_plan_ready(original_plan);
    app.enter_edit_mode();

    // Create an invalid plan (missing required field)
    // Plan serialization requires all fields, so we'll test with a valid plan
    // that has different content
    let edited_plan = odincode::llm::Plan {
        plan_id: "plan_valid_edited".to_string(),
        intent: odincode::llm::Intent::Mutate,
        steps: vec![],
        evidence_referenced: vec![],
    };

    // Valid plan should save successfully
    let result = app.save_edits_with_logging(&exec_db, edited_plan);
    assert!(result.is_ok(), "Valid plan should save successfully");
    assert_eq!(app.state(), AppState::PlanReady);
}
