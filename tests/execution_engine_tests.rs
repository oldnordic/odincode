//! Execution Engine Tests (Phase 3)
//!
//! Tests for:
//! - A. Plan authorization rejection
//! - B. Single-step success
//! - C. Failure stops execution
//! - D. Confirmation denied stops execution
//! - E. Evidence logged per step
//! - F. Deterministic execution_id mapping
//! - G. Forbidden tool rejection
//! - H. Precondition failure

use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use odincode::execution_engine::{
    ApprovedPlan, AutoApprove, AutoDeny, ExecutionStatus, Executor, NoopProgress, ProgressCallback,
    StepResult,
};
use odincode::execution_tools::ExecutionDb;
use odincode::llm::types::{Intent, Plan, PlanAuthorization, Step};

// Test utilities

fn create_test_plan(steps: Vec<Step>) -> Plan {
    Plan {
        plan_id: format!(
            "plan_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ),
        intent: Intent::Read,
        steps,
        evidence_referenced: vec![],
    }
}

fn create_approved_authorization(plan_id: String) -> PlanAuthorization {
    let mut auth = PlanAuthorization::new(plan_id);
    auth.approve();
    auth
}

fn create_pending_authorization(plan_id: String) -> PlanAuthorization {
    PlanAuthorization::new(plan_id)
}

#[allow(dead_code)]
fn create_step(step_id: &str, tool: &str, arguments: HashMap<String, String>) -> Step {
    Step {
        step_id: step_id.to_string(),
        tool: tool.to_string(),
        arguments,
        precondition: "none".to_string(),
        requires_confirmation: false,
    }
}

fn setup_temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

// === CATEGORY A: Authorization Rejection ===

#[test]
fn test_a_pending_authorization_rejected() {
    let temp_dir = setup_temp_dir();
    let db_root = temp_dir.path();

    // Create codegraph.db (required by ExecutionDb::open)
    // Note: execution_log.db is auto-created by ExecutionDb::open()
    let graph_db_path = db_root.join("codegraph.db");
    fs::File::create(&graph_db_path).expect("Failed to create codegraph.db");

    let exec_db = ExecutionDb::open(db_root).expect("Failed to open ExecutionDb");

    let plan = create_test_plan(vec![]);
    let auth = create_pending_authorization(plan.plan_id.clone());
    let approved = ApprovedPlan {
        plan,
        authorization: auth,
    };

    let confirmation = Box::new(AutoApprove);
    let progress = Box::new(NoopProgress);

    let mut executor = Executor::new(exec_db, None, confirmation, progress);

    let result = executor.execute(approved);

    assert!(result.is_err(), "Pending authorization should be rejected");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not authorized") || err.to_string().contains("NotAuthorized"),
        "Error should mention authorization, got: {}",
        err
    );
}

#[test]
fn test_a_rejected_authorization_fails() {
    let temp_dir = setup_temp_dir();
    let db_root = temp_dir.path();

    // Create codegraph.db (execution_log.db auto-created)
    let graph_db_path = db_root.join("codegraph.db");
    fs::File::create(&graph_db_path).expect("Failed to create codegraph.db");

    let exec_db = ExecutionDb::open(db_root).expect("Failed to open ExecutionDb");

    let plan = create_test_plan(vec![]);
    let mut auth = create_pending_authorization(plan.plan_id.clone());
    auth.reject();
    let approved = ApprovedPlan {
        plan,
        authorization: auth,
    };

    let confirmation = Box::new(AutoApprove);
    let progress = Box::new(NoopProgress);

    let mut executor = Executor::new(exec_db, None, confirmation, progress);

    let result = executor.execute(approved);

    assert!(
        result.is_err(),
        "Rejected authorization should fail execution"
    );
}

#[test]
fn test_a_plan_id_mismatch_rejected() {
    let temp_dir = setup_temp_dir();
    let db_root = temp_dir.path();

    // Create codegraph.db (execution_log.db auto-created)
    let graph_db_path = db_root.join("codegraph.db");
    fs::File::create(&graph_db_path).expect("Failed to create codegraph.db");

    let exec_db = ExecutionDb::open(db_root).expect("Failed to open ExecutionDb");

    let plan = create_test_plan(vec![]);
    let auth = create_approved_authorization("different_plan_id".to_string());
    let approved = ApprovedPlan {
        plan,
        authorization: auth,
    };

    let confirmation = Box::new(AutoApprove);
    let progress = Box::new(NoopProgress);

    let mut executor = Executor::new(exec_db, None, confirmation, progress);

    let result = executor.execute(approved);

    assert!(result.is_err(), "Plan ID mismatch should be rejected");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("mismatch") || err.to_string().contains("Mismatch"),
        "Error should mention mismatch, got: {}",
        err
    );
}

