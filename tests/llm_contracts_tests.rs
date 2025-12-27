//! LLM Contracts Tests (Phase 8.6.x)
//!
//! Integration tests for system prompts, chat prompts, tool schema, and evidence rendering.

use odincode::llm::contracts::{
    build_user_prompt, chat_system_prompt, render_evidence_summary, system_prompt, tool_schema,
};
use odincode::llm::types::EvidenceSummary;

// ============================================================================
// System Prompt Tests
// ============================================================================

#[test]
fn test_system_prompt_contains_constraints() {
    let prompt = system_prompt();
    assert!(prompt.contains("do NOT execute"));
    assert!(prompt.contains("do NOT have filesystem"));
    assert!(prompt.contains("do NOT have database"));
}

#[test]
fn test_system_prompt_has_evidence_requirements() {
    let prompt = system_prompt();
    assert!(prompt.contains("INSUFFICIENT_EVIDENCE"));
    assert!(prompt.contains("AVAILABLE EVIDENCE QUERIES"));
}

#[test]
fn test_system_prompt_has_structured_output_format() {
    let prompt = system_prompt();
    assert!(prompt.contains("OUTPUT FORMAT:"));
    assert!(prompt.contains("plan_id:"));
    assert!(prompt.contains("intent: READ | MUTATE | QUERY | EXPLAIN"));
}

// ============================================================================
// Chat Prompt Tests (Phase 8.6.y — Flow Restoration)
// ============================================================================

#[test]
fn test_chat_prompt_has_tools_section() {
    let prompt = chat_system_prompt();
    assert!(prompt.contains("AVAILABLE TOOLS"));
    assert!(prompt.contains("File Operations"));
    assert!(prompt.contains("Code Navigation"));
    assert!(prompt.contains("Refactoring"));
    assert!(prompt.contains("Diagnostics"));
}

#[test]
fn test_chat_prompt_lists_all_tools() {
    let prompt = chat_system_prompt();
    assert!(prompt.contains("file_read"));
    assert!(prompt.contains("file_write"));
    assert!(prompt.contains("file_search"));
    assert!(prompt.contains("splice_patch"));
    assert!(prompt.contains("lsp_check"));
}

#[test]
fn test_chat_prompt_has_tool_call_format() {
    let prompt = chat_system_prompt();
    assert!(prompt.contains("TOOL_CALL"));
    assert!(prompt.contains("tool: <tool_name>"));
    assert!(prompt.contains("args:"));
}

#[test]
fn test_chat_prompt_no_identity_injection() {
    let prompt = chat_system_prompt();
    // No "You are X" identity statements
    assert!(!prompt.contains("You are OdinCode"));
    assert!(!prompt.contains("You are an AI"));
}

#[test]
fn test_chat_prompt_no_policy_restrictions() {
    let prompt = chat_system_prompt();
    // No mentions of approval, restrictions, or workflow
    assert!(!prompt.contains("approve"));
    assert!(!prompt.contains("permission"));
    assert!(!prompt.contains("restricted"));
    assert!(!prompt.contains("NOT execute"));
    assert!(!prompt.contains("DO NOT"));
}

#[test]
fn test_chat_prompt_no_evidence_gating() {
    let prompt = chat_system_prompt();
    assert!(!prompt.contains("INSUFFICIENT_EVIDENCE"));
    assert!(!prompt.contains("evidence queries"));
}

#[test]
fn test_chat_prompt_encourages_exploration() {
    let prompt = chat_system_prompt();
    assert!(prompt.contains("Use tools whenever helpful"));
    assert!(prompt.contains("Explore freely"));
}

#[test]
fn test_chat_prompt_no_suggesting_vs_executing_language() {
    let prompt = chat_system_prompt();
    // No "suggest" or "request" language - just direct tool use
    assert!(!prompt.contains("suggest"));
    assert!(!prompt.contains("TOOL_REQUEST"));
}

#[test]
fn test_chat_prompt_no_ceremony() {
    let prompt = chat_system_prompt();
    // No rules about when to use tools or how to format responses
    assert!(!prompt.contains("WHEN TO"));
    assert!(!prompt.contains("RULES FOR"));
    assert!(!prompt.contains("MUST emit"));
    assert!(!prompt.contains("ONLY the"));
}

#[test]
fn test_chat_prompt_biases_tools_over_guessing() {
    let prompt = chat_system_prompt();
    // Exact anti-guess instruction
    assert!(prompt.contains("prefer using a tool instead of guessing"));
}

#[test]
fn test_chat_prompt_limits_to_one_tool_call() {
    let prompt = chat_system_prompt();
    // Exact one-TOOL_CALL instruction
    assert!(prompt.contains("Emit at most one TOOL_CALL per response"));
    assert!(prompt.contains("unless the user explicitly asks for multiple distinct operations"));
}

// ============================================================================
// Prompt Separation Tests
// ============================================================================

#[test]
fn test_system_prompt_and_chat_prompt_are_different() {
    let system = system_prompt();
    let chat = chat_system_prompt();

    assert_ne!(system, chat);
    assert!(system.contains("INSUFFICIENT_EVIDENCE"));
    assert!(!chat.contains("INSUFFICIENT_EVIDENCE"));
    assert!(!system.contains("TOOL_CALL"));
    assert!(chat.contains("TOOL_CALL"));
}

