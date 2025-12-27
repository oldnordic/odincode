//! Plan parser and validator
//!
//! Parses LLM JSON output into typed Plan structs.
//! Validates plans against constraints (tool whitelist, arguments, preconditions).

use serde_json;

use crate::llm::router::tool_is_allowed;
use crate::llm::types::{Intent, Plan, Step};

/// Errors from plan parsing and validation
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Invalid intent: {0}")]
    InvalidIntent(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Unknown tool: {0}")]
    UnknownTool(String),

    #[error("Invalid argument for tool '{0}': {1}")]
    InvalidArgument(String, String),

    #[error("Missing required argument '{0}' for tool '{1}'")]
    MissingArgument(String, String),

    #[error("Invalid evidence query: {0}")]
    InvalidEvidenceQuery(String),

    #[error("Precondition failed: {0}")]
    PreconditionFailed(String),

    #[error("Plan validation failed: {0}")]
    ValidationFailed(String),
}

/// Parse LLM response into typed Plan
///
/// Phase 7.3: Text-first contract alignment
/// - If response starts with `{` (after trimming/markdown stripping), parse as JSON plan
/// - Otherwise, create a text-display plan (for natural language responses)
///
/// Returns error only if JSON is well-formed but invalid (missing fields, wrong types).
/// Plain text and malformed JSON are handled gracefully.
pub fn parse_plan(response: &str) -> Result<Plan, PlanError> {
    let trimmed = response.trim();

    // Handle empty response
    if trimmed.is_empty() {
        return Ok(create_text_plan("empty".to_string(), Intent::Read));
    }

    // Phase 7.3: Extract JSON from markdown code blocks if present
    let content = extract_from_markdown(trimmed);

    // Phase 7.3: Only parse as JSON if content starts with `{`
    let json_content = content.trim_start();
    if json_content.starts_with('{') {
        // Try to parse as JSON
        match parse_json_plan(json_content) {
            Ok(plan) => Ok(plan),
            // If JSON is malformed, fall back to text display
            Err(PlanError::JsonParse(_)) => {
                Ok(create_text_plan(content.to_string(), Intent::Explain))
            }
            // Other errors (validation) should propagate
            Err(e) => Err(e),
        }
    } else {
        // Not JSON - create text display plan
        Ok(create_text_plan(content.to_string(), Intent::Explain))
    }
}

/// Extract JSON from markdown code block if present
/// Handles: ```json ... ``` and ``` ... ```
fn extract_from_markdown(text: &str) -> &str {
    // Check for markdown code block
    if let Some(start) = text.find("```") {
        if let Some(end) = text[start + 3..].find("```") {
            // Extract content between code blocks
            let inner = &text[start + 3..start + 3 + end];
            // Skip language identifier if present
            let inner_trimmed = inner.trim_start();
            if let Some(nl) = inner_trimmed.find('\n') {
                &inner_trimmed[nl + 1..]
            } else {
                inner_trimmed
            }
        } else {
            text
        }
    } else {
        text
    }
}

/// Parse JSON string into typed Plan (internal function)
///
/// Assumes content starts with `{`. Returns error if JSON is invalid
/// or missing required fields.
fn parse_json_plan(json: &str) -> Result<Plan, PlanError> {
    let raw: serde_json::Value = serde_json::from_str(json)?;

    // Parse plan_id
    let plan_id = raw["plan_id"]
        .as_str()
        .ok_or_else(|| PlanError::MissingField("plan_id".to_string()))?
        .to_string();

    // Parse intent
    let intent_str = raw["intent"]
        .as_str()
        .ok_or_else(|| PlanError::MissingField("intent".to_string()))?;

    let intent = Intent::from_str(intent_str)
        .ok_or_else(|| PlanError::InvalidIntent(intent_str.to_string()))?;

    // Parse steps
    let steps_raw = raw["steps"]
        .as_array()
        .ok_or_else(|| PlanError::MissingField("steps".to_string()))?;

    let mut steps = Vec::new();
    for (idx, step_raw) in steps_raw.iter().enumerate() {
        let step_id = step_raw["step_id"]
            .as_str()
            .ok_or_else(|| PlanError::MissingField(format!("steps[{}].step_id", idx)))?
            .to_string();

        let tool = step_raw["tool"]
            .as_str()
            .ok_or_else(|| PlanError::MissingField(format!("steps[{}].tool", idx)))?
            .to_string();

        let arguments = parse_arguments(step_raw)?;

        let precondition = step_raw["precondition"]
            .as_str()
            .unwrap_or("none")
            .to_string();

        let requires_confirmation = step_raw["requires_confirmation"].as_bool().unwrap_or(false);

        steps.push(Step {
            step_id,
            tool,
            arguments,
            precondition,
            requires_confirmation,
        });
    }

    // Parse evidence_referenced
    let evidence_raw = raw["evidence_referenced"]
        .as_array()
        .ok_or_else(|| PlanError::MissingField("evidence_referenced".to_string()))?;

    let mut evidence_referenced = Vec::new();
    for ev in evidence_raw {
        if let Some(s) = ev.as_str() {
            evidence_referenced.push(s.to_string());
        }
    }

    Ok(Plan {
        plan_id,
        intent,
        steps,
        evidence_referenced,
    })
}

