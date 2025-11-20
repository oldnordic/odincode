//! Advanced AI coding features for OdinCode
//!
//! This module provides high-performance AI coding assistance features implemented in Rust
//! to address limitations in existing tools, with focus on context awareness, refactoring,
//! dependency analysis, and pattern recognition.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::language_parsing::{LanguageParser, SupportedLanguage};
use crate::CodeFile;
use tree_sitter::{Node, Tree};

/// Represents a symbol in the code (function, variable, class, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub id: Uuid,
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub scope: Option<String>, // Parent scope (e.g., class name for methods)
    pub visibility: Visibility,
    pub language: SupportedLanguage,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Variable,
    Class,
    Interface,
    Enum,
    Struct,
    Trait,
    Module,
    Import,
    Parameter,
    Field,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

/// Represents a reference to a symbol in the code
#[derive(Debug, Clone)]
pub struct SymbolReference {
    pub symbol_id: Uuid,
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub reference_type: ReferenceType,
}

#[derive(Debug, Clone)]
pub enum ReferenceType {
    Usage,
    Definition,
    Declaration,
    Call,
    Assignment,
}

/// Represents a code pattern or anti-pattern
#[derive(Debug, Clone)]
pub struct CodePattern {
    pub id: Uuid,
    pub name: String,
    pub pattern_type: PatternType,
    pub description: String,
    pub severity: PatternSeverity,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub code_snippet: String,
}

#[derive(Debug, Clone)]
pub enum PatternType {
    DesignPattern,
    AntiPattern,
    BestPractice,
    PerformanceIssue,
    SecurityIssue,
}

#[derive(Debug, Clone)]
pub enum PatternSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Main engine for advanced AI coding features
pub struct AdvancedFeaturesEngine {
    /// Symbol table for tracking all symbols in the codebase
    symbol_table: Arc<RwLock<HashMap<Uuid, Symbol>>>,

    /// Reference tracking for symbol usage
    reference_map: Arc<RwLock<HashMap<Uuid, Vec<SymbolReference>>>>,

    /// File dependency graph
    dependency_graph: Arc<RwLock<HashMap<String, Vec<String>>>>,

    /// Pattern cache for identified patterns
    pattern_cache: Arc<RwLock<HashMap<Uuid, CodePattern>>>,

    /// Language parser for AST analysis
    language_parser: Arc<RwLock<LanguageParser>>,
}

impl AdvancedFeaturesEngine {
    /// Create a new advanced features engine
    pub async fn new() -> Result<Self> {
        Ok(Self {
            symbol_table: Arc::new(RwLock::new(HashMap::new())),
            reference_map: Arc::new(RwLock::new(HashMap::new())),
            dependency_graph: Arc::new(RwLock::new(HashMap::new())),
            pattern_cache: Arc::new(RwLock::new(HashMap::new())),
            language_parser: Arc::new(RwLock::new(LanguageParser::new()?)),
        })
    }

    /// Process a code file and extract symbols, references, and patterns
    pub async fn process_file(&self, file: &CodeFile) -> Result<()> {
        let mut parser = self.language_parser.write().await;
        let language = SupportedLanguage::from_str(&file.language)
            .ok_or_else(|| anyhow::anyhow!("Unsupported language: {}", file.language))?;

        let tree = parser.parse(&file.content, &language)?;
        drop(parser);

        // Extract symbols from the AST
        self.extract_symbols(&tree, file).await?;

        // Extract references from the AST
        self.extract_references(&tree, file).await?;

        // Analyze patterns in the code
        self.analyze_patterns(&tree, file).await?;

        // Update dependency graph
        self.update_dependency_graph(file).await?;

        Ok(())
    }

    /// Extract symbols from the AST
    async fn extract_symbols(&self, tree: &Tree, file: &CodeFile) -> Result<()> {
        let mut symbols = Vec::new();
        self.traverse_ast_for_symbols(tree.root_node(), file, &mut symbols, None)?;

        let mut symbol_table = self.symbol_table.write().await;
        for symbol in symbols {
            symbol_table.insert(symbol.id, symbol);
        }

        Ok(())
    }

