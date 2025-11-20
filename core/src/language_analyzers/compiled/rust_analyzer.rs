//! Rust-specific language analyzer

use crate::language_analyzers::core::analysis_utils::AnalysisUtils;
use crate::language_analyzers::core::base_analyzer::LanguageAnalyzer;
use crate::{CodeIssue, CodeSuggestion, IssueType, Severity, SuggestionType};
use anyhow::Result;
use tree_sitter::{Node, Tree};

/// Rust-specific analyzer
pub struct RustAnalyzer;

impl RustAnalyzer {
    pub fn new() -> Self {
        RustAnalyzer
    }

    fn analyze_rust_issues(
        &self,
        node: Node,
        file_content: &str,
        issues: &mut Vec<CodeIssue>,
    ) -> Result<()> {
        match node.kind() {
            "line_comment" | "block_comment" => {
                let content = &file_content[node.start_byte()..node.end_byte()];
                if let Some(issue) = AnalysisUtils::check_todo_comments(
                    content,
                    node.start_position().row + 1,
                    node.start_position().column,
                    &["//", "/*"],
                ) {
                    issues.push(issue);
                }
            }
            "call_expression" => {
                let content = &file_content[node.start_byte()..node.end_byte()];
                if content.contains(".collect::<Vec<_>>().len()") {
                    issues.push(CodeIssue {
                        id: uuid::Uuid::new_v4(),
                        issue_type: IssueType::Performance,
                        severity: Severity::High,
                        description: "inefficient length calculation after collect".to_string(),
                        line_number: node.start_position().row + 1,
                        column_number: node.start_position().column,
                        suggestion: Some("Use .count() or .len() directly on iterator".to_string()),
                    });
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.analyze_rust_issues(child, file_content, issues)?;
        }

        Ok(())
    }

    fn generate_rust_suggestions(
        &self,
        node: Node,
        file_content: &str,
        suggestions: &mut Vec<CodeSuggestion>,
    ) -> Result<()> {
        match node.kind() {
            "function_item" => {
                // Check function complexity
                let complexity = self.calculate_complexity(node, file_content);
                if complexity > 10 {
                    suggestions.push(AnalysisUtils::create_complexity_suggestion(
                        "function", "//",
                    ));
                }
            }
            "call_expression" => {
                let content = &file_content[node.start_byte()..node.end_byte()];
                if content.contains(".collect::<Vec<_>>().len()") {
                    suggestions.push(CodeSuggestion::new_complete(
                        "Inefficient length calculation",
                        "Use .count() instead of collecting to Vec and then getting length",
                        Severity::Warning,
                        None,
                        Some(node.start_position().row as u32 + 1),
                        false,
                    ));
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.generate_rust_suggestions(child, file_content, suggestions)?;
        }

        Ok(())
    }

    fn calculate_complexity(&self, node: Node, _file_content: &str) -> u32 {
        let complexity_nodes = [
            "if_expression",
            "if_statement",
            "for_statement",
            "while_statement",
            "loop_expression",
            "match_expression",
            "match_arm",
            "catch_clause",
            "logical_and",
            "logical_or",
        ];
        AnalysisUtils::calculate_complexity(node, &complexity_nodes)
    }
}

impl LanguageAnalyzer for RustAnalyzer {
    fn analyze_issues(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        self.analyze_rust_issues(tree.root_node(), file_content, &mut issues)?;
        Ok(issues)
    }

    fn generate_suggestions(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();
        self.generate_rust_suggestions(tree.root_node(), file_content, &mut suggestions)?;
        Ok(suggestions)
    }
}
