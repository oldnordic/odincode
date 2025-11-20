//! Java-specific language analyzer

use crate::language_analyzers::core::analysis_utils::AnalysisUtils;
use crate::language_analyzers::core::base_analyzer::LanguageAnalyzer;
use crate::{CodeIssue, CodeSuggestion, Severity};
use anyhow::Result;
use tree_sitter::{Node, Tree};

/// Java-specific analyzer
pub struct JavaAnalyzer;

impl JavaAnalyzer {
    pub fn new() -> Self {
        JavaAnalyzer
    }

    fn analyze_java_issues(
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
            "method_declaration" => {
                // Check method complexity
                let complexity = self.calculate_complexity(node, file_content);
                if complexity > 10 {
                    issues.push(AnalysisUtils::create_complexity_issue(
                        node.start_position().row + 1,
                        node.start_position().column,
                        "method",
                    ));
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.analyze_java_issues(child, file_content, issues)?;
        }

        Ok(())
    }

    fn generate_java_suggestions(
        &self,
        node: Node,
        file_content: &str,
        suggestions: &mut Vec<CodeSuggestion>,
    ) -> Result<()> {
        match node.kind() {
            "method_declaration" => {
                // Check method complexity
                let complexity = self.calculate_complexity(node, file_content);
                if complexity > 10 {
                    suggestions.push(AnalysisUtils::create_complexity_suggestion("method", "//"));
                }
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.generate_java_suggestions(child, file_content, suggestions)?;
        }

        Ok(())
    }

    fn calculate_complexity(&self, node: Node, file_content: &str) -> u32 {
        let mut complexity = 1; // Base complexity
        let cursor = node.walk();
        let mut stack = vec![node];

        while let Some(current_node) = stack.pop() {
            match current_node.kind() {
                "if_statement" | "for_statement" | "while_statement" | "do_statement"
                | "switch_statement" | "catch_clause" | "ternary_expression"
                | "binary_expression" => {
                    if file_content[current_node.start_byte()..current_node.end_byte()]
                        .contains("&&")
                        || file_content[current_node.start_byte()..current_node.end_byte()]
                            .contains("||")
                    {
                        complexity += 1;
                    } else {
                        complexity += 1;
                    }
                }
                _ => {}
            }

            // Add children to stack for processing
            let mut child_cursor = current_node.walk();
            for child in current_node.children(&mut child_cursor) {
                stack.push(child);
            }
        }

        complexity
    }
}

impl LanguageAnalyzer for JavaAnalyzer {
    fn analyze_issues(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        self.analyze_java_issues(tree.root_node(), file_content, &mut issues)?;
        Ok(issues)
    }

    fn generate_suggestions(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();
        self.generate_java_suggestions(tree.root_node(), file_content, &mut suggestions)?;
        Ok(suggestions)
    }
}
