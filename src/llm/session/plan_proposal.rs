//! Plan proposal from LLM
//!
//! Functions for generating plans via LLM calls.

use crate::llm::adapters::{create_adapter_from_config, LlmAdapter};
use crate::llm::contracts::build_user_prompt;
use crate::llm::planner::parse_plan;
use crate::llm::session::errors::SessionError;
use crate::llm::types::{EvidenceSummary, Plan, SessionContext};

/// Propose plan from context and evidence
///
/// Called by UI after receiving LLM plan.
/// Returns plan in memory; does NOT execute tools.
///
/// Phase 5: Uses real HTTP adapter for LLM calls.
pub fn propose_plan(
    context: &SessionContext,
    evidence_summary: &EvidenceSummary,
) -> Result<Plan, SessionError> {
    // Create adapter from config
    let adapter =
        create_adapter_from_config(&context.db_root).map_err(|_| SessionError::LlmNotConfigured)?;

    // Build user prompt with evidence
    let prompt = build_user_prompt(
        &context.user_intent,
        context.selected_file.as_deref(),
        context.current_diagnostic.as_deref(),
        evidence_summary,
    );

    // Call adapter (non-streaming)
    let response = adapter.generate(&prompt)?;

    // Parse response as plan
    let plan = parse_plan(&response)?;

    Ok(plan)
}

/// Propose plan with streaming callback (Phase 4.4, Phase 5)
///
/// Same as `propose_plan()` but calls `on_chunk` for each streaming fragment.
/// Callback receives incremental text for UI display during planning.
///
/// Phase 5: Uses real HTTP adapter with streaming support.
///
/// # Determinism
/// Final plan is identical to non-streamed version.
/// Streaming affects UX only, not semantics.
pub fn propose_plan_streaming<F>(
    context: &SessionContext,
    evidence_summary: &EvidenceSummary,
    mut on_chunk: F,
) -> Result<Plan, SessionError>
where
    F: FnMut(&str),
{
    // Create adapter from config
    let adapter =
        create_adapter_from_config(&context.db_root).map_err(|_| SessionError::LlmNotConfigured)?;

    // Build user prompt with evidence
    let prompt = build_user_prompt(
        &context.user_intent,
        context.selected_file.as_deref(),
        context.current_diagnostic.as_deref(),
        evidence_summary,
    );

    // Call adapter with streaming
    let response = adapter.generate_streaming(&prompt, |chunk| {
        on_chunk(chunk);
    })?;

    // Parse response as plan
    let plan = parse_plan(&response)?;

    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propose_plan_requires_config() {
        let context = SessionContext {
            user_intent: "test".to_string(),
            selected_file: None,
            current_diagnostic: None,
            db_root: std::path::PathBuf::from("/nonexistent"),
        };

        let evidence = EvidenceSummary {
            q1_tool_executions: vec![],
            q2_failures: vec![],
            q3_diagnostic_executions: vec![],
            q4_file_executions: vec![],
            q5_execution_details: None,
            q6_latest_outcome: None,
            q7_recurring: vec![],
            q8_prior_fixes: vec![],
        };

        // Should fail without proper config
        let result = propose_plan(&context, &evidence);
        assert!(result.is_err());
    }

    #[test]
    fn test_propose_plan_streaming_requires_config() {
        let context = SessionContext {
            user_intent: "test".to_string(),
            selected_file: None,
            current_diagnostic: None,
            db_root: std::path::PathBuf::from("/nonexistent"),
        };

        let evidence = EvidenceSummary {
            q1_tool_executions: vec![],
            q2_failures: vec![],
            q3_diagnostic_executions: vec![],
            q4_file_executions: vec![],
            q5_execution_details: None,
            q6_latest_outcome: None,
            q7_recurring: vec![],
            q8_prior_fixes: vec![],
        };

        // Should fail without proper config
        let result = propose_plan_streaming(&context, &evidence, |_chunk| {});
        assert!(result.is_err());
    }
}
