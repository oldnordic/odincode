//! Manager for language-specific analyzers

use crate::language_analyzers::core::base_analyzer::LanguageAnalyzer;
use crate::language_parsing::SupportedLanguage;
use anyhow::Result;
use std::collections::HashMap;

/// Manager for language-specific analyzers
pub struct LanguageAnalyzerManager {
    analyzers: HashMap<SupportedLanguage, Box<dyn LanguageAnalyzer + Send + Sync>>,
}

impl LanguageAnalyzerManager {
    /// Create a new analyzer manager with all supported language analyzers
    pub fn new() -> Result<Self> {
        let mut analyzers: HashMap<SupportedLanguage, Box<dyn LanguageAnalyzer + Send + Sync>> =
            HashMap::new();

        // Add analyzers for each supported language
        analyzers.insert(
            SupportedLanguage::Rust,
            Box::new(crate::language_analyzers::compiled::rust_analyzer::RustAnalyzer::new())
                as Box<dyn LanguageAnalyzer + Send + Sync>,
        );
        analyzers.insert(
            SupportedLanguage::JavaScript,
            Box::new(crate::language_analyzers::interpreted::javascript_analyzer::JavaScriptAnalyzer::new()) 
                as Box<dyn LanguageAnalyzer + Send + Sync>,
        );
        analyzers.insert(
            SupportedLanguage::Python,
            Box::new(crate::language_analyzers::interpreted::python_analyzer::PythonAnalyzer::new())
                as Box<dyn LanguageAnalyzer + Send + Sync>,
        );
        analyzers.insert(
            SupportedLanguage::Java,
            Box::new(crate::language_analyzers::compiled::java_analyzer::JavaAnalyzer::new())
                as Box<dyn LanguageAnalyzer + Send + Sync>,
        );
        analyzers.insert(
            SupportedLanguage::C,
            Box::new(crate::language_analyzers::compiled::c_analyzer::CAnalyzer::new())
                as Box<dyn LanguageAnalyzer + Send + Sync>,
        );
        analyzers.insert(
            SupportedLanguage::Cpp,
            Box::new(crate::language_analyzers::compiled::cpp_analyzer::CppAnalyzer::new())
                as Box<dyn LanguageAnalyzer + Send + Sync>,
        );

        Ok(LanguageAnalyzerManager { analyzers })
    }

    /// Get analyzer for a specific language
    pub fn get_analyzer(&self, language: &SupportedLanguage) -> Option<&dyn LanguageAnalyzer> {
        self.analyzers
            .get(language)
            .map(|a| a.as_ref() as &dyn LanguageAnalyzer)
    }

    /// Analyze code using appropriate language analyzer
    pub fn analyze(
        &self,
        language: &SupportedLanguage,
        tree: &tree_sitter::Tree,
        file_content: &str,
    ) -> Result<Vec<crate::CodeIssue>> {
        if let Some(analyzer) = self.get_analyzer(language) {
            analyzer.analyze_issues(tree, file_content)
        } else {
            // If no specific analyzer exists, return an empty vector
            Ok(Vec::new())
        }
    }

    /// Generate suggestions using appropriate language analyzer
    pub fn generate_suggestions(
        &self,
        language: &SupportedLanguage,
        tree: &tree_sitter::Tree,
        file_content: &str,
    ) -> Result<Vec<crate::CodeSuggestion>> {
        if let Some(analyzer) = self.get_analyzer(language) {
            analyzer.generate_suggestions(tree, file_content)
        } else {
            // If no specific analyzer exists, return an empty vector
            Ok(Vec::new())
        }
    }
}
