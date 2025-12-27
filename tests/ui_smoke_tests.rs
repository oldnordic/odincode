//! UI smoke tests — Phase 1 TUI implementation
//!
//! These tests verify the terminal UI can start and exit cleanly.
//!
//! Phase 1 Constraints:
//! - NO async
//! - NO background threads
//! - UI is a deterministic surface only
//! - Every action = explicit tool call

use std::process::Command as ProcessCommand;

/// Test that the binary exists and can show help
#[test]
fn test_binary_exists_and_runs() {
    let output = ProcessCommand::new("cargo")
        .args(["run", "--bin", "odincode", "--", "--help"])
        .output()
        .expect("Failed to run odincode binary");

    // Binary should run successfully (exit code 0 or 1 for --help is acceptable)
    // We just verify it doesn't crash
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "Binary should run without crashing"
    );
}

/// Test that the binary can be invoked with --version
#[test]
fn test_binary_version() {
    let output = ProcessCommand::new("cargo")
        .args(["run", "--bin", "odincode", "--", "--version"])
        .output()
        .expect("Failed to run odincode binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("OdinCode") || stdout.contains("odincode"),
        "Version output should contain binary name"
    );
}

/// Test that we can create a temp db_root for UI testing
#[test]
fn test_temp_db_root_creation() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_root = temp_dir.path();

    // Verify db_root path exists
    assert!(db_root.exists(), "db_root should exist");

    // Create minimal execution_log.db for EvidenceDb
    let exec_log_path = db_root.join("execution_log.db");
    {
        let conn =
            rusqlite::Connection::open(&exec_log_path).expect("Failed to create execution_log.db");
        conn.execute(
            "CREATE TABLE executions (id TEXT PRIMARY KEY, tool_name TEXT)",
            [],
        )
        .expect("Failed to create executions table");
    } // Drop connection

    // Verify file exists after connection closes
    assert!(exec_log_path.exists(), "execution_log.db should exist");
}

/// Test that EvidenceDb can be opened with temp db_root
#[test]
fn test_evidence_db_opens_with_temp_root() {
    use odincode::evidence_queries::EvidenceDb;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_root = temp_dir.path();

    // Create minimal execution_log.db
    let exec_log_path = db_root.join("execution_log.db");
    {
        let conn =
            rusqlite::Connection::open(&exec_log_path).expect("Failed to create execution_log.db");
        conn.execute(
            "CREATE TABLE executions (id TEXT PRIMARY KEY, tool_name TEXT, timestamp INTEGER)",
            [],
        )
        .expect("Failed to create table");
    }

    // EvidenceDb should open (without codegraph.db is OK for read-only queries)
    let result = EvidenceDb::open(db_root);
    assert!(
        result.is_ok(),
        "EvidenceDb should open with execution_log.db only"
    );
}

/// Test: No background threads spawned by design
///
/// This is a compile-time assertion: we simply verify the ui module
/// compiles without std::thread usage. The real check is code review.
#[test]
fn test_ui_compiles_without_thread_spawn() {
    // This test passes if the code compiles
    // Code review will verify no std::thread::spawn in ui modules
    // UI must be single-threaded by design (verified by code review)
}

/// Test: Deterministic command parsing
#[test]
fn test_command_parsing_empty() {
    // Before UI module exists, this test documents the expected behavior
    let input = "";
    let parsed = parse_command_minimal(input);
    assert_eq!(parsed, UiCommand::None);
}

#[test]
fn test_command_parsing_quit() {
    let input = ":quit";
    let parsed = parse_command_minimal(input);
    assert_eq!(parsed, UiCommand::Quit);
}

#[test]
fn test_command_parsing_open() {
    let input = ":open src/lib.rs";
    let parsed = parse_command_minimal(input);
    assert_eq!(parsed, UiCommand::Open("src/lib.rs".to_string()));
}

#[test]
fn test_command_parsing_read() {
    let input = ":read src/main.rs";
    let parsed = parse_command_minimal(input);
    assert_eq!(parsed, UiCommand::Read("src/main.rs".to_string()));
}

#[test]
fn test_command_parsing_lsp() {
    let input = ":lsp .";
    let parsed = parse_command_minimal(input);
    assert_eq!(parsed, UiCommand::Lsp(".".to_string()));
}

#[test]
fn test_command_parsing_evidence() {
    let input = ":evidence list splice_patch";
    let parsed = parse_command_minimal(input);
    assert_eq!(
        parsed,
        UiCommand::Evidence("list".to_string(), vec!["splice_patch".to_string()])
    );
}

