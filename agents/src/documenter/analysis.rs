//! Documenter Code Analysis
//!
//! This module handles code structure analysis and pattern detection
//! for documentation generation.

use anyhow::Result;
use odincode_core::CodeFile;

/// Code analysis result
#[derive(Debug, Clone)]
pub struct CodeAnalysis {
    /// Parsed code elements
    pub elements: Vec<CodeElement>,
    /// Overall complexity score
    pub complexity_score: f64,
    /// Documentation coverage percentage
    pub documentation_coverage: f64,
    /// Language-specific patterns detected
    pub patterns: Vec<String>,
    /// Import statements found
    pub imports: Vec<String>,
    /// Functions found
    pub functions: Vec<FunctionInfo>,
    /// Classes/Structs found
    pub classes: Vec<ClassInfo>,
}

/// Individual code element
#[derive(Debug, Clone)]
pub struct CodeElement {
    /// Type of element
    pub element_type: ElementType,
    /// Name of the element
    pub name: String,
    /// Line number where element starts
    pub line_number: usize,
    /// Documentation for the element (if any)
    pub documentation: Option<String>,
    /// Complexity score for this element
    pub complexity: f64,
}

/// Types of code elements
#[derive(Debug, Clone, PartialEq)]
pub enum ElementType {
    /// Function or method
    Function,
    /// Class or struct
    Class,
    /// Variable or constant
    Variable,
    /// Import statement
    Import,
    /// Comment
    Comment,
    /// Other element type
    Other(String),
}

/// Function information
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    /// Function name
    pub name: String,
    /// Line number
    pub line_number: usize,
    /// Whether function has documentation
    pub has_documentation: bool,
    /// Function complexity score
    pub complexity: f64,
    /// Parameters
    pub parameters: Vec<String>,
    /// Return type
    pub return_type: Option<String>,
}

/// Class information
#[derive(Debug, Clone)]
pub struct ClassInfo {
    /// Class name
    pub name: String,
    /// Line number
    pub line_number: usize,
    /// Whether class has documentation
    pub has_documentation: bool,
    /// Methods in the class
    pub methods: Vec<FunctionInfo>,
    /// Properties/fields
    pub properties: Vec<String>,
}

/// Code analyzer for documentation generation
pub struct CodeAnalyzer;

impl CodeAnalyzer {
    /// Analyze code structure for documentation
    pub fn analyze_code_structure(file: &CodeFile) -> Result<CodeAnalysis> {
        let lines: Vec<&str> = file.content.lines().collect();
        let mut elements = Vec::new();
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();
        let mut patterns = Vec::new();

        // Analyze each line
        for (line_num, line) in lines.iter().enumerate() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with("//") || line.starts_with("#") {
                continue;
            }

            // Detect function definitions
            if Self::detect_function_definition(line, &file.language) {
                let function_name = Self::extract_function_name(line, &file.language);
                let has_doc = Self::has_documentation(&lines, line_num);

                functions.push(FunctionInfo {
                    name: function_name.clone(),
                    line_number: line_num + 1,
                    has_documentation: has_doc,
                    complexity: Self::calculate_function_complexity(line) as f64,
                    parameters: Self::extract_parameters(line, &file.language),
                    return_type: Self::extract_return_type(line, &file.language),
                });

                elements.push(CodeElement {
                    element_type: ElementType::Function,
                    name: function_name,
                    line_number: line_num + 1,
                    documentation: Self::get_documentation(&lines, line_num),
                    complexity: Self::calculate_function_complexity(line) as f64,
                });
            }

            // Detect class definitions
            if Self::detect_class_definition(line, &file.language) {
                let class_name = Self::extract_class_name(line, &file.language);
                let has_doc = Self::has_documentation(&lines, line_num);

                classes.push(ClassInfo {
                    name: class_name.clone(),
                    line_number: line_num + 1,
                    has_documentation: has_doc,
                    methods: Vec::new(), // Will be populated during analysis
                    properties: Vec::new(),
                });

                elements.push(CodeElement {
                    element_type: ElementType::Class,
                    name: class_name,
                    line_number: line_num + 1,
                    documentation: Self::get_documentation(&lines, line_num),
                    complexity: 1.0, // Base complexity for classes
                });
            }

            // Detect import statements
            if Self::detect_import_statement(line, &file.language) {
                imports.push(line.to_string());

                elements.push(CodeElement {
                    element_type: ElementType::Import,
                    name: line.to_string(),
                    line_number: line_num + 1,
                    documentation: None,
                    complexity: 0.0,
                });
            }
        }

        // Calculate overall metrics
        let complexity_score = Self::calculate_complexity_score(&elements);
        let documentation_coverage = Self::calculate_documentation_coverage(&elements);