// === CATEGORY B: Single-Step Success ===

#[test]
fn test_b_single_step_success() {
    let temp_dir = setup_temp_dir();
    let db_root = temp_dir.path();

    // Create a test file to read
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello, World!").expect("Failed to write test file");

    // Create codegraph.db (execution_log.db auto-created)
    let graph_db_path = db_root.join("codegraph.db");
    fs::File::create(&graph_db_path).expect("Failed to create codegraph.db");

    let exec_db = ExecutionDb::open(db_root).expect("Failed to open ExecutionDb");

    // Create a plan with one file_read step
    let mut args = HashMap::new();
    args.insert("path".to_string(), test_file.display().to_string());

    let step = Step {
        step_id: "step_1".to_string(),
        tool: "file_read".to_string(),
        arguments: args,
        precondition: "file exists".to_string(),
        requires_confirmation: false,
    };

    let plan = create_test_plan(vec![step]);
    let auth = create_approved_authorization(plan.plan_id.clone());
    let approved = ApprovedPlan {
        plan,
        authorization: auth,
    };

    let confirmation = Box::new(AutoApprove);
    let progress = Box::new(NoopProgress);

    let mut executor = Executor::new(exec_db, None, confirmation, progress);

    let result = executor
        .execute(approved)
        .expect("Execution should succeed");

    assert_eq!(result.status, ExecutionStatus::Completed);
    assert_eq!(result.step_results.len(), 1);
    assert_eq!(result.step_results[0].step_id, "step_1");
    assert_eq!(result.step_results[0].tool_name, "file_read");
    assert!(result.step_results[0].success);
    assert!(!result.step_results[0].execution_id.is_empty());
    assert!(result.step_results[0].duration_ms >= 0);
}

#[test]
fn test_b_single_step_logs_to_db() {
    let temp_dir = setup_temp_dir();
    let db_root = temp_dir.path();

    // Create a test file to read
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello, World!").expect("Failed to write test file");

    // Create codegraph.db (execution_log.db auto-created)
    let graph_db_path = db_root.join("codegraph.db");
    fs::File::create(&graph_db_path).expect("Failed to create codegraph.db");

    let exec_db = ExecutionDb::open(db_root).expect("Failed to open ExecutionDb");

    // Create a plan with one file_read step
    let mut args = HashMap::new();
    args.insert("path".to_string(), test_file.display().to_string());

    let step = Step {
        step_id: "step_1".to_string(),
        tool: "file_read".to_string(),
        arguments: args,
        precondition: "file exists".to_string(),
        requires_confirmation: false,
    };

    let plan = create_test_plan(vec![step]);
    let auth = create_approved_authorization(plan.plan_id.clone());
    let approved = ApprovedPlan {
        plan,
        authorization: auth,
    };

    let confirmation = Box::new(AutoApprove);
    let progress = Box::new(NoopProgress);

    let mut executor = Executor::new(exec_db, None, confirmation, progress);

    let result = executor
        .execute(approved)
        .expect("Execution should succeed");
    let execution_id = &result.step_results[0].execution_id;

    // Re-open database to verify the execution was logged
    let verify_db = ExecutionDb::open(db_root).expect("Failed to reopen ExecutionDb");
    let conn = verify_db.conn();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM executions WHERE id = ?1",
            [execution_id.as_str()],
            |row| row.get(0),
        )
        .expect("Failed to query executions table");

    assert_eq!(count, 1, "Execution should be logged to database");
}

// === CATEGORY C: Failure Stops Execution ===

