//! UI routing rules for tool outputs
//!
//! Defines how tool execution results should be routed based on:
//! - Output kind (text, JSON, structured data)
//! - Tool classification (auto, gated, forbidden)
//! - User intent (read, modify, analyze)
//! - Result size (truncate large outputs)

use std::collections::HashSet;
use serde::{Deserialize, Serialize};

use super::tool_registry::{ToolClassification, ToolRegistry};
use super::output_kind::ToolOutputKind;

/// Routing destination for tool output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoutingDestination {
    /// Inject into chat context (visible to user)
    Chat,
    /// Log to diagnostics only (hidden from user)
    Diagnostics,
    /// Both chat and diagnostics
    Both,
    /// Suppress output (internal use only)
    None,
}

/// User intent for tool invocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UserIntent {
    /// Reading information
    Read,
    /// Modifying project state
    Modify,
    /// Analyzing codebase
    Analyze,
    /// Debugging/diagnostics
    Debug,
}

/// Routing rule for tool output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    /// Tool name pattern (empty = applies to all)
    pub tool_pattern: String,
    /// Output kind to match
    pub output_kind: ToolOutputKind,
    /// Required user intent (None = any intent)
    pub required_intent: Option<UserIntent>,
    /// Minimum result size to trigger truncation (None = no truncation)
    pub truncate_threshold: Option<usize>,
    /// Where to route the output
    pub destination: RoutingDestination,
}

impl RoutingRule {
    /// Create new routing rule
    pub fn new(tool_pattern: impl Into<String>, output_kind: ToolOutputKind) -> Self {
        Self {
            tool_pattern: tool_pattern.into(),
            output_kind,
            required_intent: None,
            truncate_threshold: None,
            destination: RoutingDestination::Chat,
        }
    }

    /// Set required user intent
    pub fn with_intent(mut self, intent: UserIntent) -> Self {
        self.required_intent = Some(intent);
        self
    }

    /// Set truncation threshold
    pub fn with_truncation(mut self, threshold: usize) -> Self {
        self.truncate_threshold = Some(threshold);
        self
    }

    /// Set routing destination
    pub fn with_destination(mut self, dest: RoutingDestination) -> Self {
        self.destination = dest;
        self
    }

    /// Check if rule matches given parameters
    pub fn matches(&self, tool: &str, output_kind: &ToolOutputKind, intent: UserIntent) -> bool {
        // Check tool pattern
        if !self.tool_pattern.is_empty() && !self.tool_matches(tool) {
            return false;
        }

        // Check output kind
        if self.output_kind != *output_kind {
            return false;
        }

        // Check intent
        if let Some(required) = self.required_intent {
            if required != intent {
                return false;
            }
        }

        true
    }

    /// Check if tool name matches pattern
    fn tool_matches(&self, tool: &str) -> bool {
        if self.tool_pattern.is_empty() {
            return true;
        }
        // Simple prefix matching for now
        if self.tool_pattern.ends_with('*') {
            let prefix = &self.tool_pattern[..self.tool_pattern.len() - 1];
            tool.starts_with(prefix)
        } else {
            tool == self.tool_pattern
        }
    }
}

/// Routing configuration
#[derive(Debug, Clone)]
pub struct RoutingConfig {
    /// Maximum output size before truncation
    pub max_output_size: usize,
    /// Whether to show tool names in output
    pub show_tool_names: bool,
    /// Whether to show execution time
    pub show_timing: bool,
    /// Default destination for unmatched tools
    pub default_destination: RoutingDestination,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            max_output_size: 10_000,
            show_tool_names: true,
            show_timing: false,
            default_destination: RoutingDestination::Chat,
        }
    }
}

/// Tool output router
///
/// Determines where and how tool outputs should be presented.
pub struct ToolRouter {
    /// Registry for tool metadata
    registry: ToolRegistry,
    /// Custom routing rules
    rules: Vec<RoutingRule>,
    /// Configuration
    config: RoutingConfig,
}

