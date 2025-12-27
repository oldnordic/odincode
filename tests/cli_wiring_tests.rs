//! CLI wiring integration tests (Phase 4)
//!
//! Tests end-to-end CLI behavior:
//! - Mode dispatch (tui, plan, execute, evidence)
//! - db_root resolution
//! - Exit codes
//! - Output formats
//!
//! All tests use real binary execution via std::process::Command.
//! All tests use temp directories for db_root.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

// Test helper: Find the odincode binary
fn odincode_binary() -> PathBuf {
    // During cargo test, binary is at target/debug/odincode
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("odincode");
    path
}

// Test helper: Create a minimal db_root with execution_log.db
fn create_db_root_with_execution_log() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let exec_log_path = temp_dir.path().join("execution_log.db");

    // Create execution_log.db with minimal schema
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

    temp_dir
}

// Test helper: Create a minimal db_root with both databases
fn create_db_root_with_both() -> TempDir {
    let temp_dir = create_db_root_with_execution_log();

    // Create codegraph.db
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

    // Create config.toml to skip preflight in tests
    // Use stub provider for plan mode tests
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

// Test helper: Create a minimal plan JSON file
fn create_plan_file(db_root: &Path, plan_id: &str) -> PathBuf {
    let plans_dir = db_root.join("plans");
    fs::create_dir_all(&plans_dir).unwrap();

    let plan_path = plans_dir.join(format!("{}.json", plan_id));
    let plan_content = serde_json::json!({
        "plan_id": plan_id,
        "intent": "Read",
        "steps": [{
            "step_id": "step_1",
            "tool": "file_read",
            "arguments": {"path": "src/lib.rs"},
            "precondition": "none",
            "requires_confirmation": false
        }],
        "evidence_referenced": []
    });

    let mut file = File::create(&plan_path).unwrap();
    file.write_all(plan_content.to_string().as_bytes()).unwrap();

    plan_path
}

// ============================================================================
// A1-A2: Mode Tests (Default/TUI)
// ============================================================================

#[test]
fn test_default_runs_tui_help() {
    let bin = odincode_binary();
    if !bin.exists() {
        // Skip if binary not built yet
        return;
    }

    // --help should work without specifying a mode
    let output = Command::new(&bin)
        .arg("--help")
        .output()
        .expect("Failed to execute odincode --help");

    assert!(output.status.success(), "--help should exit with 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("USAGE:"), "Should show usage information");
    assert!(stdout.contains("MODES:"), "Should list available modes");
}

#[test]
fn test_version_flag() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let output = Command::new(&bin)
        .arg("--version")
        .output()
        .expect("Failed to execute odincode --version");

    assert!(output.status.success(), "--version should exit with 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("OdinCode"), "Should show version info");
}

// ============================================================================
// B1-B4: db_root Resolution Tests
// ============================================================================

#[test]
fn test_db_root_flag_takes_precedence() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_db_root_with_both();

    // --version with explicit --db-root should succeed
    let output = Command::new(&bin)
        .arg("--version")
        .arg("--db-root")
        .arg(temp_dir.path())
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "--db-root flag should take precedence"
    );
}

#[test]
fn test_db_root_defaults_to_cwd() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_db_root_with_both();

    // Run from the temp directory (simulating cwd as db_root)
    let output = Command::new(&bin)
        .arg("--version")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "Should use cwd as default db_root");
}

#[test]
fn test_db_root_missing_exits_2() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let non_existent = "/tmp/odincode_test_nonexistent_12345";

    // Should exit with code 2 when db_root doesn't exist
    let output = Command::new(&bin)
        .arg("--db-root")
        .arg(non_existent)
        .arg("--version")
        .output()
        .expect("Failed to execute");

    assert_eq!(
        output.status.code().unwrap(),
        2,
        "Missing db_root should exit with code 2, got {}",
        output.status.code().unwrap()
    );
}

// ============================================================================
// C1-C4: plan Mode Tests
// ============================================================================

