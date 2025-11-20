//! JavaScript-specific language analyzer

use crate::language_analyzers::core::analysis_utils::AnalysisUtils;
use crate::language_analyzers::core::base_analyzer::LanguageAnalyzer;
use crate::{CodeIssue, CodeSuggestion, IssueType, Severity, SuggestionType};
use anyhow::Result;
use tree_sitter::{Node, Tree};

/// JavaScript-specific analyzer
pub struct JavaScriptAnalyzer;

impl JavaScriptAnalyzer {
    pub fn new() -> Self {
        JavaScriptAnalyzer
    }

    fn analyze_js_issues(
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
                    &["//", "/*"],
                ) {
                    issues.push(issue);
                }
            }
            "binary_expression" => {
                let content = &file_content[node.start_byte()..node.end_byte()];
                if content.contains("==") && !content.contains("===") {
                    issues.push(CodeIssue {
                        id: uuid::Uuid::new_v4(),
                        issue_type: IssueType::PotentialBug,
                        severity: Severity::High,
                        description: "Use of == instead of === for comparison".to_string(),
                        line_number: node.start_position().row + 1,
                        column_number: node.start_position().column,
                        suggestion: Some(
                            "Use === for comparison to avoid type coercion".to_string(),
                        ),
                    });
                }
            }
            "variable_declaration" => {
                let content = &file_content[node.start_byte()..node.end_byte()];
                if content.contains("var ") {
                    issues.push(CodeIssue {
                        id: uuid::Uuid::new_v4(),
                        issue_type: IssueType::BestPractice,
                        severity: Severity::Medium,
                        description: "Use of 'var' instead of 'let' or 'const'".to_string(),
                        line_number: node.start_position().row + 1,
                        column_number: node.start_position().column,
                        suggestion: Some(
                            "Use 'let' or 'const' instead of 'var' for better scoping".to_string(),
                        ),
                    });
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.analyze_js_issues(child, file_content, issues)?;
        }

        Ok(())
    }

    fn generate_js_suggestions(
        &self,
        node: Node,
        file_content: &str,
        suggestions: &mut Vec<CodeSuggestion>,
    ) -> Result<()> {
        match node.kind() {
            "function_declaration" | "arrow_function" => {
                // Check function complexity
                let complexity = self.calculate_complexity(node, file_content);
                if complexity > 10 {
                    suggestions.push(AnalysisUtils::create_complexity_suggestion(
                        "function", "//",
                    ));
                }
            }
            "variable_declaration" => {
                let content = &file_content[node.start_byte()..node.end_byte()];
                if content.contains("var ") {
                    suggestions.push(CodeSuggestion::new_complete(
                        "Use 'let' or 'const' instead of 'var'",
                        "Prefer 'let' or 'const' over 'var' for better scoping",
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
            self.generate_js_suggestions(child, file_content, suggestions)?;
        }

        Ok(())
    }

    fn calculate_complexity(&self, node: Node, _file_content: &str) -> u32 {
        let complexity_nodes = [
            "if_statement",
            "for_statement",
            "while_statement",
            "do_statement",
            "switch_statement",
            "catch_clause",
            "conditional_expression",
            "logical_and",
            "logical_or",
        ];
        AnalysisUtils::calculate_complexity(node, &complexity_nodes)
    }
}

impl LanguageAnalyzer for JavaScriptAnalyzer {
    fn analyze_issues(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        self.analyze_js_issues(tree.root_node(), file_content, &mut issues)?;
        Ok(issues)
    }

    fn generate_suggestions(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();
        self.generate_js_suggestions(tree.root_node(), file_content, &mut suggestions)?;
        Ok(suggestions)
    }
}