    /// Traverse the AST to find symbols
    fn traverse_ast_for_symbols(
        &self,
        node: Node,
        file: &CodeFile,
        symbols: &mut Vec<Symbol>,
        parent_scope: Option<String>,
    ) -> Result<()> {
        // Identify different types of symbols based on the node type
        let symbol_kind = match node.kind() {
            "function_declaration"
            | "function_definition"
            | "method_definition"
            | "function_item" => Some(SymbolKind::Function),
            "variable_declaration" | "variable_declarator" => Some(SymbolKind::Variable),
            "class_declaration" | "class_definition" => Some(SymbolKind::Class),
            "interface_declaration" => Some(SymbolKind::Interface),
            "enum_declaration" | "enum_item" => Some(SymbolKind::Enum),
            "struct_declaration" | "struct_item" => Some(SymbolKind::Struct),
            "trait_declaration" | "trait_item" => Some(SymbolKind::Trait),
            "module" | "namespace" | "mod_item" => Some(SymbolKind::Module),
            "import_statement" | "import_declaration" | "use_declaration" => {
                Some(SymbolKind::Import)
            }
            "parameter" => Some(SymbolKind::Parameter),
            "field_identifier" | "field_declaration" => Some(SymbolKind::Field),
            _ => None,
        };

        if let Some(kind) = symbol_kind {
            // Get the symbol name (this varies by language and node type)
            let name = self.get_node_name(node, &file.content)?;

            if !name.is_empty() {
                let start = node.start_position();
                let symbol = Symbol {
                    id: Uuid::new_v4(),
                    name,
                    kind,
                    file_path: file.path.clone(),
                    line: start.row + 1,
                    column: start.column,
                    scope: parent_scope.clone(),
                    visibility: self.infer_visibility(node, &file.content)?,
                    language: SupportedLanguage::from_str(&file.language).ok_or_else(|| {
                        anyhow::anyhow!("Unsupported language: {}", file.language)
                    })?,
                };
                symbols.push(symbol);
            }
        }

        // Process child nodes
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // For classes, functions, etc., update the parent scope
            let new_parent_scope = if matches!(
                node.kind(),
                "class_declaration"
                    | "class_definition"
                    | "function_declaration"
                    | "function_definition"
                    | "struct_declaration"
                    | "trait_declaration"
            ) {
                self.get_node_name(node, &file.content).ok()
            } else {
                parent_scope.clone()
            };

            self.traverse_ast_for_symbols(child, file, symbols, new_parent_scope)?;
        }

