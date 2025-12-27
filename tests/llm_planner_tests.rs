//! LLM Planner Tests (T2, T3, T4)
//!
//! Tests for:
//! - T2: Plan parsing accepts valid plans, rejects invalid ones with stable errors
//! - T3: Router maps intents to allowed tool calls only
//! - T4: Preconditions enforced

use std::collections::HashMap;

use odincode::llm::planner::{parse_plan, validate_plan};
use odincode::llm::router::{tool_is_allowed, ToolRouter};
use odincode::llm::types::{Intent, Plan, Step};

// === T2: Plan Parsing Tests ===

#[test]
fn test_t2_valid_plan_json_parses() {
    // Valid plan JSON must parse successfully
    let json = r#"{
        "plan_id": "plan_123",
        "intent": "MUTATE",
        "steps": [
            {
                "step_id": "step_1",
                "tool": "file_read",
                "arguments": {"path": "src/lib.rs"},
                "precondition": "file exists"
            }
        ],
        "evidence_referenced": ["Q4"]
    }"#;

    let plan = parse_plan(json).expect("Valid plan must parse");

    assert_eq!(plan.plan_id, "plan_123");
    assert_eq!(plan.intent, Intent::Mutate);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].step_id, "step_1");
    assert_eq!(plan.steps[0].tool, "file_read");
    assert_eq!(
        plan.steps[0].arguments.get("path"),
        Some(&"src/lib.rs".to_string())
    );
}

#[test]
fn test_t2_invalid_json_becomes_text_plan() {
    // Phase 7.3: Invalid JSON is treated as plain text (graceful degradation)
    let malformed = r#"{"plan_id": "plan_123", "intent": "INVALID"#;

    let result = parse_plan(malformed);

    // Before Phase 7.3: Would return Err()
    // After Phase 7.3: Returns Ok with text display plan
    assert!(
        result.is_ok(),
        "Malformed JSON should be treated as plain text, not rejected"
    );

    let plan = result.unwrap();
    assert_eq!(plan.intent, Intent::Explain);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].tool, "display_text");
}

#[test]
fn test_t2_unknown_intent_rejected() {
    // Unknown intent value must be rejected
    let json = r#"{
        "plan_id": "plan_123",
        "intent": "AUTONOMOUS_EXECUTE",
        "steps": [],
        "evidence_referenced": []
    }"#;

    let result = parse_plan(json);

    assert!(result.is_err(), "Unknown intent must be rejected");
}

#[test]
fn test_t2_all_valid_intents_accepted() {
    // All valid intents must parse correctly
    let intents = ["READ", "MUTATE", "QUERY", "EXPLAIN"];

    for intent in intents {
        let json = format!(
            r#"{{"plan_id":"plan_{}","intent":"{}","steps":[],"evidence_referenced":[]}}"#,
            intent, intent
        );

        let plan = parse_plan(&json).unwrap_or_else(|_| panic!("Intent {} must parse", intent));
        assert_eq!(plan.intent.to_string(), intent);
    }
}

#[test]
fn test_t2_missing_required_field_rejected() {
    // Missing required fields must be rejected
    let json = r#"{
        "plan_id": "plan_123",
        "steps": [],
        "evidence_referenced": []
    }"#;

    let result = parse_plan(json);

    assert!(result.is_err(), "Missing 'intent' field must be rejected");
}

#[test]
fn test_t2_missing_step_arguments_rejected() {
    // Step without required arguments must be rejected
    let json = r#"{
        "plan_id": "plan_123",
        "intent": "READ",
        "steps": [
            {
                "step_id": "step_1",
                "tool": "file_read",
                "arguments": {},
                "precondition": "file exists"
            }
        ],
        "evidence_referenced": []
    }"#;

    let plan = parse_plan(json).expect("Plan must parse (validation is separate)");
    let validation = validate_plan(&plan);

    assert!(
        validation.is_err(),
        "Missing 'path' argument must fail validation"
    );
}

