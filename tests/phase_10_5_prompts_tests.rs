//! Phase 10.5: System Prompt Generation tests
//!
//! TDD approach: Tests for generating formatted tool descriptions
//! with examples for LLM consumption.

use odincode::tools::{
    core_tools, DiscoveryEngine, format_discovery_result, format_tool, format_tools,
    estimate_tokens, system_prompt, system_prompt_with_metadata, ToolExample, ToolMetadata,
};

// ===== Basic Formatting Tests =====

#[test]
fn test_format_single_tool_description() {

    let tool = ToolMetadata {
        name: "test_tool".to_string(),
        category: odincode::tools::ToolCategory::Core,
        description: "A test tool".to_string(),
        examples: vec![],
        not_examples: vec![],
        token_cost: 100,
        gated: false,
    };

    let formatted = format_tool(&tool);

    assert!(formatted.contains("test_tool"), "Should contain tool name");
    assert!(formatted.contains("A test tool"), "Should contain description");
}

#[test]
fn test_format_tool_with_examples() {


    let tool = ToolMetadata {
        name: "file_read".to_string(),
        category: odincode::tools::ToolCategory::Core,
        description: "Read file contents".to_string(),
        examples: vec![
            ToolExample {
                scenario: "Read a known file".to_string(),
                command: "file_read src/main.rs".to_string(),
                reasoning: "Direct read when path is known".to_string(),
            },
        ],
        not_examples: vec![
            ToolExample {
                scenario: "Search for files".to_string(),
                command: "file_search pattern".to_string(),
                reasoning: "Use file_search for discovery".to_string(),
            },
        ],
        token_cost: 100,
        gated: false,
    };

    let formatted = format_tool(&tool);

    assert!(formatted.contains("**file_read**"), "Should have bold tool name");
    assert!(formatted.contains("Read file contents"), "Should have description");
    assert!(formatted.contains("When to use:"), "Should have when-to-use section");
    assert!(formatted.contains("Read a known file"), "Should have example scenario");
    assert!(formatted.contains("file_read src/main.rs"), "Should have example command");
    assert!(formatted.contains("Direct read when path is known"), "Should have reasoning");
    assert!(formatted.contains("When NOT to use:"), "Should have when-not-to-use section");
}

#[test]
fn test_format_all_core_tools() {


    let core = core_tools();
    let formatted = format_tools(&core);

    // Should contain all core tool names
    assert!(formatted.contains("file_read"), "Should include file_read");
    assert!(formatted.contains("file_search"), "Should include file_search");
    assert!(formatted.contains("splice_patch"), "Should include splice_patch");
    assert!(formatted.contains("bash_exec"), "Should include bash_exec");
    assert!(formatted.contains("display_text"), "Should include display_text");
}

#[test]
fn test_format_discovery_result() {


    let engine = DiscoveryEngine::new();
    let result = engine.discover("write a file", &[]);

    let formatted = format_discovery_result(&result);

    // Should have section header
    assert!(formatted.contains("## Available Tools"), "Should have section header");

    // Should have core tools section
    assert!(formatted.contains("### Core Tools"), "Should have core tools section");

    // Should have discovered specialized tools (file_write for this query)
    assert!(formatted.contains("file_write"), "Should include discovered tool");
}

// ===== System Prompt Tests =====

#[test]
fn test_system_prompt_includes_tool_guidance() {


    let engine = DiscoveryEngine::new();
    let result = engine.discover("", &[]);

    let prompt = system_prompt(&result);

    assert!(prompt.contains("## Tool Selection Guidelines"), "Should have guidelines header");
    assert!(prompt.contains("### Core Tools"), "Should have core tools section");
}

#[test]
fn test_system_prompt_includes_usage_principles() {


    let engine = DiscoveryEngine::new();
    let result = engine.discover("", &[]);

    let prompt = system_prompt(&result);

    assert!(prompt.contains("### Tool Usage Principles"), "Should have principles section");
    assert!(prompt.contains("Be specific"), "Should mention specificity");
}

#[test]
fn test_system_prompt_includes_common_mistakes() {


    let engine = DiscoveryEngine::new();
    let result = engine.discover("", &[]);

    let prompt = system_prompt(&result);

    assert!(prompt.contains("### Common Mistakes to Avoid"), "Should have mistakes section");
    assert!(prompt.contains("Don't use"), "Should have negative examples");
}

// ===== Gated Tool Tests =====

#[test]
fn test_gated_tool_marked_in_output() {


    let tool = ToolMetadata {
        name: "dangerous_tool".to_string(),
        category: odincode::tools::ToolCategory::Specialized,
        description: "Requires approval".to_string(),
        examples: vec![],
        not_examples: vec![],
        token_cost: 100,
        gated: true,
    };

    let formatted = format_tool(&tool);

    assert!(formatted.contains("GATED"), "Should mark tool as gated");
    assert!(formatted.contains("approval"), "Should mention approval");
    assert!(formatted.contains("⚠️"), "Should have warning emoji");
}

// ===== Empty Discovery Tests =====

#[test]
fn test_format_empty_specialized_tools() {


    let engine = DiscoveryEngine::new();
    let result = engine.discover("hello world", &[]);  // No specialized tools

    let formatted = format_discovery_result(&result);

    // Should still have core tools
    assert!(formatted.contains("### Core Tools"), "Should have core tools section");
    assert!(formatted.contains("file_read"), "Should include core tools");
}

// ===== Structured Output Tests =====

#[test]
fn test_tool_formatting_has_consistent_structure() {


    let core = core_tools();
    let formatted = format_tools(&core);

    // Each tool should be separated by blank lines
    assert!(formatted.contains("\n\n"), "Should have blank line separators");

    // Each tool should start with bold name marker
    let tool_count = formatted.matches("**").count();
    assert!(tool_count >= core.len() * 2, "Each tool should have **name**");
}

// ===== Specialized Tools Section Tests =====

#[test]
fn test_specialized_tools_section_when_discovered() {


    let engine = DiscoveryEngine::new();
    let result = engine.discover("write file and check git", &[]);

    let formatted = format_discovery_result(&result);

    // Should have specialized tools section
    assert!(formatted.contains("### Specialized Tools"), "Should have specialized section");
    assert!(formatted.contains("file_write"), "Should include file_write");
    assert!(formatted.contains("git_status"), "Should include git_status");
}

#[test]
fn test_no_specialized_section_when_none_discovered() {


    let engine = DiscoveryEngine::new();
    let result = engine.discover("hello", &[]);  // No specialized tools

    let formatted = format_discovery_result(&result);

    // Should NOT have specialized tools section
    assert!(!formatted.contains("### Specialized Tools"), "Should not have specialized section when empty");
}

// ===== Token Estimation Tests =====

#[test]
fn test_estimate_prompt_tokens() {


    let text = "This is a test prompt with some words";
    let tokens = estimate_tokens(text);

    // Simple heuristic: ~4 chars per token
    assert!(tokens > 0, "Should estimate some tokens");
    assert!(tokens < text.len(), "Token count should be less than char count");
}

#[test]
fn test_discovery_result_includes_token_estimate() {


    let engine = DiscoveryEngine::new();
    let result = engine.discover("", &[]);

    let (prompt, metadata) = system_prompt_with_metadata(&result);

    assert!(!prompt.is_empty(), "Prompt should not be empty");
    assert!(metadata.estimated_tokens > 0, "Should estimate token count");
    assert_eq!(metadata.tool_count, result.core.len(), "Tool count should match core tools");
}