impl ToolRouter {
    /// Create new router with default rules
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
            rules: Self::default_rules(),
            config: RoutingConfig::default(),
        }
    }

    /// Create router with custom registry (for testing)
    #[cfg(test)]
    pub fn with_registry(registry: ToolRegistry) -> Self {
        Self {
            registry,
            rules: Self::default_rules(),
            config: RoutingConfig::default(),
        }
    }

    /// Create empty router (for testing)
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            registry: ToolRegistry::empty(),
            rules: Vec::new(),
            config: RoutingConfig::default(),
        }
    }

    /// Build default routing rules
    fn default_rules() -> Vec<RoutingRule> {
        vec![
            // File operations -> Chat (user needs to see files)
            RoutingRule::new("file_*", ToolOutputKind::Textual)
                .with_destination(RoutingDestination::Chat),
            RoutingRule::new("file_*", ToolOutputKind::FileContent)
                .with_destination(RoutingDestination::Chat),
            RoutingRule::new("file_*", ToolOutputKind::Void)
                .with_destination(RoutingDestination::Diagnostics),

            // Search results -> Chat (structured data)
            RoutingRule::new("file_search", ToolOutputKind::Structural)
                .with_destination(RoutingDestination::Chat),
            RoutingRule::new("file_glob", ToolOutputKind::Structural)
                .with_destination(RoutingDestination::Chat),

            // Splice operations -> Chat + Diagnostics
            RoutingRule::new("splice_*", ToolOutputKind::Textual)
                .with_destination(RoutingDestination::Both)
                .with_truncation(5_000),

            // Magellan queries -> Chat (code navigation)
            RoutingRule::new("magellan_*", ToolOutputKind::Structural)
                .with_destination(RoutingDestination::Chat),

            // Compiler diagnostics -> Both
            RoutingRule::new("cargo_check", ToolOutputKind::Error)
                .with_destination(RoutingDestination::Both),

            // Git operations -> Chat (version control visibility)
            RoutingRule::new("git_*", ToolOutputKind::Textual)
                .with_destination(RoutingDestination::Chat),
            RoutingRule::new("git_*", ToolOutputKind::Structural)
                .with_destination(RoutingDestination::Chat),
        ]
    }

    /// Add custom routing rule
    pub fn add_rule(&mut self, rule: RoutingRule) {
        self.rules.push(rule);
    }

    /// Set routing configuration
    pub fn set_config(&mut self, config: RoutingConfig) {
        self.config = config;
    }

    /// Get routing destination for a tool output
    pub fn route(
        &self,
        tool: &str,
        output_kind: &ToolOutputKind,
        intent: UserIntent,
    ) -> RoutingDestination {
        // Check custom rules first
        for rule in &self.rules {
            if rule.matches(tool, output_kind, intent) {
                return rule.destination;
            }
        }

        // Default behavior based on tool classification
        if let Some(meta) = self.registry.get(tool) {
            match meta.classification {
                ToolClassification::Auto => RoutingDestination::Chat,
                ToolClassification::Gated => RoutingDestination::Both,
                ToolClassification::Forbidden => RoutingDestination::None,
            }
        } else {
            self.config.default_destination
        }
    }

    /// Check if output should be truncated
    pub fn should_truncate(&self, tool: &str, output_size: usize) -> bool {
        // Check custom rules
        for rule in &self.rules {
            if rule.tool_matches(tool) {
                if let Some(threshold) = rule.truncate_threshold {
                    return output_size > threshold;
                }
            }
        }

        // Use default config
        output_size > self.config.max_output_size
    }

    /// Truncate output if needed
    pub fn truncate_output(&self, tool: &str, output: &str) -> String {
        if !self.should_truncate(tool, output.len()) {
            return output.to_string();
        }

        let threshold = self.truncation_threshold(tool);
        let truncated = &output[..threshold.min(output.len())];

        format!(
            "{}... [truncated, {} chars total]",
            truncated,
            output.len()
        )
    }

    /// Get truncation threshold for a tool
    fn truncation_threshold(&self, tool: &str) -> usize {
        for rule in &self.rules {
            if rule.tool_matches(tool) {
                if let Some(threshold) = rule.truncate_threshold {
                    return threshold;
                }
            }
        }
        self.config.max_output_size
    }

    /// Check if output should be visible to user
    pub fn is_visible(&self, tool: &str, output_kind: &ToolOutputKind, intent: UserIntent) -> bool {
        matches!(
            self.route(tool, output_kind, intent),
            RoutingDestination::Chat | RoutingDestination::Both
        )
    }

    /// Check if output should go to diagnostics
    pub fn should_log(&self, tool: &str, output_kind: &ToolOutputKind, intent: UserIntent) -> bool {
        matches!(
            self.route(tool, output_kind, intent),
            RoutingDestination::Diagnostics | RoutingDestination::Both
        )
    }

    /// Get all tools that route to chat
    pub fn chat_routed_tools(&self) -> HashSet<String> {
        let mut result = HashSet::new();
        for tool in self.registry.available_tool_names() {
            if self.is_visible(&tool, &ToolOutputKind::Textual, UserIntent::Read) {
                result.insert(tool);
            }
        }
        result
    }

    /// Get all tools that route to diagnostics only
    pub fn diagnostic_only_tools(&self) -> HashSet<String> {
        let mut result = HashSet::new();
        for tool in self.registry.available_tool_names() {
            let dest = self.route(&tool, &ToolOutputKind::Textual, UserIntent::Read);
            if dest == RoutingDestination::Diagnostics {
                result.insert(tool);
            }
        }
        result
    }

    /// Get configuration
    pub fn config(&self) -> &RoutingConfig {
        &self.config
    }
}