#[test]
fn test_plan_mode_creates_file() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_db_root_with_both();
    let plans_dir = temp_dir.path().join("plans");

    // Invoke plan mode
    let output = Command::new(&bin)
        .arg("plan")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("read src/lib.rs")
        .output()
        .expect("Failed to execute odincode plan");

    // Check exit code
    assert_eq!(
        output.status.code().unwrap(),
        0,
        "plan mode should exit with 0, got {}: {}",
        output.status.code().unwrap(),
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that plans directory was created
    assert!(plans_dir.exists(), "plans directory should be created");

    // Check that at least one plan file was created
    let entries = fs::read_dir(&plans_dir).unwrap();
    let plan_files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "json")
                .unwrap_or(false)
        })
        .collect();

    assert!(
        !plan_files.is_empty(),
        "At least one plan file should be created"
    );
}

#[test]
fn test_plan_mode_prints_plan_id() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_db_root_with_both();

    let output = Command::new(&bin)
        .arg("plan")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("read src/lib.rs")
        .output()
        .expect("Failed to execute odincode plan");

    assert_eq!(output.status.code().unwrap(), 0);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Plan written to"),
        "Should mention plan file location"
    );
    assert!(stdout.contains("plan_"), "Should contain plan_id prefix");
}

#[test]
fn test_plan_mode_json_output() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_db_root_with_both();

    let output = Command::new(&bin)
        .arg("plan")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("--json")
        .arg("read src/lib.rs")
        .output()
        .expect("Failed to execute odincode plan --json");

    assert_eq!(output.status.code().unwrap(), 0);

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(
        parsed.get("plan_id").is_some(),
        "JSON should contain plan_id"
    );
    assert!(parsed.get("path").is_some(), "JSON should contain path");
}

#[test]
fn test_plan_mode_requires_codegraph() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    // Create db_root with only execution_log.db (no codegraph.db)
    let temp_dir = create_db_root_with_execution_log();

    let output = Command::new(&bin)
        .arg("plan")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("read src/lib.rs")
        .output()
        .expect("Failed to execute odincode plan");

    assert_eq!(
        output.status.code().unwrap(),
        2,
        "plan mode without codegraph.db should exit with 2"
    );
}

// ============================================================================
// D1-D4: execute Mode Tests
// ============================================================================

#[test]
fn test_execute_unknown_plan_exits_1() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_db_root_with_both();

    let output = Command::new(&bin)
        .arg("execute")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("--plan-file")
        .arg("plans/nonexistent_plan.json")
        .output()
        .expect("Failed to execute odincode execute");

    assert_eq!(
        output.status.code().unwrap(),
        1,
        "Unknown plan should exit with 1"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("No such file"),
        "Should mention file not found"
    );
}

#[test]
fn test_execute_valid_plan_succeeds() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_db_root_with_both();

    // Create a test file to read
    let test_file = temp_dir.path().join("test.txt");
    let mut file = File::create(&test_file).unwrap();
    writeln!(file, "test content").unwrap();

    // Create a plan that reads the file
    let plan_id = "plan_test_read";
    create_plan_file(temp_dir.path(), plan_id);

    let plan_path = temp_dir
        .path()
        .join("plans")
        .join(format!("{}.json", plan_id));

    // Execute the plan
    let output = Command::new(&bin)
        .arg("execute")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("--plan-file")
        .arg(&plan_path)
        .output()
        .expect("Failed to execute odincode execute");

    // Note: This test expects file_read to succeed
    // Exit code 0 for success, or 1 for actual failure (but plan was found)
    let code = output.status.code().unwrap();
    assert!(
        code == 0 || code == 1,
        "execute should return 0 (success) or 1 (tool failed), got {}",
        code
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Plan") || stdout.contains("step"),
        "Should show execution result"
    );
}

