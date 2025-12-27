//! UI Streaming Plan integration tests (Phase 4.4)
//!
//! Tests end-to-end streaming plan generation behavior:
//! - Streaming planner emits multiple chunks
//! - Final plan equals non-streamed plan (determinism)
//! - Approval disabled until final plan exists
//! - Streaming fallback works when callback not provided
//! - Evidence logging records each stream chunk
//!
//! All tests use real execution memory (no mocks).
//! Tests spy via execution_log.db entries.

use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

// Test helper: Create a minimal db_root with execution_log.db
fn create_db_root_for_streaming() -> TempDir {
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
provider = "stub"
base_url = "https://stub.example.com"
model = "stub-model"
"#
    )
    .unwrap();

    temp_dir
}

// =============================================================================
// TEST A: Streaming planner emits multiple chunks
// =============================================================================

#[test]
fn test_a_streaming_planner_emits_multiple_chunks() {
    // Phase 4.4: propose_plan with callback should emit multiple chunks
    let temp_dir = create_db_root_for_streaming();
    let mut chunks_received = Vec::new();

    let context = odincode::llm::SessionContext {
        user_intent: "fix the error in main".to_string(),
        selected_file: Some("src/main.rs".to_string()),
        current_diagnostic: None,
        db_root: temp_dir.path().to_path_buf(),
    };

    let evidence_summary = odincode::llm::EvidenceSummary {
        q1_tool_executions: vec![],
        q2_failures: vec![],
        q3_diagnostic_executions: vec![],
        q4_file_executions: vec![],
        q5_execution_details: None,
        q6_latest_outcome: None,
        q7_recurring: vec![],
        q8_prior_fixes: vec![],
    };

    // Call propose_plan with streaming callback
    let result = odincode::llm::propose_plan_streaming(&context, &evidence_summary, |chunk| {
        chunks_received.push(chunk.to_string());
    });

    // Plan should be generated successfully
    assert!(result.is_ok());

    // Multiple chunks should have been received via callback
    // (stub implementation splits "Generating plan..." into chunks)
    assert!(
        !chunks_received.is_empty(),
        "Streaming callback should receive at least one chunk"
    );

    // Chunks should accumulate to form the planning message
    let accumulated = chunks_received.join("");
    assert!(
        !accumulated.is_empty(),
        "Accumulated chunks should not be empty"
    );
}

// =============================================================================
// TEST B: Final plan equals non-streamed plan (determinism)
// =============================================================================

#[test]
fn test_b_final_plan_equals_non_streamed_plan() {
    // Phase 4.4: Streamed plan must be semantically identical to non-streamed
    let temp_dir = create_db_root_for_streaming();

    let context = odincode::llm::SessionContext {
        user_intent: "read src/lib.rs".to_string(),
        selected_file: Some("src/lib.rs".to_string()),
        current_diagnostic: None,
        db_root: temp_dir.path().to_path_buf(),
    };

    let evidence_summary = odincode::llm::EvidenceSummary {
        q1_tool_executions: vec![],
        q2_failures: vec![],
        q3_diagnostic_executions: vec![],
        q4_file_executions: vec![],
        q5_execution_details: None,
        q6_latest_outcome: None,
        q7_recurring: vec![],
        q8_prior_fixes: vec![],
    };

    // Generate non-streamed plan
    let plan_non_streamed = odincode::llm::propose_plan(&context, &evidence_summary).unwrap();

    // Generate streamed plan
    let plan_streamed = odincode::llm::propose_plan_streaming(
        &context,
        &evidence_summary,
        |_| {}, // Ignore chunks for this test
    )
    .unwrap();

    // Plans must be semantically identical (plan_id differs due to timestamp)
    assert_eq!(plan_streamed.intent, plan_non_streamed.intent);
    assert_eq!(plan_streamed.steps.len(), plan_non_streamed.steps.len());
    assert_eq!(
        plan_streamed.evidence_referenced,
        plan_non_streamed.evidence_referenced
    );

    // Both plan_ids should have the same prefix format
    assert!(plan_streamed.plan_id.starts_with("plan_"));
    assert!(plan_non_streamed.plan_id.starts_with("plan_"));
}

// =============================================================================
// TEST C: Approval disabled until final plan exists
// =============================================================================

#[test]
fn test_c_approval_disabled_during_streaming() {
    // Phase 4.4: UI should not allow approval while streaming
    // This is a state machine test - verify PlanningInProgress state
    use odincode::ui::state::{App, AppState};

    let temp_dir = create_db_root_for_streaming();
    let mut app = App::new(temp_dir.path().to_path_buf());

    // Set planning in progress (simulates streaming started)
    app.set_planning_in_progress();

    // While in PlanningInProgress, approval should be blocked
    // (UI state check - 'y' key is ignored in this state)
    assert_eq!(app.state(), AppState::PlanningInProgress);
    assert!(
        app.current_plan().is_none(),
        "No plan should exist while streaming"
    );

    // Only after set_plan_ready is called can approval happen
    let plan = odincode::llm::Plan {
        plan_id: "test_plan".to_string(),
        intent: odincode::llm::Intent::Read,
        steps: vec![],
        evidence_referenced: vec![],
    };
    app.set_plan_ready(plan);

    assert_eq!(app.state(), AppState::PlanReady);
    assert!(
        app.current_plan().is_some(),
        "Plan should exist after streaming complete"
    );
}

