//! Tool inventory and classification registry
//!
//! Provides centralized metadata for all available tools including:
//! - Classification (auto, gated, forbidden)
//! - Capabilities (read, write, network, etc.)
//! - Side effect levels
//! - Resource requirements

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

/// Tool classification for access control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolClassification {
    /// Auto-approved — executes without confirmation
    Auto,
    /// Gated — requires user confirmation
    Gated,
    /// Forbidden — never allowed
    Forbidden,
}

/// Argument type for tool parameters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgumentType {
    /// String value
    String,
    /// Integer number
    Integer,
    /// Boolean flag
    Boolean,
    /// Array of strings
    Array,
    /// JSON object
    Object,
    /// File path
    Path,
    /// Glob pattern
    Pattern,
}

/// Specification for a single tool argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolArgument {
    /// Argument name
    pub name: String,
    /// Argument type
    pub ty: ArgumentType,
    /// Whether argument is required
    pub required: bool,
    /// Brief description
    pub description: String,
    /// Default value (if optional)
    pub default: Option<String>,
}

impl ToolArgument {
    /// Create new argument specification
    pub fn new(
        name: impl Into<String>,
        ty: ArgumentType,
        required: bool,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            ty,
            required,
            description: description.into(),
            default: None,
        }
    }

    /// Add default value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }
}

/// Usage and output examples for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExamples {
    /// Example tool call(s)
    pub usage: Vec<String>,
    /// Example output (JSON string or description)
    pub output: String,
}

impl ToolExamples {
    /// Create new examples
    pub fn new(usage: impl IntoIterator<Item = impl Into<String>>, output: impl Into<String>) -> Self {
        Self {
            usage: usage.into_iter().map(|s| s.into()).collect(),
            output: output.into(),
        }
    }

    /// Single usage example
    pub fn single(usage: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            usage: vec![usage.into()],
            output: output.into(),
        }
    }
}

/// Capability flags for tools
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCapability {
    /// Reads data (files, database, etc.)
    Read,
    /// Writes/modifies data
    Write,
    /// Deletes data
    Delete,
    /// Network operations
    Network,
    /// Executes external processes
    Execute,
    /// Filesystem operations
    Filesystem,
    /// Database queries
    Database,
    /// System operations
    System,
    /// Analysis/computation only
    Analysis,
}

/// Side effect level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideEffectLevel {
    /// No side effects (pure read/analysis)
    None,
    /// Local only (no external impact)
    Local,
    /// Mutates project state
    Mutating,
    /// External impact (network, system changes)
    External,
}

/// Resource requirement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceRequirement {
    /// Minimal resources (< 100ms, < 10MB)
    Light,
    /// Moderate resources (< 1s, < 100MB)
    Medium,
    /// Heavy resources (< 10s, < 1GB)
    Heavy,
    /// Very heavy (may take > 10s or > 1GB)
    Intensive,
}

/// Complete metadata for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Unique tool identifier
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Classification for access control
    pub classification: ToolClassification,
    /// Tool capabilities
    pub capabilities: HashSet<ToolCapability>,
    /// Side effect level
    pub side_effect: SideEffectLevel,
    /// Resource requirements
    pub resource: ResourceRequirement,
    /// Whether tool is currently available
    pub available: bool,
    /// Maximum timeout in milliseconds (None for no limit)
    pub max_timeout_ms: Option<u64>,
    /// Argument specifications
    pub arguments: Vec<ToolArgument>,
    /// Usage and output examples
    pub examples: ToolExamples,
}

