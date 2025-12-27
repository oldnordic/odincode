//! GATED tool execution
//!
//! # Gated Tool Execution
//!
//! Executes tools that have been approved by the user.
//! These are tools that require explicit approval before running.

use std::collections::HashMap;

use crate::execution_engine::invoke_tool;
use crate::execution_engine::ToolResult;
use crate::llm::types::Step;

/// Execute a GATED tool (after user approval)
///
/// Builds a Step from the tool call and invokes it through the
/// execution engine. Bypasses AUTO tool checks since user approved.
pub fn execute_gated_tool(
    tool: &str,
    args: &HashMap<String, String>,
    tool_runner: &crate::execution_engine::ChatToolRunner,
) -> Result<ToolResult, String> {
    // Build Step
    let step = Step {
        step_id: format!("chat-{}-approved-{}", tool, uuid::Uuid::new_v4()),
        tool: tool.to_string(),
        arguments: args.clone(),
        precondition: String::new(),
        requires_confirmation: false, // Already approved
    };

    // Extract affected path for UI synchronization (Phase 9.1)
    let affected_path = tool_runner.extract_affected_path(tool, &step.arguments);

    // Execute
    let invocation = if let Some(db) = &tool_runner.exec_db {
        invoke_tool(&step, db, &tool_runner.magellan_db, tool_runner.last_query_time_ms())
            .map_err(|e| format!("Tool execution failed: {}", e))?
    } else {
        return Ok(ToolResult {
            tool: tool.to_string(),
            success: false,
            output_full: "(No execution DB available)".to_string(),
            output_preview: "(No DB)".to_string(),
            error_message: Some("Execution DB not available".to_string()),
            affected_path,
            kind: crate::execution_engine::ToolOutputKind::Error,
            structured_data: None,
            execution_id: format!("no-db-{}", uuid::Uuid::new_v4()),
        });
    };

    // Convert to ToolResult
    let output_full = invocation
        .stdout
        .or(invocation.stderr)
        .clone()
        .or(invocation.error_message.clone())
        .unwrap_or_default();

    let output_preview = if output_full.len() > 200 {
        format!("{}... (truncated)", &output_full[..200])
    } else {
        output_full.clone()
    };

    Ok(ToolResult {
        tool: tool.to_string(),
        success: invocation.success,
        output_full,
        output_preview,
        error_message: invocation.error_message,
        affected_path,
        kind: invocation.kind,
        structured_data: invocation.structured_data,
        execution_id: step.step_id.clone(),
    })
}
