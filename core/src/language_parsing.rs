//! Language parsing module for OdinCode
//!
//! This module provides language-agnostic parsing capabilities using Tree-sitter
//! to support multiple programming languages in the OdinCode system.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tree_sitter::{Language, Parser, Query, QueryCursor};

/// Supported programming languages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SupportedLanguage {
    JavaScript,
    TypeScript,
    Python,
    Java,
    C,
    Cpp,
    CSharp,
    Rust,
    Go,
    Ruby,
    PHP,
    Swift,
    // Kotlin,  // Removed due to version conflicts
    Scala,
    // R,  // Removed due to version conflicts
    Shell,
}

impl SupportedLanguage {
    /// Get the Tree-sitter language for this supported language
    pub fn get_language(&self) -> Language {
        match self {
            SupportedLanguage::JavaScript => tree_sitter_javascript::language(),
            SupportedLanguage::TypeScript => tree_sitter_typescript::language_typescript(),
            SupportedLanguage::Python => tree_sitter_python::language(),
            SupportedLanguage::Java => tree_sitter_java::language(),
            SupportedLanguage::C => tree_sitter_c::language(),
            SupportedLanguage::Cpp => tree_sitter_cpp::language(),
            SupportedLanguage::CSharp => tree_sitter_c_sharp::language(),
            SupportedLanguage::Rust => tree_sitter_rust::language(),
            SupportedLanguage::Go => tree_sitter_go::language(),
            SupportedLanguage::Ruby => tree_sitter_ruby::language(),
            SupportedLanguage::PHP => tree_sitter_php::language(),
            // SupportedLanguage::Swift => tree_sitter_swift::language(),  // Removed due to version conflicts
            // SupportedLanguage::Kotlin => tree_sitter_kotlin::language(),  // Removed due to version conflicts
            SupportedLanguage::Scala => tree_sitter_scala::language(),
            // SupportedLanguage::R => tree_sitter_r::language(),  // Removed due to version conflicts
            SupportedLanguage::Shell => tree_sitter_bash::language(),
            _ => tree_sitter_javascript::language(), // Default fallback
        }
    }

    /// Get the language name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            SupportedLanguage::JavaScript => "javascript",
            SupportedLanguage::TypeScript => "typescript",
            SupportedLanguage::Python => "python",
            SupportedLanguage::Java => "java",
            SupportedLanguage::C => "c",
            SupportedLanguage::Cpp => "cpp",
            SupportedLanguage::CSharp => "csharp",
            SupportedLanguage::Rust => "rust",
            SupportedLanguage::Go => "go",
            SupportedLanguage::Ruby => "ruby",
            SupportedLanguage::PHP => "php",
            SupportedLanguage::Swift => "swift",
            // SupportedLanguage::Kotlin => "kotlin",  // Removed due to version conflicts
            SupportedLanguage::Scala => "scala",
            // SupportedLanguage::R => "r",  // Removed due to version conflicts
            SupportedLanguage::Shell => "shell",
        }
    }

    /// Convert from a string to SupportedLanguage
    pub fn from_str(lang_str: &str) -> Option<Self> {
        match lang_str.to_lowercase().as_str() {
            "javascript" | "js" => Some(SupportedLanguage::JavaScript),
            "typescript" | "ts" => Some(SupportedLanguage::TypeScript),
            "python" | "py" => Some(SupportedLanguage::Python),
            "java" => Some(SupportedLanguage::Java),
            "c" => Some(SupportedLanguage::C),
            "cpp" | "c++" => Some(SupportedLanguage::Cpp),
            "csharp" | "c#" | "cs" => Some(SupportedLanguage::CSharp),
            "rust" | "rs" => Some(SupportedLanguage::Rust),
            "go" | "golang" => Some(SupportedLanguage::Go),
            "ruby" | "rb" => Some(SupportedLanguage::Ruby),
            "php" => Some(SupportedLanguage::PHP),
            "swift" => Some(SupportedLanguage::Swift),
            // "kotlin" | "kt" => Some(SupportedLanguage::Kotlin),  // Removed due to version conflicts
            "scala" => Some(SupportedLanguage::Scala),
            // "r" => Some(SupportedLanguage::R),  // Removed due to version conflicts
            "shell" | "bash" | "sh" => Some(SupportedLanguage::Shell),
            _ => None,
        }
    }

    /// Detect language based on file extension
    pub fn detect_language(file_path: &str) -> Result<Self> {
        let path = std::path::Path::new(file_path);
        let extension = path
            .extension()
            .ok_or_else(|| anyhow::anyhow!("No file extension found for: {}", file_path))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid file extension for: {}", file_path))?
            .to_lowercase();

        match extension.as_str() {
            "js" => Ok(SupportedLanguage::JavaScript),
            "ts" => Ok(SupportedLanguage::TypeScript),
            "py" => Ok(SupportedLanguage::Python),
            "java" => Ok(SupportedLanguage::Java),
            "c" => Ok(SupportedLanguage::C),
            "cpp" | "cxx" | "cc" => Ok(SupportedLanguage::Cpp),
            "rs" => Ok(SupportedLanguage::Rust),
            _ => Err(anyhow::anyhow!("Unsupported file extension: {}", extension)),
        }
    }
}

