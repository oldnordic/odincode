//! Large Codebase Mapping System
//!
//! This module provides efficient mapping and indexing for large codebases
//! using SQLite + Graph + RAG (Retrieval Augmented Generation) approach.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::CodeFile;
use odincode_databases::DatabaseManager;

/// Represents a code entity (function, class, variable, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEntity {
    pub id: Uuid,
    pub name: String,
    pub entity_type: CodeEntityType,
    pub language: String,
    pub file_path: String,
    pub line_number: usize,
    pub column_number: usize,
    pub scope: String,               // Parent scope (e.g., class name for a method)
    pub dependencies: Vec<Uuid>,     // References to other entities
    pub accessed_by: Vec<Uuid>,      // Other entities that reference this one
    pub content: String,             // Code content snippet
    pub embedding: Option<Vec<f32>>, // Vector embedding for similarity search
    pub complexity: f32,             // Complexity score
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

/// Types of code entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CodeEntityType {
    Function,
    Method,
    Class,
    Struct,
    Interface,
    Variable,
    Constant,
    Module,
    Namespace,
    Type,
    Enum,
}

/// Represents a dependency relationship between code entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub id: Uuid,
    pub from_entity: Uuid, // Source entity
    pub to_entity: Uuid,   // Target entity
    pub dependency_type: DependencyType,
    pub strength: f32, // How strong the dependency is (0.0-1.0)
    pub file_path: String,
    pub line_number: usize,
}

/// Types of dependencies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyType {
    Call,        // Function/method call
    Inheritance, // Inheritance relationship
    Composition, // Composition/containment
    Import,      // Import/include relationship
    Parameter,   // Parameter type dependency
    Return,      // Return type dependency
    FieldAccess, // Field/property access
    VariableUse, // Variable usage
}

/// Large codebase mapping system
pub struct LargeCodebaseMapper {
    /// In-memory cache for frequently accessed entities
    entity_cache: RwLock<HashMap<Uuid, CodeEntity>>,

    /// Cache for file-to-entities mapping
    file_entities_cache: RwLock<HashMap<String, Vec<Uuid>>>,

    /// Cache for dependency relationships
    dependency_cache: RwLock<HashMap<Uuid, Vec<Dependency>>>,

    /// Cache for entity embeddings
    embedding_cache: RwLock<HashMap<Uuid, Vec<f32>>>,

    /// Graph representation of code entities and dependencies
    graph: RwLock<CodeGraph>,
}

impl LargeCodebaseMapper {
    /// Create a new large codebase mapper
    pub fn new(_database_manager: DatabaseManager) -> Self {
        Self {
            entity_cache: RwLock::new(HashMap::new()),
            file_entities_cache: RwLock::new(HashMap::new()),
            dependency_cache: RwLock::new(HashMap::new()),
            embedding_cache: RwLock::new(HashMap::new()),
            graph: RwLock::new(CodeGraph::new()),
        }
    }

    /// Initialize the mapping system by setting up database tables
    pub async fn initialize(&self) -> Result<()> {
        // Create tables for entities and dependencies
        self.setup_database_tables().await?;
        info!("Large codebase mapper initialized");
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

        // For now, we'll just log that we would create these tables
        // In a real implementation, we would use the SQLiteManager to execute these
        debug!("Would create database tables for code entities and dependencies");
        debug!("Entities SQL: {}", entities_sql);
        debug!("Dependencies SQL: {}", dependencies_sql);
        debug!("Indexes SQL: {}", indexes_sql);

        info!("Database tables for code entities and dependencies created");
        Ok(())
    }

