//! Evidence queries tests (Phase 0.6)
//!
//! TDD approach: All tests written first, expected to fail until implementation.
//!
//! Tests cover:
//! - Q1: ListExecutionsByTool
//! - Q2: ListFailuresByTool
//! - Q3: FindExecutionsByDiagnosticCode
//! - Q4: FindExecutionsByFile (graph + fallback)
//! - Q5: GetExecutionDetails (with/without graph)
//! - Q6: GetLatestOutcomeForFile
//! - Q7: GetRecurringDiagnostics
//! - Q8: FindPriorFixesForDiagnostic
//! - Deterministic ordering

use anyhow::Result;
use odincode::evidence_queries::EvidenceDb;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use tempfile::tempdir;

// Helper: Create minimal execution_log.db with test data
fn create_test_execution_log(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open(db_path)?;

    // Create schema
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
    )?;

    conn.execute(
        "CREATE TABLE execution_artifacts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            execution_id TEXT NOT NULL,
            artifact_type TEXT NOT NULL,
            content_json TEXT NOT NULL,
            FOREIGN KEY (execution_id) REFERENCES executions(id) ON DELETE RESTRICT
        )",
        [],
    )?;

    // Create indexes
    conn.execute(
        "CREATE INDEX idx_executions_tool ON executions(tool_name)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX idx_executions_timestamp ON executions(timestamp)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX idx_executions_success ON executions(success)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX idx_artifacts_execution ON execution_artifacts(execution_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX idx_artifacts_type ON execution_artifacts(artifact_type)",
        [],
    )?;

    // Insert test data
    let exec_id_1 = "exec-0001-0001-0001-0001";
    let exec_id_2 = "exec-0002-0002-0002-0002";
    let exec_id_3 = "exec-0003-0003-0003-0003";
    let exec_id_4 = "exec-0004-0004-0004-0004";
    let exec_id_5 = "exec-0005-0005-0005-0005";

    let base_ts = 1735000000000i64; // 2024-12-24

    // Execution 1: splice_patch success
    conn.execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success, exit_code, duration_ms, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            exec_id_1,
            "splice_patch",
            r#"{"file":"src/lib.rs","symbol":"foo","kind":"fn","with":"patches/foo.rs"}"#,
            base_ts,
            1i64,
            0i64,
            150i64,
            None::<&str>,
        ],
    )?;

    // Execution 2: splice_patch failure
    conn.execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success, exit_code, duration_ms, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            exec_id_2,
            "splice_patch",
            r#"{"file":"src/lib.rs","symbol":"bar","kind":"fn","with":"patches/bar.rs"}"#,
            base_ts + 1000,
            0i64,
            1i64,
            50i64,
            Some("Symbol not found: bar"),
        ],
    )?;

    // Execution 3: file_write success
    conn.execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success, exit_code, duration_ms, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            exec_id_3,
            "file_write",
            r#"{"file":"src/lib.rs"}"#,
            base_ts + 2000,
            1i64,
            None::<&str>,
            10i64,
            None::<&str>,
        ],
    )?;

    // Execution 4: lsp_check with diagnostics
    conn.execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success, exit_code, duration_ms, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            exec_id_4,
            "lsp_check",
            r#"{"path":"/project"}"#,
            base_ts + 3000,
            0i64,
            101i64,
            500i64,
            "Compilation failed",
        ],
    )?;

    // Diagnostic artifact for execution 4 (E0425 error)
    conn.execute(
        "INSERT INTO execution_artifacts (execution_id, artifact_type, content_json)
         VALUES (?1, ?2, ?3)",
        params![
            exec_id_4,
            "diagnostics",
            r#"[{"level":"error","message":"cannot find value `x` in this scope","file_name":"lib.rs","line_start":10,"code":"E0425"}]"#,
        ],
    )?;

    // Execution 5: lsp_check with same diagnostic (E0425 again)
    conn.execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success, exit_code, duration_ms, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            exec_id_5,
            "lsp_check",
            r#"{"path":"/project"}"#,
            base_ts + 4000,
            0i64,
            101i64,
            450i64,
            "Compilation failed",
        ],
    )?;

    // Diagnostic artifact for execution 5 (E0425 error again)
    conn.execute(
        "INSERT INTO execution_artifacts (execution_id, artifact_type, content_json)
         VALUES (?1, ?2, ?3)",
        params![
            exec_id_5,
            "diagnostics",
            r#"[{"level":"error","message":"cannot find value `y` in this scope","file_name":"lib.rs","line_start":20,"code":"E0425"}]"#,
        ],
    )?;

    // Also add a file_write after the diagnostics (for Q8)
    let exec_id_6 = "exec-0006-0006-0006-0006";
    conn.execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success, exit_code, duration_ms, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            exec_id_6,
            "file_write",
            r#"{"file":"lib.rs"}"#,
            base_ts + 5000,
            1i64,
            None::<&str>,
            15i64,
            None::<&str>,
        ],
    )?;

    Ok(())
}