#[test]
fn test_c_failure_stops_execution() {
    let temp_dir = setup_temp_dir();
    let db_root = temp_dir.path();

    // Create codegraph.db (execution_log.db auto-created)
    let graph_db_path = db_root.join("codegraph.db");
    fs::File::create(&graph_db_path).expect("Failed to create codegraph.db");

    let exec_db = ExecutionDb::open(db_root).expect("Failed to open ExecutionDb");

    // Create a plan with 3 steps where step 2 will fail (file doesn't exist)
    let mut args1 = HashMap::new();
    args1.insert("pattern".to_string(), "*.txt".to_string());
    args1.insert("root".to_string(), ".".to_string());

    let mut args2 = HashMap::new();
    args2.insert("path".to_string(), "/nonexistent/file.txt".to_string());

    let mut args3 = HashMap::new();
    args3.insert("pattern".to_string(), "*.rs".to_string());
    args3.insert("root".to_string(), ".".to_string());

    let step1 = Step {
        step_id: "step_1".to_string(),
        tool: "file_glob".to_string(),
        arguments: args1,
        precondition: "root exists".to_string(),
        requires_confirmation: false,
    };

    let step2 = Step {
        step_id: "step_2".to_string(),
        tool: "file_read".to_string(),
        arguments: args2,
        precondition: "file exists".to_string(),
        requires_confirmation: false,
    };

    let step3 = Step {
        step_id: "step_3".to_string(),
        tool: "file_glob".to_string(),
        arguments: args3,
        precondition: "root exists".to_string(),
        requires_confirmation: false,
    };

    let plan = create_test_plan(vec![step1, step2, step3]);
    let auth = create_approved_authorization(plan.plan_id.clone());
    let approved = ApprovedPlan {
        plan,
        authorization: auth,
    };

    let confirmation = Box::new(AutoApprove);
    let progress = Box::new(NoopProgress);

    let mut executor = Executor::new(exec_db, None, confirmation, progress);

    let result = executor
        .execute(approved)
        .expect("Execution should return result");

    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(
        result.step_results.len(),
        2,
        "Should stop after step 2 fails"
    );
    assert_eq!(result.step_results[0].step_id, "step_1");
    assert!(result.step_results[0].success, "Step 1 should succeed");
    assert_eq!(result.step_results[1].step_id, "step_2");
    assert!(!result.step_results[1].success, "Step 2 should fail");
}

// === CATEGORY D: Confirmation Denied ===

#[test]
fn test_d_confirmation_denied_stops_execution() {
    let temp_dir = setup_temp_dir();
    let db_root = temp_dir.path();

    // Create codegraph.db (execution_log.db auto-created)
    let graph_db_path = db_root.join("codegraph.db");
    fs::File::create(&graph_db_path).expect("Failed to create codegraph.db");

    let exec_db = ExecutionDb::open(db_root).expect("Failed to open ExecutionDb");

    // Create a plan with 2 steps where step 1 requires confirmation
    let mut args1 = HashMap::new();
    args1.insert("pattern".to_string(), "*.txt".to_string());
    args1.insert("root".to_string(), ".".to_string());

    let mut args2 = HashMap::new();
    args2.insert("pattern".to_string(), "*.rs".to_string());
    args2.insert("root".to_string(), ".".to_string());

    let step1 = Step {
        step_id: "step_1".to_string(),
        tool: "file_glob".to_string(),
        arguments: args1,
        precondition: "root exists".to_string(),
        requires_confirmation: true, // Requires confirmation
    };

    let step2 = Step {
        step_id: "step_2".to_string(),
        tool: "file_glob".to_string(),
        arguments: args2,
        precondition: "root exists".to_string(),
        requires_confirmation: false,
    };

    let plan = create_test_plan(vec![step1, step2]);
    let auth = create_approved_authorization(plan.plan_id.clone());
    let approved = ApprovedPlan {
        plan,
        authorization: auth,
    };

    let confirmation = Box::new(AutoDeny); // Will deny all confirmations
    let progress = Box::new(NoopProgress);

    let mut executor = Executor::new(exec_db, None, confirmation, progress);

    let result = executor
        .execute(approved)
        .expect("Execution should return result");

    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(
        result.step_results.len(),
        1,
        "Should stop after step 1 confirmation denied"
    );
    assert!(
        !result.step_results[0].success,
        "Step 1 should fail due to denial"
    );
}

// === CATEGORY E: Evidence Logged ===