impl Default for ToolRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_router_has_rules() {
        let router = ToolRouter::new();
        assert!(!router.rules.is_empty());
    }

    #[test]
    fn test_route_file_operations_to_chat() {
        let router = ToolRouter::new();
        let dest = router.route("file_read", &ToolOutputKind::Textual, UserIntent::Read);
        assert_eq!(dest, RoutingDestination::Chat);
    }

    #[test]
    fn test_route_splice_to_both() {
        let router = ToolRouter::new();
        let dest = router.route("splice_patch", &ToolOutputKind::Textual, UserIntent::Modify);
        assert_eq!(dest, RoutingDestination::Both);
    }

    #[test]
    fn test_route_cargo_check_to_both() {
        let router = ToolRouter::new();
        let dest = router.route("cargo_check", &ToolOutputKind::Error, UserIntent::Debug);
        assert_eq!(dest, RoutingDestination::Both);
    }

    #[test]
    fn test_route_git_to_chat() {
        let router = ToolRouter::new();
        let dest = router.route("git_status", &ToolOutputKind::Textual, UserIntent::Read);
        assert_eq!(dest, RoutingDestination::Chat);
    }

    #[test]
    fn test_is_visible_for_chat_tools() {
        let router = ToolRouter::new();
        assert!(router.is_visible("file_read", &ToolOutputKind::Textual, UserIntent::Read));
    }

    #[test]
    fn test_should_log_for_diagnostics() {
        let router = ToolRouter::new();
        assert!(router.should_log("cargo_check", &ToolOutputKind::Error, UserIntent::Debug));
    }

    #[test]
    fn test_should_truncate_large_output() {
        let router = ToolRouter::new();
        let large_output = "x".repeat(20_000);
        assert!(router.should_truncate("file_read", large_output.len()));
    }

    #[test]
    fn test_should_not_truncate_small_output() {
        let router = ToolRouter::new();
        let small_output = "hello";
        assert!(!router.should_truncate("file_read", small_output.len()));
    }

    #[test]
    fn test_truncate_output_adds_marker() {
        let router = ToolRouter::new();
        let output = "x".repeat(20_000);
        let truncated = router.truncate_output("file_read", &output);
        assert!(truncated.contains("truncated"));
        assert!(truncated.contains("20000")); // Format doesn't use underscore
        assert!(truncated.contains("chars"));
    }

    #[test]
    fn test_truncate_small_output_unchanged() {
        let router = ToolRouter::new();
        let output = "hello world";
        let truncated = router.truncate_output("file_read", output);
        assert_eq!(truncated, output);
    }

    #[test]
    fn test_add_custom_rule() {
        let mut router = ToolRouter::empty();
        let rule = RoutingRule::new("custom_*", ToolOutputKind::Textual)
            .with_destination(RoutingDestination::Diagnostics);
        router.add_rule(rule);

        let dest = router.route("custom_tool", &ToolOutputKind::Textual, UserIntent::Read);
        assert_eq!(dest, RoutingDestination::Diagnostics);
    }

    #[test]
    fn test_tool_pattern_matches_exact() {
        let rule = RoutingRule::new("file_read", ToolOutputKind::Textual);
        assert!(rule.matches("file_read", &ToolOutputKind::Textual, UserIntent::Read));
        assert!(!rule.matches("file_write", &ToolOutputKind::Textual, UserIntent::Read));
    }

    #[test]
    fn test_tool_pattern_matches_wildcard() {
        let rule = RoutingRule::new("file_*", ToolOutputKind::Textual);
        assert!(rule.matches("file_read", &ToolOutputKind::Textual, UserIntent::Read));
        assert!(rule.matches("file_write", &ToolOutputKind::Textual, UserIntent::Read));
        assert!(!rule.matches("magellan_query", &ToolOutputKind::Textual, UserIntent::Read));
    }

    #[test]
    fn test_tool_pattern_matches_empty_is_any() {
        let rule = RoutingRule::new("", ToolOutputKind::Textual);
        assert!(rule.matches("file_read", &ToolOutputKind::Textual, UserIntent::Read));
        assert!(rule.matches("any_tool", &ToolOutputKind::Textual, UserIntent::Read));
    }

    #[test]
    fn test_rule_with_intent_filter() {
        let rule = RoutingRule::new("file_read", ToolOutputKind::Textual)
            .with_intent(UserIntent::Read);

        assert!(rule.matches("file_read", &ToolOutputKind::Textual, UserIntent::Read));
        assert!(!rule.matches("file_read", &ToolOutputKind::Textual, UserIntent::Modify));
    }

    #[test]
    fn test_set_config() {
        let mut router = ToolRouter::new();
        let config = RoutingConfig {
            max_output_size: 100,
            show_tool_names: false,
            show_timing: true,
            default_destination: RoutingDestination::None,
        };
        router.set_config(config);

        assert_eq!(router.config().max_output_size, 100);
        assert!(!router.config().show_tool_names);
        assert!(router.config().show_timing);
    }

    #[test]
    fn test_chat_routed_tools() {
        let router = ToolRouter::new();
        let chat_tools = router.chat_routed_tools();
        assert!(!chat_tools.is_empty());
        assert!(chat_tools.contains("file_read"));
    }

    #[test]
    fn test_diagnostic_only_tools() {
        let router = ToolRouter::new();
        let diag_tools = router.diagnostic_only_tools();
        // Void operations route to diagnostics only
        // The assertion checks that the test works - most tools route to chat
        assert!(diag_tools.len() < router.registry.len());
    }

    #[test]
    fn test_route_unknown_tool_to_default() {
        let router = ToolRouter::new();
        let dest = router.route("unknown_tool", &ToolOutputKind::Textual, UserIntent::Read);
        assert_eq!(dest, RoutingDestination::Chat); // Default
    }

    #[test]
    fn test_route_by_classification() {
        let mut router = ToolRouter::empty();
        // Use a registry with known tools
        router.registry = ToolRegistry::new();

        // Auto tools go to chat
        let dest = router.route("file_read", &ToolOutputKind::Textual, UserIntent::Read);
        assert_eq!(dest, RoutingDestination::Chat);

        // Gated tools go to both
        let dest = router.route("splice_patch", &ToolOutputKind::Textual, UserIntent::Modify);
        assert_eq!(dest, RoutingDestination::Both);
    }
}
