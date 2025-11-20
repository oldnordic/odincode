//! Core symbol types and enums

use serde::{Deserialize, Serialize};

/// Symbol information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: String,
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub scope: Option<String>,
    pub visibility: Visibility,
    pub language: String,
    pub signature: Option<String>,
    pub documentation: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Types of symbols
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Variable,
    Constant,
    Class,
    Struct,
    Interface,
    Enum,
    Trait,
    Module,
    Namespace,
    Package,
    Import,
    Parameter,
    Field,
    Property,
    Event,
    Macro,
    Template,
    TypeAlias,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Variable => "variable",
            SymbolKind::Constant => "constant",
            SymbolKind::Class => "class",
            SymbolKind::Struct => "struct",
            SymbolKind::Interface => "interface",
            SymbolKind::Enum => "enum",
            SymbolKind::Trait => "trait",
            SymbolKind::Module => "module",
            SymbolKind::Namespace => "namespace",
            SymbolKind::Package => "package",
            SymbolKind::Import => "import",
            SymbolKind::Parameter => "parameter",
            SymbolKind::Field => "field",
            SymbolKind::Property => "property",
            SymbolKind::Event => "event",
            SymbolKind::Macro => "macro",
            SymbolKind::Template => "template",
            SymbolKind::TypeAlias => "type_alias",
        }
    }
}

/// Visibility levels for symbols
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    Package,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::Private => "private",
            Visibility::Protected => "protected",
            Visibility::Internal => "internal",
            Visibility::Package => "package",
        }
    }
}

/// Symbol reference structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReference {
    pub id: String,
    pub symbol_id: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub reference_type: ReferenceType,
    pub created_at: i64,
}

/// Types of symbol references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReferenceType {
    Usage,
    Definition,
    Declaration,
    Call,
    Assignment,
    Inheritance,
    Implementation,
    Import,
    Export,
}

impl ReferenceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReferenceType::Usage => "usage",
            ReferenceType::Definition => "definition",
            ReferenceType::Declaration => "declaration",
            ReferenceType::Call => "call",
            ReferenceType::Assignment => "assignment",
            ReferenceType::Inheritance => "inheritance",
            ReferenceType::Implementation => "implementation",
            ReferenceType::Import => "import",
            ReferenceType::Export => "export",
        }
    }
}

/// Symbol relationship structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRelationship {
    pub id: String,
    pub from_symbol_id: String,
    pub to_symbol_id: String,
    pub relationship_type: RelationshipType,
    pub created_at: i64,
}

/// Types of symbol relationships
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    Calls,
    Inherits,
    Implements,
    Uses,
    Contains,
    DependsOn,
    Overrides,
    Extends,
}

impl RelationshipType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationshipType::Calls => "calls",
            RelationshipType::Inherits => "inherits",
            RelationshipType::Implements => "implements",
            RelationshipType::Uses => "uses",
            RelationshipType::Contains => "contains",
            RelationshipType::DependsOn => "depends_on",
            RelationshipType::Overrides => "overrides",
            RelationshipType::Extends => "extends",
        }
    }
}

/// Filter criteria for symbol queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolFilter {
    pub name_pattern: Option<String>,
    pub kind: Option<SymbolKind>,
    pub file_path: Option<String>,
    pub language: Option<String>,
    pub visibility: Option<Visibility>,
    pub scope: Option<String>,
}