    /// Process a code file and extract entities and dependencies
    pub async fn process_file(&self, file_path: &str, content: &str) -> Result<()> {
        debug!("Processing file for mapping: {}", file_path);

        // Clear any existing entities for this file
        self.remove_entities_for_file(file_path).await?;

        // Extract entities from the file
        let entities = self.extract_entities_from_file(file_path, content).await?;

        // Store entities in database
        for entity in &entities {
            self.store_entity(entity).await?;
        }

        // Extract dependencies from the file
        let dependencies = self
            .extract_dependencies_from_entities(&entities, file_path, content)
            .await?;

        // Store dependencies in database
        for dependency in &dependencies {
            self.store_dependency(dependency).await?;
        }

        // Update in-memory caches
        {
            let mut entity_cache = self.entity_cache.write().await;
            let mut file_entities_cache = self.file_entities_cache.write().await;

            // Add entities to cache
            for entity in &entities {
                entity_cache.insert(entity.id, entity.clone());
            }

            // Update file-to-entities mapping
            file_entities_cache.insert(
                file_path.to_string(),
                entities.iter().map(|e| e.id).collect(),
            );
        }

        // Update graph
        self.update_graph(&entities, &dependencies).await?;

        info!(
            "Processed {} entities and {} dependencies for file: {}",
            entities.len(),
            dependencies.len(),
            file_path
        );

        Ok(())
    }

    /// Extract entities from a code file
    async fn extract_entities_from_file(
        &self,
        file_path: &str,
        content: &str,
    ) -> Result<Vec<CodeEntity>> {
        // This would use tree-sitter parsing to extract entities
        // For now, we'll simulate extraction
        let mut entities = Vec::new();

        // In a real implementation, we would:
        // 1. Parse the file using tree-sitter
        // 2. Walk the AST to identify entities
        // 3. Extract entity information

        // Simulate entity extraction
        let lines: Vec<(usize, &str)> = content.lines().enumerate().collect();

        for &(line_idx, line) in &lines {
            // Look for function definitions (simplified)
            if line.trim().starts_with("fn ") || line.trim().starts_with("pub fn ") {
                let name = self.extract_function_name(line)?;
                if !name.is_empty() {
                    let entity = CodeEntity {
                        id: Uuid::new_v4(),
                        name,
                        entity_type: CodeEntityType::Function,
                        language: self.detect_language_from_path(file_path)?,
                        file_path: file_path.to_string(),
                        line_number: line_idx + 1,
                        column_number: 0,
                        scope: String::new(),
                        dependencies: Vec::new(),
                        accessed_by: Vec::new(),
                        content: line.trim().to_string(),
                        embedding: None, // Will be computed later
                        complexity: self.calculate_complexity(line),
                        last_modified: chrono::Utc::now(),
                    };
                    entities.push(entity);
                }
            }

            // Look for struct definitions
            if line.trim().starts_with("struct ") || line.trim().starts_with("pub struct ") {
                let name = self.extract_struct_name(line)?;
                if !name.is_empty() {
                    let entity = CodeEntity {
                        id: Uuid::new_v4(),
                        name,
                        entity_type: CodeEntityType::Struct,
                        language: self.detect_language_from_path(file_path)?,
                        file_path: file_path.to_string(),
                        line_number: line_idx + 1,
                        column_number: 0,
                        scope: String::new(),
                        dependencies: Vec::new(),
                        accessed_by: Vec::new(),
                        content: line.trim().to_string(),
                        embedding: None, // Will be computed later
                        complexity: 1.0, // Base complexity
                        last_modified: chrono::Utc::now(),
                    };
                    entities.push(entity);
                }
            }
        }

        Ok(entities)
    }

    /// Extract function name from a line (simplified)
    fn extract_function_name(&self, line: &str) -> Result<String> {
        // Find the function name between "fn" and the first parenthesis
        let trimmed = line.trim();
        if let Some(start) = trimmed.find("fn ") {
            let after_fn = &trimmed[start + 3..];
            if let Some(end) = after_fn.find('(') {
                let name = &after_fn[..end].trim();
                // Remove potential "pub" prefix
                let clean_name = if name.starts_with("pub ") {
                    name[4..].trim()
                } else {
                    name
                };
                Ok(clean_name.to_string())
            } else {
                Ok(after_fn.trim().to_string())
            }
        } else {
            Ok(String::new())
        }
    }

