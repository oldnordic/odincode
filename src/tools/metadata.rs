//! Tool metadata with usage examples
//!
//! # Design Principle
//!
//! Examples teach behavior better than rules (Claude Code leak).
//! Each tool MUST have:
//! - 3-5 "when to use" examples
//! - 2-3 "when NOT to use" examples
//! - Clear reasoning for each
//!
//! # Serialization
//!
//! All metadata is serializable to JSON for:
//! - Storage in configuration
//! - Transmission to LLM context builders
//! - Audit logging

use serde::{Deserialize, Serialize};

/// Helper trait for ceiling division on usize
trait DivCeil {
    fn div_ceil(self, divisor: usize) -> usize;
}

impl DivCeil for usize {
    fn div_ceil(self, divisor: usize) -> usize {
        (self + divisor - 1) / divisor
    }
}

/// Complete tool metadata for LLM consumption
///
/// This struct contains all information needed for:
/// - Tool discovery (deciding which tools to load)
/// - LLM prompt construction (tool descriptions)
/// - UI display (help text, examples)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Tool name (e.g., "file_read", "splice_patch")
    pub name: String,

    /// Tool category determines when this tool is loaded
    pub category: ToolCategory,

    /// Human-readable description of what the tool does
    pub description: String,

    /// Examples of proper usage
    pub examples: Vec<ToolExample>,

    /// Examples of when NOT to use this tool
    pub not_examples: Vec<ToolExample>,

    /// Approximate token cost when included in prompt
    pub token_cost: usize,

    /// Whether this tool requires user approval (GATED)
    pub gated: bool,
}

impl ToolMetadata {
    /// Check if this tool is valid for use
    ///
    /// Validation rules:
    /// - Core tools MUST have at least one example
    /// - Specialized tools MUST have at least one example
    /// - Internal tools MAY have zero examples
    pub fn is_valid(&self) -> bool {
        match self.category {
            ToolCategory::Core | ToolCategory::Specialized => !self.examples.is_empty(),
            ToolCategory::Internal => true,  // Internal tools don't need examples
        }
    }

    /// Check if this tool should be visible to the LLM
    ///
    /// Internal tools (approval_granted, approval_denied, llm_preflight)
    /// are never shown to the LLM — they're system-use only.
    pub fn visible_to_llm(&self) -> bool {
        !matches!(self.category, ToolCategory::Internal)
    }

    /// Estimate total token cost for this tool
    ///
    /// Includes:
    /// - Base declared token_cost
    /// - Examples cost
    /// - Not-examples cost
    ///
    /// Uses a simple heuristic: ~4 characters per token.
    /// Returns `self.token_cost` plus estimated cost of examples.
    pub fn estimate_token_cost(&self) -> usize {
        let base = (self.name.len() + self.description.len()).div_ceil(4);
        let examples_cost: usize = self.examples.iter()
            .map(|e| (e.scenario.len() + e.command.len() + e.reasoning.len()).div_ceil(4))
            .sum();
        let not_examples_cost: usize = self.not_examples.iter()
            .map(|e| (e.scenario.len() + e.command.len() + e.reasoning.len()).div_ceil(4))
            .sum();

        // Base token cost + examples/not-examples add to the total
        self.token_cost + base + examples_cost + not_examples_cost
    }
}

/// Tool category determines loading strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    /// Core tools — always loaded into LLM context
    /// Used in >50% of sessions, required for basic workflow
    Core,

    /// Specialized tools — discovered on-demand based on context
    /// Used in <20% of sessions, triggered by keywords
    Specialized,

    /// Internal tools — system use only, never shown to LLM
    /// Used for approval tracking, preflight checks, etc.
    Internal,
}

/// Usage example for a tool
///
/// Teaches when and how to use a tool with explicit reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    /// When would you use this tool?
    pub scenario: String,

    /// Example command/tool call
    pub command: String,

    /// Why is this the right approach?
    pub reasoning: String,
}

/// Discovery trigger — when to load a specialized tool
///
/// Defines conditions under which a specialized tool should be
/// discovered and added to the LLM's available tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryTrigger {
    /// Keyword in user message triggers discovery
    ///
    /// Example: "refactor" → triggers splice_plan discovery
    Keyword(String),

    /// Keyword in recent tool output triggers discovery
    ///
    /// Example: LSP output contains "error" → triggers lsp_check
    InOutput(String),

    /// Pattern of recently used tools triggers discovery
    ///
    /// Example: [file_read, file_search] used → triggers file_glob
    ToolPattern(Vec<String>),
}

/// Specialized tool with discovery rules
///
/// Combines tool metadata with the conditions under which it should
/// be discovered and loaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecializedTool {
    /// Complete tool metadata
    pub metadata: ToolMetadata,

    /// Conditions that trigger discovery of this tool
    pub triggers: Vec<DiscoveryTrigger>,
}

