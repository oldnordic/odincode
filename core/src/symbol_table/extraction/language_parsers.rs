//! Language-specific parsing utilities

use anyhow::Result;
use tree_sitter::{Language, Parser};

/// Language parser utilities
pub struct LanguageParsers;

impl LanguageParsers {
    /// Get tree-sitter language for given language
    pub fn get_language(language: &str) -> Result<Option<Language>> {
        match language {
            "rust" => Ok(Some(tree_sitter_rust::language())),
            "javascript" => Ok(Some(tree_sitter_javascript::language())),
            "typescript" => Ok(Some(tree_sitter_typescript::language_typescript())),
            "python" => Ok(Some(tree_sitter_python::language())),
            "java" => Ok(Some(tree_sitter_java::language())),
            "go" => Ok(Some(tree_sitter_go::language())),
            "cpp" => Ok(Some(tree_sitter_cpp::language())),
            "c" => Ok(Some(tree_sitter_c::language())),
            // "html" => Ok(Some(tree_sitter_html::language())),
            // "css" => Ok(Some(tree_sitter_css::language())),
            // "json" => Ok(Some(tree_sitter_json::language())),
            // "yaml" => Ok(Some(tree_sitter_yaml::language())),
            // "toml" => Ok(Some(tree_sitter_toml::language())),
            // "sql" => Ok(Some(tree_sitter_sql::language())),
            "bash" => Ok(Some(tree_sitter_bash::language())),
            "php" => Ok(Some(tree_sitter_php::language())),
            "ruby" => Ok(Some(tree_sitter_ruby::language())),
            // "swift" => Ok(Some(tree_sitter_swift::language())),
            // "kotlin" => Ok(Some(tree_sitter_kotlin::language())),
            "scala" => Ok(Some(tree_sitter_scala::language())),
            // "haskell" => Ok(Some(tree_sitter_haskell::language())),
            // "lua" => Ok(Some(tree_sitter_lua::language())),
            // "dart" => Ok(Some(tree_sitter_dart::language())),
            // "r" => Ok(Some(tree_sitter_r::language())),
            // "clojure" => Ok(Some(tree_sitter_clojure::language())),
            _ => Ok(None),
        }
    }

    /// Create parser for language
    pub fn create_parser(language: &str) -> Result<Option<Parser>> {
        if let Some(ts_language) = Self::get_language(language)? {
            let mut parser = Parser::new();
            parser.set_language(ts_language)?;
            Ok(Some(parser))
        } else {
            Ok(None)
        }
    }

    /// Parse code with language-specific parser
    pub fn parse_code(code: &str, language: &str) -> Result<Option<tree_sitter::Tree>> {
        if let Some(mut parser) = Self::create_parser(language)? {
            Ok(Some(
                parser
                    .parse(code, None)
                    .ok_or_else(|| anyhow::anyhow!("Failed to parse code"))?,
            ))
        } else {
            Ok(None)
        }
    }

    /// Check if language is supported
    pub fn is_supported(language: &str) -> bool {
        Self::get_language(language).unwrap_or(None).is_some()
    }

    /// Get list of supported languages
    pub fn supported_languages() -> Vec<&'static str> {
        vec![
            "rust",
            "javascript",
            "typescript",
            "python",
            "java",
            "go",
            "cpp",
            "c",
            "html",
            "css",
            "json",
            "yaml",
            "toml",
            "sql",
            "bash",
            "php",
            "ruby",
            "swift",
            "kotlin",
            "scala",
            "haskell",
            "lua",
            "dart",
            "r",
            "clojure",
        ]
    }

    /// Detect language from file extension
    pub fn detect_language_from_extension(file_path: &str) -> Option<&'static str> {
        if let Some(extension) = std::path::Path::new(file_path).extension() {
            match extension.to_str()? {
                "rs" => Some("rust"),
                "js" => Some("javascript"),
                "jsx" => Some("javascript"),
                "ts" => Some("typescript"),
                "tsx" => Some("typescript"),
                "py" => Some("python"),
                "java" => Some("java"),
                "go" => Some("go"),
                "cpp" | "cxx" | "cc" => Some("cpp"),
                "c" | "h" => Some("c"),
                "html" | "htm" => Some("html"),
                "css" => Some("css"),
                "json" => Some("json"),
                "yaml" | "yml" => Some("yaml"),
                "toml" => Some("toml"),
                "sql" => Some("sql"),
                "sh" | "bash" => Some("bash"),
                "php" => Some("php"),
                "rb" => Some("ruby"),
                "swift" => Some("swift"),
                "kt" => Some("kotlin"),
                "scala" => Some("scala"),
                "hs" => Some("haskell"),
                "lua" => Some("lua"),
                "dart" => Some("dart"),
                "r" => Some("r"),
                "clj" | "cljs" => Some("clojure"),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Detect language from file content
    pub fn detect_language_from_content(content: &str) -> Option<&'static str> {
        // Simple heuristics for language detection
        if content.contains("fn main()") || content.contains("use std::") {
            return Some("rust");
        }
        if content.contains("function ") || content.contains("const ") || content.contains("let ") {
            return Some("javascript");
        }
        if content.contains("def ") || content.contains("import ") || content.contains("from ") {
            return Some("python");
        }
        if content.contains("public class ") || content.contains("import java.") {
            return Some("java");
        }
        if content.contains("package main") || content.contains("func ") {
            return Some("go");
        }
        if content.contains("#include") || content.contains("int main(") {
            return Some("c");
        }
        if content.contains("#include") && (content.contains("class ") || content.contains("std::"))
        {
            return Some("cpp");
        }

        None
    }
}