#[test]
fn test_t2_unknown_tool_in_step_rejected() {
    // Unknown tool in step must be rejected
    let json = r#"{
        "plan_id": "plan_123",
        "intent": "MUTATE",
        "steps": [
            {
                "step_id": "step_1",
                "tool": "hallucinated_tool",
                "arguments": {"path": "src/lib.rs"},
                "precondition": "file exists"
            }
        ],
        "evidence_referenced": []
    }"#;

    let plan = parse_plan(json).expect("Plan must parse (validation is separate)");
    let validation = validate_plan(&plan);

    assert!(validation.is_err(), "Unknown tool must fail validation");
    let err = validation.unwrap_err();
    assert!(
        err.to_string().contains("Unknown tool"),
        "Error must mention unknown tool, got: {err}"
    );
}

#[test]
fn test_t2_error_messages_are_stable() {
    // Rejection errors must be stable (same input → same error message)
    // Use validation error (not parse error) for stability test
    let json = r#"{
        "plan_id": "plan_123",
        "intent": "MUTATE",
        "steps": [
            {
                "step_id": "step_1",
                "tool": "fake_tool",
                "arguments": {},
                "precondition": "none"
            }
        ],
        "evidence_referenced": []
    }"#;

    let plan = parse_plan(json).expect("Plan must parse");
    let err1 = validate_plan(&plan).unwrap_err();
    let err2 = validate_plan(&plan).unwrap_err();

    assert_eq!(
        err1.to_string(),
        err2.to_string(),
        "Error messages must be identical for same input"
    );
}

// === T3: Router Tests ===

#[test]
fn test_t3_whitelist_contains_all_phase_0_tools() {
    // Tool whitelist must contain all Phase 0 tools plus display_text (Phase 7.3)
    let allowed = ToolRouter::allowed_tools();

    assert!(allowed.contains("file_read"));
    assert!(allowed.contains("file_write"));
    assert!(allowed.contains("file_create"));
    assert!(allowed.contains("file_search"));
    assert!(allowed.contains("file_glob"));
    assert!(allowed.contains("splice_patch"));
    assert!(allowed.contains("splice_plan"));
    assert!(allowed.contains("symbols_in_file"));
    assert!(allowed.contains("references_to_symbol_name"));
    assert!(allowed.contains("references_from_file_to_symbol_name"));
    assert!(allowed.contains("lsp_check"));

    // Phase 7.3: Added display_text for plain text LLM responses
    assert!(allowed.contains("display_text"));

    // Phase 1.1: Added memory_query for execution log querying
    assert!(allowed.contains("memory_query"));

    // Phase 1.2: Added execution_summary for aggregate statistics
    assert!(allowed.contains("execution_summary"));

    // Phase 2: Added file_edit for patch-based text editing
    assert!(allowed.contains("file_edit"));

    // Phase 3: Added git_status, git_diff, git_log for version control
    assert!(allowed.contains("git_status"));
    assert!(allowed.contains("git_diff"));
    assert!(allowed.contains("git_log"));

    // Phase 4: Added wc, bash_exec for OS parity
    assert!(allowed.contains("wc"));
    assert!(allowed.contains("bash_exec"));

    assert_eq!(
        allowed.len(),
        20,
        "Must have exactly 20 tools (11 Phase 0 + file_edit + memory_query + execution_summary + git_status + git_diff + git_log + wc + bash_exec + display_text)"
    );
}

#[test]
fn test_t3_unknown_tool_not_allowed() {
    // Unknown tools must be rejected
    assert!(!tool_is_allowed("autonomous_agent"));
    assert!(!tool_is_allowed("speculative_execution"));
    assert!(!tool_is_allowed("direct_db_access"));
}

