//! PHP-specific language analyzer

use crate::language_analyzers::core::analysis_utils::AnalysisUtils;
use crate::language_analyzers::core::base_analyzer::LanguageAnalyzer;
use crate::{CodeIssue, CodeSuggestion, IssueType, Severity, SuggestionType};
use anyhow::Result;
use tree_sitter::{Node, Tree};

/// PHP-specific analyzer
pub struct PhpAnalyzer;

impl PhpAnalyzer {
    pub fn new() -> Self {
        PhpAnalyzer
    }

    fn analyze_php_issues(
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
                    &["//", "/*", "#"],
                ) {
                    issues.push(issue);
                }
            }
            "class_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = &file_content[name_node.start_byte()..name_node.end_byte()];
                    if !name.chars().next().unwrap_or('_').is_uppercase() {
                        issues.push(CodeIssue {
                            id: uuid::Uuid::new_v4(),
                            issue_type: IssueType::Style,
                            severity: Severity::Medium,
                            description: format!(
                                "PHP class name '{}' should start with uppercase letter",
                                name
                            ),
                            line_number: name_node.start_position().row + 1,
                            column_number: name_node.start_position().column,
                            suggestion: Some("Use PascalCase for class names".to_string()),
                        });
                    }
                }
            }
            "function_definition" | "method_declaration" => {
                let complexity = self.calculate_complexity(node, file_content);
                if complexity > 10 {
                    issues.push(CodeIssue {
                        id: uuid::Uuid::new_v4(),
                        issue_type: IssueType::Style,
                        severity: Severity::Medium,
                        description: format!(
                            "Function has high cyclomatic complexity: {}",
                            complexity
                        ),
                        line_number: node.start_position().row + 1,
                        column_number: node.start_position().column,
                        suggestion: Some("Consider breaking down complex functions".to_string()),
                    });
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.analyze_php_issues(child, file_content, issues)?;
        }

        Ok(())
    }

    fn generate_php_suggestions(
        &self,
        node: Node,
        file_content: &str,
        suggestions: &mut Vec<CodeSuggestion>,
    ) -> Result<()> {
        match node.kind() {
            "function_definition" | "method_declaration" => {
                let complexity = self.calculate_complexity(node, file_content);
                if complexity > 10 {
                    suggestions.push(AnalysisUtils::create_complexity_suggestion(
                        "function", "//",
                    ));
                }
            }
            "class_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = &file_content[name_node.start_byte()..name_node.end_byte()];
                    if !name.chars().next().unwrap_or('_').is_uppercase() {
                        suggestions.push(CodeSuggestion::new_complete(
                            "Use PascalCase for class names",
                            "Class names should follow PascalCase convention",
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
            self.generate_php_suggestions(child, file_content, suggestions)?;
        }

        Ok(())
    }

    fn calculate_complexity(&self, node: Node, _file_content: &str) -> u32 {
        let complexity_nodes = [
            "if_statement",
            "elseif_clause",
            "else_clause",
            "for_statement",
            "foreach_statement",
            "while_statement",
            "do_statement",
            "switch_statement",
            "case",
            "catch_clause",
            "logical_and",
            "logical_or",
            "ternary_expression",
        ];
        AnalysisUtils::calculate_complexity(node, &complexity_nodes)
    }
}

impl LanguageAnalyzer for PhpAnalyzer {
    fn analyze_issues(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        self.analyze_php_issues(tree.root_node(), file_content, &mut issues)?;
        Ok(issues)
    }

    fn generate_suggestions(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();
        self.generate_php_suggestions(tree.root_node(), file_content, &mut suggestions)?;
        Ok(suggestions)
    }
}