#[test]
fn test_planning_prompt_unchanged() {
    let prompt = system_prompt();

    // Planning mode still has all original constraints
    assert!(prompt.contains("CRITICAL CONSTRAINTS"));
    assert!(prompt.contains("do NOT execute code directly"));
    assert!(prompt.contains("do NOT have filesystem access"));

    // Planning mode still has evidence requirements
    assert!(prompt.contains("INSUFFICIENT_EVIDENCE"));

    // Planning mode still has structured output format
    assert!(prompt.contains("OUTPUT FORMAT:"));
}

// ============================================================================
// Tool Schema Tests
// ============================================================================

#[test]
fn test_tool_schema_is_valid_json() {
    let schema = tool_schema();
    let _: serde_json::Value = serde_json::from_str(&schema).unwrap();
}

#[test]
fn test_tool_schema_contains_all_tools() {
    let schema = tool_schema();
    assert!(schema.contains("file_read"));
    assert!(schema.contains("file_write"));
    assert!(schema.contains("file_create"));
    assert!(schema.contains("file_search"));
    assert!(schema.contains("file_glob"));
    assert!(schema.contains("splice_patch"));
    assert!(schema.contains("splice_plan"));
    assert!(schema.contains("symbols_in_file"));
    assert!(schema.contains("references_to_symbol_name"));
    assert!(schema.contains("references_from_file_to_symbol_name"));
    assert!(schema.contains("lsp_check"));
}

// ============================================================================
// Evidence Rendering Tests
// ============================================================================

#[test]
fn test_render_evidence_summary_deterministic() {
    let summary = EvidenceSummary {
        q1_tool_executions: vec![],
        q2_failures: vec![],
        q3_diagnostic_executions: vec![],
        q4_file_executions: vec![],
        q5_execution_details: None,
        q6_latest_outcome: None,
        q7_recurring: vec![],
        q8_prior_fixes: vec![],
    };

    let r1 = render_evidence_summary(&summary);
    let r2 = render_evidence_summary(&summary);
    assert_eq!(r1, r2);
}

#[test]
fn test_render_evidence_summary_with_q1_data() {
    let summary = EvidenceSummary {
        q1_tool_executions: vec![("file_read".to_string(), 5, 5, 0)],
        q2_failures: vec![],
        q3_diagnostic_executions: vec![],
        q4_file_executions: vec![],
        q5_execution_details: None,
        q6_latest_outcome: None,
        q7_recurring: vec![],
        q8_prior_fixes: vec![],
    };

    let result = render_evidence_summary(&summary);
    assert!(result.contains("Q1 (tool_executions):"));
    assert!(result.contains("file_read"));
    assert!(result.contains("5 total, 5 success, 0 failure"));
}

#[test]
fn test_render_evidence_summary_empty() {
    let summary = EvidenceSummary {
        q1_tool_executions: vec![],
        q2_failures: vec![],
        q3_diagnostic_executions: vec![],
        q4_file_executions: vec![],
        q5_execution_details: None,
        q6_latest_outcome: None,
        q7_recurring: vec![],
        q8_prior_fixes: vec![],
    };

    let result = render_evidence_summary(&summary);
    assert!(result.contains("(no evidence available)"));
}

// ============================================================================
// Build User Prompt Tests
// ============================================================================

#[test]
fn test_build_user_prompt_includes_evidence() {
    let summary = EvidenceSummary {
        q1_tool_executions: vec![("file_read".to_string(), 5, 5, 0)],
        q2_failures: vec![],
        q3_diagnostic_executions: vec![],
        q4_file_executions: vec![],
        q5_execution_details: None,
        q6_latest_outcome: None,
        q7_recurring: vec![],
        q8_prior_fixes: vec![],
    };

    let prompt = build_user_prompt("Read src/lib.rs", Some("src/lib.rs"), None, &summary);

    assert!(prompt.contains("User Request"));
    assert!(prompt.contains("Current file: src/lib.rs"));
    assert!(prompt.contains("Q1 (tool_executions)"));
}

#[test]
fn test_build_user_prompt_with_diagnostic() {
    let summary = EvidenceSummary {
        q1_tool_executions: vec![],
        q2_failures: vec![],
        q3_diagnostic_executions: vec![],
        q4_file_executions: vec![],
        q5_execution_details: None,
        q6_latest_outcome: None,
        q7_recurring: vec![],
        q8_prior_fixes: vec![],
    };

    let prompt = build_user_prompt(
        "Fix error",
        Some("src/main.rs"),
        Some("E0382: use of moved value"),
        &summary,
    );

    assert!(prompt.contains("Current diagnostic: E0382"));
    assert!(prompt.contains("Current file: src/main.rs"));
}

#[test]
fn test_public_api_exports() {
    // Verify all public functions are accessible
    let _s = system_prompt();
    let _c = chat_system_prompt();
    let _t = tool_schema();

    let summary = EvidenceSummary {
        q1_tool_executions: vec![],
        q2_failures: vec![],
        q3_diagnostic_executions: vec![],
        q4_file_executions: vec![],
        q5_execution_details: None,
        q6_latest_outcome: None,
        q7_recurring: vec![],
        q8_prior_fixes: vec![],
    };
    let _r = render_evidence_summary(&summary);
    let _b = build_user_prompt("test", None, None, &summary);

    // Suppress unused warnings
    let _ = (_s, _c, _t, _r, _b);
}