/// Parse arguments JSON object into HashMap
fn parse_arguments(
    step_raw: &serde_json::Value,
) -> Result<std::collections::HashMap<String, String>, PlanError> {
    let args_raw = step_raw["arguments"]
        .as_object()
        .ok_or_else(|| PlanError::MissingField("arguments".to_string()))?;

    let mut arguments = std::collections::HashMap::new();
    for (key, value) in args_raw {
        let value_str = if value.is_string() {
            value.as_str().unwrap().to_string()
        } else if value.is_number() {
            // Numbers stored as string for simplicity
            value.to_string()
        } else if value.is_boolean() {
            value.to_string()
        } else if value.is_null() {
            "null".to_string()
        } else {
            value.to_string()
        };
        arguments.insert(key.clone(), value_str);
    }

    Ok(arguments)
}

/// Create a text-display plan for natural language LLM responses
///
/// Phase 7.3: Used when LLM returns plain text instead of JSON plan.
/// Creates a plan with a single "display_text" step that shows the text to user.
fn create_text_plan(text: String, default_intent: Intent) -> Plan {
    // Generate a unique plan ID
    let plan_id = format!(
        "text_{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    // Create a display_text step with the LLM response
    let mut arguments = std::collections::HashMap::new();
    arguments.insert("text".to_string(), text);

    Plan {
        plan_id,
        intent: default_intent,
        steps: vec![Step {
            step_id: "display_1".to_string(),
            tool: "display_text".to_string(),
            arguments,
            precondition: "none".to_string(),
            requires_confirmation: false,
        }],
        evidence_referenced: vec![],
    }
}

/// Validate plan against all constraints
///
/// Checks:
/// - Tool whitelist
/// - Argument schemas
/// - Evidence citations
/// - Preconditions
pub fn validate_plan(plan: &Plan) -> Result<(), PlanError> {
    // Check each step
    for step in &plan.steps {
        // V1: Tool whitelist
        if !tool_is_allowed(&step.tool) {
            return Err(PlanError::UnknownTool(step.tool.clone()));
        }

        // V2: Argument schema validation
        validate_arguments_for_tool(&step.tool, &step.arguments)?;

        // V3: Preconditions checked (lightweight checks only)
        if step.precondition == "file exists" {
            if let Some(path) = step.arguments.get("path") {
                if !std::path::Path::new(path).exists() {
                    return Err(PlanError::PreconditionFailed(format!(
                        "File does not exist: {}",
                        path
                    )));
                }
            }
        }
    }

    // V4: Evidence citation validation
    for citation in &plan.evidence_referenced {
        if !is_valid_evidence_query(citation) {
            return Err(PlanError::InvalidEvidenceQuery(citation.clone()));
        }
    }

    Ok(())
}

/// Validate arguments against tool schema
fn validate_arguments_for_tool(
    tool: &str,
    arguments: &std::collections::HashMap<String, String>,
) -> Result<(), PlanError> {
    match tool {
        "file_read" | "file_create" | "file_write" => {
            if !arguments.contains_key("path") {
                return Err(PlanError::MissingArgument(
                    "path".to_string(),
                    tool.to_string(),
                ));
            }
            if (tool == "file_write" || tool == "file_create")
                && !arguments.contains_key("contents")
            {
                return Err(PlanError::MissingArgument(
                    "contents".to_string(),
                    tool.to_string(),
                ));
            }
        }
        "file_search" | "file_glob" => {
            if !arguments.contains_key("pattern") {
                return Err(PlanError::MissingArgument(
                    "pattern".to_string(),
                    tool.to_string(),
                ));
            }
        }
        "splice_patch" => {
            if !arguments.contains_key("file") {
                return Err(PlanError::MissingArgument(
                    "file".to_string(),
                    tool.to_string(),
                ));
            }
            if !arguments.contains_key("symbol") {
                return Err(PlanError::MissingArgument(
                    "symbol".to_string(),
                    tool.to_string(),
                ));
            }
            if !arguments.contains_key("with") {
                return Err(PlanError::MissingArgument(
                    "with".to_string(),
                    tool.to_string(),
                ));
            }
        }
        "splice_plan" => {
            if !arguments.contains_key("file") {
                return Err(PlanError::MissingArgument(
                    "file".to_string(),
                    tool.to_string(),
                ));
            }
        }
        "symbols_in_file" => {
            if !arguments.contains_key("pattern") {
                return Err(PlanError::MissingArgument(
                    "pattern".to_string(),
                    tool.to_string(),
                ));
            }
        }
        "references_to_symbol_name" => {
            if !arguments.contains_key("name") {
                return Err(PlanError::MissingArgument(
                    "name".to_string(),
                    tool.to_string(),
                ));
            }
        }
        "references_from_file_to_symbol_name" => {
            if !arguments.contains_key("file") {
                return Err(PlanError::MissingArgument(
                    "file".to_string(),
                    tool.to_string(),
                ));
            }
            if !arguments.contains_key("name") {
                return Err(PlanError::MissingArgument(
                    "name".to_string(),
                    tool.to_string(),
                ));
            }
        }
        _ => {}
    }

    Ok(())
}

/// Check if evidence query ID is valid
fn is_valid_evidence_query(query: &str) -> bool {
    matches!(
        query,
        "Q1" | "Q2"
            | "Q3"
            | "Q4"
            | "Q5"
            | "Q6"
            | "Q7"
            | "Q8"
            | "list_executions_by_tool"
            | "list_failures_by_tool"
            | "find_executions_by_diagnostic_code"
            | "find_executions_by_file"
            | "get_execution_details"
            | "get_latest_outcome_for_file"
            | "get_recurring_diagnostics"
            | "find_prior_fixes_for_diagnostic"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_valid_plan() {
        let json = r#"{
            "plan_id": "plan_123",
            "intent": "READ",
            "steps": [{
                "step_id": "step_1",
                "tool": "file_read",
                "arguments": {"path": "src/lib.rs"},
                "precondition": "file exists"
            }],
            "evidence_referenced": ["Q4"]
        }"#;

        let plan = parse_plan(json).unwrap();
        assert_eq!(plan.plan_id, "plan_123");
        assert_eq!(plan.intent, Intent::Read);
        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn test_parse_invalid_json_becomes_text_plan() {
        // Phase 7.3: Malformed JSON is treated as plain text
        let json = r#"{"plan_id": "invalid"#;
        let result = parse_plan(json);

        // Should return Ok with text display plan, not an error
        assert!(result.is_ok());

        let plan = result.unwrap();
        assert_eq!(plan.intent, Intent::Explain);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].tool, "display_text");
    }

    #[test]
    fn test_parse_invalid_intent() {
        let json = r#"{
            "plan_id": "plan_123",
            "intent": "INVALID",
            "steps": [],
            "evidence_referenced": []
        }"#;

        assert!(matches!(parse_plan(json), Err(PlanError::InvalidIntent(_))));
    }

    #[test]
    fn test_parse_plain_text_creates_display_plan() {
        // Phase 7.3: Plain text should create a display_text plan
        let plain_text = "Hello! I can help you with that.";

        let result = parse_plan(plain_text);
        assert!(result.is_ok());

        let plan = result.unwrap();
        assert_eq!(plan.intent, Intent::Explain);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].tool, "display_text");
        assert_eq!(plan.steps[0].arguments.get("text").unwrap(), plain_text);
    }

    #[test]
    fn test_parse_markdown_wrapped_json() {
        // Phase 7.3: JSON wrapped in markdown should be extracted and parsed
        let markdown = r#"```json
{
    "plan_id": "plan_456",
    "intent": "READ",
    "steps": [],
    "evidence_referenced": []
}
```"#;

        let result = parse_plan(markdown);
        assert!(result.is_ok());

        let plan = result.unwrap();
        assert_eq!(plan.plan_id, "plan_456");
        assert_eq!(plan.intent, Intent::Read);
    }

    #[test]
    fn test_validate_unknown_tool() {
        let plan = Plan {
            plan_id: "plan_123".to_string(),
            intent: Intent::Read,
            steps: vec![Step {
                step_id: "step_1".to_string(),
                tool: "unknown_tool".to_string(),
                arguments: HashMap::new(),
                precondition: "none".to_string(),
                requires_confirmation: false,
            }],
            evidence_referenced: vec![],
        };

        assert!(matches!(
            validate_plan(&plan),
            Err(PlanError::UnknownTool(_))
        ));
    }

    #[test]
    fn test_validate_missing_argument() {
        let plan = Plan {
            plan_id: "plan_123".to_string(),
            intent: Intent::Read,
            steps: vec![Step {
                step_id: "step_1".to_string(),
                tool: "file_read".to_string(),
                arguments: HashMap::new(), // Missing "path"
                precondition: "none".to_string(),
                requires_confirmation: false,
            }],
            evidence_referenced: vec![],
        };

        assert!(matches!(
            validate_plan(&plan),
            Err(PlanError::MissingArgument(_, _))
        ));
    }

    #[test]
    fn test_validate_invalid_evidence_query() {
        let plan = Plan {
            plan_id: "plan_123".to_string(),
            intent: Intent::Read,
            steps: vec![],
            evidence_referenced: vec!["Q99".to_string()], // Invalid
        };

        assert!(matches!(
            validate_plan(&plan),
            Err(PlanError::InvalidEvidenceQuery(_))
        ));
    }
}