#[test]
fn test_t3_intent_routes_to_correct_tools() {
    // Verify intent → tool mappings from spec
    let router = ToolRouter::new();

    // READ intent → file_read, symbols_in_file, references_to_symbol_name
    let read_tools = router.tools_for_intent(&Intent::Read);
    assert!(read_tools.contains(&"file_read".to_string()));

    // MUTATE intent → splice_patch, file_write
    let mutate_tools = router.tools_for_intent(&Intent::Mutate);
    assert!(mutate_tools.contains(&"splice_patch".to_string()));
    assert!(mutate_tools.contains(&"file_write".to_string()));

    // QUERY intent → file_search, file_glob, wc, bash_exec
    let query_tools = router.tools_for_intent(&Intent::Query);
    assert!(query_tools.contains(&"file_search".to_string()));
    assert!(query_tools.contains(&"file_glob".to_string()));
    assert!(query_tools.contains(&"wc".to_string()));
    assert!(query_tools.contains(&"bash_exec".to_string()));

    // EXPLAIN intent → evidence queries
    let explain_tools = router.tools_for_intent(&Intent::Explain);
    assert!(!explain_tools.is_empty());
}

#[test]
fn test_t3_router_is_pure_data() {
    // Router must be pure data (no IO, no randomness)
    let router1 = ToolRouter::new();
    let router2 = ToolRouter::new();

    // Same results each time
    assert_eq!(
        router1.tools_for_intent(&Intent::Mutate),
        router2.tools_for_intent(&Intent::Mutate)
    );
}

// === T4: Preconditions Tests ===

#[test]
fn test_t4_file_exists_precondition_defined() {
    // File existence precondition must be defined for file_* tools
    let preconditions = ToolRouter::preconditions_for_tool("file_read");
    assert!(preconditions.contains(&"file exists".to_string()));

    let preconditions = ToolRouter::preconditions_for_tool("file_write");
    assert!(preconditions.contains(&"file exists".to_string()));
}

#[test]
fn test_t4_splice_preconditions_defined() {
    // splice_patch must have Cargo workspace precondition
    let preconditions = ToolRouter::preconditions_for_tool("splice_patch");
    assert!(
        preconditions.contains(&"Cargo workspace exists".to_string())
            || preconditions.contains(&"file is in Cargo workspace".to_string())
    );
}

#[test]
fn test_t4_precondition_check_returns_result() {
    // Preconditions must check without side effects
    use std::path::Path;

    // Check on non-existent file
    let result = odincode::llm::router::check_file_exists(Path::new("/nonexistent/file.rs"));
    assert!(!result, "Non-existent file should fail precondition");

    // Check on current directory (should exist)
    let result = odincode::llm::router::check_file_exists(Path::new("."));
    assert!(result, "Current directory should exist");
}

#[test]
fn test_t4_precondition_failure_halts_validation() {
    // Plan with failed precondition must not validate
    let plan = Plan {
        plan_id: "plan_123".to_string(),
        intent: Intent::Mutate,
        steps: vec![Step {
            step_id: "step_1".to_string(),
            tool: "file_read".to_string(),
            arguments: {
                let mut map = HashMap::new();
                map.insert("path".to_string(), "/nonexistent/file.rs".to_string());
                map
            },
            precondition: "file exists".to_string(),
            requires_confirmation: false,
        }],
        evidence_referenced: vec![],
    };

    let result = validate_plan(&plan);
    assert!(result.is_err(), "Failed precondition must reject plan");
}

#[test]
fn test_t4_all_preconditions_defined_for_whitelist() {
    // Every tool in whitelist must have defined preconditions
    // Phase 7.3: display_text is allowed to have no preconditions (pure UI tool)
    let tools = ToolRouter::allowed_tools();

    for tool in tools {
        let preconditions = ToolRouter::preconditions_for_tool(&tool);
        if tool == "display_text" {
            // Phase 7.3: display_text has no preconditions by design
            assert!(
                preconditions.is_empty(),
                "Tool display_text should have no preconditions"
            );
        } else {
            assert!(
                !preconditions.is_empty(),
                "Tool {} must have at least one precondition",
                tool
            );
        }
    }
}
