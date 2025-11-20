//! Shell-specific language analyzer

use crate::language_analyzers::core::analysis_utils::AnalysisUtils;
use crate::language_analyzers::core::base_analyzer::LanguageAnalyzer;
use crate::{CodeIssue, CodeSuggestion, IssueType, Severity, SuggestionType};
use anyhow::Result;
use tree_sitter::{Node, Tree};

/// Shell-specific analyzer
pub struct ShellAnalyzer;

impl ShellAnalyzer {
    pub fn new() -> Self {
        ShellAnalyzer
    }

    fn analyze_shell_issues(
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
                // Check if script has proper shebang at the beginning
                if node.start_position().row == 0 && !file_content.starts_with("#!") {
                    issues.push(CodeIssue {
                        id: uuid::Uuid::new_v4(),
                        issue_type: IssueType::BestPractice,
                        severity: Severity::Low,
                        description:
                            "Shell script should start with a shebang (#!/bin/bash or #!/bin/sh)"
                                .to_string(),
                        line_number: 1,
                        column_number: 0,
                        suggestion: Some("Add shebang at the beginning of the script".to_string()),
                    });
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.analyze_shell_issues(child, file_content, issues)?;
        }

        Ok(())
    }

    fn generate_shell_suggestions(
        &self,
        node: Node,
        file_content: &str,
        suggestions: &mut Vec<CodeSuggestion>,
    ) -> Result<()> {
        match node.kind() {
            "function_definition" => {
                if node.start_position().row == 0 && !file_content.starts_with("#!") {
                    suggestions.push(CodeSuggestion::new_complete(
                        "Add shebang at the beginning of the script",
                        "Shell scripts should start with a shebang line",
                        Severity::Warning,
                        None,
                        Some(1),
                        false,
                    ));
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.generate_shell_suggestions(child, file_content, suggestions)?;
        }

        Ok(())
    }
}

impl LanguageAnalyzer for ShellAnalyzer {
    fn analyze_issues(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        self.analyze_shell_issues(tree.root_node(), file_content, &mut issues)?;
        Ok(issues)
    }

    fn generate_suggestions(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();
        self.generate_shell_suggestions(tree.root_node(), file_content, &mut suggestions)?;
        Ok(suggestions)
    }
}
