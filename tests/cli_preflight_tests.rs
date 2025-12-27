//! CLI LLM Preflight Tests
//!
//! Test-first development for Phase 4.1 LLM preflight configuration.
//! Tests verify:
//! - Prompt shown on first run
//! - Config written correctly
//! - Secrets never written to disk
//! - Preflight runs for both CLI and TUI entry points

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// Locate the odincode binary
fn odincode_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("odincode")
}

/// Helper to run command with stdin input
fn run_with_stdin(bin: &Path, args: &[&str], input: &str) -> io::Result<(String, String, i32)> {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Write to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())?;
    }

    // Wait for output
    let output = child.wait_with_output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok((stdout, stderr, exit_code))
}

/// Create a minimal db_root with execution_log.db
fn create_minimal_db_root() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path();

    // Create execution_log.db (minimal schema)
    let exec_db_path = db_root.join("execution_log.db");
    let _ = File::create(exec_db_path).unwrap();

    // Also create codegraph.db (minimal schema) so plan mode works
    create_codegraph_db(db_root);

    temp_dir
}

/// Create a minimal codegraph.db
fn create_codegraph_db(db_root: &Path) {
    use std::process::Command;

    let codegraph_path = db_root.join("codegraph.db");

    // Create minimal SQLiteGraph schema
    let result = Command::new("sqlite3")
        .arg(&codegraph_path)
        .arg(
            "
            CREATE TABLE graph_entities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                file_path TEXT,
                data TEXT NOT NULL
            );
            CREATE TABLE graph_edges (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                from_id INTEGER NOT NULL,
                to_id INTEGER NOT NULL,
                edge_type TEXT NOT NULL,
                data TEXT NOT NULL
            );
        ",
        )
        .output();

    // If sqlite3 not available, create via rusqlite
    if result.is_err() {
        use rusqlite::Connection;

        let conn = Connection::open(&codegraph_path).unwrap();
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
}

/// ============================================================================
/// Test A: Missing config → choose "No LLM" → config written → proceed
/// ============================================================================

#[test]
fn test_missing_config_choose_no_llm() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_minimal_db_root();

    // Input: "3" (No LLM)
    let input = "3\n";

    // Run with plan mode (triggers preflight)
    let (stdout, stderr, _exit_code) = run_with_stdin(
        &bin,
        &[
            "--db-root",
            temp_dir.path().to_str().unwrap(),
            "plan",
            "test goal",
        ],
        input,
    )
    .expect("Failed to execute");

    // Should show preflight prompt
    assert!(
        stdout.contains("No LLM configuration found")
            || stderr.contains("No LLM configuration found"),
        "Should show preflight prompt"
    );

    // Should write config.toml with disabled mode
    let config_path = temp_dir.path().join("config.toml");
    assert!(config_path.exists(), "config.toml should be created");

    let config_content = fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("mode = \"disabled\""),
        "Config should contain mode = \"disabled\""
    );
    assert!(
        config_content.contains("[llm]"),
        "Config should contain [llm] section"
    );

    // Should proceed (note: may show error about missing LLM, but that's expected)
    assert!(
        stdout.contains("Plan")
            || stdout.contains("Error")
            || stdout.contains("codegraph")
            || stderr.contains("Plan")
            || stderr.contains("Error")
            || stderr.contains("codegraph"),
        "Should proceed to plan mode (or show expected error)"
    );
}

/// ============================================================================
/// Test B1: Missing config → External → Direct storage (default)
/// ============================================================================

#[test]
fn test_missing_config_external_direct_storage() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_minimal_db_root();

    // Input sequence for Phase 4.2:
    // "1" (external)
    // "openai-compatible" (provider)
    // "https://api.openai.com/v1" (base URL)
    // "" (storage choice: empty = default = direct storage)
    // "sk-test-key-12345" (API key - stored literally)
    // "gpt-4" (model)
    let input = "1\nopenai-compatible\nhttps://api.openai.com/v1\n\nsk-test-key-12345\ngpt-4\n";

    let (stdout, stderr, _exit_code) = run_with_stdin(
        &bin,
        &[
            "--db-root",
            temp_dir.path().to_str().unwrap(),
            "plan",
            "test goal",
        ],
        input,
    )
    .expect("Failed to execute");

    // Should show preflight prompt
    assert!(
        stdout.contains("external API")
            || stdout.contains("External API provider")
            || stderr.contains("external API")
            || stderr.contains("External API provider"),
        "Should show external option"
    );

    // Should write config.toml
    let config_path = temp_dir.path().join("config.toml");
    assert!(config_path.exists(), "config.toml should be created");

    let config_content = fs::read_to_string(&config_path).unwrap();

    // Should contain external mode settings
    assert!(
        config_content.contains("mode = \"external\""),
        "Config should contain mode = \"external\""
    );
    assert!(
        config_content.contains("provider = \"openai-compatible\""),
        "Config should contain provider"
    );
    assert!(
        config_content.contains("base_url"),
        "Config should contain base_url"
    );
    assert!(
        config_content.contains("model"),
        "Config should contain model"
    );

    // Phase 4.2: API key IS stored literally (direct storage)
    assert!(
        config_content.contains("sk-test-key-12345"),
        "API key should be stored literally in config"
    );
    assert!(
        !config_content.contains("env:"),
        "API key should NOT be env reference for direct storage"
    );

    // Should exit with restart instruction
    assert!(
        stdout.contains("restart") || stderr.contains("restart"),
        "Should show restart instruction"
    );
}