        // Detect language-specific patterns
        patterns.extend(Self::detect_language_patterns(&file.language, &lines));

        Ok(CodeAnalysis {
            elements,
            complexity_score,
            documentation_coverage,
            patterns,
            imports,
            functions,
            classes,
        })
    }

    /// Detect function definition based on language
    fn detect_function_definition(line: &str, language: &str) -> bool {
        match language.to_lowercase().as_str() {
            "rust" => {
                line.contains("fn ")
                    && (line.contains("->") || line.contains("{") || line.ends_with('{'))
            }
            "python" => line.starts_with("def ") && line.ends_with(':'),
            "javascript" | "typescript" => {
                (line.contains("function ") || line.contains("=>") || line.contains("async"))
                    && (line.contains("{") || line.contains("=>"))
            }
            "java" | "c#" | "c++" => {
                line.contains("(")
                    && line.contains(")")
                    && (line.contains("{") || line.ends_with('{'))
            }
            "go" => line.contains("func ") && line.contains("("),
            _ => false, // Simplified detection for other languages
        }
    }

    /// Extract function name from definition
    fn extract_function_name(line: &str, language: &str) -> String {
        match language.to_lowercase().as_str() {
            "rust" => {
                if let Some(start) = line.find("fn ") {
                    let rest = &line[start + 3..];
                    if let Some(end) = rest.find('(') {
                        return rest[..end].trim().to_string();
                    }
                }
            }
            "python" => {
                if let Some(start) = line.find("def ") {
                    let rest = &line[start + 4..];
                    if let Some(end) = rest.find('(') {
                        return rest[..end].trim().to_string();
                    }
                }
            }
            "javascript" | "typescript" => {
                if line.contains("function ") {
                    if let Some(start) = line.find("function ") {
                        let rest = &line[start + 9..];
                        if let Some(end) = rest.find('(') {
                            return rest[..end].trim().to_string();
                        }
                    }
                } else if let Some(eq_pos) = line.find('=') {
                    let before_eq = &line[..eq_pos];
                    if let Some(last_space) = before_eq.rfind(' ') {
                        return before_eq[last_space + 1..].trim().to_string();
                    }
                }
            }
            _ => {}
        }
        "unknown_function".to_string()
    }

    /// Detect class definition based on language
    fn detect_class_definition(line: &str, language: &str) -> bool {
        match language.to_lowercase().as_str() {
            "rust" => {
                (line.starts_with("struct ")
                    || line.starts_with("enum ")
                    || line.starts_with("trait ")
                    || line.starts_with("impl "))
                    && (line.contains("{") || line.ends_with('{'))
            }
            "python" => line.starts_with("class ") && line.ends_with(':'),
            "java" | "c#" | "javascript" | "typescript" => {
                line.starts_with("class ") && (line.contains("{") || line.ends_with('{'))
            }
            "c++" => {
                (line.starts_with("class ") || line.starts_with("struct "))
                    && (line.contains("{") || line.ends_with('{'))
            }
            "go" => line.starts_with("type ") && line.contains("struct"),
            _ => false,
        }
    }

    /// Extract class name from definition
    fn extract_class_name(line: &str, language: &str) -> String {
        match language.to_lowercase().as_str() {
            "rust" => {
                let keywords = ["struct ", "enum ", "trait ", "impl "];
                for keyword in keywords {
                    if let Some(start) = line.find(keyword) {
                        let rest = &line[start + keyword.len()..];
                        if let Some(end) = rest.find('{') {
                            return rest[..end].trim().to_string();
                        }
                    }
                }
            }
            "python" => {
                if let Some(start) = line.find("class ") {
                    let rest = &line[start + 6..];
                    if let Some(end) = rest.find('(') {
                        return rest[..end].trim().to_string();
                    } else if let Some(end) = rest.find(':') {
                        return rest[..end].trim().to_string();
                    }
                }
            }
            "java" | "c#" | "javascript" | "typescript" | "c++" => {
                if let Some(start) = line.find("class ") {
                    let rest = &line[start + 6..];
                    if let Some(end) = rest.find('{') {
                        return rest[..end].trim().to_string();
                    }
                }
            }
            "go" => {
                if let Some(start) = line.find("type ") {
                    let rest = &line[start + 5..];
                    if let Some(struct_pos) = rest.find("struct") {
                        let name_part = &rest[..struct_pos];
                        return name_part.trim().to_string();
                    }
                }
            }
            _ => {}
        }
        "unknown_class".to_string()
    }

    /// Detect import statement based on language
    fn detect_import_statement(line: &str, language: &str) -> bool {
        match language.to_lowercase().as_str() {
            "rust" => line.starts_with("use ") || line.starts_with("mod "),
            "python" => line.starts_with("import ") || line.starts_with("from "),
            "javascript" | "typescript" => {
                line.starts_with("import ") || line.starts_with("require(")
            }
            "java" => line.starts_with("import "),
            "c#" => line.starts_with("using "),
            "c++" => line.starts_with("#include "),
            "go" => line.starts_with("import "),
            _ => false,
        }
    }

    /// Check if code element has documentation
    fn has_documentation(lines: &[&str], line_num: usize) -> bool {
        if line_num == 0 {
            return false;
        }

        // Check previous lines for documentation
        for i in (0..line_num).rev() {
            let line = lines[i].trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with("///")
                || line.starts_with("//!")
                || line.starts_with("/**")
                || line.starts_with("/*")
                || line.starts_with("//")
                || line.starts_with("#")
                || line.starts_with("\"\"\"")
                || line.starts_with("'''")
            {
                return true;
            }
            // Stop if we hit another code element
            if !line.starts_with("//") && !line.starts_with("#") && !line.is_empty() {
                break;
            }
        }
        false
    }

    /// Get documentation for code element
    fn get_documentation(lines: &[&str], line_num: usize) -> Option<String> {
        if line_num == 0 {
            return None;
        }

        let mut doc_lines = Vec::new();

        // Collect documentation from previous lines
        for i in (0..line_num).rev() {
            let line = lines[i].trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with("///")
                || line.starts_with("//!")
                || line.starts_with("//")
                || line.starts_with("#")
            {
                doc_lines.push(
                    line.trim_start_matches("///")
                        .trim_start_matches("//!")
                        .trim_start_matches("//")
                        .trim_start_matches("#")
                        .trim(),
                );
            } else if line.starts_with("/**")
                || line.starts_with("/*")
                || line.starts_with("\"\"\"")
                || line.starts_with("'''")
            {
                // Handle multi-line comments (simplified)
                break;
            } else {
                // Stop if we hit another code element
                break;
            }
        }

        if doc_lines.is_empty() {
            None
        } else {
            doc_lines.reverse();
            Some(doc_lines.join("\n"))
        }
    }

    /// Calculate function complexity (simplified)
    fn calculate_function_complexity(line: &str) -> f32 {
        let mut complexity: f32 = 1.0;

        // Add complexity for control structures
        if line.contains("if") {
            complexity += 0.5;
        }
        if line.contains("else") {
            complexity += 0.3;
        }
        if line.contains("for") {
            complexity += 0.5;
        }
        if line.contains("while") {
            complexity += 0.5;
        }
        if line.contains("match") || line.contains("switch") {
            complexity += 0.7;
        }

        // Add complexity for error handling
        if line.contains("Result") || line.contains("Option") || line.contains("try") {
            complexity += 0.3;
        }

        complexity.min(5.0) // Cap at 5.0
    }

    /// Extract function parameters (simplified)
    fn extract_parameters(line: &str, _language: &str) -> Vec<String> {
        if let Some(start) = line.find('(') {
            if let Some(end) = line.find(')') {
                let params_str = &line[start + 1..end];
                if params_str.trim().is_empty() {
                    return Vec::new();
                }
                return params_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
            }
        }
        Vec::new()
    }

    /// Extract return type (simplified)
    fn extract_return_type(line: &str, language: &str) -> Option<String> {
        match language.to_lowercase().as_str() {
            "rust" => {
                if let Some(arrow_pos) = line.find("->") {
                    let rest = &line[arrow_pos + 2..];
                    if let Some(end) = rest.find('{') {
                        return Some(rest[..end].trim().to_string());
                    }
                }
            }
            "java" | "c#" | "c++" => {
                // Simplified extraction for C-style languages
                if let Some(open_paren) = line.find('(') {
                    let before_paren = &line[..open_paren];
                    if let Some(last_space) = before_paren.rfind(' ') {
                        return Some(before_paren[last_space + 1..].trim().to_string());
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// Calculate overall complexity score
    fn calculate_complexity_score(elements: &[CodeElement]) -> f64 {
        if elements.is_empty() {
            return 0.0;
        }

        let total_complexity: f64 = elements.iter().map(|e| e.complexity).sum();
        total_complexity / elements.len() as f64
    }

    /// Calculate documentation coverage
    fn calculate_documentation_coverage(elements: &[CodeElement]) -> f64 {
        if elements.is_empty() {
            return 0.0;
        }

        let documented_count = elements
            .iter()
            .filter(|e| e.documentation.is_some())
            .count();

        documented_count as f64 / elements.len() as f64
    }

    /// Detect language-specific patterns
    fn detect_language_patterns(language: &str, lines: &[&str]) -> Vec<String> {
        let mut patterns = Vec::new();

        match language.to_lowercase().as_str() {
            "rust" => {
                // Look for Rust-specific patterns
                for line in lines {
                    if line.contains("async fn") {
                        patterns.push("async_functions".to_string());
                    }
                    if line.contains("impl") {
                        patterns.push("trait_implementation".to_string());
                    }
                    if line.contains("Result<") {
                        patterns.push("error_handling".to_string());
                    }
                }
            }
            "python" => {
                // Look for Python-specific patterns
                for line in lines {
                    if line.contains("@") {
                        patterns.push("decorators".to_string());
                    }
                    if line.contains("async def") {
                        patterns.push("async_functions".to_string());
                    }
                    if line.contains("try:") {
                        patterns.push("exception_handling".to_string());
                    }
                }
            }
            _ => {}
        }

        patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_detection_rust() {
        assert!(CodeAnalyzer::detect_function_definition(
            "pub fn test() -> Result<(), Error> {",
            "rust"
        ));
        assert!(CodeAnalyzer::detect_function_definition(
            "fn hello() {",
            "rust"
        ));
        assert!(!CodeAnalyzer::detect_function_definition(
            "let x = 5;",
            "rust"
        ));
    }

    #[test]
    fn test_function_detection_python() {
        assert!(CodeAnalyzer::detect_function_definition(
            "def test():",
            "python"
        ));
        assert!(CodeAnalyzer::detect_function_definition(
            "async def hello():",
            "python"
        ));
        assert!(!CodeAnalyzer::detect_function_definition("x = 5", "python"));
    }

    #[test]
    fn test_class_detection() {
        assert!(CodeAnalyzer::detect_class_definition(
            "class Test:",
            "python"
        ));
        assert!(CodeAnalyzer::detect_class_definition(
            "struct Test {",
            "rust"
        ));
        assert!(CodeAnalyzer::detect_class_definition(
            "class Test {",
            "java"
        ));
        assert!(!CodeAnalyzer::detect_class_definition("let x = 5;", "rust"));
    }

    #[test]
    fn test_import_detection() {
        assert!(CodeAnalyzer::detect_import_statement(
            "use std::collections::HashMap;",
            "rust"
        ));
        assert!(CodeAnalyzer::detect_import_statement("import os", "python"));
        assert!(CodeAnalyzer::detect_import_statement(
            "import React from 'react';",
            "javascript"
        ));
        assert!(!CodeAnalyzer::detect_import_statement("let x = 5;", "rust"));
    }

    #[test]
    fn test_function_name_extraction() {
        assert_eq!(
            CodeAnalyzer::extract_function_name("pub fn test() -> Result<(), Error> {", "rust"),
            "test"
        );
        assert_eq!(
            CodeAnalyzer::extract_function_name("def hello_world():", "python"),
            "hello_world"
        );
        assert_eq!(
            CodeAnalyzer::extract_function_name("function add(a, b) {", "javascript"),
            "add"
        );
    }

    #[test]
    fn test_class_name_extraction() {
        assert_eq!(
            CodeAnalyzer::extract_class_name("struct Test {", "rust"),
            "Test"
        );
        assert_eq!(
            CodeAnalyzer::extract_class_name("class MyClass:", "python"),
            "MyClass"
        );
        assert_eq!(
            CodeAnalyzer::extract_class_name("class Person {", "java"),
            "Person"
        );
    }

    #[test]
    fn test_documentation_detection() {
        let lines = vec!["/// This is a function", "pub fn test() {"];
        assert!(CodeAnalyzer::has_documentation(&lines, 1));

        let lines = vec!["let x = 5;", "pub fn test() {"];
        assert!(!CodeAnalyzer::has_documentation(&lines, 1));
    }

    #[test]
    fn test_complexity_score_calculation() {
        let elements = vec![
            CodeElement {
                element_type: ElementType::Function,
                name: "simple".to_string(),
                line_number: 1,
                documentation: None,
                complexity: 1.0,
            },
            CodeElement {
                element_type: ElementType::Function,
                name: "complex".to_string(),
                line_number: 2,
                documentation: None,
                complexity: 3.0,
            },
        ];

        let score = CodeAnalyzer::calculate_complexity_score(&elements);
        assert_eq!(score, 2.0);
    }

    #[test]
    fn test_documentation_coverage_calculation() {
        let elements = vec![
            CodeElement {
                element_type: ElementType::Function,
                name: "doc1".to_string(),
                line_number: 1,
                documentation: Some("Has docs".to_string()),
                complexity: 1.0,
            },
            CodeElement {
                element_type: ElementType::Function,
                name: "doc2".to_string(),
                line_number: 2,
                documentation: None,
                complexity: 1.0,
            },
        ];

        let coverage = CodeAnalyzer::calculate_documentation_coverage(&elements);
        assert_eq!(coverage, 0.5);
    }
}
