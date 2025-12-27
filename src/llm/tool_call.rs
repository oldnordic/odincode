//! TOOL_CALL parser (Phase 9.0)
//!
//! Extracts and parses TOOL_CALL blocks from LLM responses.
//! Deterministic line-based parser, no YAML dependencies.

use std::collections::HashMap;

/// Maximum length for tool output in preview
pub const MAX_TOOL_OUTPUT_PREVIEW: usize = 200;

/// Tool call extracted from LLM response
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    /// Tool name (e.g., "file_read", "file_search")
    pub tool: String,
    /// Tool arguments (key-value pairs)
    pub args: HashMap<String, String>,
}

/// Tool call parsing errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallError {
    /// Missing tool name
    MissingToolName,

    /// Invalid TOOL_CALL format
    InvalidFormat,

    /// Empty tool name
    EmptyToolName,

    /// Missing required argument
    MissingArgument { arg: String },

    /// Tool call is incomplete
    IncompleteToolCall,
}

/// Extract the first TOOL_CALL block from a response
///
/// Returns:
/// - Some(ToolCall, remaining_text) if TOOL_CALL found
/// - None if no TOOL_CALL found (prose response)
///
/// # Format
/// ```text
/// TOOL_CALL:
///   tool: <tool_name>
///   args:
///     <key>: <value>
/// ```
///
/// # Example
/// ```no_run
/// use odincode::llm::tool_call::extract_tool_call;
/// let response = "I'll search for that.\n\nTOOL_CALL:\n  tool: file_search\n  args:\n    pattern: main\n    root: .\n\nDone.";
/// let result = extract_tool_call(response);
/// assert!(result.is_some());
/// ```
pub fn extract_tool_call(response: &str) -> Option<(ToolCall, String)> {
    // Find TOOL_CALL: marker
    let marker = "TOOL_CALL:";
    let marker_pos = response.find(marker)?;

    // Split into tool_call section and remaining text
    let tool_section = &response[marker_pos + marker.len()..];
    let remaining_before = &response[..marker_pos];

    // Find the end of the tool call block
    // Tool call ends at:
    // - End of string
    // - Next TOOL_CALL: (multiple tool calls - only take first)
    // - A blank line followed by non-indented text (prose continuation)
    let tool_call_end = find_tool_call_end(tool_section);

    let tool_call_text = &tool_section[..tool_call_end];
    let remaining_after = &tool_section[tool_call_end..];

    // Parse the tool call
    let tool_call = parse_tool_call_block(tool_call_text).ok()?;

    // Combine remaining text (before + after)
    let remaining_prose = format!("{}{}", remaining_before.trim(), remaining_after.trim());

    Some((tool_call, remaining_prose))
}

/// Find the end of a tool call block
///
/// Returns the byte position where the tool call ends.
fn find_tool_call_end(section: &str) -> usize {
    let mut pos = 0;
    let mut has_blank_line = false;

    for line in section.lines() {
        let line = line.trim_end();
        let line_start = pos;

        // Advance position including newline
        pos += line.len()
            + if pos < section.len() && section.as_bytes()[pos] == b'\n' {
                1
            } else {
                0
            };

        // Check for next TOOL_CALL:
        if line.trim() == "TOOL_CALL:" {
            // Found another tool call, end here
            return line_start;
        }

        // Track blank lines
        if line.is_empty() {
            has_blank_line = true;
            continue;
        }

        // If we have a blank line and now non-indented content, end tool call
        if has_blank_line && !line.starts_with("  ") && !line.is_empty() {
            return line_start;
        }

        // Reset flag if we see indented content
        if line.starts_with("  ") {
            has_blank_line = false;
        }
    }

    // End of string
    pos
}