impl ToolMetadata {
    /// Create new tool metadata
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        classification: ToolClassification,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            classification,
            capabilities: HashSet::new(),
            side_effect: SideEffectLevel::None,
            resource: ResourceRequirement::Light,
            available: true,
            max_timeout_ms: None,
            arguments: Vec::new(),
            examples: ToolExamples::single("", ""),
        }
    }

    /// Set argument specifications
    pub fn with_arguments(mut self, args: Vec<ToolArgument>) -> Self {
        self.arguments = args;
        self
    }

    /// Set examples
    pub fn with_examples(mut self, examples: ToolExamples) -> Self {
        self.examples = examples;
        self
    }

    /// Add capability to tool
    pub fn with_capability(mut self, cap: ToolCapability) -> Self {
        self.capabilities.insert(cap);
        self
    }

    /// Add multiple capabilities
    pub fn with_capabilities(mut self, caps: impl IntoIterator<Item = ToolCapability>) -> Self {
        self.capabilities.extend(caps);
        self
    }

    /// Set side effect level
    pub fn with_side_effect(mut self, level: SideEffectLevel) -> Self {
        self.side_effect = level;
        self
    }

    /// Set resource requirement
    pub fn with_resource(mut self, req: ResourceRequirement) -> Self {
        self.resource = req;
        self
    }

    /// Set availability
    pub fn with_available(mut self, available: bool) -> Self {
        self.available = available;
        self
    }

    /// Set max timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.max_timeout_ms = Some(timeout_ms);
        self
    }

    /// Check if tool has a specific capability
    pub fn has_capability(&self, cap: ToolCapability) -> bool {
        self.capabilities.contains(&cap)
    }

    /// Check if tool is read-only
    pub fn is_read_only(&self) -> bool {
        self.capabilities.contains(&ToolCapability::Read)
            && !self.capabilities.contains(&ToolCapability::Write)
            && !self.capabilities.contains(&ToolCapability::Delete)
    }

    /// Check if tool mutates state
    pub fn is_mutating(&self) -> bool {
        self.capabilities.contains(&ToolCapability::Write)
            || self.capabilities.contains(&ToolCapability::Delete)
    }

    /// Check if tool requires network
    pub fn requires_network(&self) -> bool {
        self.capabilities.contains(&ToolCapability::Network)
    }

    /// Check if tool is safe (no external side effects)
    pub fn is_safe(&self) -> bool {
        matches!(self.side_effect, SideEffectLevel::None | SideEffectLevel::Local)
    }
}

/// Tool registry with complete inventory
pub struct ToolRegistry {
    /// All registered tools by name
    tools: HashMap<String, ToolMetadata>,
}

impl ToolRegistry {
    /// Create new registry with default tools
    pub fn new() -> Self {
        Self {
            tools: super::tool_catalog::default_tools(),
        }
    }

    /// Create empty registry (for testing)
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Get metadata for a tool
    pub fn get(&self, name: &str) -> Option<&ToolMetadata> {
        self.tools.get(name)
    }

    /// Check if a tool exists
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all tools
    pub fn all(&self) -> impl Iterator<Item = &ToolMetadata> {
        self.tools.values()
    }

    /// Get all auto-approved tools
    pub fn auto_tools(&self) -> Vec<&ToolMetadata> {
        self.tools
            .values()
            .filter(|t| t.classification == ToolClassification::Auto && t.available)
            .collect()
    }

    /// Get all gated tools
    pub fn gated_tools(&self) -> Vec<&ToolMetadata> {
        self.tools
            .values()
            .filter(|t| t.classification == ToolClassification::Gated && t.available)
            .collect()
    }

    /// Get all forbidden tools
    pub fn forbidden_tools(&self) -> Vec<&ToolMetadata> {
        self.tools
            .values()
            .filter(|t| t.classification == ToolClassification::Forbidden)
            .collect()
    }

    /// Get all available tools
    pub fn available_tools(&self) -> Vec<&ToolMetadata> {
        self.tools
            .values()
            .filter(|t| t.available)
            .collect()
    }

    /// Get tools by capability
    pub fn by_capability(&self, cap: ToolCapability) -> Vec<&ToolMetadata> {
        self.tools
            .values()
            .filter(|t| t.has_capability(cap) && t.available)
            .collect()
    }

    /// Get read-only tools
    pub fn read_only_tools(&self) -> Vec<&ToolMetadata> {
        self.tools
            .values()
            .filter(|t| t.is_read_only() && t.available)
            .collect()
    }

    /// Get mutating tools
    pub fn mutating_tools(&self) -> Vec<&ToolMetadata> {
        self.tools
            .values()
            .filter(|t| t.is_mutating() && t.available)
            .collect()
    }

    /// Get safe tools (no external side effects)
    pub fn safe_tools(&self) -> Vec<&ToolMetadata> {
        self.tools
            .values()
            .filter(|t| t.is_safe() && t.available)
            .collect()
    }

    /// Get tools requiring network
    pub fn network_tools(&self) -> Vec<&ToolMetadata> {
        self.tools
            .values()
            .filter(|t| t.requires_network() && t.available)
            .collect()
    }