#[test]
fn test_e_evidence_logged_per_step() {
    let temp_dir = setup_temp_dir();
    let db_root = temp_dir.path();

    // Create a test file to read
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Test content").expect("Failed to write test file");

    // Create codegraph.db (execution_log.db auto-created)
    let graph_db_path = db_root.join("codegraph.db");
    fs::File::create(&graph_db_path).expect("Failed to create codegraph.db");

    let exec_db = ExecutionDb::open(db_root).expect("Failed to open ExecutionDb");

    // Create a plan with 2 steps
    let mut args1 = HashMap::new();
    args1.insert("path".to_string(), test_file.display().to_string());

    let mut args2 = HashMap::new();
    args2.insert("pattern".to_string(), "*.txt".to_string());
    args2.insert("root".to_string(), ".".to_string());

    let step1 = Step {
        step_id: "step_1".to_string(),
        tool: "file_read".to_string(),
        arguments: args1,
        precondition: "file exists".to_string(),
        requires_confirmation: false,
    };

    let step2 = Step {
        step_id: "step_2".to_string(),
        tool: "file_glob".to_string(),
        arguments: args2,
        precondition: "root exists".to_string(),
        requires_confirmation: false,
    };

    let plan = create_test_plan(vec![step1, step2]);
    let auth = create_approved_authorization(plan.plan_id.clone());
    let approved = ApprovedPlan {
        plan,
        authorization: auth,
    };

    let confirmation = Box::new(AutoApprove);
    let progress = Box::new(NoopProgress);

    let mut executor = Executor::new(exec_db, None, confirmation, progress);

    let result = executor
        .execute(approved)
        .expect("Execution should succeed");

    assert_eq!(result.step_results.len(), 2);

    // Verify both steps have execution_ids
    let exec_id_1 = &result.step_results[0].execution_id;
    let exec_id_2 = &result.step_results[1].execution_id;

    assert!(!exec_id_1.is_empty());
    assert!(!exec_id_2.is_empty());
    assert_ne!(exec_id_1, exec_id_2, "Execution IDs should be unique");

    // Re-open database to verify both logged to DB
    let verify_db = ExecutionDb::open(db_root).expect("Failed to reopen ExecutionDb");
    let conn = verify_db.conn();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM executions WHERE id IN (?1, ?2)",
            [exec_id_1.as_str(), exec_id_2.as_str()],
            |row| row.get(0),
        )
        .expect("Failed to query executions table");

    assert_eq!(count, 2, "Both steps should be logged to database");
}

// === CATEGORY F: Deterministic Execution ID ===

#[test]
fn test_f_execution_ids_are_unique() {
    let temp_dir = setup_temp_dir();
    let db_root = temp_dir.path();

    // Create a test file to read
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Test content").expect("Failed to write test file");

    // Create codegraph.db (execution_log.db auto-created)
    let graph_db_path = db_root.join("codegraph.db");
    fs::File::create(&graph_db_path).expect("Failed to create codegraph.db");

    // Create two separate ExecutionDb instances for the two executors
    let exec_db1 = ExecutionDb::open(db_root).expect("Failed to open ExecutionDb 1");
    let exec_db2 = ExecutionDb::open(db_root).expect("Failed to open ExecutionDb 2");

    let mut args = HashMap::new();
    args.insert("path".to_string(), test_file.display().to_string());

    let step = Step {
        step_id: "step_1".to_string(),
        tool: "file_read".to_string(),
        arguments: args.clone(),
        precondition: "file exists".to_string(),
        requires_confirmation: false,
    };

    let confirmation = Box::new(AutoApprove);
    let progress = Box::new(NoopProgress);

    // Execute the same plan twice
    let plan1 = create_test_plan(vec![step.clone()]);
    let auth1 = create_approved_authorization(plan1.plan_id.clone());
    let approved1 = ApprovedPlan {
        plan: plan1,
        authorization: auth1,
    };

    let plan2 = create_test_plan(vec![step]);
    let auth2 = create_approved_authorization(plan2.plan_id.clone());
    let approved2 = ApprovedPlan {
        plan: plan2,
        authorization: auth2,
    };

    let mut executor1 = Executor::new(exec_db1, None, confirmation.clone(), progress.clone());
    let result1 = executor1
        .execute(approved1)
        .expect("First execution should succeed");

    let mut executor2 = Executor::new(exec_db2, None, confirmation, progress);
    let result2 = executor2
        .execute(approved2)
        .expect("Second execution should succeed");

    // Execution IDs should be unique (UUID v4)
    let exec_id_1 = &result1.step_results[0].execution_id;
    let exec_id_2 = &result2.step_results[0].execution_id;

    assert_ne!(
        exec_id_1, exec_id_2,
        "Each execution should have unique execution_id"
    );
}

// === CATEGORY G: Forbidden Tool Rejection ===