impl SpecializedTool {
    /// Create a new specialized tool
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        token_cost: usize,
    ) -> Self {
        Self {
            metadata: ToolMetadata {
                name: name.into(),
                category: ToolCategory::Specialized,
                description: description.into(),
                examples: Vec::new(),
                not_examples: Vec::new(),
                token_cost,
                gated: false,
            },
            triggers: Vec::new(),
        }
    }

    /// Add a usage example
    pub fn with_example(mut self, scenario: impl Into<String>, command: impl Into<String>, reasoning: impl Into<String>) -> Self {
        self.metadata.examples.push(ToolExample {
            scenario: scenario.into(),
            command: command.into(),
            reasoning: reasoning.into(),
        });
        self
    }

    /// Add a "when NOT to use" example
    pub fn with_not_example(mut self, scenario: impl Into<String>, command: impl Into<String>, reasoning: impl Into<String>) -> Self {
        self.metadata.not_examples.push(ToolExample {
            scenario: scenario.into(),
            command: command.into(),
            reasoning: reasoning.into(),
        });
        self
    }

    /// Add a discovery trigger
    pub fn with_trigger(mut self, trigger: DiscoveryTrigger) -> Self {
        self.triggers.push(trigger);
        self
    }

    /// Mark as gated (requires approval)
    pub fn gated(mut self) -> Self {
        self.metadata.gated = true;
        self
    }

    /// Check if this tool should be discovered given the query
    pub fn should_discover(&self, query: &str, recent_outputs: &[String]) -> bool {
        let query_lower = query.to_lowercase();

        for trigger in &self.triggers {
            match trigger {
                DiscoveryTrigger::Keyword(keyword) => {
                    if query_lower.contains(&keyword.to_lowercase()) {
                        return true;
                    }
                }
                DiscoveryTrigger::InOutput(pattern) => {
                    for output in recent_outputs {
                        if output.to_lowercase().contains(&pattern.to_lowercase()) {
                            return true;
                        }
                    }
                }
                DiscoveryTrigger::ToolPattern(tools) => {
                    // Check if any tool in the pattern was recently used
                    // by scanning recent outputs for tool name mentions
                    for output in recent_outputs {
                        for tool_name in tools {
                            if output.contains(tool_name) {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }
}

/// Result of tool discovery
///
/// Contains the set of tools that should be loaded for a given query.
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    /// Core tools — always present
    pub core: Vec<ToolMetadata>,

    /// Specialized tools — discovered from context
    pub specialized: Vec<ToolMetadata>,

    /// Total estimated token cost for all included tools
    pub total_token_cost: usize,
}

impl DiscoveryResult {
    /// Get all tools (core + specialized) that are visible to LLM
    pub fn visible_tools(&self) -> Vec<&ToolMetadata> {
        self.core.iter()
            .chain(self.specialized.iter())
            .filter(|t| t.visible_to_llm())
            .collect()
    }

    /// Get tool names for logging
    pub fn tool_names(&self) -> Vec<String> {
        self.core.iter()
            .chain(self.specialized.iter())
            .map(|t| t.name.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialized_tool_builder() {
        let tool = SpecializedTool::new("test_tool", "A test tool", 100)
            .with_example("Test scenario", "test command", "test reasoning")
            .with_not_example("Don't use for X", "avoid", "why not")
            .with_trigger(DiscoveryTrigger::Keyword("test".to_string()));

        assert_eq!(tool.metadata.name, "test_tool");
        assert_eq!(tool.metadata.examples.len(), 1);
        assert_eq!(tool.metadata.not_examples.len(), 1);
        assert_eq!(tool.triggers.len(), 1);
    }

    #[test]
    fn test_specialized_tool_keyword_discovery() {
        let tool = SpecializedTool::new("git_log", "Show git history", 150)
            .with_trigger(DiscoveryTrigger::Keyword("history".to_string()))
            .with_trigger(DiscoveryTrigger::Keyword("commits".to_string()));

        assert!(tool.should_discover("show me the git history", &[]));
        assert!(tool.should_discover("list all commits", &[]));
        assert!(!tool.should_discover("read a file", &[]));
    }

    #[test]
    fn test_specialized_tool_tool_pattern_discovery() {
        // ToolPattern triggers when a specific tool was recently used
        let tool = SpecializedTool::new("file_edit", "Edit file lines", 100)
            .with_trigger(DiscoveryTrigger::ToolPattern(vec!["file_read".to_string()]));

        // Should discover when file_read appears in recent outputs
        let outputs_with_file_read = vec![
            "Read 42 lines from src/lib.rs".to_string(),
            "file_read completed successfully".to_string(),
        ];
        assert!(tool.should_discover("continue", &outputs_with_file_read));

        // Should NOT discover when file_read not mentioned
        let outputs_without = vec![
            "Operation completed".to_string(),
            "No results found".to_string(),
        ];
        assert!(!tool.should_discover("continue", &outputs_without));
    }

    #[test]
    fn test_tool_metadata_estimate_tokens() {
        let metadata = ToolMetadata {
            name: "file_read".to_string(),
            category: ToolCategory::Core,
            description: "Read file contents".to_string(),
            examples: vec![
                ToolExample {
                    scenario: "Read file".to_string(),
                    command: "file_read x.rs".to_string(),
                    reasoning: "Direct".to_string(),
                },
            ],
            not_examples: vec![],
            token_cost: 100,
            gated: false,
        };

        let estimated = metadata.estimate_token_cost();
        assert!(estimated > 50);  // Should have some cost
    }
}
