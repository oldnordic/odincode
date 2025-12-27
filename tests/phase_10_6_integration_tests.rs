//! Phase 10.6: LLM Adapter Integration tests
//!
//! TDD approach: Tests for integrating progressive tool discovery
//! into the LLM session layer.

use odincode::llm::discovery::{
    discover_tools_for_chat, discover_tools_for_plan, ToolDiscoveryContext,
};
use odincode::tools::{core_tools, DiscoveryEngine};

// ===== Basic Discovery Context Tests =====

#[test]
fn test_discovery_context_new() {
    let context = ToolDiscoveryContext::new("help me write code");

    assert_eq!(context.user_query, "help me write code");
    assert!(context.recent_outputs.is_empty());
    assert!(context.discovered_tools.is_empty());
}

#[test]
fn test_discovery_context_with_recent_outputs() {
    let context = ToolDiscoveryContext::new("fix the error")
        .with_recent_output("error[E0277]: type mismatch");

    assert_eq!(context.recent_outputs.len(), 1);
    assert!(context.recent_outputs[0].contains("error"));
}

// ===== Chat Discovery Tests =====

#[test]
fn test_discover_tools_for_chat_returns_core_tools() {
    let context = ToolDiscoveryContext::new("hello world");
    let tools = discover_tools_for_chat(&context);

    // Core tools should always be included
    assert!(!tools.is_empty(), "Should have at least core tools");
}

#[test]
fn test_discover_tools_for_chat_discovers_file_write() {
    let context = ToolDiscoveryContext::new("I need to write a file");
    let tools = discover_tools_for_chat(&context);

    // Should discover file_write based on "write" keyword
    assert!(tools.contains(&"file_write".to_string()), "Should discover file_write");
}

#[test]
fn test_discover_tools_for_chat_discovers_git_tools() {
    let context = ToolDiscoveryContext::new("check git status");
    let tools = discover_tools_for_chat(&context);

    // Should discover git_status based on "git" keyword
    assert!(tools.contains(&"git_status".to_string()), "Should discover git_status");
}

#[test]
fn test_discover_tools_for_chat_with_error_output() {
    let context = ToolDiscoveryContext::new("help")
        .with_recent_output("error[E0277]: expected type, found i32");
    let tools = discover_tools_for_chat(&context);

    // Should discover lsp_check based on "error" in recent output
    assert!(tools.contains(&"lsp_check".to_string()), "Should discover lsp_check from error in output");
}

// ===== Plan Discovery Tests =====

#[test]
fn test_discover_tools_for_plan_returns_core_tools() {
    let context = ToolDiscoveryContext::new("create a new struct");
    let (tool_names, prompt) = discover_tools_for_plan(&context);

    // Core tools should always be included
    assert!(!tool_names.is_empty(), "Should have at least core tools");

    // Prompt should be generated
    assert!(!prompt.is_empty(), "Should generate system prompt");

    // Prompt should contain tool guidelines
    assert!(prompt.contains("Tool Selection Guidelines"), "Should have guidelines section");
}

#[test]
fn test_discover_tools_for_plan_includes_discovered_tools() {
    let context = ToolDiscoveryContext::new("write to file and check git diff");
    let (tool_names, _prompt) = discover_tools_for_plan(&context);

    // Should discover file_write and git_diff
    assert!(tool_names.contains(&"file_write".to_string()), "Should discover file_write");
    assert!(tool_names.contains(&"git_diff".to_string()), "Should discover git_diff");
}

#[test]
fn test_discover_tools_for_plan_generates_proper_prompt() {
    let context = ToolDiscoveryContext::new("read a file");
    let (tool_names, prompt) = discover_tools_for_plan(&context);

    // Prompt should contain available tools section
    assert!(prompt.contains("## Available Tools"), "Should have available tools section");

    // Prompt should mention core tools
    assert!(prompt.contains("### Core Tools"), "Should have core tools section");

    // Tool names should match what's in prompt
    for tool_name in &tool_names {
        assert!(prompt.contains(tool_name), "Prompt should mention {}", tool_name);
    }
}

// ===== Integration with DiscoveryEngine Tests =====

#[test]
fn test_chat_uses_discovery_engine() {
    let engine = DiscoveryEngine::new();
    let context = ToolDiscoveryContext::new("create a file");

    // Direct engine call
    let discovery = engine.discover(&context.user_query, &context.recent_outputs);
    let discovered_names: Vec<_> = discovery
        .core
        .iter()
        .chain(discovery.specialized.iter())
        .map(|t| t.name.clone())
        .collect();

    // Through chat API
    let chat_tools = discover_tools_for_chat(&context);

    // Should have the same tools
    assert_eq!(chat_tools.len(), discovered_names.len());
    for name in &chat_tools {
        assert!(discovered_names.contains(name), "{} should be in discovery", name);
    }
}

#[test]
fn test_plan_uses_discovery_engine() {
    let engine = DiscoveryEngine::new();
    let context = ToolDiscoveryContext::new("write file");

    // Direct engine call
    let discovery = engine.discover(&context.user_query, &context.recent_outputs);
    let discovered_names: Vec<_> = discovery
        .core
        .iter()
        .chain(discovery.specialized.iter())
        .map(|t| t.name.clone())
        .collect();

    // Through plan API
    let (plan_tools, _prompt) = discover_tools_for_plan(&context);

    // Should have the same tools
    assert_eq!(plan_tools.len(), discovered_names.len());
    for name in &plan_tools {
        assert!(discovered_names.contains(name), "{} should be in discovery", name);
    }
}

// ===== Token Cost Tests =====

#[test]
fn test_discovery_context_tracks_token_cost() {
    let engine = DiscoveryEngine::new();
    let context = ToolDiscoveryContext::new("read file");

    let discovery = engine.discover(&context.user_query, &context.recent_outputs);

    assert!(discovery.total_token_cost > 0, "Should calculate token cost");
}

// ===== Tool Whitelist Integration Tests =====

#[test]
fn test_discovered_tools_are_subset_of_whitelist() {
    use odincode::llm::router::TOOL_WHITELIST;

    let context = ToolDiscoveryContext::new("write file and check git status");
    let tools = discover_tools_for_chat(&context);

    // All discovered tools should be in the whitelist
    for tool in &tools {
        assert!(
            TOOL_WHITELIST.contains(&tool.as_str()),
            "{} should be in TOOL_WHITELIST",
            tool
        );
    }
}

#[test]
fn test_core_tools_in_whitelist() {
    use odincode::llm::router::TOOL_WHITELIST;

    let core = core_tools();

    // All core tools should be in the whitelist
    for tool in &core {
        assert!(
            TOOL_WHITELIST.contains(&tool.name.as_str()),
            "{} should be in TOOL_WHITELIST",
            tool.name
        );
    }
}
