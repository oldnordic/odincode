//! Chat-side tool execution (Phase 9.0)
//!
//! MAIN-thread AUTO tool execution only.
//! Reuses existing tool_mapper::invoke_tool() infrastructure.

use std::collections::HashMap;
use serde_json::Value as JsonValue;

use crate::execution_engine::output_kind::ToolOutputKind;
use crate::execution_engine::tool_mapper::invoke_tool;
use crate::execution_tools::ExecutionDb;
use crate::llm::types::Step;
use crate::magellan_tools::MagellanDb;

/// AUTO tools - execute immediately without approval
pub const AUTO_TOOLS: &[&str] = &[
    "file_read",
    "file_search",
    "file_glob",
    "symbols_in_file",
    "references_to_symbol_name",
    "references_from_file_to_symbol_name",
    "lsp_check", // AUTO but rate-limited (once per loop unless re-requested)
    "count_files",
    "count_lines",
    "fs_stats",
];

/// GATED tools - require user approval (Phase 9.1)
pub const GATED_TOOLS: &[&str] = &["file_write", "file_create"];

/// FORBIDDEN tools - never executable in chat loop
pub const FORBIDDEN_TOOLS: &[&str] = &["splice_patch", "splice_plan"];

/// Tool classification for chat mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatToolCategory {
    /// AUTO: Execute immediately (read-only tools)
    Auto,
    /// GATED: Require user approval (write tools)
    Gated,
    /// FORBIDDEN: Never execute (splice tools, unknown)
    Forbidden,
}

/// Tool execution result for chat context
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// Tool name
    pub tool: String,
    /// Execution success
    pub success: bool,
    /// Full output (for context injection)
    pub output_full: String,
    /// Preview output (truncated for UI/events)
    pub output_preview: String,
    /// Error message if any
    pub error_message: Option<String>,
    /// Path affected by this tool (Phase 9.1: for UI synchronization)
    pub affected_path: Option<String>,
    /// Semantic kind of output (Task A: for routing)
    pub kind: ToolOutputKind,
    /// Structured data payload (for UI, not injected into chat)
    pub structured_data: Option<JsonValue>,
    /// Execution ID (for memory_query reference)
    /// NOTE: Currently None in chat mode (no execution recording yet)
    pub execution_id: String,
}

/// Chat tool runner - MAIN-thread AUTO tool execution
pub struct ChatToolRunner {
    /// Magellan database for symbol queries
    pub magellan_db: Option<MagellanDb>,
    /// Execution database for logging
    pub exec_db: Option<ExecutionDb>,
    /// Track if lsp_check was used (rate limiting)
    lsp_check_used: bool,
    /// Timestamp of last memory_query (for grounding check)
    last_query_time_ms: Option<i64>,
}

impl ChatToolRunner {
    /// Create new chat tool runner
    pub fn new(magellan_db: Option<MagellanDb>, exec_db: Option<ExecutionDb>) -> Self {
        Self {
            magellan_db,
            exec_db,
            lsp_check_used: false,
            last_query_time_ms: None,
        }
    }

    /// Reset lsp_check usage (call at start of each loop)
    pub fn reset_loop_state(&mut self) {
        self.lsp_check_used = false;
    }

    /// Get the last memory_query time
    pub fn last_query_time_ms(&self) -> Option<i64> {
        self.last_query_time_ms
    }

