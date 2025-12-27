//! splice_plan: Execute multi-step refactoring plan
//!
//! Wraps `splice plan` command.

use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

use super::SpliceResult;

/// Arguments for splice plan command
#[derive(Debug, Clone)]
pub struct PlanArgs {
    /// Path to the plan.json file
    pub file: PathBuf,
}

/// Errors that can occur during splice plan operations
#[derive(Error, Debug)]
pub enum SplicePlanError {
    #[error("splice binary not found in PATH")]
    SpliceNotFound,

    #[error("Failed to execute splice: {0}")]
    ExecutionFailed(String),
}

/// Result type for splice operations
pub type Result<T> = std::result::Result<T, SplicePlanError>;

/// Execute a multi-step refactoring plan using splice
///
/// Wraps `splice plan --file <FILE>`
///
/// # Arguments
/// * `args` - Plan arguments
///
/// # Returns
/// * `Ok(SpliceResult)` - Structured result with exit code, stdout, stderr
/// * `Err(SplicePlanError)` - Error if splice not found or execution fails
///
/// # Examples
/// ```ignore
/// use odincode::splice_tools::{splice_plan, PlanArgs};
/// use std::path::PathBuf;
///
/// let args = PlanArgs {
///     file: PathBuf::from("plan.json"),
/// };
/// let result = splice_plan(&args)?;
/// ```
pub fn splice_plan(args: &PlanArgs) -> Result<SpliceResult> {
    // Build splice plan command
    let mut cmd = Command::new("splice");
    cmd.arg("plan");
    cmd.arg("--file").arg(&args.file);

    // Execute and capture output
    let output = cmd
        .output()
        .map_err(|e| SplicePlanError::ExecutionFailed(e.to_string()))?;

    // Extract exit code
    let exit_code = output.status.code().unwrap_or(1);

    // Capture stdout/stderr
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // changed_files: empty for plan (can't parse deterministically from stdout)
    let changed_files = Vec::new();

    Ok(SpliceResult {
        exit_code,
        stdout,
        stderr,
        changed_files,
        success: exit_code == 0,
    })
}