/// Language Parser that handles parsing for multiple languages
pub struct LanguageParser {
    /// Map of supported languages to their parsers
    parsers: HashMap<SupportedLanguage, Parser>,
}

impl LanguageParser {
    /// Create a new language parser
    pub fn new() -> Result<Self> {
        let mut parsers = HashMap::new();

        // Initialize parsers for all supported languages
        for lang in [
            SupportedLanguage::JavaScript,
            SupportedLanguage::TypeScript,
            SupportedLanguage::Python,
            SupportedLanguage::Java,
            SupportedLanguage::C,
            SupportedLanguage::Cpp,
            SupportedLanguage::CSharp,
            SupportedLanguage::Rust,
            SupportedLanguage::Go,
            SupportedLanguage::Ruby,
            SupportedLanguage::PHP,
            SupportedLanguage::Swift,
            // SupportedLanguage::Kotlin,  // Removed due to version conflicts
            SupportedLanguage::Scala,
            // SupportedLanguage::R,  // Removed due to version conflicts
            SupportedLanguage::Shell,
        ] {
            let mut parser = Parser::new();
            parser.set_language(lang.get_language())?;
            parsers.insert(lang, parser);
        }

        Ok(LanguageParser { parsers })
    }

    /// Parse source code for a specific language
    pub fn parse(
        &mut self,
        source_code: &str,
        language: &SupportedLanguage,
    ) -> Result<tree_sitter::Tree> {
        let parser = self
            .parsers
            .get_mut(language)
            .ok_or_else(|| anyhow::anyhow!("Language {:?} is not supported", language))?;

        let tree = parser
            .parse(source_code, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse code for language {:?}", language))?;

        Ok(tree)
    }

    /// Parse source code with automatic language detection
    pub fn parse_with_detection(
        &mut self,
        source_code: &str,
        file_path: &str,
    ) -> Result<(tree_sitter::Tree, SupportedLanguage)> {
        let language = SupportedLanguage::detect_language(file_path)?;
        let tree = self.parse(source_code, &language)?;
        Ok((tree, language))
    }

    /// Get the parser for a specific language
    pub fn get_parser(&mut self, language: &SupportedLanguage) -> Option<&mut Parser> {
        self.parsers.get_mut(language)
    }

    /// Query the AST for specific patterns
    pub fn query<'a>(
        &self,
        tree: &'a tree_sitter::Tree,
        language: &SupportedLanguage,
        query_str: &str,
        source_code: &str,
    ) -> Result<Vec<QueryMatch<'a>>> {
        let query = Query::new(language.get_language(), query_str)
            .map_err(|e| anyhow::anyhow!("Invalid query: {}", e))?;

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source_code.as_bytes());

        let mut results = Vec::new();
        for m in matches {
            let mut captures = Vec::new();
            for capture in m.captures {
                captures.push(QueryCapture {
                    index: capture.index,
                    node: capture.node,
                });
            }
            results.push(QueryMatch {
                pattern_index: m.pattern_index,
                captures,
            });
        }

        Ok(results)
    }
}