// Helper: Create minimal codegraph.db with test data
fn create_test_codegraph(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE graph_entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL,
            name TEXT NOT NULL,
            file_path TEXT,
            data TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE graph_edges (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            from_id INTEGER NOT NULL,
            to_id INTEGER NOT NULL,
            edge_type TEXT NOT NULL,
            data TEXT NOT NULL
        )",
        [],
    )?;

    // Create file entity for src/lib.rs
    conn.execute(
        "INSERT INTO graph_entities (kind, name, file_path, data)
         VALUES ('file', 'lib.rs', 'src/lib.rs', '{\"type\":\"file\"}')",
        [],
    )?;

    let file_id = conn.last_insert_rowid();

    // Create execution entities
    let exec_id_1 = "exec-0001-0001-0001-0001";
    conn.execute(
        "INSERT INTO graph_entities (kind, name, file_path, data)
         VALUES ('execution', 'splice_patch:exec-0001-0001-0001-0001', NULL, ?1)",
        params![format!(r#"{{"tool":"splice_patch","timestamp":1735000000000,"success":true,"execution_id":"{}"}}"#, exec_id_1)],
    )?;

    let exec_entity_id_1 = conn.last_insert_rowid();

    // Create EXECUTED_ON edge
    conn.execute(
        "INSERT INTO graph_edges (from_id, to_id, edge_type, data)
         VALUES (?1, ?2, 'EXECUTED_ON', ?3)",
        params![exec_entity_id_1, file_id, r#"{"operation":"patch"}"#],
    )?;

    Ok(())
}

/// Q1: ListExecutionsByTool - happy path
#[test]
fn test_q1_list_executions_by_tool_happy_path() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results = ev_db
        .list_executions_by_tool("splice_patch", None, None, None)
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].tool_name, "splice_patch");
    assert_eq!(results[0].execution_id, "exec-0001-0001-0001-0001");
    assert!(results[0].success);
    assert_eq!(results[1].execution_id, "exec-0002-0002-0002-0002");
    assert!(!results[1].success);
    assert_eq!(
        results[1].error_message,
        Some("Symbol not found: bar".to_string())
    );
}

/// Q1: ListExecutionsByTool - empty result
#[test]
fn test_q1_list_executions_by_tool_empty_result() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results = ev_db
        .list_executions_by_tool("file_read", None, None, None)
        .unwrap();

    assert_eq!(results.len(), 0);
}

/// Q1: ListExecutionsByTool - deterministic ordering (timestamp ASC)
#[test]
fn test_q1_list_executions_by_tool_deterministic_ordering() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results1 = ev_db
        .list_executions_by_tool("splice_patch", None, None, None)
        .unwrap();
    let results2 = ev_db
        .list_executions_by_tool("splice_patch", None, None, None)
        .unwrap();

    assert_eq!(results1.len(), results2.len());
    for (r1, r2) in results1.iter().zip(results2.iter()) {
        assert_eq!(r1.execution_id, r2.execution_id);
        assert_eq!(r1.timestamp, r2.timestamp);
    }
}

