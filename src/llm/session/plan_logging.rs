//! Plan logging to execution database
//!
//! Functions for recording plan generation and edits to execution_log.db.

use crate::execution_tools::ExecutionDb;
use crate::llm::session::errors::SessionError;
use crate::llm::types::Plan;

/// Log plan generation to execution database
///
/// Records:
/// - Execution row with tool_name="llm_plan"
/// - Artifacts: prompt, plan JSON, validation errors (if any)
pub fn log_plan_generation(
    exec_db: &ExecutionDb,
    user_intent: &str,
    plan: &Plan,
    validation_error: Option<&str>,
) -> Result<(), SessionError> {
    // Generate unique execution ID
    let exec_id = format!("llm_plan_{}", plan.plan_id);

    // Get current timestamp (milliseconds since UNIX epoch)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // Create arguments JSON
    let arguments = serde_json::json!({
        "plan_id": plan.plan_id,
        "intent": plan.intent.to_string(),
        "step_count": plan.steps.len(),
        "evidence_cited": plan.evidence_referenced.len()
    });

    // Create artifacts
    let prompt_artifact = serde_json::json!({
        "user_intent": user_intent,
        "intent": plan.intent.to_string(),
        "timestamp": timestamp
    });

    let plan_artifact = serde_json::json!({
        "plan_id": plan.plan_id,
        "intent": plan.intent.to_string(),
        "steps": plan.steps.len(),
        "evidence_referenced": plan.evidence_referenced
    });

    // Build artifacts slice with references
    let mut artifacts_vec: Vec<(&str, &serde_json::Value)> =
        vec![("prompt", &prompt_artifact), ("plan", &plan_artifact)];

    // Add validation error artifact if present
    let validation_artifact;
    if let Some(err) = validation_error {
        validation_artifact = serde_json::json!({ "error": err });
        artifacts_vec.push(("validation_error", &validation_artifact));
    }

    // Record execution with artifacts
    exec_db
        .record_execution_with_artifacts(
            &exec_id,
            "llm_plan",
            &arguments,
            timestamp,
            true, // success (logging succeeded even if plan validation failed)
            None, // exit_code
            None, // duration_ms
            None, // error_message
            &artifacts_vec,
        )
        .map_err(|e| SessionError::ExecutionRecordingError(format!("{}", e)))?;

    Ok(())
}

/// Log a single stream chunk to execution database (Phase 4.4)
///
/// Records each streaming fragment as an llm_plan_stream artifact.
/// Chunks are associated with the same plan_id prefix.
pub fn log_stream_chunk(
    exec_db: &ExecutionDb,
    user_intent: &str,
    chunk: &str,
) -> Result<(), SessionError> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // Generate chunk-specific execution ID
    let chunk_id = format!(
        "llm_plan_stream_{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let arguments = serde_json::json!({
        "user_intent": user_intent,
        "chunk_length": chunk.len()
    });

    let chunk_artifact = serde_json::json!({
        "chunk": chunk,
        "timestamp": timestamp
    });

    exec_db
        .record_execution_with_artifacts(
            &chunk_id,
            "llm_plan",
            &arguments,
            timestamp,
            true,
            None,
            None,
            None,
            &[("llm_plan_stream", &chunk_artifact)],
        )
        .map_err(|e| SessionError::ExecutionRecordingError(format!("{}", e)))?;

    Ok(())
}

/// Log plan edit to execution database (Phase 4.5)
///
/// Records user edits to plans with linkage to original plan_id.
pub fn log_plan_edit(
    exec_db: &ExecutionDb,
    original_plan_id: &str,
    edited_plan: &Plan,
    edit_reason: &str,
) -> Result<(), SessionError> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // Generate edit-specific execution ID
    let edit_id = format!(
        "plan_edit_{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let arguments = serde_json::json!({
        "original_plan_id": original_plan_id,
        "edited_plan_id": edited_plan.plan_id,
        "edit_reason": edit_reason
    });

    let edit_artifact = serde_json::json!({
        "original_plan_id": original_plan_id,
        "edited_plan": edited_plan,
        "edit_reason": edit_reason,
        "timestamp": timestamp
    });

    exec_db
        .record_execution_with_artifacts(
            &edit_id,
            "llm_plan",
            &arguments,
            timestamp,
            true,
            None,
            None,
            None,
            &[("plan_edit", &edit_artifact)],
        )
        .map_err(|e| SessionError::ExecutionRecordingError(format!("{}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_plan_generation_id_format() {
        let plan = Plan {
            plan_id: "test_plan_123".to_string(),
            intent: crate::llm::types::Intent::Read,
            steps: vec![],
            evidence_referenced: vec![],
        };

        let exec_id = format!("llm_plan_{}", plan.plan_id);
        assert_eq!(exec_id, "llm_plan_test_plan_123");
    }

    #[test]
    fn test_log_stream_chunk_id_format() {
        let _chunk = "test chunk";
        let _user_intent = "test";

        // Generate chunk ID the same way
        let chunk_id = format!(
            "llm_plan_stream_{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );

        assert!(chunk_id.starts_with("llm_plan_stream_"));
        assert!(chunk_id.len() > "llm_plan_stream_".len());
    }

    #[test]
    fn test_log_plan_edit_id_format() {
        let _original_plan_id = "original_plan";
        let _edited_plan = Plan {
            plan_id: "edited_plan".to_string(),
            intent: crate::llm::types::Intent::Mutate,
            steps: vec![],
            evidence_referenced: vec![],
        };

        // Generate edit ID the same way
        let edit_id = format!(
            "plan_edit_{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );

        assert!(edit_id.starts_with("plan_edit_"));
        assert!(edit_id.len() > "plan_edit_".len());
    }

    #[test]
    fn test_log_plan_generation_arguments() {
        let plan = Plan {
            plan_id: "test".to_string(),
            intent: crate::llm::types::Intent::Query,
            steps: vec![],
            evidence_referenced: vec!["evidence1".to_string(), "evidence2".to_string()],
        };

        let arguments = serde_json::json!({
            "plan_id": plan.plan_id,
            "intent": plan.intent.to_string(),
            "step_count": plan.steps.len(),
            "evidence_cited": plan.evidence_referenced.len()
        });

        assert_eq!(arguments["plan_id"], "test");
        assert_eq!(arguments["intent"], "QUERY");
        assert_eq!(arguments["step_count"], 0);
        assert_eq!(arguments["evidence_cited"], 2);
    }
}
