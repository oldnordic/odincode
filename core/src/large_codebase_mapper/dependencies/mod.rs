//! Dependencies Module
//! 
//! This module provides functionality for analyzing dependencies between code entities.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use tracing::{debug, info};

use crate::code_mapper::entities::{CodeEntity, Dependency, DependencyType};

/// Dependency analyzer for code entities
pub struct DependencyAnalyzer;

impl DependencyAnalyzer {
    /// Create a new dependency analyzer
    pub fn new() -> Self {
        Self
    }
    
    /// Analyze dependencies in a code file
    pub fn analyze_dependencies(
        &self,
        entities: &[CodeEntity],
        file_content: &str,
    ) -> Result<Vec<Dependency>> {
        debug!("Analyzing dependencies in file with {} entities", entities.len());
        
        let mut dependencies = Vec::new();
        let entity_map: HashMap<&str, &CodeEntity> = entities
            .iter()
            .map(|e| (e.name.as_str(), e))
            .collect();
        
        let lines: Vec<&str> = file_content.lines().collect();
        
        for (line_idx, line) in lines.iter().enumerate() {
            // Look for function calls (simplified)
            for (name, entity) in &entity_map {
                if line.contains(&format!("{}(", name)) {
                    // Check if this is a call to a different entity in the same file
                    if let Some(calling_entity) = entities.first() {
                        let dependency = Dependency {
                            id: Uuid::new_v4(),
                            from_entity: calling_entity.id,
                            to_entity: entity.id,
                            dependency_type: DependencyType::Call,
                            strength: 1.0,
                            file_path: entity.file_path.clone(),
                            line_number: line_idx + 1,
                        };
                        dependencies.push(dependency);
                    }
                }
            }
        }
        
        info!("Found {} dependencies", dependencies.len());
        Ok(dependencies)
    }
    
    /// Find the entity that contains a specific line (simplified)
    pub fn find_calling_entity(
        &self,
        lines: &[(usize, &str)],
        line_idx: usize,
        entities: &[CodeEntity],
    ) -> Result<Uuid> {
        // In a real implementation, this would use AST to find the containing entity
        // For now, we'll return a placeholder
        if let Some(entity) = entities.first() {
            Ok(entity.id)
        } else {
            Ok(Uuid::new_v4())
        }
    }
    
    /// Get all dependencies for an entity
    pub fn get_dependencies_for_entity(
        &self,
        entity_id: Uuid,
        dependencies: &[Dependency],
    ) -> Vec<&Dependency> {
        dependencies
            .iter()
            .filter(|dep| dep.from_entity == entity_id)
            .collect()
    }
    
    /// Get all entities that depend on a specific entity
    pub fn get_dependents_of_entity(
        &self,
        entity_id: Uuid,
        dependencies: &[Dependency],
    ) -> Vec<&Dependency> {
        dependencies
            .iter()
            .filter(|dep| dep.to_entity == entity_id)
            .collect()
    }
    
    /// Calculate dependency strength based on usage patterns
    pub fn calculate_dependency_strength(
        &self,
        dependency_type: &DependencyType,
        usage_count: usize,
    ) -> f32 {
        let base_strength = match dependency_type {
            DependencyType::Call => 0.8,
            DependencyType::Inheritance => 1.0,
            DependencyType::Composition => 0.9,
            DependencyType::Import => 0.7,
            DependencyType::Parameter => 0.6,
            DependencyType::Return => 0.6,
            DependencyType::FieldAccess => 0.5,
            DependencyType::VariableUse => 0.4,
        };
        
        // Adjust strength based on usage count
        let usage_factor = (usage_count as f32 / 10.0).min(1.0);
        (base_strength + usage_factor * 0.2).min(1.0)
    }
    
    /// Build a dependency graph from entities and dependencies
    pub fn build_dependency_graph(
        &self,
        entities: &[CodeEntity],
        dependencies: &[Dependency],
    ) -> DependencyGraph {
        let mut graph = DependencyGraph::new();
        
        // Add entities to graph
        for entity in entities {
            graph.add_entity(entity.clone());
        }
        
        // Add dependencies to graph
        for dependency in dependencies {
            graph.add_dependency(dependency.clone());
        }
        
        graph
    }
}

/// In-memory dependency graph representation
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Map of entity IDs to entities
    entities: HashMap<Uuid, CodeEntity>,
    
    /// Map of entity ID to its dependencies
    dependencies: HashMap<Uuid, Vec<Dependency>>,
    
    /// Map of entity ID to entities that depend on it
    dependents: HashMap<Uuid, Vec<Dependency>>,
}

