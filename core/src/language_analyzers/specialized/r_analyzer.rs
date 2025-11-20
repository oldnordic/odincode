//! R-specific language analyzer

use crate::language_analyzers::core::analysis_utils::AnalysisUtils;
use crate::language_analyzers::core::base_analyzer::LanguageAnalyzer;
use crate::{CodeIssue, CodeSuggestion, IssueType, Severity, SuggestionType};
use anyhow::Result;
use tree_sitter::{Node, Tree};

/// R-specific analyzer
pub struct RAnalyzer;

impl RAnalyzer {
    pub fn new() -> Self {
        RAnalyzer
    }

    fn analyze_r_issues(
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
                    &["#"],
                ) {
                    issues.push(issue);
                }
            }
            "function_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = &file_content[name_node.start_byte()..name_node.end_byte()];
                    if name.contains('_') {
                        issues.push(CodeIssue {
                            id: uuid::Uuid::new_v4(),
                            issue_type: IssueType::Style,
                            severity: Severity::Low,
                            description: format!(
                                "R function name '{}' should use dots instead of underscores",
                                name
                            ),
                            line_number: name_node.start_position().row + 1,
                            column_number: name_node.start_position().column,
                            suggestion: Some(
                                "Use dots instead of underscores in function names".to_string(),
                            ),
                        });
                    }
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.analyze_r_issues(child, file_content, issues)?;
        }

        Ok(())
    }

    fn generate_r_suggestions(
        &self,
        node: Node,
        file_content: &str,
        suggestions: &mut Vec<CodeSuggestion>,
    ) -> Result<()> {
        match node.kind() {
            "function_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = &file_content[name_node.start_byte()..name_node.end_byte()];
                    if name.contains('_') {
                        suggestions.push(CodeSuggestion::new_complete(
                            "Use dots instead of underscores in function names",
                            "R function names should use dots instead of underscores",
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
            self.generate_r_suggestions(child, file_content, suggestions)?;
        }

        Ok(())
    }
}

impl LanguageAnalyzer for RAnalyzer {
    fn analyze_issues(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        self.analyze_r_issues(tree.root_node(), file_content, &mut issues)?;
        Ok(issues)
    }

    fn generate_suggestions(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();
        self.generate_r_suggestions(tree.root_node(), file_content, &mut suggestions)?;
        Ok(suggestions)
    }
}
