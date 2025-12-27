//! LLM tool discovery integration
//!
//! Bridges progressive tool discovery with LLM session layer.
//! Provides simple APIs for chat and plan workflows to get
//! the right tools based on user query and context.

use crate::tools::{DiscoveryEngine, system_prompt};

/// Context for tool discovery
///
/// Carries the information needed to determine which tools
/// should be available to the LLM.
#[derive(Debug, Clone)]
pub struct ToolDiscoveryContext {
    /// The user's query/message
    pub user_query: String,

    /// Recent tool outputs that might trigger discovery
    pub recent_outputs: Vec<String>,

    /// Tools that were discovered in this context
    pub discovered_tools: Vec<String>,
}

impl ToolDiscoveryContext {
    /// Create a new discovery context from a user query
    pub fn new(user_query: impl Into<String>) -> Self {
        Self {
            user_query: user_query.into(),
            recent_outputs: Vec::new(),
            discovered_tools: Vec::new(),
        }
    }

    /// Add a recent output that might trigger discovery
    pub fn with_recent_output(mut self, output: impl Into<String>) -> Self {
        self.recent_outputs.push(output.into());
        self
    }

    /// Add multiple recent outputs
    pub fn with_recent_outputs(mut self, outputs: Vec<String>) -> Self {
        self.recent_outputs = outputs;
        self
    }
}

/// Discover tools for chat mode
///
/// Returns just the tool names that should be available.
/// Chat mode uses a simpler tool set than plan mode.
pub fn discover_tools_for_chat(context: &ToolDiscoveryContext) -> Vec<String> {
    let engine = DiscoveryEngine::new();
    let discovery = engine.discover(&context.user_query, &context.recent_outputs);

    let mut tools: Vec<String> = discovery
        .core
        .iter()
        .chain(discovery.specialized.iter())
        .map(|t| t.name.clone())
        .collect();

    // Sort for determinism
    tools.sort();
    tools
}

/// Discover tools for plan mode
///
/// Returns both tool names and the generated system prompt
/// with tool descriptions and examples.
pub fn discover_tools_for_plan(context: &ToolDiscoveryContext) -> (Vec<String>, String) {
    let engine = DiscoveryEngine::new();
    let discovery = engine.discover(&context.user_query, &context.recent_outputs);

    let mut tools: Vec<String> = discovery
        .core
        .iter()
        .chain(discovery.specialized.iter())
        .map(|t| t.name.clone())
        .collect();

    // Sort for determinism
    tools.sort();

    // Generate system prompt with tool descriptions
    let prompt = system_prompt(&discovery);

    (tools, prompt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_context_new() {
        let context = ToolDiscoveryContext::new("test query");
        assert_eq!(context.user_query, "test query");
        assert!(context.recent_outputs.is_empty());
        assert!(context.discovered_tools.is_empty());
    }

    #[test]
    fn test_discovery_context_with_recent_output() {
        let context = ToolDiscoveryContext::new("test")
            .with_recent_output("error");
        assert_eq!(context.recent_outputs.len(), 1);
    }

    #[test]
    fn test_discover_tools_for_chat_returns_tools() {
        let context = ToolDiscoveryContext::new("hello");
        let tools = discover_tools_for_chat(&context);
        assert!(!tools.is_empty());
    }

    #[test]
    fn test_discover_tools_for_chat_discovers_write() {
        let context = ToolDiscoveryContext::new("write a file");
        let tools = discover_tools_for_chat(&context);
        assert!(tools.contains(&"file_write".to_string()));
    }

    #[test]
    fn test_discover_tools_for_plan_returns_prompt() {
        let context = ToolDiscoveryContext::new("read file");
        let (tools, prompt) = discover_tools_for_plan(&context);
        assert!(!tools.is_empty());
        assert!(!prompt.is_empty());
        assert!(prompt.contains("Tool Selection Guidelines"));
    }
}