impl DependencyGraph {
    /// Create a new dependency graph
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
        }
    }
    
    /// Add an entity to the graph
    pub fn add_entity(&mut self, entity: CodeEntity) {
        self.entities.insert(entity.id, entity);
    }
    
    /// Add a dependency to the graph
    pub fn add_dependency(&mut self, dependency: Dependency) {
        // Add to dependencies map
        self.dependencies
            .entry(dependency.from_entity)
            .or_insert_with(Vec::new)
            .push(dependency.clone());
        
        // Add to dependents map
        self.dependents
            .entry(dependency.to_entity)
            .or_insert_with(Vec::new)
            .push(dependency);
    }
    
    /// Get an entity by its ID
    pub fn get_entity(&self, id: Uuid) -> Option<&CodeEntity> {
        self.entities.get(&id)
    }
    
    /// Get all dependencies for an entity
    pub fn get_dependencies(&self, id: Uuid) -> Option<&Vec<Dependency>> {
        self.dependencies.get(&id)
    }
    
    /// Get all dependents of an entity
    pub fn get_dependents(&self, id: Uuid) -> Option<&Vec<Dependency>> {
        self.dependents.get(&id)
    }
    
    /// Get all entities in the graph
    pub fn get_all_entities(&self) -> Vec<&CodeEntity> {
        self.entities.values().collect()
    }
    
    /// Find the shortest path between two entities
    pub fn shortest_path(&self, start: Uuid, end: Uuid) -> Option<Vec<Uuid>> {
        use std::collections::{HashMap, HashSet, VecDeque};
        
        if start == end {
            return Some(vec![start]);
        }
        
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut previous = HashMap::new();
        
        queue.push_back(start);
        visited.insert(start);
        
        while let Some(current) = queue.pop_front() {
            if let Some(dependencies) = self.get_dependencies(current) {
                for dep in dependencies {
                    let next_entity = dep.to_entity;
                    
                    if !visited.contains(&next_entity) {
                        visited.insert(next_entity);
                        previous.insert(next_entity, current);
                        queue.push_back(next_entity);
                        
                        if next_entity == end {
                            // Reconstruct path
                            let mut path = vec![end];
                            let mut current_path = end;
                            
                            while let Some(&prev) = previous.get(&current_path) {
                                path.push(prev);
                                current_path = prev;
                                
                                if prev == start {
                                    break;
                                }
                            }
                            
                            path.reverse();
                            return Some(path);
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Get the size of the graph
    pub fn size(&self) -> usize {
        self.entities.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_mapper::entities::CodeEntityType;
    use chrono::Utc;
    
    #[test]
    fn test_dependency_analyzer_creation() {
        let analyzer = DependencyAnalyzer::new();
        assert_eq!(std::mem::size_of_val(&analyzer), 0); // Zero-sized type
    }
    
    #[test]
    fn test_dependency_analysis() -> Result<()> {
        let analyzer = DependencyAnalyzer::new();
        
        let entities = vec![
            CodeEntity {
                id: Uuid::new_v4(),
                name: "helper_function".to_string(),
                entity_type: CodeEntityType::Function,
                language: "rust".to_string(),
                file_path: "test.rs".to_string(),
                line_number: 5,
                column_number: 0,
                scope: String::new(),
                dependencies: Vec::new(),
                accessed_by: Vec::new(),
                content: "fn helper_function() -> i32 { 42 }".to_string(),
                embedding: None,
                complexity: 1.0,
                last_modified: Utc::now(),
            },
            CodeEntity {
                id: Uuid::new_v4(),
                name: "main_function".to_string(),
                entity_type: CodeEntityType::Function,
                language: "rust".to_string(),
                file_path: "test.rs".to_string(),
                line_number: 10,
                column_number: 0,
                scope: String::new(),
                dependencies: Vec::new(),
                accessed_by: Vec::new(),
                content: "fn main_function() -> i32 { let value = helper_function(); value + 1 }".to_string(),
                embedding: None,
                complexity: 1.0,
                last_modified: Utc::now(),
            },
        ];
        
        let file_content = r#"
            fn helper_function() -> i32 {
                42
            }
            
            fn main_function() -> i32 {
                let value = helper_function();
                value + 1
            }
        "#;
        
        let dependencies = analyzer.analyze_dependencies(&entities, file_content)?;
        assert!(!dependencies.is_empty());
        
        // Check that we found the function call dependency
        let call_found = dependencies.iter()
            .any(|dep| matches!(dep.dependency_type, DependencyType::Call));
        assert!(call_found, "Function call dependency should be found");
        
        Ok(())
    }
    
    #[test]
    fn test_dependency_graph() -> Result<()> {
        let analyzer = DependencyAnalyzer::new();
        
        let entity1 = CodeEntity {
            id: Uuid::new_v4(),
            name: "function1".to_string(),
            entity_type: CodeEntityType::Function,
            language: "rust".to_string(),
            file_path: "test.rs".to_string(),
            line_number: 5,
            column_number: 0,
            scope: String::new(),
            dependencies: Vec::new(),
            accessed_by: Vec::new(),
            content: "fn function1() -> i32 { 42 }".to_string(),
            embedding: None,
            complexity: 1.0,
            last_modified: Utc::now(),
        };
        
        let entity2 = CodeEntity {
            id: Uuid::new_v4(),
            name: "function2".to_string(),
            entity_type: CodeEntityType::Function,
            language: "rust".to_string(),
            file_path: "test.rs".to_string(),
            line_number: 10,
            column_number: 0,
            scope: String::new(),
            dependencies: Vec::new(),
            accessed_by: Vec::new(),
            content: "fn function2() -> i32 { function1() + 1 }".to_string(),
            embedding: None,
            complexity: 1.0,
            last_modified: Utc::now(),
        };
        
        let dependency = Dependency {
            id: Uuid::new_v4(),
            from_entity: entity2.id,
            to_entity: entity1.id,
            dependency_type: DependencyType::Call,
            strength: 1.0,
            file_path: "test.rs".to_string(),
            line_number: 10,
        };
        
        let graph = analyzer.build_dependency_graph(&[entity1.clone(), entity2.clone()], &[dependency.clone()]);
        assert_eq!(graph.size(), 2);
        
        // Check that entities are in the graph
        assert!(graph.get_entity(entity1.id).is_some());
        assert!(graph.get_entity(entity2.id).is_some());
        
        // Check that dependencies are in the graph
        let deps = graph.get_dependencies(entity2.id);
        assert!(deps.is_some());
        assert_eq!(deps.unwrap().len(), 1);
        
        let deps = graph.get_dependents(entity1.id);
        assert!(deps.is_some());
        assert_eq!(deps.unwrap().len(), 1);
        
        Ok(())
    }
    
    #[test]
    fn test_shortest_path() -> Result<()> {
        let mut graph = DependencyGraph::new();
        
        let entity1 = CodeEntity {
            id: Uuid::new_v4(),
            name: "function1".to_string(),
            entity_type: CodeEntityType::Function,
            language: "rust".to_string(),
            file_path: "test.rs".to_string(),
            line_number: 5,
            column_number: 0,
            scope: String::new(),
            dependencies: Vec::new(),
            accessed_by: Vec::new(),
            content: "fn function1() -> i32 { 42 }".to_string(),
            embedding: None,
            complexity: 1.0,
            last_modified: Utc::now(),
        };
        
        let entity2 = CodeEntity {
            id: Uuid::new_v4(),
            name: "function2".to_string(),
            entity_type: CodeEntityType::Function,
            language: "rust".to_string(),
            file_path: "test.rs".to_string(),
            line_number: 10,
            column_number: 0,
            scope: String::new(),
            dependencies: Vec::new(),
            accessed_by: Vec::new(),
            content: "fn function2() -> i32 { function1() + 1 }".to_string(),
            embedding: None,
            complexity: 1.0,
            last_modified: Utc::now(),
        };
        
        let entity3 = CodeEntity {
            id: Uuid::new_v4(),
            name: "function3".to_string(),
            entity_type: CodeEntityType::Function,
            language: "rust".to_string(),
            file_path: "test.rs".to_string(),
            line_number: 15,
            column_number: 0,
            scope: String::new(),
            dependencies: Vec::new(),
            accessed_by: Vec::new(),
            content: "fn function3() -> i32 { function2() + 1 }".to_string(),
            embedding: None,
            complexity: 1.0,
            last_modified: Utc::now(),
        };
        
        let dep1 = Dependency {
            id: Uuid::new_v4(),
            from_entity: entity2.id,
            to_entity: entity1.id,
            dependency_type: DependencyType::Call,
            strength: 1.0,
            file_path: "test.rs".to_string(),
            line_number: 10,
        };
        
        let dep2 = Dependency {
            id: Uuid::new_v4(),
            from_entity: entity3.id,
            to_entity: entity2.id,
            dependency_type: DependencyType::Call,
            strength: 1.0,
            file_path: "test.rs".to_string(),
            line_number: 15,
        };
        
        graph.add_entity(entity1.clone());
        graph.add_entity(entity2.clone());
        graph.add_entity(entity3.clone());
        graph.add_dependency(dep1.clone());
        graph.add_dependency(dep2.clone());
        
        // Test path from entity3 to entity1 (should be entity3 -> entity2 -> entity1)
        let path = graph.shortest_path(entity3.id, entity1.id);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], entity3.id);
        assert_eq!(path[1], entity2.id);
        assert_eq!(path[2], entity1.id);
        
        // Test path from entity to itself
        let path = graph.shortest_path(entity1.id, entity1.id);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], entity1.id);
        
        Ok(())
    }
}