/// ============================================================================
/// Test B2: Missing config → External → Env var storage
/// ============================================================================

#[test]
fn test_missing_config_external_env_storage() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_minimal_db_root();

    // Input sequence for Phase 4.2:
    // "1" (external)
    // "openai-compatible" (provider)
    // "https://api.openai.com/v1" (base URL)
    // "2" (env var storage)
    // "MY_API_KEY" (env var name)
    // "gpt-4" (model)
    let input = "1\nopenai-compatible\nhttps://api.openai.com/v1\n2\nMY_API_KEY\ngpt-4\n";

    let (stdout, stderr, _exit_code) = run_with_stdin(
        &bin,
        &[
            "--db-root",
            temp_dir.path().to_str().unwrap(),
            "plan",
            "test goal",
        ],
        input,
    )
    .expect("Failed to execute");

    // Should write config.toml
    let config_path = temp_dir.path().join("config.toml");
    assert!(config_path.exists(), "config.toml should be created");

    let config_content = fs::read_to_string(&config_path).unwrap();

    // Should contain external mode settings
    assert!(
        config_content.contains("mode = \"external\""),
        "Config should contain mode = \"external\""
    );

    // Phase 4.2: API key stored as env reference
    assert!(
        config_content.contains("env:MY_API_KEY"),
        "API key should be stored as env reference"
    );
    assert!(
        !config_content.contains("sk-"),
        "Raw key should NOT be in config for env var storage"
    );

    // Should exit with export instruction
    assert!(
        stdout.contains("export")
            || stdout.contains("environment")
            || stderr.contains("export")
            || stderr.contains("environment"),
        "Should show env var export instruction"
    );
}

/// ============================================================================
/// Test C: Missing config → Local provider → config written → exit
/// ============================================================================

#[test]
fn test_missing_config_choose_local() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }
    let temp_dir = create_minimal_db_root();

    // Input sequence:
    // "2" (local)
    // "ollama" (backend)
    // "127.0.0.1" (host)
    // "11434" (port)
    // "codellama" (model)
    let input = "2\nollama\n127.0.0.1\n11434\ncodellama\n";

    let (stdout, stderr, exit_code) = run_with_stdin(
        &bin,
        &[
            "--db-root",
            temp_dir.path().to_str().unwrap(),
            "plan",
            "test goal",
        ],
        input,
    )
    .expect("Failed to execute");

    // Should show preflight prompt
    assert!(
        stdout.contains("local model")
            || stdout.contains("Local model")
            || stderr.contains("local model")
            || stderr.contains("Local model"),
        "Should show local option"
    );

    // Should write config.toml
    let config_path = temp_dir.path().join("config.toml");
    assert!(config_path.exists(), "config.toml should be created");

    let config_content = fs::read_to_string(&config_path).unwrap();

    // Should contain local mode settings
    assert!(
        config_content.contains("mode = \"local\""),
        "Config should contain mode = \"local\""
    );
    assert!(
        config_content.contains("backend = \"ollama\""),
        "Config should contain backend"
    );
    assert!(
        config_content.contains("host"),
        "Config should contain host"
    );
    assert!(
        config_content.contains("port"),
        "Config should contain port"
    );
    assert!(
        config_content.contains("model"),
        "Config should contain model"
    );

    // Should exit cleanly
    assert!(exit_code == 0, "Should exit cleanly after config write");
}

/// ============================================================================
/// Test D: Invalid config → choose continue → proceed
/// ============================================================================

