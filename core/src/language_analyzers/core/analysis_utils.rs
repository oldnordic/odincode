//! Common utility functions for language analysis

use crate::{CodeIssue, CodeSuggestion, IssueType, Severity, SuggestionType};
use tree_sitter::Node;
use uuid::Uuid;

/// Common analysis utilities shared across language analyzers
pub struct AnalysisUtils;

impl AnalysisUtils {
    /// Check for TODO/FIXME/HACK comments in content
    pub fn check_todo_comments(
        content: &str,
        line_number: usize,
        column_number: usize,
        comment_prefixes: &[&str],
    ) -> Option<CodeIssue> {
        for prefix in comment_prefixes {
            if content.contains(prefix)
                && (content.contains("TODO")
                    || content.contains("FIXME")
                    || content.contains("HACK"))
            {
                return Some(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::BestPractice,
                    severity: Severity::Medium,
                    description: "TODO/FIXME/HACK comment found".to_string(),
                    line_number,
                    column_number,
                    suggestion: Some("Address the technical debt".to_string()),
                });
            }
        }
        None
    }

    /// Create a complexity issue for functions/methods
    pub fn create_complexity_issue(
        line_number: usize,
        column_number: usize,
        entity_type: &str,
    ) -> CodeIssue {
        CodeIssue {
            id: Uuid::new_v4(),
            issue_type: IssueType::BestPractice,
            severity: Severity::Medium,
            description: format!("{} is too complex", entity_type),
            line_number,
            column_number,
            suggestion: Some(format!("Consider refactoring this complex {}", entity_type)),
        }
    }

    /// Create a complexity suggestion for functions/methods
    pub fn create_complexity_suggestion(entity_type: &str, comment_prefix: &str) -> CodeSuggestion {
        CodeSuggestion::complete(
            Uuid::new_v4(),
            SuggestionType::Refactor,
            format!("Complex {} detected", entity_type),
            format!(
                "{} is complex, consider breaking it into smaller {}",
                entity_type, entity_type
            ),
            Some(format!(
                "{} Break complex {} into smaller, focused {}",
                comment_prefix, entity_type, entity_type
            )),
            0.75,
            "unknown".to_string(),
            None,
            crate::Severity::Warning,
            false,
        )
    }

    /// Calculate cyclomatic complexity for a node
    pub fn calculate_complexity(node: Node, complexity_nodes: &[&str]) -> u32 {
        let mut complexity = 1; // Base complexity
        let cursor = node.walk();
        let mut stack = vec![node];

        while let Some(current_node) = stack.pop() {
            if complexity_nodes.contains(&current_node.kind()) {
                complexity += 1;
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
