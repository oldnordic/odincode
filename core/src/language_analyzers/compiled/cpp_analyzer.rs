//! C++-specific language analyzer

use crate::language_analyzers::core::analysis_utils::AnalysisUtils;
use crate::language_analyzers::core::base_analyzer::LanguageAnalyzer;
use crate::{CodeIssue, CodeSuggestion, Severity, SuggestionType};
use anyhow::Result;
use tree_sitter::{Node, Tree};

/// C++-specific analyzer
pub struct CppAnalyzer;

impl CppAnalyzer {
    pub fn new() -> Self {
        CppAnalyzer
    }

    fn analyze_cpp_issues(
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
            "function_definition" => {
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
            self.analyze_cpp_issues(child, file_content, issues)?;
        }

        Ok(())
    }

    fn generate_cpp_suggestions(
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
                    suggestions.push(AnalysisUtils::create_complexity_suggestion(
                        "function", "//",
                    ));
                }
            }
            "call_expression" => {
                let content = &file_content[node.start_byte()..node.end_byte()];
                if content.contains("gets(")
                    || content.contains("strcpy(")
                    || content.contains("strcat(")
                {
                    suggestions.push(CodeSuggestion::new_complete(
                        "Unsafe C-style string function detected",
                        "Use safer C++ alternatives like std::string",
                        Severity::Critical,
                        None,
                        Some(node.start_position().row as u32 + 1),
                        false,
                    ));
                }
            }
            "raw_pointer_type" => {
                suggestions.push(CodeSuggestion::new_complete(
                    "Raw pointer detected",
                    "Consider using smart pointers for safer memory management",
                    Severity::Warning,
                    None,
                    Some(node.start_position().row as u32 + 1),
                    false,
                ));
            }
            "raw_pointer_type" => {
                suggestions.push(CodeSuggestion::new_complete(
                    "Raw pointer detected",
                    "Consider using smart pointers for safer memory management",
                    Severity::Warning,
                    None,
                    Some(node.start_position().row as u32 + 1),
                    false,
                ));
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.generate_cpp_suggestions(child, file_content, suggestions)?;
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
            "case_statement",
            "try_statement",
            "catch_clause",
            "conditional_expression",
        ];
        AnalysisUtils::calculate_complexity(node, &complexity_nodes)
    }
}

impl LanguageAnalyzer for CppAnalyzer {
    fn analyze_issues(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        self.analyze_cpp_issues(tree.root_node(), file_content, &mut issues)?;
        Ok(issues)
    }

    fn generate_suggestions(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();
        self.generate_cpp_suggestions(tree.root_node(), file_content, &mut suggestions)?;
        Ok(suggestions)
    }
}
