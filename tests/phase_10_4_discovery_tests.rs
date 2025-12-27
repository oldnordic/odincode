//! Phase 10.4: Discovery Engine tests
//!
//! TDD approach: Tests for the tool discovery engine that determines
//! which specialized tools to load based on user query and context.

use odincode::tools::{core_tools, DiscoveryEngine, DiscoveryTrigger};

// ===== Basic Structure Tests =====

#[test]
fn test_discovery_engine_new() {
    let engine = DiscoveryEngine::new();
    // Should create without panicking — verify we can call it
    let result = engine.discover("", &[]);
    // Core tools should always be present
    assert!(!result.core.is_empty(), "Core tools should never be empty");
}

#[test]
fn test_discovery_engine_discover_returns_discovery_result() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("test query", &[]);

    // Should return a DiscoveryResult with all three fields
    assert!(!result.core.is_empty(), "Core tools should be present");
    // Specialized starts empty for generic query
    assert!(result.specialized.is_empty(), "No specialized tools for generic query");
    // Token cost should be calculated
    assert!(result.total_token_cost > 0, "Token cost should be calculated");
}

// ===== Core Tools Tests =====

#[test]
fn test_discovery_always_includes_core_tools() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("", &[]);

    // Core tools should match the 5 from core_tools()
    let expected_core = core_tools();
    assert_eq!(
        result.core.len(),
        expected_core.len(),
        "Should include exactly {} core tools",
        expected_core.len()
    );

    // Names should match
    let core_names: Vec<_> = result.core.iter().map(|t| &t.name).collect();
    let expected_names: Vec<_> = expected_core.iter().map(|t| &t.name).collect();
    assert_eq!(
        core_names, expected_names,
        "Core tool names should match"
    );
}

#[test]
fn test_core_tools_have_expected_names() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("", &[]);

    let core_names: Vec<_> = result.core.iter().map(|t| t.name.as_str()).collect();

    // Expected core tools from design
    assert!(core_names.contains(&"file_read"), "file_read should be core");
    assert!(core_names.contains(&"file_search"), "file_search should be core");
    assert!(core_names.contains(&"splice_patch"), "splice_patch should be core");
    assert!(core_names.contains(&"bash_exec"), "bash_exec should be core");
    assert!(core_names.contains(&"display_text"), "display_text should be core");
}

// ===== Keyword Discovery Tests =====

#[test]
fn test_discovers_file_write_on_write_keyword() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("I need to write a file", &[]);

    let discovered_names: Vec<_> = result.specialized.iter().map(|t| t.name.as_str()).collect();
    assert!(discovered_names.contains(&"file_write"), "file_write should be discovered by 'write' keyword");
}

#[test]
fn test_discovers_file_create_on_create_keyword() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("Create a new file", &[]);

    let discovered_names: Vec<_> = result.specialized.iter().map(|t| t.name.as_str()).collect();
    assert!(discovered_names.contains(&"file_create"), "file_create should be discovered by 'create' keyword");
}

#[test]
fn test_discovers_git_tools_on_git_keyword() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("Check git status", &[]);

    let discovered_names: Vec<_> = result.specialized.iter().map(|t| t.name.as_str()).collect();
    assert!(discovered_names.contains(&"git_status"), "git_status should be discovered by 'git' keyword");
}

#[test]
fn test_discovers_lsp_check_on_error_keyword() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("There's an error in my code", &[]);

    let discovered_names: Vec<_> = result.specialized.iter().map(|t| t.name.as_str()).collect();
    assert!(discovered_names.contains(&"lsp_check"), "lsp_check should be discovered by 'error' keyword");
}

#[test]
fn test_keyword_matching_is_case_insensitive() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("I need to WRITE this FILE", &[]);

    let discovered_names: Vec<_> = result.specialized.iter().map(|t| t.name.as_str()).collect();
    assert!(discovered_names.contains(&"file_write"), "file_write should be discovered with uppercase 'WRITE'");
}

// ===== InOutput Trigger Tests =====

#[test]
fn test_discovers_lsp_check_on_error_in_output() {
    let engine = DiscoveryEngine::new();
    let outputs = vec![
        "error[E0277]: expected type, found i32".to_string(),
    ];
    let result = engine.discover("What's wrong?", &outputs);

    let discovered_names: Vec<_> = result.specialized.iter().map(|t| t.name.as_str()).collect();
    assert!(discovered_names.contains(&"lsp_check"), "lsp_check should be discovered by 'error' in recent output");
}

