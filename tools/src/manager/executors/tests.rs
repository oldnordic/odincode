//! Tests for the tool executors

use super::*;
use std::collections::HashMap;
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_execute_command_success() {
    // Test a simple command that should succeed
    let result =
        SubprocessExecutor::execute_command("echo", &["hello"], None, None, Some(5000)).await;

    assert!(result.is_ok());
    let execution_result = result.unwrap();
    assert!(execution_result.success);
    assert_eq!(execution_result.exit_code, Some(0));
    assert!(execution_result.stdout.contains("hello"));
    assert!(execution_result.stderr.is_empty());
}

#[tokio::test]
async fn test_execute_command_failure() {
    // Test a command that should fail
    let result = SubprocessExecutor::execute_command("false", &[], None, None, Some(5000)).await;

    assert!(result.is_ok());
    let execution_result = result.unwrap();
    assert!(!execution_result.success);
    assert_eq!(execution_result.exit_code, Some(1));
}

#[tokio::test]
async fn test_execute_command_timeout() {
    // Test a command that should timeout
    let result =
        SubprocessExecutor::execute_command("sleep", &["10"], None, None, Some(1000)).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timed out"));
}

#[tokio::test]
async fn test_command_exists() {
    // Test checking if a command exists
    assert!(SubprocessExecutor::command_exists("echo").await);
    assert!(!SubprocessExecutor::command_exists("nonexistent_command_12345").await);
}

#[tokio::test]
async fn test_execute_linter() {
    // Create a temporary file to lint
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "fn main() {{\n    println!(\"Hello\");\n}}").unwrap();
    let file_path = temp_file.path();

    // Test with a simple linter-like command (using cat as a mock linter)
    let result = SubprocessExecutor::execute_linter("cat", &[], file_path, None).await;

    assert!(result.is_ok());
    let execution_result = result.unwrap();
    assert!(execution_result.success);
    assert!(execution_result.stdout.contains("println"));
}

#[tokio::test]
async fn test_execute_formatter() {
    // Create a temporary file to format
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "fn main(){{println!(\"Hello\");}}").unwrap();
    let file_path = temp_file.path();

    // Test with a simple formatter-like command (using cat as a mock formatter)
    let result = SubprocessExecutor::execute_formatter("cat", &[], file_path, None).await;

    assert!(result.is_ok());
    let execution_result = result.unwrap();
    assert!(execution_result.success);
}

#[tokio::test]
async fn test_execute_test_runner() {
    // Test with a simple test runner command
    let result = SubprocessExecutor::execute_test_runner("echo", &["test"], None).await;

    assert!(result.is_ok());
    let execution_result = result.unwrap();
    assert!(execution_result.success);
    assert!(execution_result.stdout.contains("test"));
}

#[tokio::test]
async fn test_execute_build_system() {
    // Test with a simple build command
    let result = SubprocessExecutor::execute_build_system("echo", &["build"], None).await;

    assert!(result.is_ok());
    let execution_result = result.unwrap();
    assert!(execution_result.success);
    assert!(execution_result.stdout.contains("build"));
}

#[tokio::test]
async fn test_execute_version_control() {
    // Test with a simple version control command
    let result = SubprocessExecutor::execute_version_control("echo", &["status"], None).await;

    assert!(result.is_ok());
    let execution_result = result.unwrap();
    assert!(execution_result.success);
    assert!(execution_result.stdout.contains("status"));
}

#[tokio::test]
async fn test_execute_package_manager() {
    // Test with a simple package manager command
    let result = SubprocessExecutor::execute_package_manager("echo", &["install"], None).await;

    assert!(result.is_ok());
    let execution_result = result.unwrap();
    assert!(execution_result.success);
    assert!(execution_result.stdout.contains("install"));
}
