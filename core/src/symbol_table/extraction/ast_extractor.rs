//! AST-based symbol extraction

use crate::symbol_table::core::{Symbol, SymbolKind, Visibility};
use anyhow::Result;
use sqlx::SqlitePool;
use std::collections::HashMap;
use tree_sitter::{Node, Tree};
use uuid::Uuid;

/// Extractor for symbols from AST
pub struct ASTExtractor {
    pool: SqlitePool,
}

impl ASTExtractor {
    /// Create a new AST extractor
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Extract symbols from AST and store in database
    pub async fn extract_symbols_from_ast(
        &self,
        tree: &Tree,
        file_content: &str,
        file_path: &str,
        language: &str,
    ) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let root_node = tree.root_node();

        self.extract_symbols_recursive(
            &root_node,
            file_content,
            file_path,
            language,
            &mut symbols,
            &mut String::new(),
        )?;

        // Store symbols in database
        for symbol in &symbols {
            self.store_symbol(symbol).await?;
        }

        Ok(symbols)
    }

    /// Recursively extract symbols from AST nodes
    fn extract_symbols_recursive(
        &self,
        node: &Node,
        file_content: &str,
        file_path: &str,
        language: &str,
        symbols: &mut Vec<Symbol>,
        current_scope: &mut String,
    ) -> Result<()> {
        let node_kind = node.kind();

        match language {
            "rust" => {
                self.extract_rust_symbol(node, file_content, file_path, symbols, current_scope)?
            }
            "javascript" | "typescript" => {
                self.extract_js_symbol(node, file_content, file_path, symbols, current_scope)?
            }
            "python" => {
                self.extract_python_symbol(node, file_content, file_path, symbols, current_scope)?
            }
            "java" => {
                self.extract_java_symbol(node, file_content, file_path, symbols, current_scope)?
            }
            "go" => {
                self.extract_go_symbol(node, file_content, file_path, symbols, current_scope)?
            }
            _ => {} // Unsupported language
        }

        // Process child nodes
        let mut child_count = 0;
        for child in node.children(&mut node.walk()) {
            child_count += 1;

            // Update scope for nested symbols
            let old_scope = current_scope.clone();
            if self_is_scope_changing_node(&child) {
                if let Ok(Some(name)) = self.extract_node_name(&child, file_content) {
                    if !current_scope.is_empty() {
                        current_scope.push_str("::");
                    }
                    current_scope.push_str(&name);
                }
            }

            self.extract_symbols_recursive(
                &child,
                file_content,
                file_path,
                language,
                symbols,
                current_scope,
            )?;

            // Restore scope
            *current_scope = old_scope;
        }

        Ok(())
    }

    /// Extract Rust symbols from node
    fn extract_rust_symbol(
        &self,
        node: &Node,
        file_content: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        current_scope: &str,
    ) -> Result<()> {
        match node.kind() {
            "function_item" => {
                if let Some(symbol) = self.create_function_symbol(
                    node,
                    file_content,
                    file_path,
                    "rust",
                    current_scope,
                )? {
                    symbols.push(symbol);
                }
            }
            "struct_item" => {
                if let Some(symbol) =
                    self.create_struct_symbol(node, file_content, file_path, "rust", current_scope)?
                {
                    symbols.push(symbol);
                }
            }
            "enum_item" => {
                if let Some(symbol) =
                    self.create_enum_symbol(node, file_content, file_path, "rust", current_scope)?
                {
                    symbols.push(symbol);
                }
            }
            "impl_item" => {
                // Extract methods from impl blocks
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "function_item" {
                        if let Some(symbol) = self.create_method_symbol(
                            &child,
                            file_content,
                            file_path,
                            "rust",
                            current_scope,
                        )? {
                            symbols.push(symbol);
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Extract JavaScript/TypeScript symbols
    fn extract_js_symbol(
        &self,
        node: &Node,
        file_content: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        current_scope: &str,
    ) -> Result<()> {
        match node.kind() {
            "function_declaration" | "function" => {
                if let Some(symbol) = self.create_function_symbol(
                    node,
                    file_content,
                    file_path,
                    "javascript",
                    current_scope,
                )? {
                    symbols.push(symbol);
                }
            }
            "class_declaration" | "class" => {
                if let Some(symbol) = self.create_class_symbol(
                    node,
                    file_content,
                    file_path,
                    "javascript",
                    current_scope,
                )? {
                    symbols.push(symbol);
                }
            }
            "variable_declaration" => {
                if let Some(symbol) = self.create_variable_symbol(
                    node,
                    file_content,
                    file_path,
                    "javascript",
                    current_scope,
                )? {
                    symbols.push(symbol);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Extract Python symbols
    fn extract_python_symbol(
        &self,
        node: &Node,
        file_content: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        current_scope: &str,
    ) -> Result<()> {
        match node.kind() {
            "function_definition" => {
                if let Some(symbol) = self.create_function_symbol(
                    node,
                    file_content,
                    file_path,
                    "python",
                    current_scope,
                )? {
                    symbols.push(symbol);
                }
            }
            "class_definition" => {
                if let Some(symbol) = self.create_class_symbol(
                    node,
                    file_content,
                    file_path,
                    "python",
                    current_scope,
                )? {
                    symbols.push(symbol);
                }
            }
            "assignment" => {
                if let Some(symbol) = self.create_variable_symbol(
                    node,
                    file_content,
                    file_path,
                    "python",
                    current_scope,
                )? {
                    symbols.push(symbol);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Extract Java symbols
    fn extract_java_symbol(
        &self,
        node: &Node,
        file_content: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        current_scope: &str,
    ) -> Result<()> {
        match node.kind() {
            "method_declaration" => {
                if let Some(symbol) =
                    self.create_method_symbol(node, file_content, file_path, "java", current_scope)?
                {
                    symbols.push(symbol);
                }
            }
            "class_declaration" => {
                if let Some(symbol) =
                    self.create_class_symbol(node, file_content, file_path, "java", current_scope)?
                {
                    symbols.push(symbol);
                }
            }
            "interface_declaration" => {
                if let Some(symbol) = self.create_interface_symbol(
                    node,
                    file_content,
                    file_path,
                    "java",
                    current_scope,
                )? {
                    symbols.push(symbol);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Extract Go symbols
    fn extract_go_symbol(
        &self,
        node: &Node,
        file_content: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        current_scope: &str,
    ) -> Result<()> {
        match node.kind() {
            "function_declaration" => {
                if let Some(symbol) =
                    self.create_function_symbol(node, file_content, file_path, "go", current_scope)?
                {
                    symbols.push(symbol);
                }
            }
            "type_declaration" => {
                if let Some(symbol) =
                    self.create_struct_symbol(node, file_content, file_path, "go", current_scope)?
                {
                    symbols.push(symbol);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Create a function symbol
    fn create_function_symbol(
        &self,
        node: &Node,
        file_content: &str,
        file_path: &str,
        language: &str,
        current_scope: &str,
    ) -> Result<Option<Symbol>> {
        let name = self
            .extract_node_name(node, file_content)?
            .ok_or_else(|| anyhow::anyhow!("Failed to extract function name"))?;
        let signature = self.extract_function_signature(node, file_content)?;

        Ok(Some(Symbol {
            id: Uuid::new_v4().to_string(),
            name,
            kind: SymbolKind::Function,
            file_path: file_path.to_string(),
            line: node.start_position().row as u32 + 1,
            column: node.start_position().column as u32 + 1,
            scope: if current_scope.is_empty() {
                None
            } else {
                Some(current_scope.to_string())
            },
            visibility: self.extract_visibility(node, file_content)?,
            language: language.to_string(),
            signature,
            documentation: self.extract_documentation(node, file_content)?,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        }))
    }

    /// Create a method symbol
    fn create_method_symbol(
        &self,
        node: &Node,
        file_content: &str,
        file_path: &str,
        language: &str,
        current_scope: &str,
    ) -> Result<Option<Symbol>> {
        let name = self
            .extract_node_name(node, file_content)?
            .ok_or_else(|| anyhow::anyhow!("Failed to extract name"))?;
        let signature = self.extract_function_signature(node, file_content)?;

        Ok(Some(Symbol {
            id: Uuid::new_v4().to_string(),
            name,
            kind: SymbolKind::Method,
            file_path: file_path.to_string(),
            line: node.start_position().row as u32 + 1,
            column: node.start_position().column as u32 + 1,
            scope: if current_scope.is_empty() {
                None
            } else {
                Some(current_scope.to_string())
            },
            visibility: self.extract_visibility(node, file_content)?,
            language: language.to_string(),
            signature,
            documentation: self.extract_documentation(node, file_content)?,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        }))
    }

    /// Create a class symbol
    fn create_class_symbol(
        &self,
        node: &Node,
        file_content: &str,
        file_path: &str,
        language: &str,
        current_scope: &str,
    ) -> Result<Option<Symbol>> {
        let name = self
            .extract_node_name(node, file_content)?
            .ok_or_else(|| anyhow::anyhow!("Failed to extract name"))?;

        Ok(Some(Symbol {
            id: Uuid::new_v4().to_string(),
            name,
            kind: SymbolKind::Class,
            file_path: file_path.to_string(),
            line: node.start_position().row as u32 + 1,
            column: node.start_position().column as u32 + 1,
            scope: if current_scope.is_empty() {
                None
            } else {
                Some(current_scope.to_string())
            },
            visibility: self.extract_visibility(node, file_content)?,
            language: language.to_string(),
            signature: None,
            documentation: self.extract_documentation(node, file_content)?,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        }))
    }

    /// Create a struct symbol
    fn create_struct_symbol(
        &self,
        node: &Node,
        file_content: &str,
        file_path: &str,
        language: &str,
        current_scope: &str,
    ) -> Result<Option<Symbol>> {
        let name = self
            .extract_node_name(node, file_content)?
            .ok_or_else(|| anyhow::anyhow!("Failed to extract name"))?;

        Ok(Some(Symbol {
            id: Uuid::new_v4().to_string(),
            name,
            kind: SymbolKind::Struct,
            file_path: file_path.to_string(),
            line: node.start_position().row as u32 + 1,
            column: node.start_position().column as u32 + 1,
            scope: if current_scope.is_empty() {
                None
            } else {
                Some(current_scope.to_string())
            },
            visibility: self.extract_visibility(node, file_content)?,
            language: language.to_string(),
            signature: None,
            documentation: self.extract_documentation(node, file_content)?,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        }))
    }

    /// Create an enum symbol
    fn create_enum_symbol(
        &self,
        node: &Node,
        file_content: &str,
        file_path: &str,
        language: &str,
        current_scope: &str,
    ) -> Result<Option<Symbol>> {
        let name = self
            .extract_node_name(node, file_content)?
            .ok_or_else(|| anyhow::anyhow!("Failed to extract name"))?;

        Ok(Some(Symbol {
            id: Uuid::new_v4().to_string(),
            name,
            kind: SymbolKind::Enum,
            file_path: file_path.to_string(),
            line: node.start_position().row as u32 + 1,
            column: node.start_position().column as u32 + 1,
            scope: if current_scope.is_empty() {
                None
            } else {
                Some(current_scope.to_string())
            },
            visibility: self.extract_visibility(node, file_content)?,
            language: language.to_string(),
            signature: None,
            documentation: self.extract_documentation(node, file_content)?,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        }))
    }

    /// Create an interface symbol
    fn create_interface_symbol(
        &self,
        node: &Node,
        file_content: &str,
        file_path: &str,
        language: &str,
        current_scope: &str,
    ) -> Result<Option<Symbol>> {
        let name = self
            .extract_node_name(node, file_content)?
            .ok_or_else(|| anyhow::anyhow!("Failed to extract name"))?;

        Ok(Some(Symbol {
            id: Uuid::new_v4().to_string(),
            name,
            kind: SymbolKind::Interface,
            file_path: file_path.to_string(),
            line: node.start_position().row as u32 + 1,
            column: node.start_position().column as u32 + 1,
            scope: if current_scope.is_empty() {
                None
            } else {
                Some(current_scope.to_string())
            },
            visibility: self.extract_visibility(node, file_content)?,
            language: language.to_string(),
            signature: None,
            documentation: self.extract_documentation(node, file_content)?,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        }))
    }

    /// Create a variable symbol
    fn create_variable_symbol(
        &self,
        node: &Node,
        file_content: &str,
        file_path: &str,
        language: &str,
        current_scope: &str,
    ) -> Result<Option<Symbol>> {
        let name = self
            .extract_node_name(node, file_content)?
            .ok_or_else(|| anyhow::anyhow!("Failed to extract name"))?;

        Ok(Some(Symbol {
            id: Uuid::new_v4().to_string(),
            name,
            kind: SymbolKind::Variable,
            file_path: file_path.to_string(),
            line: node.start_position().row as u32 + 1,
            column: node.start_position().column as u32 + 1,
            scope: if current_scope.is_empty() {
                None
            } else {
                Some(current_scope.to_string())
            },
            visibility: self.extract_visibility(node, file_content)?,
            language: language.to_string(),
            signature: None,
            documentation: None,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        }))
    }

    /// Extract node name
    fn extract_node_name(&self, node: &Node, file_content: &str) -> Result<Option<String>> {
        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "identifier" | "type_identifier" => {
                    let start = child.start_byte();
                    let end = child.end_byte();
                    return Ok(Some(file_content[start..end].to_string()));
                }
                _ => {}
            }
        }
        Ok(None)
    }

    /// Extract function signature
    fn extract_function_signature(
        &self,
        node: &Node,
        file_content: &str,
    ) -> Result<Option<String>> {
        let start = node.start_byte();
        let end = node.end_byte();
        Ok(Some(file_content[start..end].to_string()))
    }

    /// Extract visibility
    fn extract_visibility(&self, node: &Node, file_content: &str) -> Result<Visibility> {
        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "public" => return Ok(Visibility::Public),
                "private" => return Ok(Visibility::Private),
                "protected" => return Ok(Visibility::Protected),
                "internal" => return Ok(Visibility::Internal),
                _ => {}
            }
        }
        Ok(Visibility::Private) // Default
    }

    /// Extract documentation
    fn extract_documentation(&self, node: &Node, file_content: &str) -> Result<Option<String>> {
        // Look for comments before the node
        let mut walker = node.walk();
        let prev_sibling = node.prev_sibling();
        if let Some(sibling) = prev_sibling {
            if sibling.kind() == "comment"
                || sibling.kind() == "line_comment"
                || sibling.kind() == "block_comment"
            {
                let start = sibling.start_byte();
                let end = sibling.end_byte();
                return Ok(Some(file_content[start..end].to_string()));
            }
        }
        Ok(None)
    }

    /// Store symbol in database
    async fn store_symbol(&self, symbol: &Symbol) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO symbols (
                id, name, kind, file_path, line, column, scope, visibility, 
                language, signature, documentation, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&symbol.id)
        .bind(&symbol.name)
        .bind(symbol.kind.as_str())
        .bind(&symbol.file_path)
        .bind(symbol.line)
        .bind(symbol.column)
        .bind(&symbol.scope)
        .bind(symbol.visibility.as_str())
        .bind(&symbol.language)
        .bind(&symbol.signature)
        .bind(&symbol.documentation)
        .bind(symbol.created_at)
        .bind(symbol.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Check if node creates a new scope
fn self_is_scope_changing_node(node: &Node) -> bool {
    matches!(
        node.kind(),
        "function_item"
            | "class_declaration"
            | "struct_item"
            | "enum_item"
            | "impl_item"
            | "function_definition"
            | "class_definition"
            | "method_declaration"
            | "interface_declaration"
    )
}