    /// Update the last memory_query time (call when memory_query succeeds)
    pub fn record_memory_query(&mut self) {
        self.last_query_time_ms = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        );
    }

    /// Create runner without databases (for testing)
    pub fn new_no_db() -> Self {
        Self {
            magellan_db: None,
            exec_db: None,
            lsp_check_used: false,
            last_query_time_ms: None,
        }
    }

    /// Classify a tool by category
    pub fn classify_tool(&self, tool: &str) -> ChatToolCategory {
        if FORBIDDEN_TOOLS.contains(&tool) {
            return ChatToolCategory::Forbidden;
        }
        if GATED_TOOLS.contains(&tool) {
            return ChatToolCategory::Gated;
        }
        if AUTO_TOOLS.contains(&tool) {
            return ChatToolCategory::Auto;
        }
        // Unknown tools are forbidden
        ChatToolCategory::Forbidden
    }

    /// Check if a tool is AUTO
    pub fn is_auto_tool(&self, tool: &str) -> bool {
        matches!(self.classify_tool(tool), ChatToolCategory::Auto)
    }

    /// Check if a tool is GATED
    pub fn is_gated_tool(&self, tool: &str) -> bool {
        matches!(self.classify_tool(tool), ChatToolCategory::Gated)
    }

    /// Check if a tool is FORBIDDEN
    pub fn is_forbidden_tool(&self, tool: &str) -> bool {
        matches!(self.classify_tool(tool), ChatToolCategory::Forbidden)
    }

    /// Execute an AUTO tool
    ///
    /// Returns ToolResult with success status and output.
    /// Returns Err if tool is not AUTO or execution fails catastrophically.
    pub fn execute_auto_tool(
        &mut self,
        tool: &str,
        args: &HashMap<String, String>,
    ) -> Result<ToolResult, String> {
        // Check if tool is AUTO
        if !self.is_auto_tool(tool) {
            return Err(format!("Tool '{}' is not AUTO", tool));
        }

        // Rate-limit lsp_check (once per loop unless re-requested)
        if tool == "lsp_check" && self.lsp_check_used {
            return Err("lsp_check already used in this loop (rate limit)".to_string());
        }

        // Build a Step from tool call
        let step = self.build_step(tool, args)?;

        // Extract affected_path for UI synchronization (Phase 9.1)
        let affected_path = self.extract_affected_path(tool, args);

        // Execute using existing tool_mapper
        // Note: If exec_db is not available, tool execution is skipped
        // This is acceptable for chat mode where user experience > logging
        let invocation = if let Some(db) = &self.exec_db {
            invoke_tool(&step, db, &self.magellan_db, self.last_query_time_ms)
                .map_err(|e| format!("Tool execution failed: {}", e))?
        } else {
            // No DB available - return a mock result for testing
            // In production, exec_db should always be provided
            return Ok(ToolResult {
                tool: tool.to_string(),
                success: false,
                output_full: "(No execution DB available - tool execution skipped)".to_string(),
                output_preview: "(No DB - skipped)".to_string(),
                error_message: Some("Execution DB not available".to_string()),
                affected_path,
                kind: ToolOutputKind::Error,
                structured_data: None,
                execution_id: String::new(), // No ID generated in chat mode yet
            });
        };

        // Track lsp_check usage
        if tool == "lsp_check" {
            self.lsp_check_used = true;
        }

        // Record memory_query time (for grounding check)
        if tool == "memory_query" && invocation.success {
            self.record_memory_query();
        }

        // Convert to ToolResult (Task A: include kind and structured_data)
        let output_full = if let Some(stdout) = &invocation.stdout {
            stdout.clone()
        } else if let Some(stderr) = &invocation.stderr {
            stderr.clone()
        } else if let Some(error) = &invocation.error_message {
            error.clone()
        } else {
            String::new()
        };

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
            execution_id: step.step_id.clone(), // Use step_id as execution_id reference
        })
    }

    /// Extract affected path from tool arguments (Phase 9.1)
    pub fn extract_affected_path(
        &self,
        tool: &str,
        args: &HashMap<String, String>,
    ) -> Option<String> {
        match tool {
            "file_read" | "file_write" | "file_create" => args.get("path").cloned(),
            _ => None,
        }
    }

    /// Reset rate-limit tracking (call when starting a new loop)
    pub fn reset_rate_limits(&mut self) {
        self.lsp_check_used = false;
    }

    /// Build a Step from tool name and arguments
    fn build_step(&self, tool: &str, args: &HashMap<String, String>) -> Result<Step, String> {
        Ok(Step {
            step_id: format!("chat-{}-{}", tool, uuid::Uuid::new_v4()),
            tool: tool.to_string(),
            arguments: args.clone(),
            precondition: String::new(), // Not used for chat tool execution
            requires_confirmation: false, // AUTO tools don't require confirmation
        })
    }
}

