//! UI Routing Tests (Phase 8: 3-way router architecture)
//!
//! Tests end-to-end routing behavior:
//! - Commands (start with "/") execute immediately
//! - Chat (default) routes to LLM, no approval for display_text
//! - Plan (/plan, /apply) triggers structured plan mode with approval
//!
//! All tests use real execution memory (no mocks).

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

// Test helper: Find the odincode binary
#[allow(dead_code)]
fn odincode_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("odincode");
    path
}

// Test helper: Create a minimal db_root with all databases
fn create_db_root_with_all() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Create execution_log.db with minimal schema
    let exec_log_path = temp_dir.path().join("execution_log.db");
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

    // Insert test symbol data for /find tests
    conn.execute(
        "INSERT INTO graph_entities (id, kind, name, file_path, data) VALUES
            (1, 'Symbol', 'main', 'src/main.rs', '{\"byte_start\": 0, \"byte_end\": 4}'),
            (2, 'Symbol', 'read_file', 'src/lib.rs', '{\"byte_start\": 10, \"byte_end\": 18}'),
            (3, 'Symbol', 'write_file', 'src/lib.rs', '{\"byte_start\": 20, \"byte_end\": 30}'),
            (4, 'File', 'main.rs', 'src/main.rs', '{}'),
            (5, 'File', 'lib.rs', 'src/lib.rs', '{}')",
        [],
    )
    .unwrap();

    // Create config.toml to skip preflight (LLM enabled)
    let config_path = temp_dir.path().join("config.toml");
    let mut config_file = File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"[llm]
mode = "external"
provider = "openai-compatible"
base_url = "https://api.openai.com/v1"
api_key = "sk-test-key-12345"
model = "gpt-4"
"#
    )
    .unwrap();

    temp_dir
}

// ============================================================================
// Test A: Chat (default) routes to LLM
// ============================================================================

#[test]
fn test_a_chat_routes_to_llm() {
    // Phase 8: Non-command input is Chat
    let cmd = odincode::ui::parse_command("read src/lib.rs");
    assert!(
        matches!(cmd, odincode::ui::Command::Chat(_)),
        "Non-command input should route to Chat, got: {:?}",
        cmd
    );
}

#[test]
fn test_a_colon_is_chat_not_command() {
    // Phase 8: ":" prefix is now chat, NOT a command
    let cmd = odincode::ui::parse_command(":read src/lib.rs");
    assert!(
        matches!(cmd, odincode::ui::Command::Chat(_)),
        "Colon prefix should be Chat, got: {:?}",
        cmd
    );
}

// ============================================================================
// Test B: Commands (start with "/") execute immediately
// ============================================================================

#[test]
fn test_b_slash_commands_parse_correctly() {
    // Phase 8: "/" prefix commands

    // /quit
    let cmd = odincode::ui::parse_command("/quit");
    assert!(
        matches!(cmd, odincode::ui::Command::Quit),
        "/quit should parse as Quit, got: {:?}",
        cmd
    );

    // /q
    let cmd = odincode::ui::parse_command("/q");
    assert!(
        matches!(cmd, odincode::ui::Command::Quit),
        "/q should parse as Quit, got: {:?}",
        cmd
    );

    // /exit
    let cmd = odincode::ui::parse_command("/exit");
    assert!(
        matches!(cmd, odincode::ui::Command::Quit),
        "/exit should parse as Quit, got: {:?}",
        cmd
    );

    // /help
    let cmd = odincode::ui::parse_command("/help");
    assert!(
        matches!(cmd, odincode::ui::Command::Help),
        "/help should parse as Help, got: {:?}",
        cmd
    );

    // /open
    let cmd = odincode::ui::parse_command("/open src/lib.rs");
    assert!(
        matches!(cmd, odincode::ui::Command::Open(_)),
        "/open should parse as Open, got: {:?}",
        cmd
    );

    // /read
    let cmd = odincode::ui::parse_command("/read src/lib.rs");
    assert!(
        matches!(cmd, odincode::ui::Command::Read(_)),
        "/read should parse as Read, got: {:?}",
        cmd
    );

    // /lsp
    let cmd = odincode::ui::parse_command("/lsp");
    assert!(
        matches!(cmd, odincode::ui::Command::Lsp(_)),
        "/lsp should parse as Lsp, got: {:?}",
        cmd
    );

    // /find
    let cmd = odincode::ui::parse_command("/find main");
    assert!(
        matches!(cmd, odincode::ui::Command::Find(_)),
        "/find should parse as Find, got: {:?}",
        cmd
    );
}

#[test]
fn test_b_chat_not_command() {
    // Verify it's NOT parsed as command
    let cmd = odincode::ui::parse_command("read src/lib.rs");
    assert!(
        !matches!(cmd, odincode::ui::Command::Read(_)),
        "Chat input should NOT be Read command"
    );
    assert!(
        matches!(cmd, odincode::ui::Command::Chat(_)),
        "Chat input should be Chat variant"
    );
}

// ============================================================================
// Test C: Plan commands
// ============================================================================

