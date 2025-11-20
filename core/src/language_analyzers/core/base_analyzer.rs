//! Base analyzer trait and common types for language-specific analyzers

use crate::{CodeIssue, CodeSuggestion};
use anyhow::Result;
use tree_sitter::Tree;

/// Trait for language-specific analyzers
pub trait LanguageAnalyzer {
    /// Analyze AST for code issues
    fn analyze_issues(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeIssue>>;

    /// Generate suggestions based on AST
    fn generate_suggestions(&self, tree: &Tree, file_content: &str) -> Result<Vec<CodeSuggestion>>;
}
