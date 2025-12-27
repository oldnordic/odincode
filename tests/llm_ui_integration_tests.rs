//! LLM UI Integration Tests (T6, T7)
//!
//! Tests for:
//! - T6: UI boundary — "propose plan → user approves → tool executes" is representable without executing tools
//! - T7: Logging — a "plan generated" interaction records an execution row + artifacts in execution_log.db

use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::tempdir;

use odincode::execution_tools::ExecutionDb;
use odincode::llm::types::{
    AuthorizationStatus, Intent, Plan, PlanAuthorization, SessionContext, Step,
};

// === T6: UI Boundary Tests ===

#[test]
fn test_t6_plan_can_be_created_in_memory() {
    // Plan must be creatable without any IO
    let plan = Plan {
        plan_id: "plan_test_001".to_string(),
        intent: Intent::Read,
        steps: vec![Step {
            step_id: "step_1".to_string(),
            tool: "file_read".to_string(),
            arguments: {
                let mut map = HashMap::new();
                map.insert("path".to_string(), "src/lib.rs".to_string());
                map
            },
            precondition: "file exists".to_string(),
            requires_confirmation: false,
        }],
        evidence_referenced: vec!["Q4".to_string()],
    };

    assert_eq!(plan.plan_id, "plan_test_001");
    assert_eq!(plan.steps.len(), 1);
    assert!(!plan.steps[0].requires_confirmation);
}

#[test]
fn test_t6_plan_authorization_tracks_user_decision() {
    // Plan authorization must track user decision without executing tools
    let mut auth = PlanAuthorization::new("plan_auth_001".to_string());

    // Initially pending
    assert_eq!(auth.status(), AuthorizationStatus::Pending);
    assert!(!auth.is_approved());

    // User approves
    auth.approve();
    assert_eq!(auth.status(), AuthorizationStatus::Approved);
    assert!(auth.is_approved());

    // User can still revoke
    auth.revoke();
    assert_eq!(auth.status(), AuthorizationStatus::Rejected);
    assert!(!auth.is_approved());
}

#[test]
fn test_t6_step_level_authorization() {
    // Individual steps can require confirmation
    let plan = Plan {
        plan_id: "plan_step_auth".to_string(),
        intent: Intent::Mutate,
        steps: vec![
            Step {
                step_id: "step_1".to_string(),
                tool: "file_read".to_string(),
                arguments: {
                    let mut map = HashMap::new();
                    map.insert("path".to_string(), "src/lib.rs".to_string());
                    map
                },
                precondition: "file exists".to_string(),
                requires_confirmation: false, // Read doesn't require confirmation
            },
            Step {
                step_id: "step_2".to_string(),
                tool: "splice_patch".to_string(),
                arguments: {
                    let mut map = HashMap::new();
                    map.insert("file".to_string(), "src/lib.rs".to_string());
                    map.insert("symbol".to_string(), "foo".to_string());
                    map.insert("with".to_string(), "patches/fix.rs".to_string());
                    map
                },
                precondition: "symbol exists".to_string(),
                requires_confirmation: true, // Mutations require confirmation
            },
        ],
        evidence_referenced: vec![],
    };

    // Find steps requiring confirmation
    let needs_confirmation: Vec<&Step> = plan
        .steps
        .iter()
        .filter(|s| s.requires_confirmation)
        .collect();

    assert_eq!(needs_confirmation.len(), 1);
    assert_eq!(needs_confirmation[0].step_id, "step_2");
}

#[test]
fn test_t6_ui_displays_plan_without_executing() {
    // UI must be able to render plan without side effects
    let plan = Plan {
        plan_id: "plan_display".to_string(),
        intent: Intent::Mutate,
        steps: vec![Step {
            step_id: "step_1".to_string(),
            tool: "file_read".to_string(),
            arguments: {
                let mut map = HashMap::new();
                map.insert("path".to_string(), "src/lib.rs".to_string());
                map
            },
            precondition: "file exists".to_string(),
            requires_confirmation: false,
        }],
        evidence_referenced: vec!["Q8".to_string()],
    };

    // Render for UI (pure function, no IO)
    let display = odincode::llm::session::render_plan_for_ui(&plan);

    assert!(display.contains("plan_display"));
    assert!(display.contains("file_read"));
    assert!(display.contains("src/lib.rs"));
    assert!(display.contains("Q8"));
}

#[test]
fn test_t6_session_context_holds_user_intent() {
    // Session must capture user intent without processing it
    let context = SessionContext {
        user_intent: "Fix the E0425 error in src/lib.rs".to_string(),
        selected_file: Some("src/lib.rs".to_string()),
        current_diagnostic: Some("E0425".to_string()),
        db_root: PathBuf::from("."),
    };

    assert_eq!(context.user_intent, "Fix the E0425 error in src/lib.rs");
    assert_eq!(context.selected_file, Some("src/lib.rs".to_string()));
}