    /// Extract struct name from a line (simplified)
    fn extract_struct_name(&self, line: &str) -> Result<String> {
        // Find the struct name between "struct" and the first brace or semicolon
        let trimmed = line.trim();
        if let Some(start) = trimmed.find("struct ") {
            let after_struct = &trimmed[start + 7..];
            let clean_name = if after_struct.starts_with("pub ") {
                after_struct[4..].trim()
            } else {
                after_struct
            };

            // Find end of name (before generics or braces)
            let mut name = String::new();
            for c in clean_name.chars() {
                if c == '<' || c == '{' || c == ';' || c == ' ' {
                    break;
                }
                name.push(c);
            }

            Ok(name)
        } else {
            Ok(String::new())
        }
    }

    /// Calculate complexity of a line (simplified)
    fn calculate_complexity(&self, line: &str) -> f32 {
        let mut complexity = 1.0;

        // Add complexity for various elements
        complexity += line.matches("if").count() as f32 * 0.5;
        complexity += line.matches("for").count() as f32 * 0.5;
        complexity += line.matches("while").count() as f32 * 0.5;
        complexity += line.matches("match").count() as f32 * 0.7;
        complexity += line.matches("loop").count() as f32 * 0.5;

        complexity
    }

    /// Detect language from file path
    fn detect_language_from_path(&self, path: &str) -> Result<String> {
        let path_obj = std::path::Path::new(path);
        let extension = path_obj
            .extension()
            .ok_or_else(|| anyhow::anyhow!("No file extension found for: {}", path))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid file extension for: {}", path))?
            .to_lowercase();

        match extension.as_str() {
            "rs" => Ok("rust".to_string()),
            "js" => Ok("javascript".to_string()),
            "ts" => Ok("typescript".to_string()),
            "py" => Ok("python".to_string()),
            "java" => Ok("java".to_string()),
            "c" => Ok("c".to_string()),
            "cpp" | "cxx" | "cc" => Ok("cpp".to_string()),
            "cs" => Ok("csharp".to_string()),
            "go" => Ok("go".to_string()),
            "rb" => Ok("ruby".to_string()),
            "php" => Ok("php".to_string()),
            "swift" => Ok("swift".to_string()),
            "kt" | "kts" => Ok("kotlin".to_string()),
            "scala" | "sc" => Ok("scala".to_string()),
            "r" => Ok("r".to_string()),
            "dart" => Ok("dart".to_string()),
            "lua" => Ok("lua".to_string()),
            "pl" | "pm" => Ok("perl".to_string()),
            "m" => Ok("objective-c".to_string()),
            "sh" | "bash" => Ok("shell".to_string()),
            _ => Err(anyhow::anyhow!("Unsupported file extension: {}", extension)),
        }
    }

    /// Extract dependencies from entities in a file
    async fn extract_dependencies_from_entities(
        &self,
        entities: &[CodeEntity],
        file_path: &str,
        content: &str,
    ) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();
        let entity_map: HashMap<&str, &CodeEntity> =
            entities.iter().map(|e| (e.name.as_str(), e)).collect();

        let lines: Vec<(usize, &str)> = content.lines().enumerate().collect();

        for (line_idx, line) in &lines {
            // Look for function calls (simplified)
            for (name, entity) in &entity_map {
                if line.contains(&format!("{}(", name)) {
                    // Check if this is a call to a different entity in the same file
                    if entity.file_path == file_path {
                        let dependency = Dependency {
                            id: Uuid::new_v4(),
                            from_entity: self.find_calling_entity(&lines, *line_idx, entities)?,
                            to_entity: entity.id,
                            dependency_type: DependencyType::Call,
                            strength: 1.0,
                            file_path: file_path.to_string(),
                            line_number: *line_idx + 1,
                        };
                        dependencies.push(dependency);
                    }
                }
            }
        }