        Ok(())
    }

    /// Extract references to symbols from the AST
    async fn extract_references(&self, tree: &Tree, file: &CodeFile) -> Result<()> {
        let mut references = Vec::new();
        self.traverse_ast_for_references(tree.root_node(), file, &mut references)?;

        let mut ref_map = self.reference_map.write().await;
        for reference in references {
            ref_map
                .entry(reference.symbol_id)
                .or_insert_with(Vec::new)
                .push(reference);
        }

        Ok(())
    }

    /// Traverse the AST to find references to symbols
    fn traverse_ast_for_references(
        &self,
        node: Node,
        file: &CodeFile,
        references: &mut Vec<SymbolReference>,
    ) -> Result<()> {
        // This is a simplified implementation - in a real system, we would need to
        // match identifiers to symbols in our symbol table
        match node.kind() {
            "identifier" | "field_identifier" | "call_expression" => {
                // In a real implementation, we would try to match this identifier
                // to a symbol in our symbol table and create a reference
                let start = node.start_position();
                let identifier_name = self.get_node_text(node, &file.content)?;

                // For now, we'll just record that there's an identifier at this location
                // In a real system, we would resolve it to an actual symbol
                println!(
                    "Found identifier: {} at {}:{} in {}",
                    identifier_name,
                    start.row + 1,
                    start.column,
                    file.path
                );
            }
            _ => {}
        }

        // Process child nodes
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.traverse_ast_for_references(child, file, references)?;
        }

        Ok(())
    }

    /// Analyze patterns in the code
    async fn analyze_patterns(&self, tree: &Tree, file: &CodeFile) -> Result<()> {
        let mut patterns = Vec::new();
        self.traverse_ast_for_patterns(tree.root_node(), file, &mut patterns)?;

        let mut pattern_cache = self.pattern_cache.write().await;
        for pattern in patterns {
            pattern_cache.insert(pattern.id, pattern);
        }

        Ok(())
    }

    /// Traverse the AST to identify patterns
    fn traverse_ast_for_patterns(
        &self,
        node: Node,
        file: &CodeFile,
        patterns: &mut Vec<CodePattern>,
    ) -> Result<()> {
        // Identify potential patterns based on AST structure
        match node.kind() {
            // Potential performance issues
            "for_in_statement" => {
                // JavaScript for-in loops can be slow for arrays
                if file.language == "javascript" || file.language == "typescript" {
                    let start = node.start_position();
                    let end = node.end_position();

                    let code_snippet = file
                        .content
                        .lines()
                        .skip(start.row)
                        .take(end.row - start.row + 1)
                        .collect::<Vec<_>>()
                        .join("\n");

                    patterns.push(CodePattern {
                        id: Uuid::new_v4(),
                        name: "Inefficient for-in loop".to_string(),
                        pattern_type: PatternType::PerformanceIssue,
                        description: "Using for-in on arrays can be inefficient, consider using for-of or traditional for loop".to_string(),
                        severity: PatternSeverity::Medium,
                        file_path: file.path.clone(),
                        start_line: start.row + 1,
                        end_line: end.row + 1,
                        code_snippet,
                    });
                }
            }
            // Potential security issues
            "call_expression" => {
                // Check for potentially unsafe operations
                let call_text = self.get_node_text(node, &file.content)?;
                if call_text.contains("eval") || call_text.contains("exec") {
                    let start = node.start_position();
                    let end = node.end_position();

                    let code_snippet = file
                        .content
                        .lines()
                        .skip(start.row)
                        .take(end.row - start.row + 1)
                        .collect::<Vec<_>>()
                        .join("\n");

                    patterns.push(CodePattern {
                        id: Uuid::new_v4(),
                        name: "Potentially unsafe function call".to_string(),
                        pattern_type: PatternType::SecurityIssue,
                        description: "Use of potentially unsafe function that could lead to injection vulnerabilities".to_string(),
                        severity: PatternSeverity::High,
                        file_path: file.path.clone(),
                        start_line: start.row + 1,
                        end_line: end.row + 1,
                        code_snippet,
                    });
                }
            }
            _ => {}
        }

        // Process child nodes
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.traverse_ast_for_patterns(child, file, patterns)?;
        }

        Ok(())
    }

    /// Update the dependency graph based on import statements
    async fn update_dependency_graph(&self, file: &CodeFile) -> Result<()> {
        // This is a simplified dependency analysis
        // In a real implementation, we would extract import/require/use statements
        // and build a proper dependency graph

        let mut graph = self.dependency_graph.write().await;

        // For now, we'll just add the file to the graph with empty dependencies
        // In reality, we would parse the file to find imports and dependencies
        graph.insert(file.path.clone(), Vec::new());

        Ok(())
    }

    /// Get the text content of a node
    fn get_node_text(&self, node: Node, source_code: &str) -> Result<String> {
        Ok(source_code[node.start_byte()..node.end_byte()].to_string())
    }

    /// Get the name of a node (identifier name)
    fn get_node_name(&self, node: Node, source_code: &str) -> Result<String> {
        // For many language constructs, the name is in a child identifier node
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "type_identifier" {
                return Ok(source_code[child.start_byte()..child.end_byte()].to_string());
            }
        }

        // If no identifier child found, return the node text
        Ok(source_code[node.start_byte()..node.end_byte()].to_string())
    }

    /// Infer visibility from a node
    fn infer_visibility(&self, node: Node, source_code: &str) -> Result<Visibility> {
        let node_text = &source_code[node.start_byte()..node.end_byte()];

        if node_text.contains("private") || node_text.contains("protected") {
            Ok(Visibility::Private)
        } else {
            Ok(Visibility::Public) // Default to public
        }
    }

    /// Get all symbols for a specific file
    pub async fn get_symbols_for_file(&self, file_path: &str) -> Result<Vec<Symbol>> {
        let symbols = self.symbol_table.read().await;
        Ok(symbols
            .values()
            .filter(|symbol| symbol.file_path == file_path)
            .cloned()
            .collect())
    }

    /// Get all references to a specific symbol
    pub async fn get_references_to_symbol(&self, symbol_id: Uuid) -> Result<Vec<SymbolReference>> {
        let references = self.reference_map.read().await;
        Ok(references.get(&symbol_id).cloned().unwrap_or_default())
    }

    /// Get all patterns found in a specific file
    pub async fn get_patterns_for_file(&self, file_path: &str) -> Result<Vec<CodePattern>> {
        let patterns = self.pattern_cache.read().await;
        Ok(patterns
            .values()
            .filter(|pattern| pattern.file_path == file_path)
            .cloned()
            .collect())
    }

    /// Find symbols by name (for code completion)
    pub async fn find_symbols_by_name(&self, name: &str) -> Result<Vec<Symbol>> {
        let symbols = self.symbol_table.read().await;
        let name_lower = name.to_lowercase();
        Ok(symbols
            .values()
            .filter(|symbol| symbol.name.to_lowercase().contains(&name_lower))
            .cloned()
            .collect())
    }

    /// Get symbol context for a specific location in a file
    pub async fn get_context_at_position(
        &self,
        file_path: &str,
        line: usize,
        column: usize,
    ) -> Result<Context> {
        // Find the symbol at the given position
        let symbols = self.symbol_table.read().await;
        let target_symbol = symbols
            .values()
            .find(|symbol| {
                symbol.file_path == file_path && symbol.line == line && symbol.column <= column
            })
            .cloned();

        // Get related symbols (references, definitions, etc.)
        let mut related_symbols: Vec<Symbol> = Vec::new();
        if let Some(ref sym) = target_symbol {
            if let Ok(refs) = self.get_references_to_symbol(sym.id).await {
                for reference in refs {
                    let sym_table = self.symbol_table.read().await;
                    if let Some(ref_symbol) = sym_table.get(&reference.symbol_id) {
                        if !related_symbols.contains(ref_symbol) {
                            related_symbols.push(ref_symbol.clone());
                        }
                    }
                }
            }
        }

        Ok(Context {
            current_symbol: target_symbol,
            related_symbols,
            references: Vec::new(), // Would populate with actual references in a full implementation
        })
    }
}

