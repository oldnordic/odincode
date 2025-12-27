//! Execution engine errors

use std::io;

/// Execution engine errors
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Invalid plan: {0}")]
    InvalidPlan(String),

    #[error("Plan not authorized: {0}")]
    NotAuthorized(String),

    #[error("Plan ID mismatch: plan='{plan}', auth='{auth}'")]
    PlanIdMismatch { plan: String, auth: String },

    #[error("Precondition failed for step '{step}': {precondition} - {reason}")]
    PreconditionFailed {
        step: String,
        precondition: String,
        reason: String,
    },

    #[error("Tool not found in whitelist: '{0}'")]
    ToolNotFound(String),

    #[error("Missing required argument '{argument}' for tool '{tool}'")]
    MissingArgument { tool: String, argument: String },

    #[error("Tool execution failed: {tool} - {error}")]
    ToolExecutionFailed { tool: String, error: String },

    #[error("Execution DB error: {0}")]
    ExecutionDbError(#[from] crate::execution_tools::Error),

    #[error("Recording error: {0}")]
    RecordingError(String),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Confirmation denied by user for step '{0}'")]
    ConfirmationDenied(String),

    #[error("Grounding required: {reason}")]
    GroundingRequired {
        tool: String,
        reason: String,
        required_query: String,
    },
}
