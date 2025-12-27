//! Execution result types

use serde::{Deserialize, Serialize};

/// Execution result
///
/// Contains all step results executed (up to failure point).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub plan_id: String,
    pub status: ExecutionStatus,
    pub step_results: Vec<StepResult>,
    pub total_duration_ms: i64,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Completed, // All steps succeeded
    Failed,    // Stopped at failure
    Partial,   // Reserved for future
}

/// Single step execution result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub tool_name: String,
    pub success: bool,
    pub execution_id: String,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: i64,
    pub diagnostic_artifacts: Vec<DiagnosticArtifact>,
}

/// Diagnostic artifact (from lsp_check)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticArtifact {
    pub level: String,
    pub message: String,
    pub file_name: String,
    pub line_start: i64,
    pub code: Option<String>,
}

impl From<crate::lsp_tools::Diagnostic> for DiagnosticArtifact {
    fn from(diag: crate::lsp_tools::Diagnostic) -> Self {
        DiagnosticArtifact {
            level: diag.level,
            message: diag.message,
            file_name: diag.file_name,
            line_start: diag.line_start as i64,
            code: diag.code,
        }
    }
}