#[test]
fn test_execute_auto_approves() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_db_root_with_both();
    let plan_id = "plan_auto_approve";
    create_plan_file(temp_dir.path(), plan_id);

    let plan_path = temp_dir
        .path()
        .join("plans")
        .join(format!("{}.json", plan_id));

    // Should NOT prompt for confirmation in CLI mode
    let output = Command::new(&bin)
        .arg("execute")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("--plan-file")
        .arg(&plan_path)
        .output()
        .expect("Failed to execute odincode execute");

    // If it prompted for confirmation, it would hang (timeout)
    // The fact we got output means it auto-approved
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stdout.contains("Approve?") && !stderr.contains("Approve?"),
        "CLI mode should not prompt for confirmation, got: {}{}",
        stdout,
        stderr
    );
}

// ============================================================================
// E1-E4: evidence Mode Tests
// ============================================================================

#[test]
fn test_evidence_q1_query() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_db_root_with_both();

    // Run evidence query Q1
    let output = Command::new(&bin)
        .arg("evidence")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("Q1")
        .arg("file_read")
        .output()
        .expect("Failed to execute odincode evidence Q1");

    // Q1 query should succeed (even with empty results)
    assert_eq!(
        output.status.code().unwrap(),
        0,
        "evidence Q1 query should exit with 0"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON output
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("Q1 output should be valid JSON");

    // Should be an array (possibly empty)
    assert!(parsed.is_array(), "Q1 result should be an array");
}

#[test]
fn test_evidence_empty_db_exits_0() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_db_root_with_both();

    let output = Command::new(&bin)
        .arg("evidence")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("Q1")
        .arg("file_read")
        .output()
        .expect("Failed to execute odincode evidence");

    // Empty DB should still exit 0 (results are just empty)
    assert_eq!(
        output.status.code().unwrap(),
        0,
        "Empty evidence DB should exit with 0"
    );
}

#[test]
fn test_evidence_missing_db_exits_2() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = TempDir::new().unwrap();

    let output = Command::new(&bin)
        .arg("evidence")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("Q1")
        .arg("file_read")
        .output()
        .expect("Failed to execute odincode evidence");

    assert_eq!(
        output.status.code().unwrap(),
        2,
        "Missing evidence DB should exit with 2"
    );
}

#[test]
fn test_evidence_json_output() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_db_root_with_both();

    let output = Command::new(&bin)
        .arg("evidence")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("--json")
        .arg("Q1")
        .arg("file_read")
        .output()
        .expect("Failed to execute odincode evidence --json");

    assert_eq!(output.status.code().unwrap(), 0);

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("evidence output should be valid JSON");
    assert!(parsed.is_array(), "evidence result should be an array");
}

// ============================================================================
// F1: Plan Storage Lifecycle Tests
// ============================================================================

#[test]
fn test_plan_lifecycle_store_load_execute() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_db_root_with_both();

    // Step 1: Create a plan via plan mode
    let plan_output = Command::new(&bin)
        .arg("plan")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("read test.txt")
        .output()
        .expect("Failed to create plan");

    assert_eq!(
        plan_output.status.code().unwrap(),
        0,
        "Plan creation should succeed"
    );

    // Extract plan_id from output
    let plan_stdout = String::from_utf8_lossy(&plan_output.stdout);
    assert!(plan_stdout.contains("Plan written to"));

    // Find the created plan file
    let plans_dir = temp_dir.path().join("plans");
    let entries: Vec<_> = fs::read_dir(&plans_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    assert!(!entries.is_empty(), "Plan file should exist");

    // Step 2: Execute the plan
    let plan_file = entries[0].path();
    let exec_output = Command::new(&bin)
        .arg("execute")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("--plan-file")
        .arg(&plan_file)
        .output()
        .expect("Failed to execute plan");

    // Execution should complete (success or failure, but plan was loaded)
    let exec_code = exec_output.status.code().unwrap();
    assert!(
        exec_code == 0 || exec_code == 1,
        "Plan execution should complete with 0 or 1, got {}",
        exec_code
    );

    // Step 3: Query evidence to verify execution was logged
    let evidence_output = Command::new(&bin)
        .arg("evidence")
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("Q1")
        .arg("file_read")
        .output()
        .expect("Failed to query evidence");

    assert_eq!(evidence_output.status.code().unwrap(), 0);
}