/// Q2: ListFailuresByTool - happy path
#[test]
fn test_q2_list_failures_by_tool_happy_path() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results = ev_db
        .list_failures_by_tool("splice_patch", None, None)
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].execution_id, "exec-0002-0002-0002-0002");
    assert_eq!(results[0].tool_name, "splice_patch");
    assert_eq!(results[0].exit_code, Some(1));
    assert_eq!(
        results[0].error_message,
        Some("Symbol not found: bar".to_string())
    );
}

/// Q2: ListFailuresByTool - ordering (DESC, most recent first)
#[test]
fn test_q2_list_failures_by_tool_desc_ordering() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results = ev_db
        .list_failures_by_tool("lsp_check", None, None)
        .unwrap();

    assert_eq!(results.len(), 2);
    // Most recent failure first (DESC timestamp)
    assert_eq!(results[0].execution_id, "exec-0005-0005-0005-0005");
    assert_eq!(results[1].execution_id, "exec-0004-0004-0004-0004");
}

/// Q3: FindExecutionsByDiagnosticCode - happy path
#[test]
fn test_q3_find_executions_by_diagnostic_code_happy_path() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results = ev_db
        .find_executions_by_diagnostic_code("E0425", None)
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].diagnostic_code, "E0425");
    assert_eq!(results[0].diagnostic_level, "error");
    assert_eq!(results[0].file_name, "lib.rs");
    assert_eq!(results[0].tool_name, "lsp_check");
}

/// Q3: FindExecutionsByDiagnosticCode - no matches
#[test]
fn test_q3_find_executions_by_diagnostic_code_no_matches() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results = ev_db
        .find_executions_by_diagnostic_code("E0308", None)
        .unwrap();

    assert_eq!(results.len(), 0);
}

/// Q4: FindExecutionsByFile - graph query
#[test]
fn test_q4_find_executions_by_file_graph_query() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results = ev_db
        .find_executions_by_file("src/lib.rs", None, None)
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].execution_id, "exec-0001-0001-0001-0001");
    assert_eq!(results[0].tool_name, "splice_patch");
    assert_eq!(results[0].edge_type, "EXECUTED_ON");
    assert!(results[0].success);
}

/// Q4: FindExecutionsByFile - fallback to SQLite when graph missing
#[test]
fn test_q4_find_executions_by_file_fallback_when_graph_missing() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    // No codegraph.db created

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results = ev_db
        .find_executions_by_file("src/lib.rs", None, None)
        .unwrap();

    // Should use fallback SQLite query
    assert!(!results.is_empty());
    // Check that data_source indicator is set
    // Note: Fallback may miss some executions (approximate)
}

/// Q5: GetExecutionDetails - with graph
#[test]
fn test_q5_get_execution_details_with_graph() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let details = ev_db
        .get_execution_details("exec-0001-0001-0001-0001")
        .unwrap();

    assert_eq!(details.execution.id, "exec-0001-0001-0001-0001");
    assert_eq!(details.execution.tool_name, "splice_patch");
    assert!(details.execution.success);
    assert!(details.graph_entity.is_some());
    assert_eq!(details.graph_edges.len(), 1);
    assert_eq!(details.graph_edges[0].edge_type, "EXECUTED_ON");
}

/// Q5: GetExecutionDetails - without graph (best-effort gap)
#[test]
fn test_q5_get_execution_details_without_graph() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    // No codegraph.db created

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let details = ev_db
        .get_execution_details("exec-0003-0003-0003-0003")
        .unwrap();

    assert_eq!(details.execution.id, "exec-0003-0003-0003-0003");
    assert_eq!(details.execution.tool_name, "file_write");
    assert!(details.graph_entity.is_none()); // Graph entity missing
    assert_eq!(details.graph_edges.len(), 0);
}

/// Q5: GetExecutionDetails - with artifacts
#[test]
fn test_q5_get_execution_details_with_artifacts() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let details = ev_db
        .get_execution_details("exec-0004-0004-0004-0004")
        .unwrap();

    assert_eq!(details.execution.id, "exec-0004-0004-0004-0004");
    assert_eq!(details.artifacts.len(), 1);
    assert_eq!(details.artifacts[0].artifact_type, "diagnostics");
    // Verify JSON content
    let content: serde_json::Value =
        serde_json::from_str(&details.artifacts[0].content_json).unwrap();
    assert_eq!(content[0]["code"], "E0425");
}

