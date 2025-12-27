//! System prompt generation for tool descriptions
//!
//! # Purpose
//!
//! Convert tool metadata into formatted prompts for LLM consumption.
//! Each tool includes:
//! - Description
//! - "When to use" examples with reasoning
//! - "When NOT to use" examples with reasoning

use crate::tools::metadata::{DiscoveryResult, ToolCategory, ToolExample, ToolMetadata};

/// Format a single tool's description with examples
pub fn format_tool(tool: &ToolMetadata) -> String {
    let mut output = String::new();

    // Tool name (bold markdown)
    output.push_str(&format!("**{}**", tool.name));

    // Gated indicator
    if tool.gated {
        output.push_str(" ⚠️ *GATED: Requires approval*\n");
    } else {
        output.push('\n');
    }

    // Description
    output.push_str(&format!("{}\n", tool.description));

    // Examples (when to use)
    if !tool.examples.is_empty() {
        output.push_str("\n**When to use:**\n");
        for example in &tool.examples {
            output.push_str(&format!("- *{}*: `{}`\n", example.scenario, example.command));
            output.push_str(&format!("  - {}\n", example.reasoning));
        }
    }

    // Not-examples (when NOT to use)
    if !tool.not_examples.is_empty() {
        output.push_str("\n**When NOT to use:**\n");
        for example in &tool.not_examples {
            output.push_str(&format!("- *{}*: `{}`\n", example.scenario, example.command));
            output.push_str(&format!("  - {}\n", example.reasoning));
        }
    }

    output
}

/// Format multiple tools for inclusion in system prompt
pub fn format_tools(tools: &[ToolMetadata]) -> String {
    tools.iter()
        .filter(|t| t.visible_to_llm())
        .map(format_tool)
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

/// Format discovery result into organized sections
pub fn format_discovery_result(result: &DiscoveryResult) -> String {
    let mut output = String::new();

    output.push_str("## Available Tools\n\n");

    // Core tools section
    if !result.core.is_empty() {
        output.push_str("### Core Tools\n\n");
        output.push_str(&format_tools(&result.core));
        output.push('\n');
    }

    // Specialized tools section (only if any discovered)
    if !result.specialized.is_empty() {
        output.push_str("\n### Specialized Tools\n\n");
        output.push_str(&format_tools(&result.specialized));
    }

    output
}

/// Generate system prompt for LLM with tool descriptions
pub fn system_prompt(result: &DiscoveryResult) -> String {
    let mut prompt = String::new();

    // Tool selection guidelines
    prompt.push_str(TOOL_GUIDANCE);

    // Available tools
    prompt.push_str("\n\n");
    prompt.push_str(&format_discovery_result(result));

    prompt
}

/// Metadata about generated prompt
#[derive(Debug, Clone)]
pub struct PromptMetadata {
    /// Estimated token count
    pub estimated_tokens: usize,
    /// Number of tools included
    pub tool_count: usize,
    /// Number of specialized tools discovered
    pub specialized_count: usize,
}

/// Generate system prompt with metadata for logging
pub fn system_prompt_with_metadata(result: &DiscoveryResult) -> (String, PromptMetadata) {
    let prompt = system_prompt(result);

    let metadata = PromptMetadata {
        estimated_tokens: estimate_tokens(&prompt),
        tool_count: result.core.len() + result.specialized.len(),
        specialized_count: result.specialized.len(),
    };

    (prompt, metadata)
}

/// Estimate token count for a string
///
/// Uses simple heuristic: ~4 characters per token
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4  // Ceiling division
}

/// Tool usage guidance for system prompt
const TOOL_GUIDANCE: &str = r#"## Tool Selection Guidelines

You have access to a curated set of tools. More tools will be added as needed based on your task.

### Tool Usage Principles

1. **Be specific**: Use `file_read` for known paths, `file_search` for discovery
2. **Start simple**: Try core tools before requesting specialized ones
3. **Read before editing**: Always `file_read` before making changes
4. **Check before assuming**: Verify errors actually exist before running diagnostics

### Common Mistakes to Avoid

- ❌ Don't use `file_search` when you know the exact path — use `file_read`
- ❌ Don't read multiple files when one will suffice
- ❌ Don't run bash commands for file operations (use `file_*` tools)
- ❌ Don't make changes without reading the file first

"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tool_basic() {
        let tool = ToolMetadata {
            name: "test".to_string(),
            category: ToolCategory::Core,
            description: "Test tool".to_string(),
            examples: vec![],
            not_examples: vec![],
            token_cost: 100,
            gated: false,
        };

        let formatted = format_tool(&tool);
        assert!(formatted.contains("**test**"));
        assert!(formatted.contains("Test tool"));
    }

    #[test]
    fn test_format_tool_with_examples() {
        let tool = ToolMetadata {
            name: "file_read".to_string(),
            category: ToolCategory::Core,
            description: "Read file".to_string(),
            examples: vec![
                ToolExample {
                    scenario: "Read known file".to_string(),
                    command: "file_read src/lib.rs".to_string(),
                    reasoning: "Direct access".to_string(),
                },
            ],
            not_examples: vec![
                ToolExample {
                    scenario: "Find files".to_string(),
                    command: "file_search pattern".to_string(),
                    reasoning: "Use search".to_string(),
                },
            ],
            token_cost: 100,
            gated: false,
        };

        let formatted = format_tool(&tool);
        assert!(formatted.contains("**When to use:**"));
        assert!(formatted.contains("**When NOT to use:**"));
        assert!(formatted.contains("Read known file"));
        assert!(formatted.contains("file_read src/lib.rs"));
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("1234"), 1);
        assert_eq!(estimate_tokens("12345"), 2);
        assert_eq!(estimate_tokens(""), 0);
    }
}
