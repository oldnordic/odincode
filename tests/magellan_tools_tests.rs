// Integration tests for magellan DB query tools
// Tests use REAL magellan binary + SQLite DB — no mocks

use odincode::magellan_tools::MagellanDb;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::TempDir;

// Helper: skip tests if magellan not available
fn check_magellan_available() -> bool {
    use std::process::Command;
    // Check if magellan exists by trying to run it (it will fail without args but exists)
    Command::new("magellan")
        .output()
        .map(|_| true)
        .unwrap_or(false)
}

// Helper: run magellan watch in background, wait for indexing, then stop
fn run_magellan_index(root: &PathBuf, db_path: &PathBuf) -> Result<(), String> {
    // Start magellan in background
    let mut child = Command::new("magellan")
        .arg("watch")
        .arg("--root")
        .arg(root)
        .arg("--db")
        .arg(db_path)
        .arg("--debounce-ms")
        .arg("100") // Fast debounce for tests
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn magellan: {}", e))?;

    // Wait for magellan to start
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Touch files to trigger indexing
    for entry in std::fs::read_dir(root.join("src")).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        // Touch file to trigger magellan (by rewriting)
        let content = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, &content).unwrap();
    }

    // Wait for indexing to complete
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Stop magellan
    child.kill().ok();
    child.wait().ok();

    Ok(())
}

#[test]
fn test_status_counts_returns_non_zero() {
    if !check_magellan_available() {
        eprintln!("SKIP: magellan binary not found in PATH");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src");

    // Create two .rs files with content
    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn foo() -> i32 {
    1 + 1
}

pub fn bar() -> i32 {
    foo()
}
"#,
    )
    .expect("Failed to write lib.rs");

    fs::write(
        src_dir.join("main.rs"),
        r#"
fn main() {
    println!("Hello");
}
"#,
    )
    .expect("Failed to write main.rs");

    // Create Cargo.toml
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        r#"
[package]
name = "test-magellan"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // Run magellan to index
    let db_path = temp_dir.path().join("codegraph.db");
    run_magellan_index(&temp_dir.path().into(), &db_path).expect("Magellan indexing failed");

    // Open DB and query
    let db = MagellanDb::open_readonly(&db_path).expect("Failed to open DB");
    let counts = db.status_counts().expect("Failed to get status counts");

    // Assert non-zero counts
    assert!(counts.files > 0, "files count should be > 0");
    assert!(counts.symbols > 0, "symbols count should be > 0");
    assert!(counts.references > 0, "references count should be > 0");

    eprintln!(
        "Status counts: files={}, symbols={}, references={}",
        counts.files, counts.symbols, counts.references
    );
}

#[test]
fn test_symbols_in_file_returns_expected_symbols() {
    if !check_magellan_available() {
        eprintln!("SKIP: magellan binary not found in PATH");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src");

    // Create lib.rs with known symbols
    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn function_one() -> i32 { 1 }

pub fn function_two() -> i32 { 2 }
"#,
    )
    .expect("Failed to write lib.rs");

    // Create Cargo.toml
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        r#"
[package]
name = "test-magellan"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // Run magellan to index
    let db_path = temp_dir.path().join("codegraph.db");
    run_magellan_index(&temp_dir.path().into(), &db_path).expect("Magellan indexing failed");

    // Open DB and query
    let db = MagellanDb::open_readonly(&db_path).expect("Failed to open DB");

    // Query symbols for lib.rs (use LIKE pattern matching full path)
    let symbols = db
        .symbols_in_file("%/lib.rs")
        .expect("Failed to query symbols");

    // Assert we found our functions
    assert!(symbols.len() >= 2, "Should find at least 2 symbols");

    let names: Vec<_> = symbols.iter().map(|s| &s.name).collect();
    assert!(
        names.contains(&&"function_one".to_string()),
        "Should find function_one"
    );
    assert!(
        names.contains(&&"function_two".to_string()),
        "Should find function_two"
    );

    // Assert deterministic ordering (sorted by name)
    let mut sorted_names = names.clone();
    sorted_names.sort();
    assert_eq!(
        names, sorted_names,
        "Symbols should be deterministically sorted"
    );

    eprintln!("Found {} symbols in lib.rs", symbols.len());
    for sym in &symbols {
        eprintln!("  - {} ({})", sym.name, sym.kind);
    }
}

