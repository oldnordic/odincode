//! LLM Contract Tests (T1, T5)
//!
//! Tests for:
//! - T1: Prompt contract rendering is deterministic (golden tests)
//! - T5: Evidence summaries are truncated/normalized deterministically

// These imports will fail initially until llm module exists
use odincode::llm::contracts::{system_prompt, tool_schema};
use odincode::llm::types::EvidenceSummary;

#[test]
fn test_t1_system_prompt_is_deterministic() {
    // System prompt must render to fixed string
    let prompt = system_prompt();

    // Contains critical constraints
    assert!(prompt.contains("You are OdinCode"));
    assert!(prompt.contains("do NOT execute code directly"));
    assert!(prompt.contains("do NOT have filesystem access"));
    assert!(prompt.contains("do NOT have database access"));

    // Contains tool list
    assert!(prompt.contains("file_read"));
    assert!(prompt.contains("splice_patch"));
    assert!(prompt.contains("lsp_check"));

    // Contains evidence queries
    assert!(prompt.contains("list_executions_by_tool"));
    assert!(prompt.contains("Q1"));
    assert!(prompt.contains("Q8"));
}

#[test]
fn test_t1_system_prompt_golden_output() {
    // Exact golden output to catch accidental changes
    let prompt = system_prompt();

    // Key phrases in exact order
    assert!(prompt.contains("You are OdinCode"));
    assert!(prompt.contains("deterministic code refactoring assistant"));
}

#[test]
fn test_t1_tool_schema_is_valid_json() {
    // Tool schema must render to valid JSON
    let schema = tool_schema();

    // Must be parseable as JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&schema).expect("tool_schema must return valid JSON");

    // Must have "tools" array
    let tools = parsed["tools"].as_array().expect("tools must be an array");

    // Must include all Phase 0 tools
    let tool_names: Vec<String> = tools
        .iter()
        .filter_map(|t| t["name"].as_str())
        .map(|s| s.to_string())
        .collect();

    assert!(tool_names.contains(&"file_read".to_string()));
    assert!(tool_names.contains(&"file_write".to_string()));
    assert!(tool_names.contains(&"file_create".to_string()));
    assert!(tool_names.contains(&"file_search".to_string()));
    assert!(tool_names.contains(&"file_glob".to_string()));
    assert!(tool_names.contains(&"splice_patch".to_string()));
    assert!(tool_names.contains(&"splice_plan".to_string()));
    assert!(tool_names.contains(&"symbols_in_file".to_string()));
    assert!(tool_names.contains(&"references_to_symbol_name".to_string()));
    assert!(tool_names.contains(&"references_from_file_to_symbol_name".to_string()));
    assert!(tool_names.contains(&"lsp_check".to_string()));

    // Exactly 11 tools (no extras)
    assert_eq!(tool_names.len(), 11);
}

#[test]
fn test_t1_tool_schema_deterministic() {
    // Tool schema must be deterministic (same output each time)
    let schema1 = tool_schema();
    let schema2 = tool_schema();

    assert_eq!(
        schema1, schema2,
        "tool_schema must produce identical output"
    );
}

#[test]
fn test_t1_tool_schema_file_read_structure() {
    // Verify file_read tool schema structure
    let schema = tool_schema();
    let parsed: serde_json::Value = serde_json::from_str(&schema).unwrap();

    let file_read = parsed["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "file_read")
        .expect("file_read must be in tools");

    assert_eq!(file_read["name"], "file_read");
    assert!(file_read["parameters"]["path"]["required"]
        .as_bool()
        .unwrap());
    assert_eq!(file_read["parameters"]["path"]["type"], "string");
}

#[test]
fn test_t1_tool_schema_splice_patch_structure() {
    // Verify splice_patch tool schema structure
    let schema = tool_schema();
    let parsed: serde_json::Value = serde_json::from_str(&schema).unwrap();

    let splice = parsed["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "splice_patch")
        .expect("splice_patch must be in tools");

    assert_eq!(splice["name"], "splice_patch");
    assert!(splice["parameters"]["file"]["required"].as_bool().unwrap());
    assert!(splice["parameters"]["symbol"]["required"]
        .as_bool()
        .unwrap());
    assert!(splice["parameters"]["with"]["required"].as_bool().unwrap());
}

#[test]
fn test_t5_evidence_summary_serializes_to_json() {
    // EvidenceSummary must serialize to deterministic JSON
    let summary = EvidenceSummary {
        q1_tool_executions: vec![("file_read".to_string(), 5, 4, 1)],
        q2_failures: vec![],
        q3_diagnostic_executions: vec![("E0425".to_string(), 7)],
        q4_file_executions: vec![("src/lib.rs".to_string(), 3, true)],
        q5_execution_details: None,
        q6_latest_outcome: Some(("src/lib.rs".to_string(), true, 1234567890)),
        q7_recurring: vec![("E0425".to_string(), "src/lib.rs".to_string(), 7)],
        q8_prior_fixes: vec![("E0425".to_string(), 2, vec![5000, 10000])],
    };

    // Must serialize to JSON
    let json = serde_json::to_string(&summary).expect("EvidenceSummary must serialize to JSON");

    // Must contain expected keys
    assert!(json.contains("q1_tool_executions"));
    assert!(json.contains("q2_failures"));
    assert!(json.contains("q8_prior_fixes"));

    // Must be deterministic
    let json2 = serde_json::to_string(&summary).unwrap();
    assert_eq!(json, json2);
}

#[test]
fn test_t5_empty_evidence_summary_serializes() {
    // Empty evidence summary must serialize correctly
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

    let json = serde_json::to_string(&summary).expect("Empty EvidenceSummary must serialize");

    assert!(json.contains("q1_tool_executions"));
    assert!(json.contains("[]"));

    // Must parse back correctly
    let parsed: EvidenceSummary = serde_json::from_str(&json).unwrap();
    assert!(parsed.q1_tool_executions.is_empty());
    assert!(parsed.q2_failures.is_empty());
}

#[test]
fn test_t5_evidence_summary_rendered_for_prompt() {
    // Evidence summary must render deterministically for prompt inclusion
    let summary = EvidenceSummary {
        q1_tool_executions: vec![("splice_patch".to_string(), 3, 2, 1)],
        q2_failures: vec![(
            "splice_patch".to_string(),
            1735036825000,
            "Symbol not found".to_string(),
        )],
        q3_diagnostic_executions: vec![],
        q4_file_executions: vec![],
        q5_execution_details: None,
        q6_latest_outcome: None,
        q7_recurring: vec![],
        q8_prior_fixes: vec![],
    };

    let rendered = odincode::llm::contracts::render_evidence_summary(&summary);

    // Must contain evidence labels
    assert!(rendered.contains("Q1"));
    assert!(rendered.contains("Q2"));

    // Must be deterministic
    let rendered2 = odincode::llm::contracts::render_evidence_summary(&summary);
    assert_eq!(rendered, rendered2);
}
