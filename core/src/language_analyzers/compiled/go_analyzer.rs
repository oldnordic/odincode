//! Go-specific language analyzer

use crate::language_analyzers::core::analysis_utils::AnalysisUtils;
use crate::language_analyzers::core::base_analyzer::LanguageAnalyzer;
use crate::{CodeIssue, CodeSuggestion, Severity, SuggestionType};
use anyhow::Result;
use tree_sitter::{Node, Tree};

/// Go-specific analyzer
pub struct GoAnalyzer;

impl GoAnalyzer {
    pub fn new() -> Self {
        GoAnalyzer
    }

    fn analyze_go_issues(
        &self,
        node: Node,
        file_content: &str,
        issues: &mut Vec<CodeIssue>,
    ) -> Result<()> {
        match node.kind() {
            "comment" => {
                let content = &file_content[node.start_byte()..node.end_byte()];
                if let Some(issue) = AnalysisUtils::check_todo_comments(
                    content,
                    node.start_position().row + 1,
                    node.start_position().column,
                    &["//"],
                ) {
                    issues.push(issue);
                }
            }
            "function_declaration" => {
                // Check function complexity
                let complexity = self.calculate_complexity(node, file_content);
                if complexity > 10 {
                    issues.push(AnalysisUtils::create_complexity_issue(
                        node.start_position().row + 1,
                        node.start_position().column,
                        "function",
                    ));
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.analyze_go_issues(child, file_content, issues)?;
        }

        Ok(())
    }

    fn generate_go_suggestions(
        &self,
        node: Node,
        file_content: &str,
        suggestions: &mut Vec<CodeSuggestion>,
    ) -> Result<()> {
        match node.kind() {
            "function_declaration" => {
                // Check function complexity
                let complexity = self.calculate_complexity(node, file_content);
                if complexity > 10 {
                    suggestions.push(AnalysisUtils::create_complexity_suggestion(
                        "function", "//",
                    ));
                }
            }
            "expression_statement" => {
                let content = &file_content[node.start_byte()..node.end_byte()];
                if content.contains("_ = ") {
                    suggestions.push(CodeSuggestion::new_complete(
                        "Discarded return value detected",
                        "Handle error value instead of discarding it",
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
            self.generate_go_suggestions(child, file_content, suggestions)?;
        }

        Ok(())
    }

    fn calculate_complexity(&self, node: Node, _file_content: &str) -> u32 {
        let complexity_nodes = [
            "if_statement",
            "for_statement",
            "while_statement",
            "block",
            "switch_statement",
            "case_statement",
            "if_expression",
            "conditional_expression",
        ];
        AnalysisUtils::calculate_complexity(node, &complexity_nodes)
    }
}

impl LanguageAnalyzer for GoAnalyzer {
    fn analyze_issues(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        self.analyze_go_issues(tree.root_node(), file_content, &mut issues)?;
        Ok(issues)
    }

    fn generate_suggestions(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();
        self.generate_go_suggestions(tree.root_node(), file_content, &mut suggestions)?;
        Ok(suggestions)
    }
}
