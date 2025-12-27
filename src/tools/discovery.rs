//! Progressive tool discovery engine
//!
//! # Discovery Algorithm
//!
//! 1. Start with core tools (always loaded)
//! 2. Analyze user query for trigger keywords
//! 3. Check recent tool outputs for secondary triggers
//! 4. Return core + discovered specialized tools

use crate::tools::{core, metadata::ToolMetadata, specialized_tools, SpecializedTool};

/// Discovery engine — determines which tools to load based on context
pub struct DiscoveryEngine {
    core_tools: Vec<ToolMetadata>,
    specialized_tools: Vec<SpecializedTool>,
}

impl DiscoveryEngine {
    /// Create a new discovery engine with all tools loaded
    pub fn new() -> Self {
        Self {
            core_tools: core::core_tools(),
            specialized_tools: specialized_tools(),
        }
    }

    /// Discover tools based on user query and context
    ///
    /// # Arguments
    ///
    /// * `user_query` — The user's message/question
    /// * `recent_outputs` — Recent tool output to check for secondary triggers
    ///
    /// # Returns
    ///
    /// A `DiscoveryResult` containing:
    /// - Core tools (always included)
    /// - Discovered specialized tools (based on triggers)
    /// - Total estimated token cost
    pub fn discover(
        &self,
        user_query: &str,
        recent_outputs: &[String],
    ) -> crate::tools::metadata::DiscoveryResult {
        let mut discovered = Vec::new();

        for tool in &self.specialized_tools {
            if self.should_discover(tool, user_query, recent_outputs) {
                discovered.push(tool.metadata.clone());
            }
        }

        let total_cost = self.calculate_tokens(&self.core_tools, &discovered);

        crate::tools::metadata::DiscoveryResult {
            core: self.core_tools.clone(),
            specialized: discovered,
            total_token_cost: total_cost,
        }
    }

    /// Determine if a specialized tool should be discovered
    fn should_discover(
        &self,
        tool: &SpecializedTool,
        query: &str,
        outputs: &[String],
    ) -> bool {
        tool.should_discover(query, outputs)
    }

    /// Calculate total token cost for a set of tools
    fn calculate_tokens(&self, core: &[ToolMetadata], specialized: &[ToolMetadata]) -> usize {
        core.iter().map(|t| t.token_cost).sum::<usize>()
            + specialized.iter().map(|t| t.token_cost).sum::<usize>()
    }
}

impl Default for DiscoveryEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_engine_new() {
        let engine = DiscoveryEngine::new();
        assert!(!engine.core_tools.is_empty(), "Core tools should be loaded");
        assert!(!engine.specialized_tools.is_empty(), "Specialized tools should be loaded");
    }

    #[test]
    fn test_discovery_returns_result() {
        let engine = DiscoveryEngine::new();
        let result = engine.discover("", &[]);

        assert!(!result.core.is_empty(), "Result should have core tools");
        assert!(result.total_token_cost > 0, "Result should calculate token cost");
    }

    #[test]
    fn test_discover_by_keyword() {
        let engine = DiscoveryEngine::new();
        let result = engine.discover("write a file", &[]);

        let found = result.specialized.iter().any(|t| t.name == "file_write");
        assert!(found, "Should discover file_write by 'write' keyword");
    }

    #[test]
    fn test_no_discovery_for_generic_query() {
        let engine = DiscoveryEngine::new();
        let result = engine.discover("hello world", &[]);

        assert!(
            result.specialized.is_empty(),
            "Generic query should not discover specialized tools"
        );
    }
}