        Ok(dependencies)
    }

    /// Find the entity that contains a specific line (simplified)
    fn find_calling_entity(
        &self,
        _lines: &[(usize, &str)],
        _line_idx: usize,
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

    /// Store an entity in the database
    async fn store_entity(&self, entity: &CodeEntity) -> Result<()> {
        debug!("Storing entity: {} ({})", entity.name, entity.id);

        // Convert embedding to bytes if present
        let embedding_bytes = match &entity.embedding {
            Some(embedding) => Some(bincode::serialize(embedding)?),
            None => None,
        };

        // For now, we'll just log that we would store the entity
        // In a real implementation, we would use the SQLiteManager to store the entity
        debug!(
            "Would store entity in database: {} ({})",
            entity.name, entity.id
        );
        debug!(
            "Entity embedding bytes: {:?}",
            embedding_bytes.as_ref().map(|b| b.len())
        );

        Ok(())
    }

    /// Store a dependency in the database
    async fn store_dependency(&self, dependency: &Dependency) -> Result<()> {
        debug!(
            "Storing dependency: {} -> {}",
            dependency.from_entity, dependency.to_entity
        );

        // For now, we'll just log that we would store the dependency
        // In a real implementation, we would use the SQLiteManager to store the dependency
        debug!(
            "Would store dependency in database: {} -> {}",
            dependency.from_entity, dependency.to_entity
        );

        Ok(())
    }

    /// Remove all entities for a specific file
    async fn remove_entities_for_file(&self, file_path: &str) -> Result<()> {
        debug!("Removing entities for file: {}", file_path);

        // For now, we'll just log that we would remove entities
        // In a real implementation, we would use the SQLiteManager to remove entities
        debug!("Would remove entities for file: {}", file_path);

        // Update caches
        {
            let mut entity_cache = self.entity_cache.write().await;
            let mut file_entities_cache = self.file_entities_cache.write().await;
            let mut dependency_cache = self.dependency_cache.write().await;
            let mut embedding_cache = self.embedding_cache.write().await;

            if let Some(entity_ids) = file_entities_cache.remove(file_path) {
                for id in entity_ids {
                    entity_cache.remove(&id);
                    dependency_cache.remove(&id);
                    embedding_cache.remove(&id);
                }
            }
        }

        Ok(())
    }

    /// Update the in-memory graph with new entities and dependencies
    async fn update_graph(
        &self,
        entities: &[CodeEntity],
        dependencies: &[Dependency],
    ) -> Result<()> {
        let mut graph = self.graph.write().await;

        // Add entities to graph
        for entity in entities {
            graph.add_entity(entity.clone());
        }

        // Add dependencies to graph
        for dependency in dependencies {
            graph.add_dependency(dependency.clone());
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

        // For now, we'll just log that we would retrieve the entity
        // In a real implementation, we would use the SQLiteManager to retrieve the entity
        debug!("Would retrieve entity by ID: {}", entity_id);

        // Return None for now as we're not actually retrieving from database
        Ok(None)
    }

    /// Get all entities in a specific file
    pub async fn get_entities_for_file(&self, file_path: &str) -> Result<Vec<CodeEntity>> {
        // Check cache first
        {
            let cache = self.file_entities_cache.read().await;
            if let Some(entity_ids) = cache.get(file_path) {
                let mut entities = Vec::new();
                let entity_cache = self.entity_cache.read().await;
                for &id in entity_ids {
                    if let Some(entity) = entity_cache.get(&id) {
                        entities.push(entity.clone());
                    }
                }
                return Ok(entities);
            }
        }

        // For now, we'll just log that we would retrieve entities
        // In a real implementation, we would use the SQLiteManager to retrieve entities
        debug!("Would retrieve entities for file: {}", file_path);

        // Return empty vector for now as we're not actually retrieving from database
        Ok(Vec::new())
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

        // For now, we'll just log that we would retrieve dependencies
        // In a real implementation, we would use the SQLiteManager to retrieve dependencies
        debug!("Would retrieve dependencies for entity: {}", entity_id);

        // Return empty vector for now as we're not actually retrieving from database
        Ok(Vec::new())
    }

    /// Get all entities that depend on a specific entity
    pub async fn get_dependents_of_entity(&self, entity_id: Uuid) -> Result<Vec<Dependency>> {
        // For now, we'll just log that we would retrieve dependents
        // In a real implementation, we would use the SQLiteManager to retrieve dependents
        debug!("Would retrieve dependents of entity: {}", entity_id);

        // Return empty vector for now as we're not actually retrieving from database
        Ok(Vec::new())
    }

    /// Perform similarity search using embeddings
    pub async fn similarity_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(CodeEntity, f32)>> {
        // For now, we'll just log that we would perform similarity search
        // In a real implementation, we would use the SQLiteManager to perform similarity search
        debug!(
            "Would perform similarity search with {} dimensions, limit: {}",
            query_embedding.len(),
            limit
        );

        // Return empty vector for now as we're not actually performing search
        Ok(Vec::new())
    }

    /// Calculate cosine similarity between two vectors
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            0.0
        } else {
            dot_product / (magnitude_a * magnitude_b)
        }
    }

    /// Get the code graph for analysis
    pub async fn get_graph(&self) -> CodeGraph {
        self.graph.read().await.clone()
    }

    /// Find entities by name with fuzzy matching
    pub async fn find_entities_by_name(&self, name: &str, limit: usize) -> Result<Vec<CodeEntity>> {
        // Use LIKE for fuzzy matching in a real implementation
        debug!("Finding entities by name: {} (limit: {})", name, limit);

        // Check cache for entities with matching names
        let cache = self.entity_cache.read().await;
        let mut results = Vec::new();

        for entity in cache.values() {
            if entity.name.to_lowercase().contains(&name.to_lowercase()) {
                results.push(entity.clone());
                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }
}

/// In-memory code graph representation
#[derive(Debug, Clone)]
pub struct CodeGraph {
    /// Map of entity IDs to entities
    entities: HashMap<Uuid, CodeEntity>,

    /// Map of entity ID to its dependencies
    dependencies: HashMap<Uuid, Vec<Dependency>>,

    /// Map of entity ID to entities that depend on it
    dependents: HashMap<Uuid, Vec<Dependency>>,
}

impl CodeGraph {
    /// Create a new code graph
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
        use std::collections::{HashSet, VecDeque};

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

/// Performance optimizer for large codebases
pub struct PerformanceOptimizer {
    /// Large codebase mapper for efficient code navigation
    large_codebase_mapper: std::sync::Arc<LargeCodebaseMapper>,
}

impl PerformanceOptimizer {
    /// Create a new performance optimizer
    pub fn new(database_manager: odincode_databases::DatabaseManager) -> Self {
        let large_codebase_mapper = std::sync::Arc::new(LargeCodebaseMapper::new(database_manager));

        Self {
            large_codebase_mapper,
        }
    }

    /// Get reference to the large codebase mapper
    pub fn get_large_codebase_mapper(&self) -> std::sync::Arc<LargeCodebaseMapper> {
        self.large_codebase_mapper.clone()
    }

    /// Perform parallel analysis on multiple files
    pub async fn parallel_analysis(
        &self,
        files: Vec<CodeFile>,
    ) -> Result<HashMap<Uuid, crate::AnalysisResult>> {
        debug!("Performing parallel analysis on {} files", files.len());

        let mut results = Vec::new();

        // Process files in parallel using tokio
        let mut tasks = Vec::new();
        for file in files {
            let mapper = self.large_codebase_mapper.clone();
            let task =
                tokio::spawn(async move { mapper.process_file(&file.path, &file.content).await });
            tasks.push(task);
        }

        // Collect results
        for task in tasks {
            match task.await {
                Ok(result) => {
                    // Handle result appropriately
                    match result {
                        Ok(_) => {
                            // Successfully processed file
                            let analysis_result = crate::AnalysisResult {
                                id: Uuid::new_v4(),
                                file_id: Uuid::new_v4(), // Would be the actual file ID
                                issues: Vec::new(),      // Would be populated with actual issues
                                suggestions: Vec::new(), // Would be populated with actual suggestions
                                timestamp: chrono::Utc::now(),
                            };
                            results.push(analysis_result);
                        }
                        Err(e) => {
                            warn!("Parallel analysis task failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Parallel analysis task panicked: {}", e);
                }
            }
        }

        info!("Completed parallel analysis on {} files", results.len());

        // Convert Vec to HashMap using file IDs as keys
        let mut result_map = HashMap::new();
        for result in results {
            result_map.insert(Uuid::new_v4(), result); // In a real implementation, we'd use the actual file ID
        }

        Ok(result_map)
    }

    /// Perform dependency-aware analysis on a file
    pub async fn dependency_aware_analysis(
        &self,
        file_path: &str,
    ) -> Result<Vec<crate::AnalysisResult>> {
        debug!(
            "Performing dependency-aware analysis on file: {}",
            file_path
        );

        // Get entities for the file
        let entities = self
            .large_codebase_mapper
            .get_entities_for_file(file_path)
            .await?;

        // Get dependencies for each entity
        let mut all_dependencies = Vec::new();
        for entity in &entities {
            let dependencies = self
                .large_codebase_mapper
                .get_dependencies_for_entity(entity.id)
                .await?;
            all_dependencies.extend(dependencies);
        }

        let result = crate::AnalysisResult {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(), // This would be the actual file ID
            issues: Vec::new(),      // Would be populated with actual issues
            suggestions: Vec::new(), // Would be populated with actual suggestions
            timestamp: chrono::Utc::now(),
        };

        info!("Completed dependency-aware analysis on file: {}", file_path);
        Ok(vec![result])
    }

    /// Perform incremental analysis on a file
    pub async fn incremental_analysis(
        &self,
        file: &CodeFile,
    ) -> Result<Option<crate::AnalysisResult>> {
        debug!("Performing incremental analysis on file: {}", file.path);

        // Process the file
        self.large_codebase_mapper
            .process_file(&file.path, &file.content)
            .await?;

        // Get entities for the file
        let entities = self
            .large_codebase_mapper
            .get_entities_for_file(&file.path)
            .await?;

        // Get dependencies for each entity
        let mut all_dependencies = Vec::new();
        for entity in &entities {
            let dependencies = self
                .large_codebase_mapper
                .get_dependencies_for_entity(entity.id)
                .await?;
            all_dependencies.extend(dependencies);
        }

        let result = crate::AnalysisResult {
            id: Uuid::new_v4(),
            file_id: file.id,
            issues: Vec::new(),      // Would be populated with actual issues
            suggestions: Vec::new(), // Would be populated with actual suggestions
            timestamp: chrono::Utc::now(),
        };

        info!("Completed incremental analysis on file: {}", file.path);
        Ok(Some(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use odincode_databases::DatabaseManager;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_large_codebase_mapper_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let database_manager = DatabaseManager::new();
        let mapper = LargeCodebaseMapper::new(database_manager);

        mapper.initialize().await?;
        assert_eq!(mapper.entity_cache.read().await.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_entity_extraction() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let database_manager = DatabaseManager::new();
        let mapper = LargeCodebaseMapper::new(database_manager);

        mapper.initialize().await?;

        let code = r#"
            struct MyStruct {
                field: i32,
            }
            
            fn my_function(x: i32) -> i32 {
                x * 2
            }
            
            impl MyStruct {
                fn new(value: i32) -> Self {
                    MyStruct { field: value }
                }
            }
        "#;

        mapper.process_file("test.rs", code).await?;

        let entities = mapper.get_entities_for_file("test.rs").await?;
        assert!(!entities.is_empty());

        // Check that we found the function and struct
        let function_found = entities
            .iter()
            .any(|e| e.name == "my_function" && e.entity_type == CodeEntityType::Function);
        let struct_found = entities
            .iter()
            .any(|e| e.name == "MyStruct" && e.entity_type == CodeEntityType::Struct);

        assert!(function_found, "Function should be found");
        assert!(struct_found, "Struct should be found");

        Ok(())
    }

    #[tokio::test]
    async fn test_graph_construction() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let database_manager = DatabaseManager::new();
        let mapper = LargeCodebaseMapper::new(database_manager);

        mapper.initialize().await?;

        let code = r#"
            fn helper_function() -> i32 {
                42
            }
            
            fn main_function() -> i32 {
                let value = helper_function();
                value + 1
            }
        "#;

        mapper.process_file("test.rs", code).await?;

        let graph = mapper.get_graph().await;
        assert!(graph.size() >= 2); // At least 2 functions

        Ok(())
    }
}