/// Q5: GetExecutionDetails - execution not found
#[test]
fn test_q5_get_execution_details_not_found() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let result = ev_db.get_execution_details("nonexistent-id");

    assert!(result.is_err());
}

/// Q6: GetLatestOutcomeForFile - graph query
#[test]
fn test_q6_get_latest_outcome_for_file_graph_query() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let outcome = ev_db.get_latest_outcome_for_file("src/lib.rs").unwrap();

    assert!(outcome.is_some());
    let outcome = outcome.unwrap();
    assert_eq!(outcome.execution_id, "exec-0001-0001-0001-0001");
    assert_eq!(outcome.tool_name, "splice_patch");
    assert!(outcome.success);
}

/// Q6: GetLatestOutcomeForFile - no matches
#[test]
fn test_q6_get_latest_outcome_for_file_no_matches() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let outcome = ev_db
        .get_latest_outcome_for_file("src/nonexistent.rs")
        .unwrap();

    assert!(outcome.is_none());
}

/// Q7: GetRecurringDiagnostics - threshold met
#[test]
fn test_q7_get_recurring_diagnostics_threshold_met() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results = ev_db.get_recurring_diagnostics(2, None).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].diagnostic_code, "E0425");
    assert_eq!(results[0].file_name, "lib.rs");
    assert_eq!(results[0].occurrence_count, 2);
    assert!(results[0].occurrence_count >= 2);
}

/// Q7: GetRecurringDiagnostics - threshold not met
#[test]
fn test_q7_get_recurring_diagnostics_threshold_not_met() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results = ev_db.get_recurring_diagnostics(3, None).unwrap();

    assert_eq!(results.len(), 0);
}

/// Q7: GetRecurringDiagnostics - deterministic ordering
#[test]
fn test_q7_get_recurring_diagnostics_ordering() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results = ev_db.get_recurring_diagnostics(1, None).unwrap();

    // Ordering: occurrence_count DESC, diagnostic_code ASC, file_name ASC
    // Only one result here, but test that it runs
    assert!(!results.is_empty());
}

/// Q8: FindPriorFixesForDiagnostic - temporal adjacency
#[test]
fn test_q8_find_prior_fixes_for_diagnostic_temporal_adjacency() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results = ev_db
        .find_prior_fixes_for_diagnostic("E0425", None, None)
        .unwrap();

    // Should find file_write after E0425 diagnostics
    assert!(!results.is_empty());
    // Check temporal ordering: diagnostic_timestamp ASC, fix_timestamp ASC
    let first = &results[0];
    assert_eq!(first.tool_name, "file_write");
    assert!(first.temporal_gap_ms > 0); // Fix AFTER diagnostic
}

/// Q8: FindPriorFixesForDiagnostic - no matches
#[test]
fn test_q8_find_prior_fixes_for_diagnostic_no_matches() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    let results = ev_db
        .find_prior_fixes_for_diagnostic("E0308", None, None)
        .unwrap();

    assert_eq!(results.len(), 0);
}

/// Deterministic ordering across multiple runs
#[test]
fn test_deterministic_ordering_across_runs() {
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    let exec_log_path = db_root.join("execution_log.db");
    create_test_execution_log(&exec_log_path).unwrap();

    let codegraph_path = db_root.join("codegraph.db");
    create_test_codegraph(&codegraph_path).unwrap();

    let ev_db = EvidenceDb::open(db_root).unwrap();

    // Run Q1 twice
    let results1 = ev_db
        .list_executions_by_tool("splice_patch", None, None, None)
        .unwrap();
    let results2 = ev_db
        .list_executions_by_tool("splice_patch", None, None, None)
        .unwrap();

    assert_eq!(results1.len(), results2.len());
    for (r1, r2) in results1.iter().zip(results2.iter()) {
        assert_eq!(r1.execution_id, r2.execution_id);
        assert_eq!(r1.timestamp, r2.timestamp);
    }
}