#[test]
fn test_c_plan_commands_parse_correctly() {
    // Phase 8: /plan and /apply are explicit plan triggers

    // /plan (no args needed now)
    let cmd = odincode::ui::parse_command("/plan");
    assert!(
        matches!(cmd, odincode::ui::Command::Plan),
        "/plan should parse as Plan, got: {:?}",
        cmd
    );

    // /p (short form)
    let cmd = odincode::ui::parse_command("/p");
    assert!(
        matches!(cmd, odincode::ui::Command::Plan),
        "/p should parse as Plan, got: {:?}",
        cmd
    );

    // /apply
    let cmd = odincode::ui::parse_command("/apply");
    assert!(
        matches!(cmd, odincode::ui::Command::Apply),
        "/apply should parse as Apply, got: {:?}",
        cmd
    );
}

// ============================================================================
// Test D: Validation still works
// ============================================================================

#[test]
fn test_d_plan_validation_still_works() {
    // Verify plan validation logic wasn't broken
    let plan = odincode::llm::Plan {
        plan_id: "test_plan".to_string(),
        intent: odincode::llm::Intent::Read,
        steps: vec![],
        evidence_referenced: vec!["Q99".to_string()], // Invalid evidence query
    };

    let result = odincode::llm::validate_plan(&plan);
    assert!(
        result.is_err(),
        "Plan with invalid evidence should fail validation"
    );

    let err = result.unwrap_err();
    assert!(
        matches!(err, odincode::llm::PlanError::InvalidEvidenceQuery(_)),
        "Should get InvalidEvidenceQuery error, got: {:?}",
        err
    );
}

// ============================================================================
// Test E: Help text updated
// ============================================================================

#[test]
fn test_e_help_text_mentions_slash_commands() {
    let help_text = odincode::ui::render_help();
    assert!(
        help_text.contains("/quit"),
        "Help text should mention /quit command"
    );
    assert!(
        help_text.contains("/help"),
        "Help text should mention /help command"
    );
    assert!(
        help_text.contains("chat"),
        "Help text should mention chat mode"
    );
}

// ============================================================================
// Test F: /find returns sorted results
// ============================================================================

#[test]
fn test_f_find_command_returns_sorted_results() {
    let temp_dir = create_db_root_with_all();

    // Open Magellan DB
    let magellan_db =
        odincode::magellan_tools::MagellanDb::open_readonly(temp_dir.path().join("codegraph.db"))
            .unwrap();

    // Query for symbols - results should be sorted by name
    let symbols = magellan_db.symbols_in_file("%").unwrap();
    assert_eq!(symbols.len(), 3, "Should find 3 test symbols");

    // Verify deterministic order (sorted by name)
    assert_eq!(symbols[0].name, "main", "First symbol should be 'main'");
    assert_eq!(
        symbols[1].name, "read_file",
        "Second symbol should be 'read_file'"
    );
    assert_eq!(
        symbols[2].name, "write_file",
        "Third symbol should be 'write_file'"
    );
}

// ============================================================================
// Test G: State transitions still work
// ============================================================================

#[test]
fn test_g_planning_state_transitions() {
    use odincode::llm::Intent;
    use odincode::ui::state::{App, AppState};

    let temp_dir = create_db_root_with_all();
    let mut app = App::new(temp_dir.path().to_path_buf());

    // Initial state should be Running
    assert_eq!(app.state(), AppState::Running);

    // After planning, state should transition to PlanningInProgress
    app.set_planning_in_progress();
    assert_eq!(app.state(), AppState::PlanningInProgress);

    // After plan ready, state should be PlanReady
    let plan = odincode::llm::Plan {
        plan_id: "test_plan".to_string(),
        intent: Intent::Read,
        steps: vec![],
        evidence_referenced: vec![],
    };
    app.set_plan_ready(plan);
    assert_eq!(app.state(), AppState::PlanReady);

    // After error, state should be PlanError
    app.set_plan_error("Test error".to_string());
    assert_eq!(app.state(), AppState::PlanError);
}

// ============================================================================
// Test H: Unknown command is rejected
// ============================================================================

#[test]
fn test_h_unknown_command_rejected() {
    // Test that unknown commands return Command::None
    let cmd = odincode::ui::parse_command("/unknown_command");
    assert!(
        matches!(cmd, odincode::ui::Command::None),
        "Unknown command should return None, got: {:?}",
        cmd
    );

    // Test that just "/" alone returns None
    let cmd2 = odincode::ui::parse_command("/");
    assert!(
        matches!(cmd2, odincode::ui::Command::None),
        "Lone '/' should return None, got: {:?}",
        cmd2
    );
}

// ============================================================================
// Test I: /find command parsing
// ============================================================================

#[test]
fn test_i_find_command_parsing() {
    // Test /find with pattern
    let cmd = odincode::ui::parse_command("/find main");
    match cmd {
        odincode::ui::Command::Find(pattern) => {
            assert_eq!(pattern, "main", "Find pattern should be 'main'");
        }
        _ => panic!("/find should parse as Find command, got: {:?}", cmd),
    }
}

// ============================================================================
// Test J: Empty and whitespace input handling
// ============================================================================

#[test]
fn test_j_empty_input_returns_none() {
    // Empty input
    let cmd1 = odincode::ui::parse_command("");
    assert!(
        matches!(cmd1, odincode::ui::Command::None),
        "Empty input should return None"
    );

    // Whitespace only
    let cmd2 = odincode::ui::parse_command("   ");
    assert!(
        matches!(cmd2, odincode::ui::Command::None),
        "Whitespace input should return None"
    );

    // Just "/"
    let cmd3 = odincode::ui::parse_command("/");
    assert!(
        matches!(cmd3, odincode::ui::Command::None),
        "Lone '/' should return None"
    );
}