/// Parse a TOOL_CALL block text
///
/// # Format
/// ```text
///   tool: <tool_name>
///   args:
///     <key>: <value>
/// ```
fn parse_tool_call_block(block: &str) -> Result<ToolCall, ToolCallError> {
    let mut tool_name: Option<String> = None;
    let mut args: HashMap<String, String> = HashMap::new();
    let mut current_key: Option<String> = None;

    for line in block.lines() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Check for tool: line
        if let Some(rest) = line.strip_prefix("tool:") {
            let tool = rest.trim().to_string();
            if tool.is_empty() {
                return Err(ToolCallError::EmptyToolName);
            }
            tool_name = Some(tool);
            continue;
        }

        // Check for args: line (just skip, args follow)
        if line == "args:" {
            continue;
        }

        // Parse argument (key: value)
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            current_key = Some(key.clone());
            args.insert(key, value);
        } else if let Some(key) = &current_key {
            // Continuation of multi-line value
            let existing = args.get(key).cloned().unwrap_or_default();
            args.insert(key.clone(), format!("{} {}", existing, line));
        }
    }

    // Validate we got a tool name
    let tool = tool_name.ok_or(ToolCallError::MissingToolName)?;

    // Validate tool is not empty
    if tool.is_empty() {
        return Err(ToolCallError::EmptyToolName);
    }

    Ok(ToolCall { tool, args })
}

/// Check if response contains a TOOL_CALL block
///
/// Returns true if "TOOL_CALL:" marker is present.
pub fn has_tool_call(response: &str) -> bool {
    response.contains("TOOL_CALL:")
}

/// Get tool output preview (truncated)
///
/// Truncates output to MAX_TOOL_OUTPUT_PREVIEW characters.
pub fn truncate_output(output: &str) -> String {
    if output.len() <= MAX_TOOL_OUTPUT_PREVIEW {
        output.to_string()
    } else {
        format!(
            "{}... (truncated, {} total chars)",
            &output[..MAX_TOOL_OUTPUT_PREVIEW],
            output.len()
        )
    }
}