#[test]
fn test_in_output_trigger_is_case_insensitive() {
    let engine = DiscoveryEngine::new();
    let outputs = vec![
        "ERROR: compilation failed".to_string(),
    ];
    let result = engine.discover("Help", &outputs);

    let discovered_names: Vec<_> = result.specialized.iter().map(|t| t.name.as_str()).collect();
    assert!(discovered_names.contains(&"lsp_check"), "lsp_check should be discovered by 'ERROR' in output");
}

// ===== Token Cost Calculation Tests =====

#[test]
fn test_token_cost_includes_core_tools() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("", &[]);

    // With no specialized tools discovered, total should equal core tools cost
    let core_cost: usize = result.core.iter().map(|t| t.token_cost).sum();
    assert_eq!(
        result.total_token_cost, core_cost,
        "Token cost should equal core tools cost when no specialized tools discovered"
    );
}

#[test]
fn test_token_cost_includes_specialized_tools() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("write and create files", &[]);

    // Should discover file_write and file_create
    let specialized_cost: usize = result.specialized.iter().map(|t| t.token_cost).sum();
    let core_cost: usize = result.core.iter().map(|t| t.token_cost).sum();

    assert!(specialized_cost > 0, "Should have specialized tool costs");
    assert_eq!(
        result.total_token_cost,
        core_cost + specialized_cost,
        "Total token cost should equal core + specialized"
    );
}

#[test]
fn test_total_token_cost_is_reasonable() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("", &[]);

    // Core tools should cost less than 2000 tokens
    assert!(
        result.total_token_cost < 2000,
        "Core tool cost {} should be < 2000 tokens",
        result.total_token_cost
    );
}

// ===== DiscoveryResult Methods Tests =====

#[test]
fn test_visible_tools_excludes_internal() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("", &[]);

    let visible = result.visible_tools();

    // All core tools should be visible
    assert_eq!(
        visible.len(),
        result.core.len(),
        "All core tools should be visible"
    );
}

#[test]
fn test_tool_names_returns_all_names() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("", &[]);

    let names = result.tool_names();

    // Should have exactly as many names as core tools
    assert_eq!(
        names.len(),
        result.core.len(),
        "Should have one name per tool"
    );

    // All names should be unique
    let unique_names: std::collections::HashSet<_> = names.iter().collect();
    assert_eq!(
        unique_names.len(),
        names.len(),
        "All tool names should be unique"
    );
}

// ===== Multiple Tools Discovery Tests =====

#[test]
fn test_discovers_multiple_tools_from_single_query() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("write file and check git diff", &[]);

    let discovered_names: Vec<_> = result.specialized.iter().map(|t| t.name.as_str()).collect();

    assert!(discovered_names.contains(&"file_write"), "Should discover file_write");
    assert!(discovered_names.contains(&"git_diff"), "Should discover git_diff");
}

#[test]
fn test_no_specialized_tools_for_generic_query() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("hello world", &[]);

    assert!(
        result.specialized.is_empty(),
        "Generic query should not discover any specialized tools"
    );
}

// ===== DiscoveryTrigger Integration Tests =====

#[test]
fn test_specialized_tool_with_keyword_trigger() {
    // Verify that Keyword triggers work as expected
    use odincode::tools::SpecializedTool;

    let tool = SpecializedTool::new("test_tool", "A test", 100)
        .with_trigger(DiscoveryTrigger::Keyword("test".to_string()));

    assert!(
        tool.should_discover("this is a test", &[]),
        "Keyword trigger should match query"
    );
    assert!(
        !tool.should_discover("no match here", &[]),
        "Keyword trigger should not match unrelated query"
    );
}

#[test]
fn test_specialized_tool_with_in_output_trigger() {
    use odincode::tools::SpecializedTool;

    let tool = SpecializedTool::new("test_tool", "A test", 100)
        .with_trigger(DiscoveryTrigger::InOutput("error".to_string()));

    assert!(
        tool.should_discover("help", &["error: something went wrong".to_string()]),
        "InOutput trigger should match output"
    );
    assert!(
        !tool.should_discover("help", &["everything is fine".to_string()]),
        "InOutput trigger should not match unrelated output"
    );
}

#[test]
fn test_specialized_tool_with_multiple_triggers() {
    use odincode::tools::SpecializedTool;

    let tool = SpecializedTool::new("test_tool", "A test", 100)
        .with_trigger(DiscoveryTrigger::Keyword("write".to_string()))
        .with_trigger(DiscoveryTrigger::InOutput("error".to_string()));

    assert!(
        tool.should_discover("write a file", &[]),
        "Should discover via keyword trigger"
    );
    assert!(
        tool.should_discover("help", &["error: failed".to_string()]),
        "Should discover via output trigger"
    );
    assert!(
        !tool.should_discover("hello", &["everything fine".to_string()]),
        "Should not discover without matching trigger"
    );
}
