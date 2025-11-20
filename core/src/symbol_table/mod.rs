//! Symbol table database with AST integration for OdinCode
//!
//! This module provides comprehensive symbol table management with AST integration,
//! enabling detailed code analysis and navigation capabilities.

pub mod analysis;
pub mod core;
pub mod extraction;
pub mod references;

// Re-export main types for backward compatibility
pub use analysis::{DuplicateDetector, StatisticsCollector, UsageAnalyzer};
pub use core::{
    ReferenceType, RelationshipType, Symbol, SymbolFilter, SymbolKind, SymbolReference,
    SymbolRelationship, SymbolTableManager, Visibility,
};
pub use extraction::{ASTExtractor, LanguageParsers, SignatureAnalyzer};
pub use references::{HierarchyAnalyzer, ReferenceManager, RelationshipManager};
