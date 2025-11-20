//! Type conversion helpers for database query results
//!
//! Provides conversion functions from string representations to strongly-typed enums
//! for node types, relationship types, and chunk types.

use crate::graph_database::{NodeType, RelationshipType};
use crate::rag_database::ChunkType;
use anyhow::Result;

/// Convert string to NodeType
pub fn str_to_node_type(s: &str) -> Result<NodeType> {
    match s {
        "function" => Ok(NodeType::Function),
        "class" => Ok(NodeType::Class),
        "module" => Ok(NodeType::Module),
        "variable" => Ok(NodeType::Variable),
        "interface" => Ok(NodeType::Interface),
        "enum" => Ok(NodeType::Enum),
        "struct" => Ok(NodeType::Struct),
        "trait" => Ok(NodeType::Trait),
        "file" => Ok(NodeType::File),
        "package" => Ok(NodeType::Package),
        "import" => Ok(NodeType::Import),
        _ => Err(anyhow::anyhow!("Invalid node type: {}", s)),
    }
}

/// Convert string to RelationshipType
pub fn str_to_relationship_type(s: &str) -> Result<RelationshipType> {
    match s {
        "contains" => Ok(RelationshipType::Contains),
        "imports" => Ok(RelationshipType::Imports),
        "calls" => Ok(RelationshipType::Calls),
        "extends" => Ok(RelationshipType::Extends),
        "implements" => Ok(RelationshipType::Implements),
        "uses" => Ok(RelationshipType::Uses),
        "parameter" => Ok(RelationshipType::Parameter),
        "return" => Ok(RelationshipType::Return),
        "field" => Ok(RelationshipType::Field),
        "dependency" => Ok(RelationshipType::Dependency),
        "reference" => Ok(RelationshipType::Reference),
        _ => Err(anyhow::anyhow!("Invalid relationship type: {}", s)),
    }
}

/// Convert string to ChunkType
pub fn str_to_chunk_type(s: &str) -> Result<ChunkType> {
    match s {
        "function" => Ok(ChunkType::Function),
        "class" => Ok(ChunkType::Class),
        "method" => Ok(ChunkType::Method),
        "module" => Ok(ChunkType::Module),
        "block" => Ok(ChunkType::Block),
        "statement" => Ok(ChunkType::Statement),
        "expression" => Ok(ChunkType::Expression),
        "comment" => Ok(ChunkType::Comment),
        "documentation" => Ok(ChunkType::Documentation),
        "test" => Ok(ChunkType::Test),
        _ => Err(anyhow::anyhow!("Invalid chunk type: {}", s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_to_node_type() {
        assert!(matches!(
            str_to_node_type("function").unwrap(),
            NodeType::Function
        ));
        assert!(matches!(
            str_to_node_type("class").unwrap(),
            NodeType::Class
        ));
        assert!(str_to_node_type("invalid").is_err());
    }

    #[test]
    fn test_str_to_relationship_type() {
        assert!(matches!(
            str_to_relationship_type("contains").unwrap(),
            RelationshipType::Contains
        ));
        assert!(matches!(
            str_to_relationship_type("calls").unwrap(),
            RelationshipType::Calls
        ));
        assert!(str_to_relationship_type("invalid").is_err());
    }

    #[test]
    fn test_str_to_chunk_type() {
        assert!(matches!(
            str_to_chunk_type("function").unwrap(),
            ChunkType::Function
        ));
        assert!(matches!(
            str_to_chunk_type("module").unwrap(),
            ChunkType::Module
        ));
        assert!(str_to_chunk_type("invalid").is_err());
    }
}