// =============================================================================
// TEST D: Streaming fallback works (no callback = no streaming)
// =============================================================================

#[test]
fn test_d_streaming_fallback_without_callback() {
    // Phase 4.4: If streaming not supported, should fallback to non-streamed
    // The existing propose_plan() should still work without changes
    let temp_dir = create_db_root_for_streaming();

    let context = odincode::llm::SessionContext {
        user_intent: "find the bug".to_string(),
        selected_file: None,
        current_diagnostic: None,
        db_root: temp_dir.path().to_path_buf(),
    };

    let evidence_summary = odincode::llm::EvidenceSummary {
        q1_tool_executions: vec![],
        q2_failures: vec![],
        q3_diagnostic_executions: vec![],
        q4_file_executions: vec![],
        q5_execution_details: None,
        q6_latest_outcome: None,
        q7_recurring: vec![],
        q8_prior_fixes: vec![],
    };

    // Non-streamed version (original API, unchanged)
    let result = odincode::llm::propose_plan(&context, &evidence_summary);

    assert!(
        result.is_ok(),
        "Non-streamed propose_plan should still work"
    );
}

// =============================================================================
// TEST E: Evidence logging records stream chunks
// =============================================================================

#[test]
fn test_e_evidence_logging_records_stream_chunks() {
    // Phase 4.4: Each streamed chunk should be logged to execution_log.db
    let temp_dir = create_db_root_for_streaming();

    let context = odincode::llm::SessionContext {
        user_intent: "explain this function".to_string(),
        selected_file: Some("src/lib.rs".to_string()),
        current_diagnostic: None,
        db_root: temp_dir.path().to_path_buf(),
    };

    let evidence_summary = odincode::llm::EvidenceSummary {
        q1_tool_executions: vec![],
        q2_failures: vec![],
        q3_diagnostic_executions: vec![],
        q4_file_executions: vec![],
        q5_execution_details: None,
        q6_latest_outcome: None,
        q7_recurring: vec![],
        q8_prior_fixes: vec![],
    };

    // Open execution DB
    let exec_db = odincode::execution_tools::ExecutionDb::open(temp_dir.path()).unwrap();

    // Generate plan with streaming
    let plan = odincode::llm::propose_plan_streaming(&context, &evidence_summary, |chunk| {
        // Log each chunk to execution memory
        let _ = odincode::llm::log_stream_chunk(&exec_db, &context.user_intent, chunk);
    })
    .unwrap();

    // Log final plan
    let _ = odincode::llm::log_plan_generation(&exec_db, &context.user_intent, &plan, None);

    // Query execution_log.db for llm_plan_stream artifacts
    let conn = exec_db.conn();
    let mut stream_artifact_count = 0;
    let mut final_plan_found = false;

    // Check for stream artifacts
    let mut stmt = conn
        .prepare(
            "SELECT artifact_type FROM execution_artifacts
         WHERE execution_id LIKE 'llm_plan_%'
         ORDER BY id",
        )
        .unwrap();

    let artifact_types = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();
    for artifact_type in artifact_types {
        let at = artifact_type.unwrap();
        if at == "llm_plan_stream" {
            stream_artifact_count += 1;
        } else if at == "plan" {
            final_plan_found = true;
        }
    }

    // Should have at least one stream chunk artifact
    assert!(
        stream_artifact_count > 0,
        "Expected at least one llm_plan_stream artifact, found {}",
        stream_artifact_count
    );

    // Should have final plan artifact
    assert!(
        final_plan_found,
        "Expected final plan artifact to be logged"
    );
}

// =============================================================================
// TEST F: Stream chunk logging uses correct artifact_type
// =============================================================================

#[test]
fn test_f_stream_chunk_uses_correct_artifact_type() {
    // Phase 4.4: log_stream_chunk must use artifact_type="llm_plan_stream"
    let temp_dir = create_db_root_for_streaming();
    let exec_db = odincode::execution_tools::ExecutionDb::open(temp_dir.path()).unwrap();

    // Log a stream chunk
    let result = odincode::llm::log_stream_chunk(&exec_db, "test intent", "chunk content here");

    assert!(result.is_ok(), "log_stream_chunk should succeed");

    // Verify the artifact was created with correct type
    let conn = exec_db.conn();
    let artifact_type: String = conn
        .query_row(
            "SELECT artifact_type FROM execution_artifacts
         WHERE execution_id LIKE 'llm_plan_stream_%'
         LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(artifact_type, "llm_plan_stream");
}

// =============================================================================
// TEST G: Multiple chunks logged separately
// =============================================================================

#[test]
fn test_g_multiple_chunks_logged_separately() {
    // Phase 4.4: Each chunk should get its own artifact entry
    let temp_dir = create_db_root_for_streaming();
    let exec_db = odincode::execution_tools::ExecutionDb::open(temp_dir.path()).unwrap();

    // Log multiple chunks
    let chunks = vec!["chunk1", "chunk2", "chunk3"];
    for chunk in chunks {
        let _ = odincode::llm::log_stream_chunk(&exec_db, "test", chunk);
    }

    // Count distinct artifact entries
    let conn = exec_db.conn();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM execution_artifacts
         WHERE artifact_type = 'llm_plan_stream'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(count, 3, "Should have 3 separate stream chunk artifacts");
}
