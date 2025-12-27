// Integration tests for splice tools
// Tests use REAL splice binary — no mocks

use odincode::splice_tools::{splice_patch, splice_plan, PatchArgs, PlanArgs};
use std::fs;
use tempfile::TempDir;

// Helper: skip tests if splice not available
fn skip_if_no_splice() {
    use std::process::Command;
    if Command::new("splice").arg("--help").output().is_err() {
        eprintln!("SKIP: splice binary not found in PATH");
        std::process::exit(0);
    }
}

#[test]
fn test_patch_success_simple_function() {
    skip_if_no_splice();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a minimal Cargo project structure
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    let source_file = src_dir.join("lib.rs");

    // Create a valid Rust file with a function
    let source_content = r#"
fn foo() -> i32 {
    1 + 1
}
"#;
    fs::write(&source_file, source_content).expect("Failed to write source file");

    // Create Cargo.toml
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-splice"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // Create replacement file
    let replacement_file = temp_dir.path().join("replacement.txt");
    let replacement_content = r#"
fn foo() -> i32 {
    2 + 2
}
"#;
    fs::write(&replacement_file, replacement_content).expect("Failed to write replacement");

    // Build args
    let args = PatchArgs {
        file: source_file.clone(),
        symbol: "foo".to_string(),
        kind: Some("function".to_string()),
        with: replacement_file,
        analyzer: None, // No LSP validation for simplicity
    };

    // Execute splice patch
    let result = splice_patch(&args).expect("splice_patch should not return Err");

    // Assert success
    assert_eq!(result.exit_code, 0, "exit_code should be 0 on success");
    assert!(result.success, "success should be true");
    assert!(!result.stdout.is_empty(), "stdout should not be empty");
    assert!(
        result.changed_files.contains(&source_file),
        "source file should be in changed_files"
    );

    // Verify file was actually changed
    let new_content = fs::read_to_string(&source_file).expect("Failed to read modified file");
    assert!(
        new_content.contains("2 + 2"),
        "file should contain new content"
    );
    assert!(
        !new_content.contains("1 + 1"),
        "file should not contain old content"
    );
}

#[test]
fn test_patch_failure_invalid_symbol() {
    skip_if_no_splice();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a minimal Cargo project structure
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    let source_file = src_dir.join("lib.rs");

    // Create a valid Rust file
    let source_content = r#"
fn foo() -> i32 {
    1 + 1
}
"#;
    fs::write(&source_file, source_content).expect("Failed to write source file");

    // Create Cargo.toml
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-splice"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // Create replacement file
    let replacement_file = temp_dir.path().join("replacement.txt");
    let replacement_content = r#"
fn nonexistent() -> i32 {
    0
}
"#;
    fs::write(&replacement_file, replacement_content).expect("Failed to write replacement");

    // Build args with non-existent symbol
    let args = PatchArgs {
        file: source_file,
        symbol: "nonexistent".to_string(),
        kind: Some("function".to_string()),
        with: replacement_file,
        analyzer: None,
    };

    // Execute splice patch
    let result = splice_patch(&args).expect("splice_patch should not return Err");

    // Assert failure
    assert_ne!(
        result.exit_code, 0,
        "exit_code should be non-zero on failure"
    );
    assert!(!result.success, "success should be false");
    // stderr should contain error info
    assert!(
        !result.stderr.is_empty() || !result.stdout.is_empty(),
        "stderr or stdout should have error details"
    );
}

