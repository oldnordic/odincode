//! Integration tests for execution_tools (Phase 0.5.2)
//!
//! These tests verify:
//! - SQLite schema creation and trigger enforcement
//! - Execution recording with artifacts
//! - SQLiteGraph integration (execution entities + edges)
//! - Failure semantics (SQLite persists on graph failure)
//! - Deterministic query ordering
//! - Forbidden edge rejection
//! - Full workflow integration
//!
//! Tests use REAL SQLite databases (no mocks) and create temp db_root.

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;

// Test helper: Create minimal codegraph.db schema
fn create_minimal_codegraph_db(db_path: &Path) -> Result<()> {
    let conn = Connection::open(db_path).context("Failed to create codegraph.db")?;

    // Create graph_entities table
    conn.execute(
        "CREATE TABLE graph_entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL,
            name TEXT NOT NULL,
            file_path TEXT,
            data TEXT NOT NULL
        )",
        [],
    )
    .context("Failed to create graph_entities table")?;

    // Create graph_edges table
    conn.execute(
        "CREATE TABLE graph_edges (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            from_id INTEGER NOT NULL,
            to_id INTEGER NOT NULL,
            edge_type TEXT NOT NULL,
            data TEXT NOT NULL
        )",
        [],
    )
    .context("Failed to create graph_edges table")?;

    Ok(())
}

// Test helper: Create a test file entity in codegraph.db
fn create_test_file_entity(conn: &Connection, file_path: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO graph_entities (kind, name, file_path, data) VALUES (?, ?, ?, ?)",
        ["File", file_path, file_path, "{}"],
    )
    .context("Failed to insert test file entity")?;

    Ok(conn.last_insert_rowid())
}

// =============================================================================
// TEST A: SQLite Schema Creation + Trigger Enforcement
// =============================================================================

#[test]
fn test_schema_creation_creates_executions_table() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();
    let codegraph_path = db_root.join("codegraph.db");

    // Setup: Create minimal codegraph.db
    create_minimal_codegraph_db(&codegraph_path).unwrap();

    // Action: Open ExecutionDb (will fail because module doesn't exist)
    let result = odincode::execution_tools::ExecutionDb::open(db_root);

    // Expected: Compilation error - execution_tools module doesn't exist
    // This test will fail to compile, which is the CORRECT failure mode
    match result {
        Ok(_) => {
            // If we get here, module exists - verify executions table
            let exec_log_path = db_root.join("execution_log.db");
            let conn = Connection::open(&exec_log_path).unwrap();

            // Check executions table exists
            let table_exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='executions'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(table_exists, 1, "executions table should exist");
        }
        Err(_) => {
            // Expected: Module doesn't exist yet
            panic!("ExecutionDb not implemented - this is expected failure");
        }
    }
}

#[test]
fn test_trigger_enforces_tool_name_validation() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();
    let codegraph_path = db_root.join("codegraph.db");

    create_minimal_codegraph_db(&codegraph_path).unwrap();

    // This test will fail to compile - expected
    let exec_db = odincode::execution_tools::ExecutionDb::open(db_root).unwrap();

    // Try to insert execution with invalid tool_name
    let result = exec_db.conn().execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success)
         VALUES ('test-id', 'invalid_tool', '{}', 1735036800000, TRUE)",
        [],
    );

    // Expected: Trigger aborts with "Invalid tool_name"
    assert!(result.is_err(), "Trigger should reject invalid tool_name");
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Invalid tool_name"),
        "Error should mention invalid tool_name"
    );
}

#[test]
fn test_trigger_enforces_timestamp_validation() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();
    let codegraph_path = db_root.join("codegraph.db");

    create_minimal_codegraph_db(&codegraph_path).unwrap();

    // This test will fail to compile - expected
    let exec_db = odincode::execution_tools::ExecutionDb::open(db_root).unwrap();

    // Try to insert execution with future timestamp (year 2100)
    let result = exec_db.conn().execute(
        "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success)
         VALUES ('test-id', 'file_read', '{}', 4102444800000, TRUE)", // 2100-01-01
        [],
    );

    // Expected: Trigger aborts with "Timestamp in future"
    assert!(result.is_err(), "Trigger should reject future timestamp");
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Timestamp in future"),
        "Error should mention future timestamp"
    );
}