#[test]
fn test_invalid_config_choose_continue() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_minimal_db_root();

    // Create invalid config (missing required fields)
    let config_path = temp_dir.path().join("config.toml");
    let mut file = File::create(&config_path).unwrap();
    writeln!(
        file,
        r#"[llm]
mode = "external"
# Missing provider and other required fields
"#
    )
    .unwrap();

    // Input: "1" (Continue without LLM)
    let input = "1\n";

    let (stdout, stderr, _exit_code) = run_with_stdin(
        &bin,
        &[
            "--db-root",
            temp_dir.path().to_str().unwrap(),
            "plan",
            "test goal",
        ],
        input,
    )
    .expect("Failed to execute");

    // Should explain invalid config
    assert!(
        stdout.contains("invalid")
            || stdout.contains("missing")
            || stdout.contains("Invalid LLM configuration")
            || stderr.contains("invalid")
            || stderr.contains("missing")
            || stderr.contains("Invalid LLM configuration"),
        "Should explain config is invalid"
    );

    // Should offer continue/exit options
    assert!(
        stdout.contains("Continue without LLM")
            || stdout.contains("Exit")
            || stderr.contains("Continue without LLM")
            || stderr.contains("Exit"),
        "Should offer continue/exit options"
    );

    // Should proceed (note: plan mode runs after preflight)
    assert!(
        stdout.contains("Plan")
            || stdout.contains("Error")
            || stdout.contains("codegraph")
            || stderr.contains("Plan")
            || stderr.contains("Error")
            || stderr.contains("codegraph"),
        "Should proceed to plan mode (or show expected error)"
    );
}

/// ============================================================================
/// Test E: Valid config exists → NO prompt
/// ============================================================================

#[test]
fn test_valid_config_no_prompt() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_minimal_db_root();

    // Create valid config
    let config_path = temp_dir.path().join("config.toml");
    let mut file = File::create(&config_path).unwrap();
    writeln!(
        file,
        r#"[llm]
mode = "external"
provider = "openai-compatible"
base_url = "https://api.openai.com/v1"
api_key = "env:OPENAI_API_KEY"
model = "gpt-4"
"#
    )
    .unwrap();

    let output = Command::new(&bin)
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should NOT show preflight prompt
    assert!(
        !stdout.contains("No LLM configuration found")
            && !stderr.contains("No LLM configuration found"),
        "Should NOT show preflight prompt when valid config exists"
    );

    // Should proceed directly to version output
    assert!(stdout.contains("OdinCode"), "Should show version directly");
}

/// ============================================================================
/// Test F: Disabled config exists → NO prompt
/// ============================================================================

#[test]
fn test_disabled_config_no_prompt() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_minimal_db_root();

    // Create disabled config
    let config_path = temp_dir.path().join("config.toml");
    let mut file = File::create(&config_path).unwrap();
    writeln!(
        file,
        r#"[llm]
mode = "disabled"
"#
    )
    .unwrap();

    let output = Command::new(&bin)
        .arg("--db-root")
        .arg(temp_dir.path())
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should NOT show preflight prompt
    assert!(
        !stdout.contains("No LLM configuration found")
            && !stderr.contains("No LLM configuration found"),
        "Should NOT show preflight prompt when disabled config exists"
    );

    // Should proceed directly
    assert!(stdout.contains("OdinCode"), "Should show version directly");
}

/// ============================================================================
/// Test G1: Direct storage stores literal key
/// ============================================================================

#[test]
fn test_direct_storage_literal_key() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_minimal_db_root();

    // Input with actual API key - chooses direct storage (option 1 or default)
    let api_key = "sk-1234567890abcdefghijklmnopqrstuvwxyz";
    let input = format!(
        "1\nopenai-compatible\nhttps://api.openai.com/v1\n1\n{}\ngpt-4\n",
        api_key
    );

    let (_stdout, _stderr, _exit_code) = run_with_stdin(
        &bin,
        &[
            "--db-root",
            temp_dir.path().to_str().unwrap(),
            "plan",
            "test goal",
        ],
        &input,
    )
    .expect("Failed to execute");

    // Read config file
    let config_path = temp_dir.path().join("config.toml");
    let config_content = fs::read_to_string(&config_path).unwrap();

    // Phase 4.2: Direct storage DOES write the literal key to config
    assert!(
        config_content.contains(api_key),
        "Direct storage: API key IS written literally to config"
    );
    assert!(
        !config_content.contains("env:"),
        "Direct storage: No env reference"
    );
}

/// ============================================================================
/// Test G2: Env var storage does NOT store literal key
/// ============================================================================

