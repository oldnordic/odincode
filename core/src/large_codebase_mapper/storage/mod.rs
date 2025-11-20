//! Storage Module for Code Mapper
//! 
//! This module provides storage functionality for code entities and dependencies
//! using both in-memory caching and persistent storage.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::code_mapper::entities::{CodeEntity, Dependency};
use odincode_databases::DatabaseManager;

/// Storage manager for code entities and dependencies
pub struct StorageManager {
    /// Database manager for persistent storage
    database_manager: DatabaseManager,
    
    /// In-memory cache for frequently accessed entities
    entity_cache: Arc<RwLock<HashMap<Uuid, CodeEntity>>>,
    
    /// Cache for file-to-entities mapping
    file_entities_cache: Arc<RwLock<HashMap<String, Vec<Uuid>>>>,
    
    /// Cache for dependency relationships
    dependency_cache: Arc<RwLock<HashMap<Uuid, Vec<Dependency>>>>,
    
    /// Cache for entity embeddings
    embedding_cache: Arc<RwLock<HashMap<Uuid, Vec<f32>>>>,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new(database_manager: DatabaseManager) -> Self {
        Self {
            database_manager,
            entity_cache: Arc::new(RwLock::new(HashMap::new())),
            file_entities_cache: Arc::new(RwLock::new(HashMap::new())),
            dependency_cache: Arc::new(RwLock::new(HashMap::new())),
            embedding_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Initialize the storage system by setting up database tables
    pub async fn initialize(&self) -> Result<()> {
        // Create tables for entities and dependencies
        self.setup_database_tables().await?;
        info!("Storage manager initialized");
        Ok(())
    }
    
    /// Set up database tables for storing code entities and dependencies
    async fn setup_database_tables(&self) -> Result<()> {
        // SQL to create entities table
        let entities_sql = r#"
            CREATE TABLE IF NOT EXISTS code_entities (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                language TEXT NOT NULL,
                file_path TEXT NOT NULL,
                line_number INTEGER NOT NULL,
                column_number INTEGER NOT NULL,
                scope TEXT,
                content TEXT NOT NULL,
                embedding BLOB,
                complexity REAL,
                last_modified TEXT NOT NULL
            )
        "#;
        
        // SQL to create dependencies table
        let dependencies_sql = r#"
            CREATE TABLE IF NOT EXISTS dependencies (
                id TEXT PRIMARY KEY,
                from_entity TEXT NOT NULL,
                to_entity TEXT NOT NULL,
                dependency_type TEXT NOT NULL,
                strength REAL DEFAULT 1.0,
                file_path TEXT NOT NULL,
                line_number INTEGER NOT NULL,
                FOREIGN KEY (from_entity) REFERENCES code_entities (id),
                FOREIGN KEY (to_entity) REFERENCES code_entities (id)
            )
        "#;
        
        // SQL to create indexes for performance
        let indexes_sql = r#"
            CREATE INDEX IF NOT EXISTS idx_entities_file ON code_entities(file_path);
            CREATE INDEX IF NOT EXISTS idx_entities_type ON code_entities(entity_type);
            CREATE INDEX IF NOT EXISTS idx_entities_name ON code_entities(name);
            CREATE INDEX IF NOT EXISTS idx_dependencies_from ON dependencies(from_entity);
            CREATE INDEX IF NOT EXISTS idx_dependencies_to ON dependencies(to_entity);
        "#;
        
        // Get database connection (using the correct method)
        let db = self.database_manager.get_connection(Uuid::new_v4()).await?;
        if let Some(db_conn) = db {
            // Execute table creation
            db_conn.execute_batch(entities_sql).await?;
            db_conn.execute_batch(dependencies_sql).await?;
            db_conn.execute_batch(indexes_sql).await?;
            
            info!("Database tables for code entities and dependencies created");
        } else {
            warn!("No database connection available, using in-memory storage only");
        }
        
        Ok(())
    }
    
    /// Store an entity in both cache and database
    pub async fn store_entity(&self, entity: &CodeEntity) -> Result<()> {
        debug!("Storing entity: {} ({})", entity.name, entity.id);
        
        // Store in cache
        {
            let mut cache = self.entity_cache.write().await;
            cache.insert(entity.id, entity.clone());
        }
        
        // Store in database
        self.store_entity_in_database(entity).await?;
        
        info!("Stored entity: {} ({})", entity.name, entity.id);
        Ok(())
    }
    
    /// Store an entity in the database
    async fn store_entity_in_database(&self, entity: &CodeEntity) -> Result<()> {
        let db = self.database_manager.get_connection(Uuid::new_v4()).await?;
        if let Some(db_conn) = db {
            // Convert embedding to bytes if present
            let embedding_bytes = match &entity.embedding {
                Some(embedding) => Some(bincode::serialize(embedding)?),
                None => None,
            };
            
            let sql = r#"
                INSERT OR REPLACE INTO code_entities 
                (id, name, entity_type, language, file_path, line_number, column_number, scope, content, embedding, complexity, last_modified)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#;
            
            db_conn.execute(
                sql,
                &[
                    &entity.id.to_string(),
                    &entity.name,
                    &format!("{:?}", entity.entity_type),
                    &entity.language,
                    &entity.file_path,
                    &(entity.line_number as i64),
                    &(entity.column_number as i64),
                    &entity.scope,
                    &entity.content,
                    &embedding_bytes,
                    &entity.complexity,
                    &entity.last_modified.to_rfc3339(),
                ]
            ).await?;
        }
        
        Ok(())
    }
    
    /// Store a dependency in both cache and database
    pub async fn store_dependency(&self, dependency: &Dependency) -> Result<()> {
        debug!("Storing dependency: {} -> {}", dependency.from_entity, dependency.to_entity);
        
        // Store in cache
        {
            let mut cache = self.dependency_cache.write().await;
            cache
                .entry(dependency.from_entity)
                .or_insert_with(Vec::new)
                .push(dependency.clone());
        }
        
        // Store in database
        self.store_dependency_in_database(dependency).await?;
        
        info!("Stored dependency: {} -> {}", dependency.from_entity, dependency.to_entity);
        Ok(())
    }
    
    /// Store a dependency in the database
    async fn store_dependency_in_database(&self, dependency: &Dependency) -> Result<()> {
        let db = self.database_manager.get_connection(Uuid::new_v4()).await?;
        if let Some(db_conn) = db {
            let sql = r#"
                INSERT OR REPLACE INTO dependencies 
                (id, from_entity, to_entity, dependency_type, strength, file_path, line_number)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            "#;
            
            db_conn.execute(
                sql,
                &[
                    &dependency.id.to_string(),
                    &dependency.from_entity.to_string(),
                    &dependency.to_entity.to_string(),
                    &format!("{:?}", dependency.dependency_type),
                    &dependency.strength,
                    &dependency.file_path,
                    &(dependency.line_number as i64),
                ]
            ).await?;
        }
        
        Ok(())
    }
    
    /// Get an entity by its ID
    pub async fn get_entity_by_id(&self, entity_id: Uuid) -> Result<Option<CodeEntity>> {
        // Check cache first
        {
            let cache = self.entity_cache.read().await;
            if let Some(entity) = cache.get(&entity_id) {
                return Ok(Some(entity.clone()));
            }
        }
        
        // Check database
        let entity = self.get_entity_by_id_from_database(entity_id).await?;
        if let Some(ref entity) = entity {
            // Add to cache
            {
                let mut cache = self.entity_cache.write().await;
                cache.insert(entity_id, entity.clone());
            }
        }
        
        Ok(entity)
    }
    
    /// Get an entity by its ID from the database
    async fn get_entity_by_id_from_database(&self, entity_id: Uuid) -> Result<Option<CodeEntity>> {
        let db = self.database_manager.get_connection(Uuid::new_v4()).await?;
        if let Some(db_conn) = db {
            let sql = "SELECT * FROM code_entities WHERE id = ?";
            let mut stmt = db_conn.prepare(sql).await?;
            let mut rows = stmt.query([entity_id.to_string()]).await?;
            
            if let Some(row) = rows.next().await? {
                let entity = CodeEntity {
                    id: Uuid::parse_str(row.get::<String>(0)?.as_str())?,
                    name: row.get(1)?,
                    entity_type: self.parse_entity_type(&row.get::<String>(2)?)?,
                    language: row.get(3)?,
                    file_path: row.get(4)?,
                    line_number: row.get::<i64>(5)? as usize,
                    column_number: row.get::<i64>(6)? as usize,
                    scope: row.get(7)?,
                    content: row.get(8)?,
                    embedding: match row.get::<Option<Vec<u8>>>(9)? {
                        Some(bytes) => Some(bincode::deserialize(&bytes[..])?),
                        None => None,
                    },
                    complexity: row.get(10)?,
                    last_modified: chrono::DateTime::parse_from_rfc3339(&row.get::<String>(11)?)?.into(),
                };
                
                return Ok(Some(entity));
            }
        }
        
        Ok(None)
    }
    
    /// Parse entity type from string
    fn parse_entity_type(&self, type_str: &str) -> Result<crate::code_mapper::entities::CodeEntityType> {
        use crate::code_mapper::entities::CodeEntityType;
        
        match type_str {
            "Function" => Ok(CodeEntityType::Function),
            "Method" => Ok(CodeEntityType::Method),
            "Class" => Ok(CodeEntityType::Class),
            "Struct" => Ok(CodeEntityType::Struct),
            "Interface" => Ok(CodeEntityType::Interface),
            "Variable" => Ok(CodeEntityType::Variable),
            "Constant" => Ok(CodeEntityType::Constant),
            "Module" => Ok(CodeEntityType::Module),
            "Namespace" => Ok(CodeEntityType::Namespace),
            "Type" => Ok(CodeEntityType::Type),
            "Enum" => Ok(CodeEntityType::Enum),
            _ => Err(anyhow::anyhow!("Unknown entity type: {}", type_str)),
        }
    }
    
    /// Get all entities in a specific file
    pub async fn get_entities_for_file(&self, file_path: &str) -> Result<Vec<CodeEntity>> {
        // Check cache first
        {
            let cache = self.file_entities_cache.read().await;
            if let Some(entity_ids) = cache.get(file_path) {
                let mut entities = Vec::new();
                for &id in entity_ids {
                    if let Some(entity) = self.get_entity_by_id(id).await? {
                        entities.push(entity);
                    }
                }
                return Ok(entities);
            }
        }
        
        // Check database
        let entities = self.get_entities_for_file_from_database(file_path).await?;
        
        // Update cache
        {
            let mut cache = self.file_entities_cache.write().await;
            cache.insert(
                file_path.to_string(),
                entities.iter().map(|e| e.id).collect()
            );
        }
        
        // Add entities to entity cache
        {
            let mut cache = self.entity_cache.write().await;
            for entity in &entities {
                cache.insert(entity.id, entity.clone());
            }
        }
        
        Ok(entities)
    }
    
    /// Get all entities in a specific file from the database
    async fn get_entities_for_file_from_database(&self, file_path: &str) -> Result<Vec<CodeEntity>> {
        let mut entities = Vec::new();
        
        let db = self.database_manager.get_connection(Uuid::new_v4()).await?;
        if let Some(db_conn) = db {
            let sql = "SELECT * FROM code_entities WHERE file_path = ?";
            let mut stmt = db_conn.prepare(sql).await?;
            let mut rows = stmt.query([file_path]).await?;
            
            while let Some(row) = rows.next().await? {
                let entity = CodeEntity {
                    id: Uuid::parse_str(row.get::<String>(0)?.as_str())?,
                    name: row.get(1)?,
                    entity_type: self.parse_entity_type(&row.get::<String>(2)?)?,
                    language: row.get(3)?,
                    file_path: row.get(4)?,
                    line_number: row.get::<i64>(5)? as usize,
                    column_number: row.get::<i64>(6)? as usize,
                    scope: row.get(7)?,
                    content: row.get(8)?,
                    embedding: match row.get::<Option<Vec<u8>>>(9)? {
                        Some(bytes) => Some(bincode::deserialize(&bytes[..])?),
                        None => None,
                    },
                    complexity: row.get(10)?,
                    last_modified: chrono::DateTime::parse_from_rfc3339(&row.get::<String>(11)?)?.into(),
                };
                
                entities.push(entity);
            }
        }
        
        Ok(entities)
    }
    
    /// Get all dependencies for an entity
    pub async fn get_dependencies_for_entity(&self, entity_id: Uuid) -> Result<Vec<Dependency>> {
        // Check cache first
        {
            let cache = self.dependency_cache.read().await;
            if let Some(dependencies) = cache.get(&entity_id) {
                return Ok(dependencies.clone());
            }
        }
        
        // Check database
        let dependencies = self.get_dependencies_for_entity_from_database(entity_id).await?;
        
        // Update cache
        {
            let mut cache = self.dependency_cache.write().await;
            cache.insert(entity_id, dependencies.clone());
        }
        
        Ok(dependencies)
    }
    
    /// Get all dependencies for an entity from the database
    async fn get_dependencies_for_entity_from_database(&self, entity_id: Uuid) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();
        
        let db = self.database_manager.get_connection(Uuid::new_v4()).await?;
        if let Some(db_conn) = db {
            let sql = "SELECT * FROM dependencies WHERE from_entity = ?";
            let mut stmt = db_conn.prepare(sql).await?;
            let mut rows = stmt.query([entity_id.to_string()]).await?;
            
            while let Some(row) = rows.next().await? {
                let dependency = Dependency {
                    id: Uuid::parse_str(row.get::<String>(0)?.as_str())?,
                    from_entity: Uuid::parse_str(row.get::<String>(1)?.as_str())?,
                    to_entity: Uuid::parse_str(row.get::<String>(2)?.as_str())?,
                    dependency_type: self.parse_dependency_type(&row.get::<String>(3)?)?,
                    strength: row.get(4)?,
                    file_path: row.get(5)?,
                    line_number: row.get::<i64>(6)? as usize,
                };
                
                dependencies.push(dependency);
            }
        }
        
        Ok(dependencies)
    }
    
    /// Parse dependency type from string
    fn parse_dependency_type(&self, type_str: &str) -> Result<crate::code_mapper::entities::DependencyType> {
        use crate::code_mapper::entities::DependencyType;
        
        match type_str {
            "Call" => Ok(DependencyType::Call),
            "Inheritance" => Ok(DependencyType::Inheritance),
            "Composition" => Ok(DependencyType::Composition),
            "Import" => Ok(DependencyType::Import),
            "Parameter" => Ok(DependencyType::Parameter),
            "Return" => Ok(DependencyType::Return),
            "FieldAccess" => Ok(DependencyType::FieldAccess),
            "VariableUse" => Ok(DependencyType::VariableUse),
            _ => Err(anyhow::anyhow!("Unknown dependency type: {}", type_str)),
        }
    }
    
    /// Remove all entities for a specific file
    pub async fn remove_entities_for_file(&self, file_path: &str) -> Result<()> {
        debug!("Removing entities for file: {}", file_path);
        
        // Remove from database first (due to foreign key constraints)
        self.remove_entities_for_file_from_database(file_path).await?;
        
        // Update caches
        {
            let mut entity_cache = self.entity_cache.write().await;
            let mut file_entities_cache = self.file_entities_cache.write().await;
            
            if let Some(entity_ids) = file_entities_cache.remove(file_path) {
                for id in entity_ids {
                    entity_cache.remove(&id);
                }
            }
        }
        
        info!("Removed entities for file: {}", file_path);
        Ok(())
    }
    
    /// Remove all entities for a specific file from the database
    async fn remove_entities_for_file_from_database(&self, file_path: &str) -> Result<()> {
        let db = self.database_manager.get_connection(Uuid::new_v4()).await?;
        if let Some(db_conn) = db {
            // Remove dependencies first (due to foreign key constraints)
            let sql_deps = "DELETE FROM dependencies WHERE file_path = ?";
            db_conn.execute(sql_deps, &[file_path]).await?;
            
            // Remove entities
            let sql_entities = "DELETE FROM code_entities WHERE file_path = ?";
            db_conn.execute(sql_entities, &[file_path]).await?;
        }
        
        Ok(())
    }
    
    /// Clear all caches
    pub async fn clear_caches(&self) {
        let mut entity_cache = self.entity_cache.write().await;
        let mut file_entities_cache = self.file_entities_cache.write().await;
        let mut dependency_cache = self.dependency_cache.write().await;
        let mut embedding_cache = self.embedding_cache.write().await;
        
        entity_cache.clear();
        file_entities_cache.clear();
        dependency_cache.clear();
        embedding_cache.clear();
        
        info!("All caches cleared");
    }
    
    /// Get the database manager
    pub fn get_database_manager(&self) -> &DatabaseManager {
        &self.database_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use odincode_databases::DatabaseManager;
    use tempfile::TempDir;
    use crate::code_mapper::entities::{CodeEntity, CodeEntityType, Dependency, DependencyType};
    use chrono::Utc;
    
    #[tokio::test]
    async fn test_storage_manager_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let database_manager = DatabaseManager::new_with_path(&db_path)?;
        let storage_manager = StorageManager::new(database_manager);
        
        storage_manager.initialize().await?;
        assert_eq!(storage_manager.entity_cache.read().await.len(), 0);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_entity_storage_and_retrieval() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let database_manager = DatabaseManager::new_with_path(&db_path)?;
        let storage_manager = StorageManager::new(database_manager);
        
        storage_manager.initialize().await?;
        
        let entity = CodeEntity {
            id: Uuid::new_v4(),
            name: "test_function".to_string(),
            entity_type: CodeEntityType::Function,
            language: "rust".to_string(),
            file_path: "test.rs".to_string(),
            line_number: 10,
            column_number: 5,
            scope: String::new(),
            dependencies: Vec::new(),
            accessed_by: Vec::new(),
            content: "fn test_function() -> i32 { 42 }".to_string(),
            embedding: None,
            complexity: 1.0,
            last_modified: Utc::now(),
        };
        
        // Store entity
        storage_manager.store_entity(&entity).await?;
        
        // Retrieve entity
        let retrieved = storage_manager.get_entity_by_id(entity.id).await?;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "test_function");
        assert_eq!(retrieved.entity_type, CodeEntityType::Function);
        assert_eq!(retrieved.language, "rust");
        assert_eq!(retrieved.file_path, "test.rs");
        assert_eq!(retrieved.line_number, 10);
        assert_eq!(retrieved.complexity, 1.0);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_file_entities_retrieval() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let database_manager = DatabaseManager::new_with_path(&db_path)?;
        let storage_manager = StorageManager::new(database_manager);
        
        storage_manager.initialize().await?;
        
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
        
        // Store entities
        storage_manager.store_entity(&entity1).await?;
        storage_manager.store_entity(&entity2).await?;
        
        // Retrieve entities for file
        let entities = storage_manager.get_entities_for_file("test.rs").await?;
        assert_eq!(entities.len(), 2);
        
        // Check that we found both entities
        let function1_found = entities.iter().any(|e| e.name == "function1");
        let function2_found = entities.iter().any(|e| e.name == "function2");
        assert!(function1_found, "function1 should be found");
        assert!(function2_found, "function2 should be found");
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_dependency_storage_and_retrieval() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let database_manager = DatabaseManager::new_with_path(&db_path)?;
        let storage_manager = StorageManager::new(database_manager);
        
        storage_manager.initialize().await?;
        
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
        
        // Store entities and dependency
        storage_manager.store_entity(&entity1).await?;
        storage_manager.store_entity(&entity2).await?;
        storage_manager.store_dependency(&dependency).await?;
        
        // Retrieve dependencies
        let dependencies = storage_manager.get_dependencies_for_entity(entity2.id).await?;
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].from_entity, entity2.id);
        assert_eq!(dependencies[0].to_entity, entity1.id);
        assert_eq!(dependencies[0].dependency_type, DependencyType::Call);
        assert_eq!(dependencies[0].strength, 1.0);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_remove_entities_for_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let database_manager = DatabaseManager::new_with_path(&db_path)?;
        let storage_manager = StorageManager::new(database_manager);
        
        storage_manager.initialize().await?;
        
        let entity = CodeEntity {
            id: Uuid::new_v4(),
            name: "test_function".to_string(),
            entity_type: CodeEntityType::Function,
            language: "rust".to_string(),
            file_path: "test.rs".to_string(),
            line_number: 10,
            column_number: 5,
            scope: String::new(),
            dependencies: Vec::new(),
            accessed_by: Vec::new(),
            content: "fn test_function() -> i32 { 42 }".to_string(),
            embedding: None,
            complexity: 1.0,
            last_modified: Utc::now(),
        };
        
        // Store entity
        storage_manager.store_entity(&entity).await?;
        
        // Verify entity exists
        let retrieved = storage_manager.get_entity_by_id(entity.id).await?;
        assert!(retrieved.is_some());
        
        // Remove entities for file
        storage_manager.remove_entities_for_file("test.rs").await?;
        
        // Verify entity no longer exists
        let retrieved = storage_manager.get_entity_by_id(entity.id).await?;
        assert!(retrieved.is_none());
        
        // Verify file entities are empty
        let entities = storage_manager.get_entities_for_file("test.rs").await?;
        assert!(entities.is_empty());
        
        Ok(())
    }
}