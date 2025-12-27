//! Session errors
//!
//! Error types for LLM session operations.

use crate::execution_tools::Error;
use crate::llm::adapters::transport::AdapterError;
use crate::llm::planner::PlanError;

/// Session errors
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Plan error: {0}")]
    PlanError(#[from] PlanError),

    #[error("Execution DB error: {0}")]
    ExecutionDbError(#[from] Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Execution recording error: {0}")]
    ExecutionRecordingError(String),

    #[error("Adapter error: {0}")]
    AdapterError(#[from] AdapterError),

    #[error("LLM not configured")]
    LlmNotConfigured,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_error_display() {
        let err = SessionError::LlmNotConfigured;
        assert!(err.to_string().contains("not configured"));
    }

    #[test]
    fn test_session_error_from_plan_error() {
        let plan_err = PlanError::InvalidIntent("test".to_string());
        let session_err: SessionError = plan_err.into();
        assert!(matches!(session_err, SessionError::PlanError(_)));
    }

    #[test]
    fn test_session_error_from_serialization() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let session_err: SessionError = json_err.into();
        assert!(matches!(session_err, SessionError::SerializationError(_)));
    }

    #[test]
    fn test_session_error_recording() {
        let err = SessionError::ExecutionRecordingError("test error".to_string());
        assert!(err.to_string().contains("test error"));
    }
}
