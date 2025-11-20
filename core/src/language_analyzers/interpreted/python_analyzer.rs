//! Python-specific language analyzer

use crate::language_analyzers::core::analysis_utils::AnalysisUtils;
use crate::language_analyzers::core::base_analyzer::LanguageAnalyzer;
use crate::{CodeIssue, CodeSuggestion, IssueType, Severity};
use anyhow::Result;
use tree_sitter::{Node, Tree};

/// Python-specific analyzer
pub struct PythonAnalyzer;

impl PythonAnalyzer {
    pub fn new() -> Self {
        PythonAnalyzer
    }

    fn analyze_python_issues(
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
            "import_from_statement" => {
                let content = &file_content[node.start_byte()..node.end_byte()];
                if content.contains("import *") {
                    issues.push(CodeIssue {
                        id: uuid::Uuid::new_v4(),
                        issue_type: IssueType::BestPractice,
                        severity: Severity::Medium,
                        description: "wildcard import found".to_string(),
                        line_number: node.start_position().row + 1,
                        column_number: node.start_position().column,
                        suggestion: Some(
                            "Use explicit imports instead of wildcard imports".to_string(),
                        ),
                    });
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.analyze_python_issues(child, file_content, issues)?;
        }

        Ok(())
    }

    fn generate_python_suggestions(
        &self,
        node: Node,
        file_content: &str,
        suggestions: &mut Vec<CodeSuggestion>,
    ) -> Result<()> {
        match node.kind() {
            "function_definition" => {
                // Check function complexity
                let complexity = self.calculate_complexity(node, file_content);
                if complexity > 10 {
                    suggestions.push(AnalysisUtils::create_complexity_suggestion("function", "#"));
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.generate_python_suggestions(child, file_content, suggestions)?;
        }

        Ok(())
    }

    fn calculate_complexity(&self, node: Node, _file_content: &str) -> u32 {
        let complexity_nodes = [
            "if_statement",
            "for_statement",
            "while_statement",
            "try_statement",
            "with_statement",
            "match_statement",
            "except_clause",
            "boolean_operator",
        ];
        AnalysisUtils::calculate_complexity(node, &complexity_nodes)
    }
}

impl LanguageAnalyzer for PythonAnalyzer {
    fn analyze_issues(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        self.analyze_python_issues(tree.root_node(), file_content, &mut issues)?;
        Ok(issues)
    }

    fn generate_suggestions(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();
        self.generate_python_suggestions(tree.root_node(), file_content, &mut suggestions)?;
        Ok(suggestions)
    }
}