#[test]
fn test_t6_propose_plan_does_not_execute_tools() {
    // Proposing a plan must NOT execute any tools
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    // Create minimal codegraph.db (required by ExecutionDb)
    let codegraph_path = db_root.join("codegraph.db");
    {
        let conn = rusqlite::Connection::open(&codegraph_path).unwrap();
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
        .unwrap();
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
        .unwrap();
    }

    // Create execution_log.db (required by ExecutionDb)
    let exec_log_path = db_root.join("execution_log.db");
    {
        let conn = rusqlite::Connection::open(&exec_log_path).unwrap();
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
    }

    // Create config.toml with stub provider
    let config_path = db_root.join("config.toml");
    std::fs::write(
        &config_path,
        r#"[llm]
mode = "external"
provider = "stub"
base_url = "https://stub.example.com"
model = "stub-model"
"#,
    )
    .unwrap();

    let context = SessionContext {
        user_intent: "Read src/lib.rs".to_string(),
        selected_file: Some("src/lib.rs".to_string()),
        current_diagnostic: None,
        db_root: db_root.to_path_buf(),
    };

    // This should only create a plan in memory, no IO
    let plan = odincode::llm::session::propose_plan(
        &context,
        &odincode::llm::types::EvidenceSummary {
            q1_tool_executions: vec![],
            q2_failures: vec![],
            q3_diagnostic_executions: vec![],
            q4_file_executions: vec![],
            q5_execution_details: None,
            q6_latest_outcome: None,
            q7_recurring: vec![],
            q8_prior_fixes: vec![],
        },
    )
    .expect("Plan generation should succeed");

    // Plan exists in memory only
    assert!(!plan.plan_id.is_empty());
    assert_eq!(plan.intent, Intent::Read);

    // No files were accessed, no tools executed
    // (We can't directly prove no IO, but plan has deterministic output)
}

// === T7: Logging Tests ===

#[test]
fn test_t7_plan_generation_logged_to_execution_db() {
    // Plan generation must be logged to execution_log.db
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    // Create minimal codegraph.db (required by ExecutionDb)
    let codegraph_path = db_root.join("codegraph.db");
    {
        let conn = rusqlite::Connection::open(&codegraph_path).unwrap();
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
        .unwrap();
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
        .unwrap();
    }

    let exec_db = ExecutionDb::open(db_root).unwrap();

    // Log plan generation
    let plan = Plan {
        plan_id: "plan_logged_001".to_string(),
        intent: Intent::Read,
        steps: vec![],
        evidence_referenced: vec![],
    };

    let result = odincode::llm::session::log_plan_generation(
        &exec_db,
        "user intent here",
        &plan,
        None, // no error
    );

    assert!(
        result.is_ok(),
        "Plan generation must be logged successfully"
    );

    // Verify execution was recorded
    let executions = exec_db
        .query_by_tool("llm_plan")
        .expect("Should find logged llm_plan execution");

    assert!(
        !executions.is_empty(),
        "llm_plan execution must be recorded"
    );
    assert_eq!(executions[0].tool_name, "llm_plan");
    assert!(executions[0].success, "Logging should succeed");
}