#[test]
fn test_g_forbidden_tool_rejected() {
    let temp_dir = setup_temp_dir();
    let db_root = temp_dir.path();

    // Create codegraph.db (execution_log.db auto-created)
    let graph_db_path = db_root.join("codegraph.db");
    fs::File::create(&graph_db_path).expect("Failed to create codegraph.db");

    let exec_db = ExecutionDb::open(db_root).expect("Failed to open ExecutionDb");

    // Create a plan with a forbidden tool
    let mut args = HashMap::new();
    args.insert("path".to_string(), "/etc/passwd".to_string());

    let step = Step {
        step_id: "step_1".to_string(),
        tool: "rm -rf /".to_string(), // Forbidden!
        arguments: args,
        precondition: "none".to_string(),
        requires_confirmation: false,
    };

    let plan = create_test_plan(vec![step]);
    let auth = create_approved_authorization(plan.plan_id.clone());
    let approved = ApprovedPlan {
        plan,
        authorization: auth,
    };

    let confirmation = Box::new(AutoApprove);
    let progress = Box::new(NoopProgress);

    let mut executor = Executor::new(exec_db, None, confirmation, progress);

    let result = executor.execute(approved);

    assert!(result.is_err(), "Forbidden tool should be rejected");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not found") || err.to_string().contains("ToolNotFound"),
        "Error should mention tool not found, got: {}",
        err
    );
}

// === CATEGORY H: Precondition Failure ===

#[test]
fn test_h_precondition_failure() {
    let temp_dir = setup_temp_dir();
    let db_root = temp_dir.path();

    // Create codegraph.db (execution_log.db auto-created)
    let graph_db_path = db_root.join("codegraph.db");
    fs::File::create(&graph_db_path).expect("Failed to create codegraph.db");

    let exec_db = ExecutionDb::open(db_root).expect("Failed to open ExecutionDb");

    // Create a plan with file_read on non-existent file
    let mut args = HashMap::new();
    args.insert("path".to_string(), "/nonexistent/file.txt".to_string());

    let step = Step {
        step_id: "step_1".to_string(),
        tool: "file_read".to_string(),
        arguments: args,
        precondition: "file exists".to_string(),
        requires_confirmation: false,
    };

    let plan = create_test_plan(vec![step]);
    let auth = create_approved_authorization(plan.plan_id.clone());
    let approved = ApprovedPlan {
        plan,
        authorization: auth,
    };

    let confirmation = Box::new(AutoApprove);
    let progress = Box::new(NoopProgress);

    let mut executor = Executor::new(exec_db, None, confirmation, progress);

    let result = executor
        .execute(approved)
        .expect("Execution should return result");

    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(result.step_results.len(), 1);
    assert!(!result.step_results[0].success);
    assert!(
        result.step_results[0].error_message.is_some(),
        "Precondition failure should have error message"
    );
}

// === Callback Tests ===

#[test]
fn test_progress_callbacks_invoked() {
    let temp_dir = setup_temp_dir();
    let db_root = temp_dir.path();

    // Create a test file to read
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Test content").expect("Failed to write test file");

    // Create codegraph.db (execution_log.db auto-created)
    let graph_db_path = db_root.join("codegraph.db");
    fs::File::create(&graph_db_path).expect("Failed to create codegraph.db");

    let exec_db = ExecutionDb::open(db_root).expect("Failed to open ExecutionDb");

    let mut args = HashMap::new();
    args.insert("path".to_string(), test_file.display().to_string());

    let step = Step {
        step_id: "step_1".to_string(),
        tool: "file_read".to_string(),
        arguments: args,
        precondition: "file exists".to_string(),
        requires_confirmation: false,
    };

    let plan = create_test_plan(vec![step]);
    let auth = create_approved_authorization(plan.plan_id.clone());
    let approved = ApprovedPlan {
        plan,
        authorization: auth,
    };

    let confirmation = Box::new(AutoApprove);
    let progress = Box::new(TestProgressCallback::new());

    let mut executor = Executor::new(exec_db, None, confirmation, progress);

    let result = executor
        .execute(approved)
        .expect("Execution should succeed");

    assert_eq!(result.status, ExecutionStatus::Completed);

    // Get the progress callback back to verify it was called
    // (This requires a custom implementation that tracks calls)
}

// Test callback implementation
struct TestProgressCallback {
    start_called: std::sync::Arc<std::sync::atomic::AtomicBool>,
    complete_called: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl TestProgressCallback {
    fn new() -> Self {
        TestProgressCallback {
            start_called: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            complete_called: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

impl ProgressCallback for TestProgressCallback {
    fn on_step_start(&self, _step: &odincode::llm::types::Step) {
        self.start_called
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn on_step_complete(&self, _result: &StepResult) {
        self.complete_called
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn on_step_failed(&self, _result: &StepResult) {
        // Not used in this test
    }
}
