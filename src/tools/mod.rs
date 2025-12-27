//! Tool system with progressive discovery
//!
//! # Architecture
//!
//! This module implements progressive tool discovery:
//! - Core tools are always loaded (5 tools, ~1,100 tokens)
//! - Specialized tools are discovered on-demand based on context
//! - Internal tools are never shown to LLM
//!
//! # Module Structure
//!
//! - `metadata`: Tool metadata types and discovery result types
//! - `core`: Core tool definitions (Phase 10.2) ✅
//! - `specialized`: Specialized tool definitions (Phase 10.3) ✅
//! - `discovery`: Discovery engine (Phase 10.4) ✅
//! - `prompts`: System prompt generation (Phase 10.5)
//!
//! # Usage
//!
//! ```rust,no_run
//! use odincode::tools::{core_tools, specialized_tools, ToolCategory};
//!
//! let core = core_tools();
//! let specialized = specialized_tools();
//! // core: 5 tools always loaded
//! // specialized: 15 tools discovered on-demand
//! ```

pub mod core;
pub mod discovery;
pub mod metadata;
pub mod prompts;
pub mod specialized;

// Re-export commonly used types
pub use core::core_tools;
pub use discovery::DiscoveryEngine;
pub use metadata::{
    DiscoveryResult, DiscoveryTrigger, SpecializedTool,
    ToolCategory, ToolExample, ToolMetadata,
};
pub use prompts::{estimate_tokens, format_discovery_result, format_tool, format_tools, PromptMetadata, system_prompt, system_prompt_with_metadata};
pub use specialized::specialized_tools;

// Tests module
#[cfg(test)]
mod tests {
    pub use super::core::*;
    pub use super::metadata::*;
    pub use super::specialized::*;
}

/// Maximum number of tools that should ever be loaded at once
///
/// This prevents context bloat. If more tools are needed,
/// the LLM should be instructed to complete its current
/// task before requesting additional tools.
pub const MAX_TOOLS_AT_ONCE: usize = 10;