/// Format tool result for context injection
///
/// Returns a formatted string like:
/// ```text
/// [SYSTEM TOOL RESULT]
/// Tool: file_search
/// Status: success
/// Output: Found 3 matches...
/// ```
pub fn format_tool_result(tool: &str, success: bool, output: &str) -> String {
    let status = if success { "success" } else { "error" };
    let truncated = truncate_output(output);
    format!(
        "[SYSTEM TOOL RESULT]\nTool: {}\nStatus: {}\nOutput: {}\n",
        tool, status, truncated
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_tool_call_true() {
        let response = "I'll search for that.\n\nTOOL_CALL:\n  tool: file_search\n  args:\n    pattern: main\n\nDone.";
        assert!(has_tool_call(response));
    }

    #[test]
    fn test_has_tool_call_false() {
        let response = "This is just regular text with no tool calls.";
        assert!(!has_tool_call(response));
    }

    #[test]
    fn test_extract_tool_call_simple() {
        let response = "TOOL_CALL:\n  tool: file_read\n  args:\n    path: src/lib.rs";
        let result = extract_tool_call(response);

        assert!(result.is_some());
        let (tool_call, remaining) = result.unwrap();

        assert_eq!(tool_call.tool, "file_read");
        assert_eq!(tool_call.args.get("path"), Some(&"src/lib.rs".to_string()));
        assert!(remaining.trim().is_empty());
    }

    #[test]
    fn test_extract_tool_call_with_prose() {
        let response = "I'll read that file for you.\n\nTOOL_CALL:\n  tool: file_read\n  args:\n    path: src/lib.rs\n\nDone!";
        let result = extract_tool_call(response);

        assert!(result.is_some());
        let (tool_call, remaining) = result.unwrap();

        assert_eq!(tool_call.tool, "file_read");
        assert!(remaining.contains("I'll read that file"));
        assert!(remaining.contains("Done!"));
    }

    #[test]
    fn test_extract_tool_call_multiple_args() {
        let response = "TOOL_CALL:\n  tool: file_search\n  args:\n    pattern: main\n    root: .";
        let result = extract_tool_call(response);

        assert!(result.is_some());
        let (tool_call, _remaining) = result.unwrap();

        assert_eq!(tool_call.tool, "file_search");
        assert_eq!(tool_call.args.get("pattern"), Some(&"main".to_string()));
        assert_eq!(tool_call.args.get("root"), Some(&".".to_string()));
    }

    #[test]
    fn test_extract_tool_call_none_prose_only() {
        let response = "This is just regular text without any tool calls.";
        let result = extract_tool_call(response);

        assert!(result.is_none());
    }

    #[test]
    fn test_extract_tool_call_missing_tool_name() {
        let response = "TOOL_CALL:\n  args:\n    path: src/lib.rs";
        let result = extract_tool_call(response);

        // Missing tool name -> parse fails -> None
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_tool_call_empty_tool_name() {
        let response = "TOOL_CALL:\n  tool: \n  args:\n    path: src/lib.rs";
        let result = extract_tool_call(response);

        // Empty tool name -> parse fails -> None
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_tool_call_block_valid() {
        let block = "  tool: file_read\n  args:\n    path: src/lib.rs";
        let result = parse_tool_call_block(block);

        assert!(result.is_ok());
        let tool_call = result.unwrap();

        assert_eq!(tool_call.tool, "file_read");
        assert_eq!(tool_call.args.get("path"), Some(&"src/lib.rs".to_string()));
    }

    #[test]
    fn test_parse_tool_call_block_no_args() {
        let block = "  tool: lsp_check\n  args:";
        let result = parse_tool_call_block(block);

        assert!(result.is_ok());
        let tool_call = result.unwrap();

        assert_eq!(tool_call.tool, "lsp_check");
        assert!(tool_call.args.is_empty());
    }

    #[test]
    fn test_truncate_output_short() {
        let output = "Short output";
        let result = truncate_output(output);
        assert_eq!(result, "Short output");
    }

    #[test]
    fn test_truncate_output_long() {
        let output = "a".repeat(300);
        let result = truncate_output(&output);
        assert!(result.contains("(truncated"));
        assert!(result.len() < 350); // Should be less than original + message
    }

    #[test]
    fn test_format_tool_result_success() {
        let result = format_tool_result("file_read", true, "File content here");
        assert!(result.contains("[SYSTEM TOOL RESULT]"));
        assert!(result.contains("Tool: file_read"));
        assert!(result.contains("Status: success"));
        assert!(result.contains("Output: File content here"));
    }

    #[test]
    fn test_format_tool_result_error() {
        let result = format_tool_result("file_read", false, "File not found");
        assert!(result.contains("Status: error"));
        assert!(result.contains("File not found"));
    }

    #[test]
    fn test_extract_tool_call_with_spaces_in_args() {
        let response = "TOOL_CALL:\n  tool: file_search\n  args:\n    pattern: fn main\n    root: /home/user/project";
        let result = extract_tool_call(response);

        assert!(result.is_some());
        let (tool_call, _) = result.unwrap();

        assert_eq!(tool_call.tool, "file_search");
        assert_eq!(tool_call.args.get("pattern"), Some(&"fn main".to_string()));
        assert_eq!(
            tool_call.args.get("root"),
            Some(&"/home/user/project".to_string())
        );
    }

    #[test]
    fn test_find_tool_call_end_with_next_tool_call() {
        let section = "  tool: file_read\n  args:\n    path: a.rs\n\nTOOL_CALL:\n  tool: file_read\n  args:\n    path: b.rs";
        let end = find_tool_call_end(section);
        // Should end before the second TOOL_CALL
        assert!(end < section.len());
        let extracted = &section[..end];
        assert!(extracted.contains("path: a.rs"));
        // The remaining text should contain the second tool call
        let remaining = &section[end..];
        assert!(remaining.contains("TOOL_CALL:"));
    }
}