/// Represents the context around a specific location in code
pub struct Context {
    pub current_symbol: Option<Symbol>,
    pub related_symbols: Vec<Symbol>,
    pub references: Vec<SymbolReference>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodeFile;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_advanced_features_engine_creation() {
        let engine = AdvancedFeaturesEngine::new().await.unwrap();
        assert!(engine.symbol_table.read().await.is_empty());
        assert!(engine.reference_map.read().await.is_empty());
        assert!(engine.dependency_graph.read().await.is_empty());
        assert!(engine.pattern_cache.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_process_rust_file() {
        let engine = AdvancedFeaturesEngine::new().await.unwrap();

        let rust_code = r#"
            pub struct MyStruct {
                pub field: i32,
            }
            
            impl MyStruct {
                pub fn new(value: i32) -> Self {
                    Self { field: value }
                }
                
                pub fn get_field(&self) -> i32 {
                    self.field
                }
            }
        "#;

        let file = CodeFile {
            id: Uuid::new_v4(),
            path: "test.rs".to_string(),
            content: rust_code.to_string(),
            language: "rust".to_string(),
            modified: chrono::Utc::now(),
        };

        engine.process_file(&file).await.unwrap();

        // Check that symbols were extracted
        let symbols = engine.get_symbols_for_file("test.rs").await.unwrap();
        assert!(!symbols.is_empty());

        // Check for struct symbol
        let struct_symbol = symbols
            .iter()
            .find(|s| s.name == "MyStruct" && matches!(s.kind, SymbolKind::Struct));
        assert!(struct_symbol.is_some());

        // Check for function symbols
        let functions: Vec<_> = symbols
            .iter()
            .filter(|s| matches!(s.kind, SymbolKind::Function))
            .collect();
        assert!(!functions.is_empty());
    }

    #[tokio::test]
    async fn test_process_javascript_file() {
        let engine = AdvancedFeaturesEngine::new().await.unwrap();

        let js_code = r#"
            function calculateSum(arr) {
                let sum = 0;
                for (let i = 0; i < arr.length; i++) {
                    sum += arr[i];
                }
                return sum;
            }
            
            // This is potentially inefficient
            for (let item in array) {
                console.log(item);
            }
        "#;

        let file = CodeFile {
            id: Uuid::new_v4(),
            path: "test.js".to_string(),
            content: js_code.to_string(),
            language: "javascript".to_string(),
            modified: chrono::Utc::now(),
        };

        engine.process_file(&file).await.unwrap();

        // Check that patterns were identified
        let patterns = engine.get_patterns_for_file("test.js").await.unwrap();
        let inefficient_loops: Vec<_> = patterns
            .iter()
            .filter(|p| p.name.contains("Inefficient"))
            .collect();

        // We expect to find the inefficient for-in pattern
        assert!(!inefficient_loops.is_empty());
    }

    #[tokio::test]
    async fn test_find_symbols_by_name() {
        let engine = AdvancedFeaturesEngine::new().await.unwrap();

        let rust_code = r#"
            pub fn my_function() -> i32 {
                42
            }
            
            pub struct MyStruct {
                value: i32,
            }
        "#;

        let file = CodeFile {
            id: Uuid::new_v4(),
            path: "test.rs".to_string(),
            content: rust_code.to_string(),
            language: "rust".to_string(),
            modified: chrono::Utc::now(),
        };

        engine.process_file(&file).await.unwrap();

        // Find symbols containing "my"
        let symbols = engine.find_symbols_by_name("my").await.unwrap();
        assert!(!symbols.is_empty());

        // Should find both "my_function" and "MyStruct"
        let function_symbol = symbols.iter().find(|s| s.name == "my_function");
        let struct_symbol = symbols.iter().find(|s| s.name == "MyStruct");
        assert!(function_symbol.is_some());
        assert!(struct_symbol.is_some());
    }
}
