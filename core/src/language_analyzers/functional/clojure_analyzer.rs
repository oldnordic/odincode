//! Clojure-specific language analyzer

use crate::language_analyzers::core::analysis_utils::AnalysisUtils;
use crate::language_analyzers::core::base_analyzer::LanguageAnalyzer;
use crate::{CodeIssue, CodeSuggestion, IssueType, Severity, SuggestionType};
use anyhow::Result;
use tree_sitter::{Node, Tree};

/// Clojure-specific analyzer
pub struct ClojureAnalyzer;

impl ClojureAnalyzer {
    pub fn new() -> Self {
        ClojureAnalyzer
    }

    fn analyze_clojure_issues(
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
                    &[";"],
                ) {
                    issues.push(issue);
                }
            }
            "list_lit" => {
                let depth = self.calculate_nesting_depth(node, file_content);
                if depth > 5 {
                    issues.push(CodeIssue {
                        id: uuid::Uuid::new_v4(),
                        issue_type: IssueType::Style,
                        severity: Severity::Medium,
                        description: format!(
                            "Expression has deep nesting ({} levels). Consider using let-binding or threading macros.",
                            depth
                        ),
                        line_number: node.start_position().row + 1,
                        column_number: node.start_position().column,
                        suggestion: Some("Use let-binding or threading macros to reduce nesting".to_string()),
                    });
                }
            }
            "defn" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = &file_content[name_node.start_byte()..name_node.end_byte()];
                    if name.contains('_') {
                        issues.push(CodeIssue {
                            id: uuid::Uuid::new_v4(),
                            issue_type: IssueType::Style,
                            severity: Severity::Low,
                            description: format!(
                                "Clojure function name '{}' should use kebab-case instead of snake_case",
                                name
                            ),
                            line_number: name_node.start_position().row + 1,
                            column_number: name_node.start_position().column,
                            suggestion: Some("Use kebab-case for function names".to_string()),
                        });
                    }
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.analyze_clojure_issues(child, file_content, issues)?;
        }

        Ok(())
    }

    fn generate_clojure_suggestions(
        &self,
        node: Node,
        file_content: &str,
        suggestions: &mut Vec<CodeSuggestion>,
    ) -> Result<()> {
        match node.kind() {
            "list_lit" => {
                let depth = self.calculate_nesting_depth(node, file_content);
                if depth > 5 {
                    suggestions.push(CodeSuggestion::new_complete(
                        "Deep nesting detected",
                        "Consider using let-binding or threading macros to improve readability",
                        Severity::Warning,
                        None,
                        Some(node.start_position().row as u32 + 1),
                        false,
                    ));
                }
            }
            "defn" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = &file_content[name_node.start_byte()..name_node.end_byte()];
                    if name.contains('_') {
                        suggestions.push(CodeSuggestion::new_complete(
                            "Use kebab-case for function names",
                            "Function names in Clojure should use kebab-case convention",
                            Severity::Warning,
                            None,
                            Some(node.start_position().row as u32 + 1),
                            false,
                        ));
                    }
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.generate_clojure_suggestions(child, file_content, suggestions)?;
        }

        Ok(())
    }

    fn calculate_nesting_depth(&self, node: Node, _file_content: &str) -> usize {
        let mut max_depth = 0;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "list_lit" {
                let child_depth = 1 + self.calculate_nesting_depth(child, _file_content);
                max_depth = max_depth.max(child_depth);
            }
        }

        max_depth
    }
}

impl LanguageAnalyzer for ClojureAnalyzer {
    fn analyze_issues(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        self.analyze_clojure_issues(tree.root_node(), file_content, &mut issues)?;
        Ok(issues)
    }

    fn generate_suggestions(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();
        self.generate_clojure_suggestions(tree.root_node(), file_content, &mut suggestions)?;
        Ok(suggestions)
    }
}