#[test]
fn test_t7_plan_artifacts_stored() {
    // Plan artifacts are stored in execution_artifacts table
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    // Create minimal codegraph.db
    let codegraph_path = db_root.join("codegraph.db");
    {
        let conn = rusqlite::Connection::open(&codegraph_path).unwrap();
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
        .unwrap();
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
        .unwrap();
    }

    let exec_db = ExecutionDb::open(db_root).unwrap();

    let plan = Plan {
        plan_id: "plan_artifacts_001".to_string(),
        intent: Intent::Mutate,
        steps: vec![Step {
            step_id: "step_1".to_string(),
            tool: "splice_patch".to_string(),
            arguments: {
                let mut map = HashMap::new();
                map.insert("file".to_string(), "src/lib.rs".to_string());
                map.insert("symbol".to_string(), "foo".to_string());
                map.insert("with".to_string(), "patches/fix.rs".to_string());
                map
            },
            precondition: "symbol exists".to_string(),
            requires_confirmation: true,
        }],
        evidence_referenced: vec!["Q8".to_string()],
    };

    let user_prompt = "Fix E0425 error";

    // Log with artifacts
    odincode::llm::session::log_plan_generation(&exec_db, user_prompt, &plan, None)
        .expect("Logging should succeed");

    // Query execution_artifacts table directly to verify artifacts stored
    let exec_log_path = db_root.join("execution_log.db");
    let conn = rusqlite::Connection::open(&exec_log_path).unwrap();

    let mut artifact_stmt = conn
        .prepare("SELECT artifact_type FROM execution_artifacts WHERE execution_id = ?")
        .unwrap();

    let artifact_types: Vec<String> = artifact_stmt
        .query_map(["llm_plan_plan_artifacts_001"], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(
        artifact_types.contains(&"prompt".to_string()),
        "Prompt artifact must be stored"
    );
    assert!(
        artifact_types.contains(&"plan".to_string()),
        "Plan artifact must be stored"
    );
}

#[test]
fn test_t7_validation_errors_logged_as_artifacts() {
    // Validation errors must be logged as artifacts
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    // Create minimal codegraph.db
    let codegraph_path = db_root.join("codegraph.db");
    {
        let conn = rusqlite::Connection::open(&codegraph_path).unwrap();
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
        .unwrap();
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
        .unwrap();
    }

    let exec_db = ExecutionDb::open(db_root).unwrap();

    let validation_error = Some("Unknown tool: fake_tool");

    odincode::llm::session::log_plan_generation(
        &exec_db,
        "user intent",
        &Plan {
            plan_id: "plan_error_001".to_string(),
            intent: Intent::Mutate,
            steps: vec![],
            evidence_referenced: vec![],
        },
        validation_error,
    )
    .expect("Logging should succeed even with validation error");

    // Check validation error was logged in execution_artifacts table
    let exec_log_path = db_root.join("execution_log.db");
    let conn = rusqlite::Connection::open(&exec_log_path).unwrap();

    let mut artifact_stmt = conn
        .prepare("SELECT artifact_type FROM execution_artifacts WHERE execution_id = ?")
        .unwrap();

    let artifact_types: Vec<String> = artifact_stmt
        .query_map(["llm_plan_plan_error_001"], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(
        artifact_types.contains(&"validation_error".to_string()),
        "Validation error must be stored as artifact"
    );
}

#[test]
fn test_t7_audit_trail_reconstructable() {
    // Full audit trail must be reconstructable from execution_log.db
    let temp_dir = tempdir().unwrap();
    let db_root = temp_dir.path();

    // Create minimal codegraph.db
    let codegraph_path = db_root.join("codegraph.db");
    {
        let conn = rusqlite::Connection::open(&codegraph_path).unwrap();
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
        .unwrap();
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
        .unwrap();
    }

    let exec_db = ExecutionDb::open(db_root).unwrap();

    // Record a plan generation
    let plan = Plan {
        plan_id: "plan_audit_001".to_string(),
        intent: Intent::Read,
        steps: vec![],
        evidence_referenced: vec!["Q4".to_string(), "Q8".to_string()],
    };

    odincode::llm::session::log_plan_generation(&exec_db, "Fix E0425 in src/lib.rs", &plan, None)
        .expect("Logging should succeed");

    // Verify execution recorded
    let executions = exec_db.query_by_tool("llm_plan").unwrap();
    assert!(!executions.is_empty(), "Execution must be recorded");
    assert_eq!(executions[0].tool_name, "llm_plan");
    assert!(executions[0].success);

    // Verify artifacts recorded in execution_log.db
    let exec_log_path = db_root.join("execution_log.db");
    let conn = rusqlite::Connection::open(&exec_log_path).unwrap();

    let mut artifact_stmt = conn.prepare(
        "SELECT artifact_type, content_json FROM execution_artifacts WHERE execution_id = ? ORDER BY artifact_type"
    ).unwrap();

    let artifacts: Vec<(String, String)> = artifact_stmt
        .query_map(["llm_plan_plan_audit_001"], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Check for prompt artifact
    let prompt_artifact = artifacts.iter().find(|(t, _)| t == "prompt");
    assert!(prompt_artifact.is_some(), "Prompt must be stored");

    // Verify user intent recoverable from prompt artifact
    let (_, prompt_json) = prompt_artifact.unwrap();
    let prompt_value: serde_json::Value = serde_json::from_str(prompt_json).unwrap();
    assert!(
        prompt_value["user_intent"].is_string(),
        "User intent must be recoverable"
    );
    assert_eq!(
        prompt_value["user_intent"].as_str().unwrap(),
        "Fix E0425 in src/lib.rs"
    );

    // Check for plan artifact
    let plan_artifact = artifacts.iter().find(|(t, _)| t == "plan");
    assert!(plan_artifact.is_some(), "Plan must be stored");
}
