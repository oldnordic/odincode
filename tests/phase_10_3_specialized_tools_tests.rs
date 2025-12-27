//! Phase 10.3: Specialized tool definitions tests
//!
//! TDD approach: Tests for specialized tool metadata with discovery triggers.

use odincode::tools::{specialized_tools, DiscoveryTrigger, ToolCategory};

#[test]
fn test_specialized_tools_returns_fifteen_tools() {
    let tools = specialized_tools();
    assert_eq!(tools.len(), 15, "Should have exactly 15 specialized tools");
}

#[test]
fn test_all_specialized_tools_have_specialized_category() {
    let tools = specialized_tools();
    for tool in &tools {
        assert_eq!(
            tool.metadata.category,
            ToolCategory::Specialized,
            "{} should be Specialized",
            tool.metadata.name
        );
    }
}

#[test]
fn test_all_specialized_tools_valid() {
    let tools = specialized_tools();
    for tool in &tools {
        assert!(
            tool.metadata.is_valid(),
            "{} should be valid (has examples)",
            tool.metadata.name
        );
    }
}

#[test]
fn test_all_specialized_tools_visible_to_llm() {
    let tools = specialized_tools();
    for tool in &tools {
        assert!(
            tool.metadata.visible_to_llm(),
            "{} should be visible to LLM",
            tool.metadata.name
        );
    }
}

#[test]
fn test_all_specialized_tools_have_triggers() {
    let tools = specialized_tools();
    for tool in &tools {
        assert!(
            !tool.triggers.is_empty(),
            "{} should have at least one discovery trigger",
            tool.metadata.name
        );
    }
}

// Individual tool presence tests

#[test]
fn test_file_write_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "file_write"));
}

#[test]
fn test_file_create_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "file_create"));
}

#[test]
fn test_file_glob_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "file_glob"));
}

#[test]
fn test_file_edit_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "file_edit"));
}

#[test]
fn test_splice_plan_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "splice_plan"));
}

#[test]
fn test_symbols_in_file_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "symbols_in_file"));
}

#[test]
fn test_references_to_symbol_name_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "references_to_symbol_name"));
}

#[test]
fn test_references_from_file_to_symbol_name_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "references_from_file_to_symbol_name"));
}

#[test]
fn test_lsp_check_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "lsp_check"));
}

#[test]
fn test_memory_query_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "memory_query"));
}

#[test]
fn test_execution_summary_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "execution_summary"));
}

#[test]
fn test_git_status_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "git_status"));
}

#[test]
fn test_git_diff_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "git_diff"));
}

#[test]
fn test_git_log_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "git_log"));
}

#[test]
fn test_wc_in_specialized_tools() {
    let tools = specialized_tools();
    assert!(tools.iter().any(|t| t.metadata.name == "wc"));
}

// Discovery trigger tests

#[test]
fn test_file_write_has_write_trigger() {
    let tools = specialized_tools();
    let file_write = tools.iter().find(|t| t.metadata.name == "file_write").unwrap();

    assert!(
        file_write.triggers.iter().any(|t| matches!(t, DiscoveryTrigger::Keyword(k) if k.to_lowercase().contains("write"))),
        "file_write should have 'write' keyword trigger"
    );
}

#[test]
fn test_file_create_has_create_trigger() {
    let tools = specialized_tools();
    let file_create = tools.iter().find(|t| t.metadata.name == "file_create").unwrap();

    assert!(
        file_create.triggers.iter().any(|t| matches!(t, DiscoveryTrigger::Keyword(k) if k.to_lowercase().contains("create"))),
        "file_create should have 'create' keyword trigger"
    );
}

#[test]
fn test_lsp_check_has_error_trigger() {
    let tools = specialized_tools();
    let lsp_check = tools.iter().find(|t| t.metadata.name == "lsp_check").unwrap();

    assert!(
        lsp_check.triggers.iter().any(|t| matches!(t, DiscoveryTrigger::Keyword(k) if k.to_lowercase().contains("error") || k.to_lowercase().contains("diagnostic"))),
        "lsp_check should have 'error' or 'diagnostic' keyword trigger"
    );
}

#[test]
fn test_git_tools_have_git_trigger() {
    let tools = specialized_tools();
    let git_tools: Vec<_> = tools
        .iter()
        .filter(|t| t.metadata.name.starts_with("git_"))
        .collect();

    assert_eq!(git_tools.len(), 3, "Should have 3 git tools");

    for tool in git_tools {
        assert!(
            tool.triggers.iter().any(|t| matches!(t, DiscoveryTrigger::Keyword(k) if k.to_lowercase().contains("git"))),
            "{} should have 'git' keyword trigger",
            tool.metadata.name
        );
    }
}

// Example validation tests

#[test]
fn test_all_specialized_tools_have_examples() {
    let tools = specialized_tools();
    for tool in &tools {
        assert!(
            !tool.metadata.examples.is_empty(),
            "{} should have examples",
            tool.metadata.name
        );
    }
}

#[test]
fn test_all_specialized_tools_have_not_examples() {
    let tools = specialized_tools();
    for tool in &tools {
        assert!(
            !tool.metadata.not_examples.is_empty(),
            "{} should have not_examples",
            tool.metadata.name
        );
    }
}

#[test]
fn test_all_examples_have_reasoning() {
    let tools = specialized_tools();

    for tool in &tools {
        for example in &tool.metadata.examples {
            assert!(
                !example.reasoning.is_empty(),
                "{} example '{}' should have reasoning",
                tool.metadata.name,
                example.scenario
            );
        }

        for example in &tool.metadata.not_examples {
            assert!(
                !example.reasoning.is_empty(),
                "{} not_example '{}' should have reasoning",
                tool.metadata.name,
                example.scenario
            );
        }
    }
}

#[test]
fn test_splice_plan_discoverable_by_multi_step_keywords() {
    let tools = specialized_tools();
    let splice_plan = tools.iter().find(|t| t.metadata.name == "splice_plan").unwrap();

    let query = "I need to do a multi-step refactoring";
    assert!(
        splice_plan.should_discover(query, &[]),
        "splice_plan should be discoverable by 'multi-step' keyword"
    );
}

#[test]
fn test_symbols_in_file_discoverable_by_function_keyword() {
    let tools = specialized_tools();
    let symbols = tools.iter().find(|t| t.metadata.name == "symbols_in_file").unwrap();

    let query = "show me all functions in this file";
    assert!(
        symbols.should_discover(query, &[]),
        "symbols_in_file should be discoverable by 'function' keyword"
    );
}