/// Format tool result for context injection (public helper)
pub fn format_tool_result_for_context(result: &ToolResult) -> String {
    use crate::llm::tool_call::format_tool_result;
    format_tool_result(&result.tool, result.success, &result.output_full)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_tools_list() {
        assert_eq!(AUTO_TOOLS.len(), 10);
        assert!(AUTO_TOOLS.contains(&"file_read"));
        assert!(AUTO_TOOLS.contains(&"file_search"));
        assert!(AUTO_TOOLS.contains(&"file_glob"));
        assert!(AUTO_TOOLS.contains(&"symbols_in_file"));
        assert!(AUTO_TOOLS.contains(&"references_to_symbol_name"));
        assert!(AUTO_TOOLS.contains(&"references_from_file_to_symbol_name"));
        assert!(AUTO_TOOLS.contains(&"lsp_check"));
        assert!(AUTO_TOOLS.contains(&"count_files"));
        assert!(AUTO_TOOLS.contains(&"count_lines"));
        assert!(AUTO_TOOLS.contains(&"fs_stats"));
    }

    #[test]
    fn test_gated_tools_list() {
        assert_eq!(GATED_TOOLS.len(), 2);
        assert!(GATED_TOOLS.contains(&"file_write"));
        assert!(GATED_TOOLS.contains(&"file_create"));
    }

    #[test]
    fn test_forbidden_tools_list() {
        assert_eq!(FORBIDDEN_TOOLS.len(), 2);
        assert!(FORBIDDEN_TOOLS.contains(&"splice_patch"));
        assert!(FORBIDDEN_TOOLS.contains(&"splice_plan"));
    }

    #[test]
    fn test_classify_auto_tools() {
        let runner = ChatToolRunner::new_no_db();

        for tool in AUTO_TOOLS {
            assert_eq!(
                runner.classify_tool(tool),
                ChatToolCategory::Auto,
                "Tool {} should be AUTO",
                tool
            );
        }
    }

    #[test]
    fn test_classify_gated_tools() {
        let runner = ChatToolRunner::new_no_db();

        for tool in GATED_TOOLS {
            assert_eq!(
                runner.classify_tool(tool),
                ChatToolCategory::Gated,
                "Tool {} should be GATED",
                tool
            );
        }
    }

    #[test]
    fn test_classify_forbidden_tools() {
        let runner = ChatToolRunner::new_no_db();

        for tool in FORBIDDEN_TOOLS {
            assert_eq!(
                runner.classify_tool(tool),
                ChatToolCategory::Forbidden,
                "Tool {} should be FORBIDDEN",
                tool
            );
        }
    }

    #[test]
    fn test_classify_unknown_tool() {
        let runner = ChatToolRunner::new_no_db();
        assert_eq!(
            runner.classify_tool("unknown_tool"),
            ChatToolCategory::Forbidden
        );
    }

    #[test]
    fn test_is_auto_tool() {
        let runner = ChatToolRunner::new_no_db();

        assert!(runner.is_auto_tool("file_read"));
        assert!(runner.is_auto_tool("file_search"));
        assert!(runner.is_auto_tool("lsp_check"));
        assert!(runner.is_auto_tool("count_files"));
        assert!(runner.is_auto_tool("count_lines"));
        assert!(runner.is_auto_tool("fs_stats"));

        assert!(!runner.is_auto_tool("file_write"));
        assert!(!runner.is_auto_tool("splice_patch"));
        assert!(!runner.is_auto_tool("unknown"));
    }

    #[test]
    fn test_is_gated_tool() {
        let runner = ChatToolRunner::new_no_db();

        assert!(runner.is_gated_tool("file_write"));
        assert!(runner.is_gated_tool("file_create"));

        assert!(!runner.is_gated_tool("file_read"));
        assert!(!runner.is_gated_tool("splice_patch"));
    }

    #[test]
    fn test_is_forbidden_tool() {
        let runner = ChatToolRunner::new_no_db();

        assert!(runner.is_forbidden_tool("splice_patch"));
        assert!(runner.is_forbidden_tool("splice_plan"));
        assert!(runner.is_forbidden_tool("unknown"));

        assert!(!runner.is_forbidden_tool("file_read"));
        assert!(!runner.is_forbidden_tool("file_write"));
    }

    #[test]
    fn test_execute_auto_tool_rejects_gated() {
        let mut runner = ChatToolRunner::new_no_db();
        let args = HashMap::new();

        let result = runner.execute_auto_tool("file_write", &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not AUTO"));
    }

    #[test]
    fn test_execute_auto_tool_rejects_forbidden() {
        let mut runner = ChatToolRunner::new_no_db();
        let args = HashMap::new();

        let result = runner.execute_auto_tool("splice_patch", &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not AUTO"));
    }

    #[test]
    fn test_execute_auto_tool_without_db() {
        let mut runner = ChatToolRunner::new_no_db();
        let mut args = HashMap::new();
        args.insert("path".to_string(), ".".to_string());

        // Without exec_db, should return a mock result
        let result = runner.execute_auto_tool("file_read", &args);
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        assert!(!tool_result.success);
        assert!(tool_result
            .output_full
            .contains("No execution DB available"));
        assert!(tool_result.error_message.is_some());
    }

    #[test]
    fn test_format_tool_result_for_context() {
        let result = ToolResult {
            tool: "file_read".to_string(),
            success: true,
            output_full: "File content here".to_string(),
            output_preview: "File content here".to_string(),
            error_message: None,
            affected_path: Some("src/lib.rs".to_string()),
            kind: ToolOutputKind::FileContent,
            structured_data: None,
            execution_id: "test-exec-123".to_string(),
        };

        let formatted = format_tool_result_for_context(&result);
        assert!(formatted.contains("[SYSTEM TOOL RESULT]"));
        assert!(formatted.contains("Tool: file_read"));
        assert!(formatted.contains("Status: success"));
        assert!(formatted.contains("Output: File content here"));
    }

    #[test]
    fn test_format_tool_result_error() {
        let result = ToolResult {
            tool: "file_read".to_string(),
            success: false,
            output_full: "Error: File not found".to_string(),
            output_preview: "Error: File not found".to_string(),
            error_message: Some("File not found".to_string()),
            affected_path: Some("missing.txt".to_string()),
            kind: ToolOutputKind::Error,
            structured_data: None,
            execution_id: "test-exec-456".to_string(),
        };

        let formatted = format_tool_result_for_context(&result);
        assert!(formatted.contains("Status: error"));
        // output_full contains the error message
        assert!(formatted.contains("Error: File not found"));
    }
}