#[test]
fn test_trigger_enforces_artifact_type_validation() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();
    let codegraph_path = db_root.join("codegraph.db");

    create_minimal_codegraph_db(&codegraph_path).unwrap();

    // This test will fail to compile - expected
    let exec_db = odincode::execution_tools::ExecutionDb::open(db_root).unwrap();

    // First insert an execution (required for foreign key)
    exec_db
        .conn()
        .execute(
            "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success)
         VALUES ('test-id', 'file_read', '{}', 1735036800000, TRUE)",
            [],
        )
        .unwrap();

    // Try to insert artifact with invalid type
    let result = exec_db.conn().execute(
        "INSERT INTO execution_artifacts (execution_id, artifact_type, content_json)
         VALUES ('test-id', 'invalid_type', '{}')",
        [],
    );

    // Expected: Trigger aborts with "Invalid artifact_type"
    assert!(
        result.is_err(),
        "Trigger should reject invalid artifact_type"
    );
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Invalid artifact_type") || err_msg.contains("invalid artifact_type"),
        "Error should mention invalid artifact_type"
    );
}

#[test]
fn test_trigger_enforces_json_validation() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();
    let codegraph_path = db_root.join("codegraph.db");

    create_minimal_codegraph_db(&codegraph_path).unwrap();

    // This test will fail to compile - expected
    let exec_db = odincode::execution_tools::ExecutionDb::open(db_root).unwrap();

    // First insert an execution
    exec_db
        .conn()
        .execute(
            "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success)
         VALUES ('test-id', 'file_read', '{}', 1735036800000, TRUE)",
            [],
        )
        .unwrap();

    // Try to insert artifact with invalid JSON
    let result = exec_db.conn().execute(
        "INSERT INTO execution_artifacts (execution_id, artifact_type, content_json)
         VALUES ('test-id', 'stdout', 'not valid json')",
        [],
    );

    // Expected: Trigger aborts with "Invalid JSON"
    assert!(result.is_err(), "Trigger should reject invalid JSON");
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Invalid JSON"),
        "Error should mention invalid JSON"
    );
}

// =============================================================================
// TEST B: Record Execution Success
// =============================================================================