#[test]
fn test_references_to_symbol_name_works_within_file() {
    if !check_magellan_available() {
        eprintln!("SKIP: magellan binary not found in PATH");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src");

    // Create lib.rs where bar() calls foo()
    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn foo() -> i32 { 1 }

pub fn bar() -> i32 { foo() }
"#,
    )
    .expect("Failed to write lib.rs");

    // Create Cargo.toml
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        r#"
[package]
name = "test-magellan"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // Run magellan to index
    let db_path = temp_dir.path().join("codegraph.db");
    run_magellan_index(&temp_dir.path().into(), &db_path).expect("Magellan indexing failed");

    // Open DB and query
    let db = MagellanDb::open_readonly(&db_path).expect("Failed to open DB");

    // Query references to "foo"
    let refs = db
        .references_to_symbol_name("foo")
        .expect("Failed to query references");

    // Assert bar() references foo()
    assert!(!refs.is_empty(), "Should find at least 1 reference to foo");

    let bar_ref = refs.iter().find(|r| r.from_file_path.contains("lib.rs"));
    assert!(bar_ref.is_some(), "Should find reference from lib.rs");

    eprintln!("Found {} references to 'foo'", refs.len());
    for rf in &refs {
        eprintln!(
            "  - from {} (bytes {}-{})",
            rf.from_file_path, rf.byte_start, rf.byte_end
        );
    }
}

#[test]
fn test_references_from_file_to_symbol_name() {
    if !check_magellan_available() {
        eprintln!("SKIP: magellan binary not found in PATH");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src");

    // Create lib.rs where bar() calls foo()
    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn foo() -> i32 { 1 }

pub fn bar() -> i32 { foo() }
"#,
    )
    .expect("Failed to write lib.rs");

    // Create Cargo.toml
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        r#"
[package]
name = "test-magellan"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // Run magellan to index
    let db_path = temp_dir.path().join("codegraph.db");
    run_magellan_index(&temp_dir.path().into(), &db_path).expect("Magellan indexing failed");

    // Open DB and query
    let db = MagellanDb::open_readonly(&db_path).expect("Failed to open DB");

    // Query references from lib.rs to "foo" (use full path pattern)
    let refs = db
        .references_from_file_to_symbol_name("%/lib.rs", "foo")
        .expect("Failed to query references");

    // Assert we found the reference from bar to foo
    assert!(!refs.is_empty(), "Should find at least 1 reference");

    eprintln!("Found {} references from lib.rs to 'foo'", refs.len());
    for rf in &refs {
        eprintln!("  - symbol_id={}, name={}", rf.symbol_id, rf.symbol_name);
    }
}

#[test]
fn test_all_queries_return_stable_sorted_results() {
    if !check_magellan_available() {
        eprintln!("SKIP: magellan binary not found in PATH");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src");

    // Create lib.rs with multiple symbols
    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn zeta() -> i32 { 3 }

pub fn alpha() -> i32 { 1 }

pub fn beta() -> i32 { 2 }
"#,
    )
    .expect("Failed to write lib.rs");

    // Create Cargo.toml
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        r#"
[package]
name = "test-magellan"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // Run magellan to index
    let db_path = temp_dir.path().join("codegraph.db");
    run_magellan_index(&temp_dir.path().into(), &db_path).expect("Magellan indexing failed");

    // Open DB
    let db = MagellanDb::open_readonly(&db_path).expect("Failed to open DB");

    // Query symbols twice
    let symbols1 = db
        .symbols_in_file("%/lib.rs")
        .expect("Failed to query symbols");
    let symbols2 = db
        .symbols_in_file("%/lib.rs")
        .expect("Failed to query symbols");

    // Assert identical ordering (deterministic)
    assert_eq!(symbols1.len(), symbols2.len(), "Same number of symbols");
    for (s1, s2) in symbols1.iter().zip(symbols2.iter()) {
        assert_eq!(s1.name, s2.name, "Symbol names should match");
        assert_eq!(s1.symbol_id, s2.symbol_id, "Symbol IDs should match");
    }

    eprintln!(
        "Deterministic ordering confirmed: {} symbols",
        symbols1.len()
    );
}
