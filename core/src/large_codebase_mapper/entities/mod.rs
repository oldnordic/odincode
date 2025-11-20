//! Entities Module
//! 
//! This module defines the core data structures for representing code entities
//! in the OdinCode system.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Represents a code entity (function, class, variable, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeEntity {
    /// Unique identifier for the entity
    pub id: Uuid,
    
    /// Name of the entity
    pub name: String,
    
    /// Type of the entity
    pub entity_type: CodeEntityType,
    
    /// Programming language
    pub language: String,
    
    /// File path where the entity is defined
    pub file_path: String,
    
    /// Line number where the entity is defined
    pub line_number: usize,
    
    /// Column number where the entity is defined
    pub column_number: usize,
    
    /// Scope of the entity (e.g., parent class name)
    pub scope: String,
    
    /// References to other entities that this entity depends on
    pub dependencies: Vec<Uuid>,
    
    /// References to other entities that depend on this entity
    pub accessed_by: Vec<Uuid>,
    
    /// Code content snippet
    pub content: String,
    
    /// Vector embedding for similarity search (optional)
    pub embedding: Option<Vec<f32>>,
    
    /// Complexity score (0.0-1.0)
    pub complexity: f32,
    
    /// Last modification timestamp
    pub last_modified: DateTime<Utc>,
}

/// Types of code entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CodeEntityType {
    /// Function
    Function,
    
    /// Method
    Method,
    
    /// Class
    Class,
    
    /// Struct
    Struct,
    
    /// Interface
    Interface,
    
    /// Variable
    Variable,
    
    /// Constant
    Constant,
    
    /// Module
    Module,
    
    /// Namespace
    Namespace,
    
    /// Type alias
    Type,
    
    /// Enum
    Enum,
}

/// Represents a dependency relationship between code entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dependency {
    /// Unique identifier for the dependency
    pub id: Uuid,
    
    /// Source entity (the one that depends on something)
    pub from_entity: Uuid,
    
    /// Target entity (the one being depended on)
    pub to_entity: Uuid,
    
    /// Type of dependency
    pub dependency_type: DependencyType,
    
    /// Strength of the dependency (0.0-1.0)
    pub strength: f32,
    
    /// File path where the dependency is defined
    pub file_path: String,
    
    /// Line number where the dependency is defined
    pub line_number: usize,
}

/// Types of dependencies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyType {
    /// Function/method call
    Call,
    
    /// Inheritance relationship
    Inheritance,
    
    /// Composition/containment
    Composition,
    
    /// Import/include relationship
    Import,
    
    /// Parameter type dependency
    Parameter,
    
    /// Return type dependency
    Return,
    
    /// Field/property access
    FieldAccess,
    
    /// Variable usage
    VariableUse,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_code_entity_creation() {
        let entity = CodeEntity {
            id: Uuid::new_v4(),
            name: "test_function".to_string(),
            entity_type: CodeEntityType::Function,
            language: "rust".to_string(),
            file_path: "/path/to/file.rs".to_string(),
            line_number: 10,
            column_number: 5,
            scope: "TestModule".to_string(),
            dependencies: vec![],
            accessed_by: vec![],
            content: "fn test_function() {}".to_string(),
            embedding: None,
            complexity: 0.5,
            last_modified: Utc::now(),
        };
        
        assert_eq!(entity.name, "test_function");
        assert_eq!(entity.entity_type, CodeEntityType::Function);
        assert_eq!(entity.language, "rust");
        assert_eq!(entity.file_path, "/path/to/file.rs");
        assert_eq!(entity.line_number, 10);
        assert_eq!(entity.complexity, 0.5);
    }
    
    #[test]
    fn test_dependency_creation() {
        let from_id = Uuid::new_v4();
        let to_id = Uuid::new_v4();
        
        let dependency = Dependency {
            id: Uuid::new_v4(),
            from_entity: from_id,
            to_entity: to_id,
            dependency_type: DependencyType::Call,
            strength: 0.8,
            file_path: "/path/to/file.rs".to_string(),
            line_number: 15,
        };
        
        assert_eq!(dependency.from_entity, from_id);
        assert_eq!(dependency.to_entity, to_id);
        assert_eq!(dependency.dependency_type, DependencyType::Call);
        assert_eq!(dependency.strength, 0.8);
        assert_eq!(dependency.line_number, 15);
    }
}