#[test]
fn test_record_execution_success() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();
    let codegraph_path = db_root.join("codegraph.db");

    create_minimal_codegraph_db(&codegraph_path).unwrap();

    // This test will fail to compile - expected
    let exec_db = odincode::execution_tools::ExecutionDb::open(db_root).unwrap();

    let execution_id = "550e8400-e29b-41d4-a716-446655440000";
    let args = json!({"file": "src/lib.rs"});
    let timestamp = 1735036800000i64;

    // Action: Record execution
    exec_db
        .record_execution(
            execution_id,
            "file_read",
            &args,
            timestamp,
            true,
            None,
            Some(150),
            None,
        )
        .unwrap();

    // Assert: executions table has exactly 1 row
    let count: i64 = exec_db
        .conn()
        .query_row("SELECT COUNT(*) FROM executions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1, "Should have exactly 1 execution");

    // Assert: Row data matches input
    let (id, tool_name, success, duration): (String, String, bool, Option<i64>) = exec_db
        .conn()
        .query_row(
            "SELECT id, tool_name, success, duration_ms FROM executions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();

    assert_eq!(id, execution_id);
    assert_eq!(tool_name, "file_read");
    assert!(success);
    assert_eq!(duration, Some(150));
}

#[test]
fn test_record_execution_creates_graph_entity() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();
    let codegraph_path = db_root.join("codegraph.db");

    create_minimal_codegraph_db(&codegraph_path).unwrap();

    // This test will fail to compile - expected
    let exec_db = odincode::execution_tools::ExecutionDb::open(db_root).unwrap();

    let execution_id = "550e8400-e29b-41d4-a716-446655440000";
    let args = json!({"file": "src/lib.rs"});
    let timestamp = 1735036800000i64;

    // Action: Record execution
    exec_db
        .record_execution(
            execution_id,
            "file_read",
            &args,
            timestamp,
            true,
            None,
            Some(150),
            None,
        )
        .unwrap();

    // Assert: graph_entities has 1 execution node
    let count: i64 = exec_db
        .graph_conn()
        .query_row(
            "SELECT COUNT(*) FROM graph_entities WHERE kind = 'execution'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "Should have exactly 1 execution entity");

    // Assert: Entity name format is "tool_name:uuid"
    let name: String = exec_db
        .graph_conn()
        .query_row(
            "SELECT name FROM graph_entities WHERE kind = 'execution'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "file_read:550e8400-e29b-41d4-a716-446655440000");
}

// =============================================================================
// TEST C: Record Execution with Artifacts
// =============================================================================

#[test]
fn test_record_execution_with_artifacts() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();
    let codegraph_path = db_root.join("codegraph.db");

    create_minimal_codegraph_db(&codegraph_path).unwrap();

    // This test will fail to compile - expected
    let exec_db = odincode::execution_tools::ExecutionDb::open(db_root).unwrap();

    let execution_id = "550e8400-e29b-41d4-a716-446655440000";
    let args = json!({"file": "src/lib.rs", "symbol": "foo"});
    let timestamp = 1735036800000i64;

    let stdout = json!({"text": "Patched src/lib.rs"});
    let stderr = json!({"text": ""});
    let diagnostics = json!([
        {"level": "error", "message": "E0425", "file_name": "lib.rs", "line_start": 10, "code": "E0425"}
    ]);

    // Action: Record execution with artifacts
    exec_db
        .record_execution_with_artifacts(
            execution_id,
            "splice_patch",
            &args,
            timestamp,
            true,
            None,
            Some(500),
            None,
            &[
                ("stdout", &stdout),
                ("stderr", &stderr),
                ("diagnostics", &diagnostics),
            ],
        )
        .unwrap();

    // Assert: execution_artifacts has 3 rows
    let count: i64 = exec_db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM execution_artifacts WHERE execution_id = ?",
            [execution_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 3, "Should have exactly 3 artifacts");

    // Assert: All artifact types present
    let types: Vec<String> = exec_db.conn().prepare(
        "SELECT artifact_type FROM execution_artifacts WHERE execution_id = ? ORDER BY artifact_type"
    ).unwrap()
        .query_map([execution_id], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(types, vec!["diagnostics", "stderr", "stdout"]);
}

// =============================================================================
// TEST D: Graph Write — Execution Entity + EXECUTED_ON Edge
// =============================================================================

#[test]
fn test_graph_write_creates_executed_on_edge() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();
    let codegraph_path = db_root.join("codegraph.db");

    create_minimal_codegraph_db(&codegraph_path).unwrap();

    // Setup: Create a test file entity
    let graph_conn = Connection::open(&codegraph_path).unwrap();
    let _file_id = create_test_file_entity(&graph_conn, "src/lib.rs").unwrap();

    // This test will fail to compile - expected
    let exec_db = odincode::execution_tools::ExecutionDb::open(db_root).unwrap();

    let execution_id = "550e8400-e29b-41d4-a716-446655440000";
    let args = json!({"file": "src/lib.rs"});
    let timestamp = 1735036800000i64;

    // Action: Record execution on file
    exec_db
        .record_execution_on_file(
            execution_id,
            "file_read",
            &args,
            timestamp,
            true,
            None,
            Some(50),
            None,
            "src/lib.rs",
        )
        .unwrap();

    // Assert: graph_edges has EXECUTED_ON edge
    let count: i64 = exec_db
        .graph_conn()
        .query_row(
            "SELECT COUNT(*) FROM graph_edges WHERE edge_type = 'EXECUTED_ON'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "Should have exactly 1 EXECUTED_ON edge");

    // Assert: Edge links execution entity to file entity
    let (from_id, to_id): (i64, i64) = exec_db
        .graph_conn()
        .query_row(
            "SELECT from_id, to_id FROM graph_edges WHERE edge_type = 'EXECUTED_ON'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    // Verify from_id is execution entity
    let from_kind: String = exec_db
        .graph_conn()
        .query_row(
            "SELECT kind FROM graph_entities WHERE id = ?",
            [from_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(from_kind, "execution");

    // Verify to_id is file entity
    let to_kind: String = exec_db
        .graph_conn()
        .query_row(
            "SELECT kind FROM graph_entities WHERE id = ?",
            [to_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(to_kind, "File");
}

// =============================================================================
// TEST E: Failure Semantics — Graph Failure After SQLite Commit
// =============================================================================

#[test]
fn test_graph_failure_preserves_sqlite_data() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();
    let codegraph_path = db_root.join("codegraph.db");

    // Setup: Create INVALID codegraph.db (trigger blocks execution inserts)
    let conn = Connection::open(&codegraph_path).unwrap();
    conn.execute(
        "CREATE TABLE graph_entities (id INTEGER PRIMARY KEY, kind TEXT, name TEXT, file_path TEXT, data TEXT)",
        [],
    ).unwrap();
    // Add trigger to block execution entity inserts → will cause INSERT failure
    conn.execute(
        "CREATE TRIGGER block_execution_entities BEFORE INSERT ON graph_entities
        WHEN NEW.kind = 'execution'
        BEGIN
            SELECT RAISE(ABORT, 'Execution entities not allowed');
        END",
        [],
    )
    .unwrap();
    // Deliberately skip graph_edges table → will cause edge INSERT failure (if any)

    // This test will fail to compile - expected
    let exec_db = odincode::execution_tools::ExecutionDb::open(db_root).unwrap();

    let execution_id = "550e8400-e29b-41d4-a716-446655440000";
    let args = json!({"file": "src/lib.rs"});
    let timestamp = 1735036800000i64;

    // Action: Record execution (SQLite should succeed, graph should fail)
    let result = exec_db.record_execution(
        execution_id,
        "file_read",
        &args,
        timestamp,
        true,
        None,
        Some(50),
        None,
    );

    // Expected: Returns Ok (SQLite success) or Error with graph failure details
    // Critical: execution_log.db should have the data
    match result {
        Ok(_) => {
            // Verify execution was written to SQLite
            let count: i64 = exec_db
                .conn()
                .query_row("SELECT COUNT(*) FROM executions", [], |row| row.get(0))
                .unwrap();
            assert_eq!(count, 1, "SQLite should have execution row");

            // Verify graph has NO execution entity (graph write failed)
            let count: i64 = exec_db
                .graph_conn()
                .query_row(
                    "SELECT COUNT(*) FROM graph_entities WHERE kind = 'execution'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            assert_eq!(count, 0, "Graph should not have execution entity");
        }
        Err(e) => {
            // If function returns error, verify SQLite data still persisted
            let count: i64 = exec_db
                .conn()
                .query_row("SELECT COUNT(*) FROM executions", [], |row| row.get(0))
                .unwrap_or(0);
            assert_eq!(
                count, 1,
                "SQLite should still have execution row despite error"
            );

            // Verify error mentions graph failure
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("graph") || err_msg.contains("Graph"),
                "Error should mention graph failure"
            );
        }
    }
}

// =============================================================================
// TEST F: Deterministic Query Ordering
// =============================================================================

#[test]
fn test_query_by_tool_returns_deterministically_ordered() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();
    let codegraph_path = db_root.join("codegraph.db");

    create_minimal_codegraph_db(&codegraph_path).unwrap();

    // This test will fail to compile - expected
    let exec_db = odincode::execution_tools::ExecutionDb::open(db_root).unwrap();

    // Setup: Record 5 executions with OUT-OF-ORDER timestamps
    let timestamps = [
        1735036800000i64, // t1
        1735036900000i64, // t2
        1735037000000i64, // t3
        1735037100000i64, // t4
        1735037200000i64, // t5
    ];

    // Insert in random order: t3, t1, t5, t2, t4
    let insert_order = [2, 0, 4, 1, 3];
    for (idx, pos) in insert_order.iter().enumerate() {
        let id = format!("exec-{}", idx);
        exec_db
            .record_execution(
                &id,
                "file_read",
                &json!({"index": idx}),
                timestamps[*pos],
                true,
                None,
                Some(100),
                None,
            )
            .unwrap();
    }

    // Action: Query by tool_name
    let executions = exec_db.query_by_tool("file_read").unwrap();

    // Assert: Results ordered by timestamp ASC (t1, t2, t3, t4, t5)
    assert_eq!(executions.len(), 5, "Should have 5 executions");

    for (i, exec) in executions.iter().enumerate() {
        assert_eq!(
            exec.timestamp, timestamps[i],
            "Execution {} should have timestamp {}",
            i, i
        );
    }
}

// =============================================================================
// TEST G: Forbidden Edge Detection
// =============================================================================

#[test]
fn test_forbidden_execution_to_execution_edge_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();
    let codegraph_path = db_root.join("codegraph.db");

    create_minimal_codegraph_db(&codegraph_path).unwrap();

    // This test will fail to compile - expected
    let exec_db = odincode::execution_tools::ExecutionDb::open(db_root).unwrap();

    // Setup: Create an execution
    let execution_id = "550e8400-e29b-41d4-a716-446655440000";
    exec_db
        .record_execution(
            execution_id,
            "file_read",
            &json!({}),
            1735036800000,
            true,
            None,
            Some(50),
            None,
        )
        .unwrap();

    // Get execution entity ID
    let entity_id: i64 = exec_db
        .graph_conn()
        .query_row(
            "SELECT id FROM graph_entities WHERE kind = 'execution'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    // Action: Try to create execution → execution edge (FORBIDDEN)
    let result = exec_db.create_graph_edge(
        entity_id,
        entity_id, // Self-reference
        "EXECUTED_ON",
        &json!({"test": "self-reference"}),
    );

    // Assert: Returns error
    assert!(result.is_err(), "Should reject execution → execution edge");
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("forbidden") || err_msg.contains("Forbidden"),
        "Error should mention forbidden pattern"
    );
}

// =============================================================================
// TEST H: Full Workflow Integration
// =============================================================================

#[test]
fn test_full_workflow_logging() {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();
    let codegraph_path = db_root.join("codegraph.db");

    create_minimal_codegraph_db(&codegraph_path).unwrap();

    // Setup: Create test file entities
    let graph_conn = Connection::open(&codegraph_path).unwrap();
    let _cargo_toml_id = create_test_file_entity(&graph_conn, "Cargo.toml").unwrap();
    let _lib_rs_id = create_test_file_entity(&graph_conn, "src/lib.rs").unwrap();
    let _dot_id = create_test_file_entity(&graph_conn, ".").unwrap();

    // This test will fail to compile - expected
    let exec_db = odincode::execution_tools::ExecutionDb::open(db_root).unwrap();

    let start_time = 1735036800000i64;

    // Action 1: file_read on Cargo.toml
    exec_db
        .record_execution_on_file(
            "exec-1",
            "file_read",
            &json!({"path": "Cargo.toml"}),
            start_time,
            true,
            None,
            Some(10),
            None,
            "Cargo.toml",
        )
        .unwrap();

    // Action 2: splice_patch on lib.rs
    exec_db
        .record_execution_on_file(
            "exec-2",
            "splice_patch",
            &json!({"file": "src/lib.rs", "symbol": "foo"}),
            start_time + 1000,
            true,
            Some(0),
            Some(500),
            None,
            "src/lib.rs",
        )
        .unwrap();

    // Action 3: lsp_check on project
    exec_db
        .record_execution_on_file(
            "exec-3",
            "lsp_check",
            &json!({"path": "."}),
            start_time + 2000,
            true,
            Some(0),
            Some(1500),
            None,
            ".",
        )
        .unwrap();

    // Assert: 3 execution rows
    let exec_count: i64 = exec_db
        .conn()
        .query_row("SELECT COUNT(*) FROM executions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(exec_count, 3, "Should have 3 execution rows");

    // Assert: 3 execution entities in graph
    let entity_count: i64 = exec_db
        .graph_conn()
        .query_row(
            "SELECT COUNT(*) FROM graph_entities WHERE kind = 'execution'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(entity_count, 3, "Should have 3 execution entities");

    // Assert: EXECUTED_ON edges exist
    let edge_count: i64 = exec_db
        .graph_conn()
        .query_row(
            "SELECT COUNT(*) FROM graph_edges WHERE edge_type = 'EXECUTED_ON'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(edge_count, 3, "Should have 3 EXECUTED_ON edges");

    // Assert: Chronological query returns in order
    let executions = exec_db.query_all().unwrap();
    assert_eq!(executions.len(), 3);
    assert_eq!(executions[0].timestamp, start_time);
    assert_eq!(executions[1].timestamp, start_time + 1000);
    assert_eq!(executions[2].timestamp, start_time + 2000);
}
