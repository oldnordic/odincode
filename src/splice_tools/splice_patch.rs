//! splice_patch: Apply patch to symbol's span
//!
//! Wraps `splice patch` command.

use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

use super::SpliceResult;

/// Arguments for splice patch command
#[derive(Debug, Clone)]
pub struct PatchArgs {
    /// Path to the source file containing the symbol
    pub file: PathBuf,
    /// Symbol name to patch
    pub symbol: String,
    /// Optional symbol kind filter (function, struct, enum, trait, impl)
    pub kind: Option<String>,
    /// Path to file containing replacement content
    pub with: PathBuf,
    /// Optional rust-analyzer validation mode (off, os, path)
    pub analyzer: Option<String>,
}

/// Errors that can occur during splice patch operations
#[derive(Error, Debug)]
pub enum SplicePatchError {
    #[error("splice binary not found in PATH")]
    SpliceNotFound,

    #[error("Failed to execute splice: {0}")]
    ExecutionFailed(String),
}

/// Result type for splice operations
pub type Result<T> = std::result::Result<T, SplicePatchError>;

/// Apply a patch to a symbol using splice
///
/// Wraps `splice patch --file <FILE> --symbol <SYMBOL> --with <FILE>`
///
/// # Arguments
/// * `args` - Patch arguments
///
/// # Returns
/// * `Ok(SpliceResult)` - Structured result with exit code, stdout, stderr
/// * `Err(SplicePatchError)` - Error if splice not found or execution fails
///
/// # Examples
/// ```ignore
/// use odincode::splice_tools::{splice_patch, PatchArgs};
/// use std::path::PathBuf;
///
/// let args = PatchArgs {
///     file: PathBuf::from("src/lib.rs"),
///     symbol: "foo".to_string(),
///     kind: Some("function".to_string()),
///     with: PathBuf::from("replacement.txt"),
///     analyzer: None,
/// };
/// let result = splice_patch(&args)?;
/// ```
pub fn splice_patch(args: &PatchArgs) -> Result<SpliceResult> {
    // Build splice patch command
    let mut cmd = Command::new("splice");
    cmd.arg("patch");
    cmd.arg("--file").arg(&args.file);
    cmd.arg("--symbol").arg(&args.symbol);
    cmd.arg("--with").arg(&args.with);

    // Add optional kind
    if let Some(ref kind) = args.kind {
        cmd.arg("--kind").arg(kind);
    }

    // Add optional analyzer
    if let Some(ref analyzer) = args.analyzer {
        cmd.arg("--analyzer").arg(analyzer);
    }

    // Execute and capture output
    let output = cmd
        .output()
        .map_err(|e| SplicePatchError::ExecutionFailed(e.to_string()))?;

    // Extract exit code
    let exit_code = output.status.code().unwrap_or(1);

    // Capture stdout/stderr
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Determine changed files (best-effort: if success and stdout contains "Patched", add the file)
    let mut changed_files = Vec::new();
    if exit_code == 0 && stdout.contains("Patched") {
        changed_files.push(args.file.clone());
    }

    Ok(SpliceResult {
        exit_code,
        stdout,
        stderr,
        changed_files,
        success: exit_code == 0,
    })
}