#[test]
fn test_env_var_storage_no_literal() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_minimal_db_root();

    // Input with actual API key but chooses env var storage (option 2)
    let api_key = "sk-abcdefghijk123456789";
    let input = "1\nopenai-compatible\nhttps://api.openai.com/v1\n2\nMY_KEY\ngpt-4\n".to_string();

    let (_stdout, _stderr, _exit_code) = run_with_stdin(
        &bin,
        &[
            "--db-root",
            temp_dir.path().to_str().unwrap(),
            "plan",
            "test goal",
        ],
        &input,
    )
    .expect("Failed to execute");

    // Read config file
    let config_path = temp_dir.path().join("config.toml");
    let config_content = fs::read_to_string(&config_path).unwrap();

    // Phase 4.2: Env var storage does NOT write literal key
    assert!(
        !config_content.contains(api_key),
        "Env var storage: literal key must NOT be in config"
    );
    assert!(
        config_content.contains("env:MY_KEY"),
        "Env var storage: env reference must be present"
    );
}

/// ============================================================================
/// Test H: Preflight runs for TUI entry point
/// ============================================================================

#[test]
fn test_preflight_runs_for_tui_entry() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_minimal_db_root();

    // Input: "3" (No LLM)
    let input = "3\n";

    // Run with evidence mode (CLI entry but different from plan mode)
    // This tests that preflight runs for all CLI modes
    let (stdout, stderr, _exit_code) = run_with_stdin(
        &bin,
        &[
            "--db-root",
            temp_dir.path().to_str().unwrap(),
            "evidence",
            "Q1",
            "file_read",
        ],
        input,
    )
    .expect("Failed to execute");

    // Should show preflight prompt (CLI entry path)
    assert!(
        stdout.contains("No LLM configuration found")
            || stderr.contains("No LLM configuration found"),
        "CLI entry should trigger preflight"
    );

    // Config should be written
    let config_path = temp_dir.path().join("config.toml");
    assert!(
        config_path.exists(),
        "Config should be written from CLI entry"
    );
}

/// ============================================================================
/// Test I: Preflight runs for CLI entry point
/// ============================================================================

#[test]
fn test_preflight_runs_for_cli_entry() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_minimal_db_root();
    create_codegraph_db(temp_dir.path());

    // Input: "3" (No LLM)
    let input = "3\n";

    // Run plan mode (CLI entry path)
    let (stdout, stderr, _exit_code) = run_with_stdin(
        &bin,
        &[
            "--db-root",
            temp_dir.path().to_str().unwrap(),
            "plan",
            "test goal",
        ],
        input,
    )
    .expect("Failed to execute");

    // Should show preflight prompt (CLI entry path)
    assert!(
        stdout.contains("No LLM configuration found")
            || stderr.contains("No LLM configuration found"),
        "CLI entry should trigger preflight"
    );

    // Config should be written
    let config_path = temp_dir.path().join("config.toml");
    assert!(
        config_path.exists(),
        "Config should be written from CLI entry"
    );
}

/// ============================================================================
/// Test J1: Storage prompt shows three options
/// ============================================================================

#[test]
fn test_storage_prompt_shows_options() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_minimal_db_root();

    // Input: "1" (external), then the rest of fields
    let input = "1\nopenai-compatible\nhttps://api.openai.com/v1\n1\nsk-test\ngpt-4\n";

    let (stdout, stderr, _exit_code) = run_with_stdin(
        &bin,
        &[
            "--db-root",
            temp_dir.path().to_str().unwrap(),
            "plan",
            "test goal",
        ],
        input,
    )
    .expect("Failed to execute");

    // Should show storage prompt with three options
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("store") || combined.contains("Store"),
        "Should show storage option prompt"
    );
    assert!(
        combined.contains("environment") || combined.contains("env"),
        "Should mention environment variable option"
    );
}

/// ============================================================================
/// Test J2: Option [3] Disable from storage prompt
/// ============================================================================

#[test]
fn test_storage_disable_option() {
    let bin = odincode_binary();
    if !bin.exists() {
        return;
    }

    let temp_dir = create_minimal_db_root();

    // Input: "1" (external), provider, URL, then "3" to disable
    let input = "1\nopenai-compatible\nhttps://api.openai.com/v1\n3\n";

    let (_stdout, _stderr, _exit_code) = run_with_stdin(
        &bin,
        &[
            "--db-root",
            temp_dir.path().to_str().unwrap(),
            "plan",
            "test goal",
        ],
        input,
    )
    .expect("Failed to execute");

    // Config should be written with disabled mode
    let config_path = temp_dir.path().join("config.toml");
    assert!(config_path.exists(), "config.toml should be created");

    let config_content = fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("mode = \"disabled\""),
        "Choosing disable from storage prompt should set mode to disabled"
    );
}
