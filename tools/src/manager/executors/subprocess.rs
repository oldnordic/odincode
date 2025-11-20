//! Subprocess Execution Module
//!
//! This module provides real subprocess execution for external tools.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Execution result from a subprocess
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Whether the execution was successful
    pub success: bool,
    /// Exit code of the process
    pub exit_code: Option<i32>,
    /// Standard output content
    pub stdout: String,
    /// Standard error content
    pub stderr: String,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

/// Subprocess executor for running external tools
pub struct SubprocessExecutor;

impl SubprocessExecutor {
    /// Execute a command with arguments
    pub async fn execute_command(
        command: &str,
        args: &[&str],
        working_dir: Option<&Path>,
        env_vars: Option<&HashMap<String, String>>,
        timeout_ms: Option<u64>,
    ) -> Result<ExecutionResult> {
        let start_time = std::time::Instant::now();

        debug!("Executing command: {} with args: {:?}", command, args);

        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Set working directory if provided
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        // Set environment variables if provided
        if let Some(env) = env_vars {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        // Execute the command and capture output
        let output = if let Some(timeout) = timeout_ms {
            // First spawn the process
            let spawned = cmd.spawn().context("Failed to spawn command")?;

            // Then wait for it with timeout
            tokio::time::timeout(
                std::time::Duration::from_millis(timeout),
                spawned.wait_with_output(),
            )
            .await
            .context("Command execution timed out")?
            .context("Failed to wait for command")?
        } else {
            cmd.spawn()
                .context("Failed to spawn command")?
                .wait_with_output()
                .await
                .context("Failed to wait for command")?
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let success = output.status.success();
        let exit_code = output.status.code();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if success {
            info!(
                "Command '{}' completed successfully in {}ms",
                command, duration_ms
            );
        } else {
            warn!(
                "Command '{}' failed with exit code {:?} in {}ms",
                command, exit_code, duration_ms
            );

            if !stderr.is_empty() {
                error!("Command stderr: {}", stderr);
            }
        }

        let result = ExecutionResult {
            success,
            exit_code,
            stdout,
            stderr,
            duration_ms,
        };

        Ok(result)
    }

    /// Check if a command is available in the system
    pub async fn command_exists(command: &str) -> bool {
        let result = Self::execute_command(command, &["--version"], None, None, Some(5000)).await;
        result.is_ok()
    }

    /// Execute a linter command and parse the output
    pub async fn execute_linter(
        command: &str,
        args: &[&str],
        file_path: &Path,
        working_dir: Option<&Path>,
    ) -> Result<ExecutionResult> {
        let mut full_args = args.to_vec();
        full_args.push(file_path.to_str().context("Invalid file path")?);

        Self::execute_command(command, &full_args, working_dir, None, Some(30000)).await
    }

    /// Execute a formatter command and apply the formatting
    pub async fn execute_formatter(
        command: &str,
        args: &[&str],
        file_path: &Path,
        working_dir: Option<&Path>,
    ) -> Result<ExecutionResult> {
        let mut full_args = args.to_vec();
        full_args.push(file_path.to_str().context("Invalid file path")?);

        Self::execute_command(command, &full_args, working_dir, None, Some(15000)).await
    }

    /// Execute a test runner command
    pub async fn execute_test_runner(
        command: &str,
        args: &[&str],
        working_dir: Option<&Path>,
    ) -> Result<ExecutionResult> {
        Self::execute_command(command, args, working_dir, None, Some(120000)).await
    }

    /// Execute a build system command
    pub async fn execute_build_system(
        command: &str,
        args: &[&str],
        working_dir: Option<&Path>,
    ) -> Result<ExecutionResult> {
        Self::execute_command(command, args, working_dir, None, Some(300000)).await
    }

    /// Execute a version control command
    pub async fn execute_version_control(
        command: &str,
        args: &[&str],
        working_dir: Option<&Path>,
    ) -> Result<ExecutionResult> {
        Self::execute_command(command, args, working_dir, None, Some(60000)).await
    }

    /// Execute a package manager command
    pub async fn execute_package_manager(
        command: &str,
        args: &[&str],
        working_dir: Option<&Path>,
    ) -> Result<ExecutionResult> {
        Self::execute_command(command, args, working_dir, None, Some(180000)).await
    }
}