/// Represents a query match in the AST
#[derive(Debug, Clone)]
pub struct QueryMatch<'a> {
    pub pattern_index: usize,
    pub captures: Vec<QueryCapture<'a>>,
}

/// Represents a captured node in a query
#[derive(Debug, Clone)]
pub struct QueryCapture<'a> {
    pub index: u32,
    pub node: tree_sitter::Node<'a>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_language_detection() {
        let temp_dir = TempDir::new().unwrap();

        assert_eq!(
            SupportedLanguage::detect_language("test.rs").unwrap(),
            SupportedLanguage::Rust
        );
        assert_eq!(
            SupportedLanguage::detect_language("test.js").unwrap(),
            SupportedLanguage::JavaScript
        );
        assert_eq!(
            SupportedLanguage::detect_language("test.py").unwrap(),
            SupportedLanguage::Python
        );
        assert_eq!(
            SupportedLanguage::detect_language("test.java").unwrap(),
            SupportedLanguage::Java
        );
        assert_eq!(
            SupportedLanguage::detect_language("test.c").unwrap(),
            SupportedLanguage::C
        );
        assert_eq!(
            SupportedLanguage::detect_language("test.cpp").unwrap(),
            SupportedLanguage::Cpp
        );
        assert_eq!(
            SupportedLanguage::detect_language("test.ts").unwrap(),
            SupportedLanguage::TypeScript
        );
    }

    #[test]
    fn test_language_from_str() {
        assert_eq!(
            SupportedLanguage::from_str("rust"),
            Some(SupportedLanguage::Rust)
        );
        assert_eq!(
            SupportedLanguage::from_str("javascript"),
            Some(SupportedLanguage::JavaScript)
        );
        assert_eq!(
            SupportedLanguage::from_str("python"),
            Some(SupportedLanguage::Python)
        );
        assert_eq!(SupportedLanguage::from_str("invalid"), None);
    }

    #[test]
    fn test_parse_rust_code() {
        let mut parser = LanguageParser::new().unwrap();
        let code = r#"
            fn main() {
                println!("Hello, world!");
            }
        "#;

        let tree = parser.parse(code, &SupportedLanguage::Rust).unwrap();
        let root_node = tree.root_node();

        assert_eq!(root_node.kind(), "source_file");
        assert!(root_node.has_error() == false);
    }

    #[test]
    fn test_parse_javascript_code() {
        let mut parser = LanguageParser::new().unwrap();
        let code = r#"
            function hello() {
                console.log("Hello, world!");
            }
        "#;

        let tree = parser.parse(code, &SupportedLanguage::JavaScript).unwrap();
        let root_node = tree.root_node();

        assert_eq!(root_node.kind(), "program");
        assert!(root_node.has_error() == false);
    }

    #[test]
    fn test_parse_python_code() {
        let mut parser = LanguageParser::new().unwrap();
        let code = r#"
            def hello():
                print("Hello, world!")
        "#;

        let tree = parser.parse(code, &SupportedLanguage::Python).unwrap();
        let root_node = tree.root_node();

        assert_eq!(root_node.kind(), "module");
        assert!(root_node.has_error() == false);
    }

    #[test]
    fn test_parse_java_code() {
        let mut parser = LanguageParser::new().unwrap();
        let code = r#"
            public class HelloWorld {
                public static void main(String[] args) {
                    System.out.println("Hello, world!");
                }
            }
        "#;

        let tree = parser.parse(code, &SupportedLanguage::Java).unwrap();
        let root_node = tree.root_node();

        assert_eq!(root_node.kind(), "program");
        assert!(root_node.has_error() == false);
    }

    #[test]
    fn test_parse_c_code() {
        let mut parser = LanguageParser::new().unwrap();
        let code = r#"
            #include <stdio.h>
            
            int main() {
                printf("Hello, world!\n");
                return 0;
            }
        "#;

        let tree = parser.parse(code, &SupportedLanguage::C).unwrap();
        let root_node = tree.root_node();

        assert_eq!(root_node.kind(), "translation_unit");
        assert!(root_node.has_error() == false);
    }

    #[test]
    fn test_parse_cpp_code() {
        let mut parser = LanguageParser::new().unwrap();
        let code = r#"
            #include <iostream>
            
            int main() {
                std::cout << "Hello, world!" << std::endl;
                return 0;
            }
        "#;

        let tree = parser.parse(code, &SupportedLanguage::Cpp).unwrap();
        let root_node = tree.root_node();

        assert_eq!(root_node.kind(), "translation_unit");
        assert!(root_node.has_error() == false);
    }

    #[test]
    fn test_parse_typescript_code() {
        let mut parser = LanguageParser::new().unwrap();
        let code = r#"
            interface Person {
                name: string;
                age: number;
            }
            
            function greet(person: Person): string {
                return `Hello, ${person.name}!`;
            }
        "#;

        let tree = parser.parse(code, &SupportedLanguage::TypeScript).unwrap();
        let root_node = tree.root_node();

        assert_eq!(root_node.kind(), "program");
        assert!(root_node.has_error() == false);
    }

    #[test]
    fn test_parse_with_detection() {
        let mut parser = LanguageParser::new().unwrap();

        let rust_code = r#"fn main() { println!("Hello"); }"#;
        let (tree, lang) = parser.parse_with_detection(rust_code, "test.rs").unwrap();
        assert_eq!(lang, SupportedLanguage::Rust);
        assert_eq!(tree.root_node().kind(), "source_file");

        let js_code = r#"function test() { console.log("Hello"); }"#;
        let (tree, lang) = parser.parse_with_detection(js_code, "test.js").unwrap();
        assert_eq!(lang, SupportedLanguage::JavaScript);
        assert_eq!(tree.root_node().kind(), "program");

        let py_code = r#"def test(): print("Hello")"#;
        let (tree, lang) = parser.parse_with_detection(py_code, "test.py").unwrap();
        assert_eq!(lang, SupportedLanguage::Python);
        assert_eq!(tree.root_node().kind(), "module");
    }

    #[test]
    fn test_query_function_calls() {
        let mut parser = LanguageParser::new().unwrap();
        let code = r#"
            fn main() {
                println!("Hello, world!");
                let x = some_function();
                another_function(x);
            }
        "#;

        let tree = parser.parse(code, &SupportedLanguage::Rust).unwrap();

        // Query for function calls and macro invocations
        let query_str = r#"
            (call_expression
                function: (identifier) @function_name)
            (macro_invocation
                macro: (identifier) @function_name)
        "#;

        let results = parser
            .query(&tree, &SupportedLanguage::Rust, query_str, code)
            .unwrap();
        assert_eq!(results.len(), 3); // println!, some_function, and another_function

        // Check that we found the function names
        let mut function_names = Vec::new();
        for result in results {
            for capture in result.captures {
                let name = &code[capture.node.byte_range()];
                function_names.push(name.to_string());
            }
        }

        assert!(function_names.contains(&"println".to_string())); // Macro name without !
        assert!(function_names.contains(&"some_function".to_string()));
        assert!(function_names.contains(&"another_function".to_string()));
    }

    #[test]
    fn test_query_variables() {
        let mut parser = LanguageParser::new().unwrap();
        let code = r#"
            fn main() {
                let x = 5;
                let y = 10;
                let z = x + y;
            }
        "#;

        let tree = parser.parse(code, &SupportedLanguage::Rust).unwrap();

        // Query for variable declarations
        let query_str = r#"
            (let_declaration
                pattern: (identifier) @variable_name)
        "#;

        let results = parser
            .query(&tree, &SupportedLanguage::Rust, query_str, code)
            .unwrap();
        assert_eq!(results.len(), 3); // x, y, z

        // Check that we found the variable names
        let mut variable_names = Vec::new();
        for result in results {
            for capture in result.captures {
                let name = &code[capture.node.byte_range()];
                variable_names.push(name.to_string());
            }
        }

        assert!(variable_names.contains(&"x".to_string()));
        assert!(variable_names.contains(&"y".to_string()));
        assert!(variable_names.contains(&"z".to_string()));
    }
}