#[test]
fn test_plan_success_multi_step() {
    skip_if_no_splice();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a minimal Cargo project structure
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    // Create two source files
    let file1 = src_dir.join("file1.rs");
    let file2 = src_dir.join("file2.rs");

    fs::write(
        &file1,
        r#"
fn foo() -> i32 {
    1 + 1
}
"#,
    )
    .expect("Failed to write file1");

    fs::write(
        &file2,
        r#"
fn bar() -> i32 {
    2 + 2
}
"#,
    )
    .expect("Failed to write file2");

    // Create Cargo.toml
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-splice"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/file1.rs"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // Create replacement files
    let repl1 = temp_dir.path().join("repl1.txt");
    let repl2 = temp_dir.path().join("repl2.txt");

    fs::write(&repl1, "fn foo() -> i32 { 10 }").expect("Failed to write repl1");
    fs::write(&repl2, "fn bar() -> i32 { 20 }").expect("Failed to write repl2");

    // Create plan.json
    let plan_file = temp_dir.path().join("plan.json");
    let plan_json = r#"{
  "steps": [
    {
      "file": "src/file1.rs",
      "symbol": "foo",
      "kind": "function",
      "with": "repl1.txt"
    },
    {
      "file": "src/file2.rs",
      "symbol": "bar",
      "kind": "function",
      "with": "repl2.txt"
    }
  ]
}"#;
    fs::write(&plan_file, plan_json).expect("Failed to write plan");

    // Build args
    let args = PlanArgs {
        file: plan_file.clone(),
    };

    // Execute splice plan
    let result = splice_plan(&args).expect("splice_plan should not return Err");

    // Debug output on failure
    if result.exit_code != 0 {
        eprintln!("=== PLAN EXECUTION FAILED ===");
        eprintln!("exit_code: {}", result.exit_code);
        eprintln!("stdout: {}", result.stdout);
        eprintln!("stderr: {}", result.stderr);
    }

    // Assert success
    assert_eq!(result.exit_code, 0, "exit_code should be 0 on success");
    assert!(result.success, "success should be true");
    assert!(!result.stdout.is_empty(), "stdout should not be empty");

    // Verify both files were changed
    let content1 = fs::read_to_string(&file1).expect("Failed to read file1");
    let content2 = fs::read_to_string(&file2).expect("Failed to read file2");
    assert!(content1.contains("10"), "file1 should contain new value");
    assert!(content2.contains("20"), "file2 should contain new value");
}

#[test]
fn test_plan_failure_invalid_plan_file() {
    skip_if_no_splice();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let missing_plan = temp_dir.path().join("nonexistent.json");

    let args = PlanArgs { file: missing_plan };

    // Execute splice plan
    let result = splice_plan(&args).expect("splice_plan should not return Err");

    // Assert failure
    assert_ne!(
        result.exit_code, 0,
        "exit_code should be non-zero for missing plan"
    );
    assert!(!result.success, "success should be false");
    assert!(
        !result.stderr.is_empty() || !result.stdout.is_empty(),
        "should have error output"
    );
}

#[test]
fn test_passthrough_contract() {
    skip_if_no_splice();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a minimal Cargo project structure
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    let source_file = src_dir.join("lib.rs");

    // Create invalid Rust (missing closing brace)
    let source_content = "fn foo() { 1 + 1 ";
    fs::write(&source_file, source_content).expect("Failed to write source");

    // Create Cargo.toml
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-splice"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("Failed to write Cargo.toml");

    let replacement_file = temp_dir.path().join("replacement.txt");
    fs::write(&replacement_file, "fn foo() { 0 }").expect("Failed to write replacement");

    let args = PatchArgs {
        file: source_file,
        symbol: "foo".to_string(),
        kind: Some("function".to_string()),
        with: replacement_file,
        analyzer: None,
    };

    // Execute first time
    let result1 = splice_patch(&args).expect("splice_patch should not return Err");

    // Verify passthrough
    assert_ne!(result1.exit_code, 0, "should fail on invalid input");
    assert!(!result1.success, "success should be false");

    // Verify stdout/stderr captured (non-empty on failure)
    let has_output = !result1.stdout.is_empty() || !result1.stderr.is_empty();
    assert!(has_output, "should capture stdout/stderr from splice");

    // Execute again with identical args
    let result2 = splice_patch(&args).expect("splice_patch should not return Err");

    // Verify deterministic output (same exit code and output)
    assert_eq!(
        result1.exit_code, result2.exit_code,
        "exit codes should match"
    );
    assert_eq!(
        result1.stdout, result2.stdout,
        "stdout should be deterministic"
    );
    assert_eq!(
        result1.stderr, result2.stderr,
        "stderr should be deterministic"
    );
}
