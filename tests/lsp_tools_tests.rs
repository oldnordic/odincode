// Integration tests for LSP tools
// Tests use REAL cargo check --message-format=json — no mocks

use odincode::lsp_tools::lsp_check;
use std::fs;
use tempfile::TempDir;

// Helper: skip tests if cargo is unavailable
fn check_cargo_available() -> bool {
    use std::process::Command;
    Command::new("cargo")
        .arg("--version")
        .output()
        .map(|_| true)
        .unwrap_or(false)
}

#[test]
fn test_lsp_check_valid_project() {
    if !check_cargo_available() {
        eprintln!("SKIP: cargo not found in PATH");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src");

    // Create valid Rust file
    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn valid_function() -> i32 {
    1 + 1
}
"#,
    )
    .expect("Failed to write lib.rs");

    // Create Cargo.toml
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        r#"
[package]
name = "test-lsp"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // Run lsp_check
    let diagnostics = lsp_check(temp_dir.path()).expect("lsp_check failed");

    // Valid project should have no errors
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.level == "error").collect();
    assert!(errors.is_empty(), "Valid project should have no errors");

    eprintln!("Valid project: {} diagnostics", diagnostics.len());
}

#[test]
fn test_lsp_check_invalid_project() {
    if !check_cargo_available() {
        eprintln!("SKIP: cargo not found in PATH");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src");

    // Create Rust file with error
    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn invalid_function() -> i32 {
    undefined_variable
}
"#,
    )
    .expect("Failed to write lib.rs");

    // Create Cargo.toml
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        r#"
[package]
name = "test-lsp"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // Run lsp_check
    let diagnostics = lsp_check(temp_dir.path()).expect("lsp_check failed");

    // Invalid project should have errors
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.level == "error").collect();
    assert!(!errors.is_empty(), "Invalid project should have errors");

    // Check first error has expected fields
    let first_error = &errors[0];
    assert!(!first_error.message.is_empty(), "Error should have message");
    assert!(
        first_error.file_name.contains("lib.rs"),
        "Should point to lib.rs"
    );
    assert!(first_error.line_start > 0, "Should have line number");

    eprintln!("Invalid project: {} errors", errors.len());
    for err in &errors {
        eprintln!("  - Line {}: {}", err.line_start, err.message);
    }
}

#[test]
fn test_lsp_check_parse_diagnostic_output() {
    if !check_cargo_available() {
        eprintln!("SKIP: cargo not found in PATH");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src");

    // Create Rust file with multiple errors
    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn foo() -> i32 {
    let x = undefined_var;
    let y = another_undefined;
    1
}
"#,
    )
    .expect("Failed to write lib.rs");

    // Create Cargo.toml
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        r#"
[package]
name = "test-lsp"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // Run lsp_check
    let diagnostics = lsp_check(temp_dir.path()).expect("lsp_check failed");

    // Should parse multiple diagnostics
    assert!(!diagnostics.is_empty(), "Should have diagnostics");

    // Check all have required fields
    for diag in &diagnostics {
        assert!(!diag.message.is_empty(), "Each diagnostic needs message");
        assert!(!diag.level.is_empty(), "Each diagnostic needs level");
        assert!(
            !diag.file_name.is_empty(),
            "Each diagnostic needs file_name"
        );
    }

    eprintln!("Parsed {} diagnostics", diagnostics.len());
}

#[test]
fn test_lsp_check_deterministic_ordering() {
    if !check_cargo_available() {
        eprintln!("SKIP: cargo not found in PATH");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src");

    // Create Rust file with errors
    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn zeta() -> i32 { 3 }

pub fn alpha() -> i32 { undefined_alpha }

pub fn beta() -> i32 { undefined_beta }
"#,
    )
    .expect("Failed to write lib.rs");

    // Create Cargo.toml
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        r#"
[package]
name = "test-lsp"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // Run lsp_check twice
    let diagnostics1 = lsp_check(temp_dir.path()).expect("lsp_check failed");
    let diagnostics2 = lsp_check(temp_dir.path()).expect("lsp_check failed");

    // Assert identical results (deterministic)
    assert_eq!(
        diagnostics1.len(),
        diagnostics2.len(),
        "Same number of diagnostics"
    );

    for (d1, d2) in diagnostics1.iter().zip(diagnostics2.iter()) {
        assert_eq!(d1.message, d2.message, "Same message");
        assert_eq!(d1.line_start, d2.line_start, "Same line");
        assert_eq!(d1.code, d2.code, "Same error code");
    }

    eprintln!("Deterministic: {} diagnostics", diagnostics1.len());
}