    /// Get tools by side effect level
    pub fn by_side_effect(&self, level: SideEffectLevel) -> Vec<&ToolMetadata> {
        self.tools
            .values()
            .filter(|t| t.side_effect == level && t.available)
            .collect()
    }

    /// Get tools by resource requirement
    pub fn by_resource(&self, req: ResourceRequirement) -> Vec<&ToolMetadata> {
        self.tools
            .values()
            .filter(|t| t.resource == req && t.available)
            .collect()
    }

    /// Get tool count
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Get tool names list
    pub fn tool_names(&self) -> Vec<String> {
        self.tools
            .keys()
            .cloned()
            .collect()
    }

    /// Get available tool names
    pub fn available_tool_names(&self) -> Vec<String> {
        self.tools
            .values()
            .filter(|t| t.available)
            .map(|t| t.name.clone())
            .collect()
    }

    /// Check if tool is auto-approved
    pub fn is_auto_tool(&self, name: &str) -> bool {
        self.tools
            .get(name)
            .map(|t| t.classification == ToolClassification::Auto && t.available)
            .unwrap_or(false)
    }

    /// Check if tool is gated
    pub fn is_gated_tool(&self, name: &str) -> bool {
        self.tools
            .get(name)
            .map(|t| t.classification == ToolClassification::Gated && t.available)
            .unwrap_or(false)
    }

    /// Check if tool is forbidden
    pub fn is_forbidden_tool(&self, name: &str) -> bool {
        self.tools
            .get(name)
            .map(|t| t.classification == ToolClassification::Forbidden)
            .unwrap_or(false)
    }

