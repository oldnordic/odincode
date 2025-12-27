//! Splice tools: deterministic wrapper around splice binary
//!
//! Wraps the splice CLI tool for span-safe refactoring.
//! All operations use the REAL splice binary — no mocks.

mod splice_patch;
mod splice_plan;

// Re-export public API
pub use splice_patch::{splice_patch, PatchArgs};
pub use splice_plan::{splice_plan, PlanArgs};

// Shared result type
use std::path::PathBuf;

/// Result from a splice invocation
#[derive(Debug, Clone, PartialEq)]
pub struct SpliceResult {
    /// Exit code from splice process
    pub exit_code: i32,
    /// Standard output captured
    pub stdout: String,
    /// Standard error captured
    pub stderr: String,
    /// Files that were changed (best-effort detection)
    pub changed_files: Vec<PathBuf>,
    /// Whether the operation succeeded (exit_code == 0)
    pub success: bool,
}
