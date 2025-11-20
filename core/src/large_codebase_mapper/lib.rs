//! Code Mapper Module
//! 
//! This module provides functionality for mapping and analyzing large codebases
//! using efficient indexing and search capabilities.

pub mod entities;
pub mod dependencies;
pub mod storage;

pub use entities::{CodeEntity, CodeEntityType, Dependency, DependencyType};
pub use dependencies::{DependencyAnalyzer, DependencyGraph};
pub use storage::StorageManager;

/// Performance optimizer for large codebases
pub struct PerformanceOptimizer {
    /// Storage manager for entities and dependencies
    storage_manager: StorageManager,
    
    /// Dependency analyzer for analyzing code relationships
    dependency_analyzer: DependencyAnalyzer,
}

impl PerformanceOptimizer {
    /// Create a new performance optimizer
    pub fn new(database_manager: odincode_databases::DatabaseManager) -> Self {
        let storage_manager = StorageManager::new(database_manager);
        let dependency_analyzer = DependencyAnalyzer::new();
        
        Self {
            storage_manager,
            dependency_analyzer,
        }
    }
    
    /// Get reference to the storage manager
    pub fn get_storage_manager(&self) -> &StorageManager {
        &self.storage_manager
    }
    
    /// Get reference to the dependency analyzer
    pub fn get_dependency_analyzer(&self) -> &DependencyAnalyzer {
        &self.dependency_analyzer
    }
}