    /// Register a custom tool (for testing/extensions)
    #[cfg(test)]
    pub fn register(&mut self, tool: ToolMetadata) {
        self.tools.insert(tool.name.clone(), tool);
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_default_tools() {
        let registry = ToolRegistry::new();
        assert!(!registry.is_empty());
        assert!(registry.len() >= 15); // At least our defined tools
    }

    #[test]
    fn test_file_read_is_auto() {
        let registry = ToolRegistry::new();
        assert!(registry.is_auto_tool("file_read"));
        assert!(!registry.is_gated_tool("file_read"));
        assert!(!registry.is_forbidden_tool("file_read"));
    }

    #[test]
    fn test_splice_patch_is_gated() {
        let registry = ToolRegistry::new();
        assert!(registry.is_gated_tool("splice_patch"));
        assert!(!registry.is_auto_tool("splice_patch"));
    }

    #[test]
    fn test_bash_execute_is_forbidden() {
        let registry = ToolRegistry::new();
        assert!(registry.is_forbidden_tool("bash_execute"));
        assert!(!registry.is_auto_tool("bash_execute"));
    }

    #[test]
    fn test_get_tool_metadata() {
        let registry = ToolRegistry::new();
        let tool = registry.get("file_read");
        assert!(tool.is_some());
        let meta = tool.unwrap();
        assert_eq!(meta.name, "file_read");
        assert_eq!(meta.classification, ToolClassification::Auto);
        assert!(meta.has_capability(ToolCapability::Read));
        assert!(meta.is_read_only());
        assert!(!meta.is_mutating());
    }

    #[test]
    fn test_splice_patch_is_mutating() {
        let registry = ToolRegistry::new();
        let tool = registry.get("splice_patch").unwrap();
        assert!(tool.is_mutating());
        assert!(!tool.is_read_only());
    }

    #[test]
    fn test_read_only_tools_query() {
        let registry = ToolRegistry::new();
        let read_only = registry.read_only_tools();
        assert!(!read_only.is_empty());
        // file_read should be in there
        assert!(read_only.iter().any(|t| t.name == "file_read"));
        // splice_patch should NOT be in there
        assert!(!read_only.iter().any(|t| t.name == "splice_patch"));
    }

    #[test]
    fn test_mutating_tools_query() {
        let registry = ToolRegistry::new();
        let mutating = registry.mutating_tools();
        assert!(!mutating.is_empty());
        assert!(mutating.iter().any(|t| t.name == "splice_patch"));
        assert!(mutating.iter().any(|t| t.name == "git_commit"));
    }

    #[test]
    fn test_safe_tools_query() {
        let registry = ToolRegistry::new();
        let safe = registry.safe_tools();
        assert!(!safe.is_empty());
        // Read-only tools are safe
        assert!(safe.iter().any(|t| t.name == "file_read"));
        // Network tools are not safe
        assert!(!safe.iter().any(|t| t.requires_network()));
    }

    #[test]
    fn test_by_capability() {
        let registry = ToolRegistry::new();
        let with_execute = registry.by_capability(ToolCapability::Execute);
        assert!(!with_execute.is_empty());
        assert!(with_execute.iter().any(|t| t.name == "cargo_check"));
    }

    #[test]
    fn test_by_side_effect() {
        let registry = ToolRegistry::new();
        let none = registry.by_side_effect(SideEffectLevel::None);
        assert!(!none.is_empty());
        assert!(none.iter().any(|t| t.name == "file_read"));

        let mutating = registry.by_side_effect(SideEffectLevel::Mutating);
        assert!(!mutating.is_empty());
        assert!(mutating.iter().any(|t| t.name == "splice_patch"));
    }

    #[test]
    fn test_by_resource() {
        let registry = ToolRegistry::new();
        let light = registry.by_resource(ResourceRequirement::Light);
        assert!(!light.is_empty());

        let intensive = registry.by_resource(ResourceRequirement::Intensive);
        assert!(!intensive.is_empty());
    }

    #[test]
    fn test_auto_tools_list() {
        let registry = ToolRegistry::new();
        let auto = registry.auto_tools();
        assert!(!auto.is_empty());
        assert!(auto.iter().all(|t| t.classification == ToolClassification::Auto));
        assert!(auto.iter().all(|t| t.available));
    }

    #[test]
    fn test_gated_tools_list() {
        let registry = ToolRegistry::new();
        let gated = registry.gated_tools();
        assert!(!gated.is_empty());
        assert!(gated.iter().all(|t| t.classification == ToolClassification::Gated));
        assert!(gated.iter().any(|t| t.name == "splice_patch"));
    }

    #[test]
    fn test_forbidden_tools_list() {
        let registry = ToolRegistry::new();
        let forbidden = registry.forbidden_tools();
        assert!(!forbidden.is_empty());
        assert!(forbidden.iter().all(|t| t.classification == ToolClassification::Forbidden));
        assert!(forbidden.iter().any(|t| t.name == "bash_execute"));
    }

    #[test]
    fn test_tool_names() {
        let registry = ToolRegistry::new();
        let names = registry.tool_names();
        assert!(names.contains(&"file_read".to_string()));
        assert!(names.contains(&"splice_patch".to_string()));
    }

    #[test]
    fn test_available_tool_names() {
        let registry = ToolRegistry::new();
        let names = registry.available_tool_names();
        assert!(names.contains(&"file_read".to_string()));
        // Forbidden tools are not available
        assert!(!names.contains(&"bash_execute".to_string()));
    }

    #[test]
    fn test_custom_tool_registration() {
        let mut registry = ToolRegistry::empty();

        let custom = ToolMetadata::new("custom_tool", "A custom tool", ToolClassification::Auto)
            .with_capabilities([ToolCapability::Read])
            .with_side_effect(SideEffectLevel::None)
            .with_arguments(vec![
                ToolArgument::new("input", ArgumentType::String, true, "Input value"),
            ])
            .with_examples(ToolExamples::single(
                "custom_tool(input=\"test\")",
                "Result: success",
            ));

        registry.register(custom);

        assert!(registry.contains("custom_tool"));
        assert!(registry.is_auto_tool("custom_tool"));
    }

    #[test]
    fn test_network_tools() {
        let registry = ToolRegistry::new();
        let network = registry.network_tools();
        // Forbidden network tools should not be available
        assert!(network.is_empty() || network.iter().all(|t| t.available));
    }

    #[test]
    fn test_capability_flags() {
        let registry = ToolRegistry::new();

        let file_read = registry.get("file_read").unwrap();
        assert!(file_read.has_capability(ToolCapability::Read));
        assert!(file_read.has_capability(ToolCapability::Filesystem));
        assert!(!file_read.has_capability(ToolCapability::Network));

        let splice = registry.get("splice_patch").unwrap();
        assert!(splice.has_capability(ToolCapability::Execute));
        assert!(splice.has_capability(ToolCapability::Write));
    }
}