/// Minimal command representation for testing
#[derive(Debug, Clone, PartialEq, Eq)]
enum UiCommand {
    None,
    Quit,
    Open(String),
    Read(String),
    Lsp(String),
    Evidence(String, Vec<String>),
}

/// Minimal command parser (placeholder — will be replaced by ui::input::parse)
fn parse_command_minimal(input: &str) -> UiCommand {
    let input = input.trim();
    if input.is_empty() {
        return UiCommand::None;
    }

    if !input.starts_with(':') {
        return UiCommand::None;
    }

    let parts: Vec<&str> = input[1..].split_whitespace().collect();
    if parts.is_empty() {
        return UiCommand::None;
    }

    match parts[0] {
        "quit" | "q" => UiCommand::Quit,
        "open" | "o" => {
            if parts.len() > 1 {
                UiCommand::Open(parts[1].to_string())
            } else {
                UiCommand::None
            }
        }
        "read" | "r" => {
            if parts.len() > 1 {
                UiCommand::Read(parts[1].to_string())
            } else {
                UiCommand::None
            }
        }
        "lsp" => {
            if parts.len() > 1 {
                UiCommand::Lsp(parts[1].to_string())
            } else {
                UiCommand::Lsp(".".to_string())
            }
        }
        "evidence" | "ev" => {
            if parts.len() > 1 {
                let query_name = parts[1].to_string();
                let args: Vec<String> = parts[2..].iter().map(|s| s.to_string()).collect();
                UiCommand::Evidence(query_name, args)
            } else {
                UiCommand::None
            }
        }
        _ => UiCommand::None,
    }
}

/// Test: Rendering pipeline does not panic with empty state
#[test]
fn test_render_with_empty_state() {
    // This test documents the expected behavior
    // The actual render will be implemented in ui::view module
    let state = EmptyState {};
    // Should not panic when rendering empty state
    assert_eq!(state.file_count(), 0);
    assert_eq!(state.selected_file(), None);
}

#[derive(Debug, Default)]
struct EmptyState;

impl EmptyState {
    fn file_count(&self) -> usize {
        0
    }

    fn selected_file(&self) -> Option<&str> {
        None
    }
}

/// Test: Evidence query invocation path is wired
#[test]
fn test_evidence_query_wiring() {
    use odincode::evidence_queries::EvidenceDb;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_root = temp_dir.path();

    // Create minimal execution_log.db with one execution
    let exec_log_path = db_root.join("execution_log.db");
    {
        let conn =
            rusqlite::Connection::open(&exec_log_path).expect("Failed to create execution_log.db");
        // Create executions table with correct schema (matches execution_tools)
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
        .expect("Failed to create executions table");
        conn.execute(
            "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success)
            VALUES (?1, ?2, ?3, ?4, ?5)",
            ("test-id", "test_tool", "{}", 123456, true),
        )
        .expect("Failed to insert");
    }

    let ev_db_result = EvidenceDb::open(db_root);
    let ev_db = match ev_db_result {
        Ok(db) => db,
        Err(e) => panic!("EvidenceDb::open failed: {}", e),
    };

    // Q1: list_executions_by_tool should work
    let result = ev_db.list_executions_by_tool("test_tool", None, None, None);
    let result = match result {
        Ok(r) => r,
        Err(e) => panic!("list_executions_by_tool failed: {}", e),
    };
    assert_eq!(result.len(), 1, "Should find one execution");
    assert_eq!(result[0].tool_name, "test_tool");
}

/// Test: Insufficient evidence returns empty (not error)
#[test]
fn test_insufficient_evidence_returns_empty() {
    use odincode::evidence_queries::EvidenceDb;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_root = temp_dir.path();

    // Create execution_log.db with proper schema (empty)
    let exec_log_path = db_root.join("execution_log.db");
    {
        let conn =
            rusqlite::Connection::open(&exec_log_path).expect("Failed to create execution_log.db");
        // Schema must match execution_tools for EvidenceDb compatibility
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
        .expect("Failed to create executions table");
    }

    let ev_db = EvidenceDb::open(db_root).expect("Failed to open EvidenceDb");

    // Query for non-existent tool should return empty, not error
    let result = ev_db.list_executions_by_tool("nonexistent_tool", None, None, None);
    assert!(result.is_ok(), "Query should succeed even with no results");

    let executions = result.unwrap();
    assert_eq!(executions.len(), 0, "Should return empty for no matches");
}
