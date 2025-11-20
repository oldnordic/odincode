//! LTMC Memory Search Bridge Search
//!
//! This module contains search-related functionality for the memory search bridge.

use anyhow::Result;
use odincode_databases::DatabaseType;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::bridges::memory_search::core::MemorySearchBridge;
use crate::models::{LearningPattern, PatternType};
use odincode_databases::faiss::VectorSearchResult;
use odincode_databases::neo4j::GraphNode;

impl MemorySearchBridge {
    /// Search for patterns using hybrid search across all databases
    pub async fn search_patterns_hybrid(
        &self,
        pattern_type: Option<PatternType>,
        query: &str,
        limit: usize,
    ) -> Result<Vec<LearningPattern>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("MemorySearchBridge not initialized"));
        }

        debug!("Performing hybrid search for: {}", query);

        // 1. Check Redis cache first for recent/frequent patterns
        let cached_patterns = if let Ok(cached) = self.get_cached_patterns(query, limit).await {
            debug!(
                "Found {} cached patterns for query: {}",
                cached.len(),
                query
            );
            cached
        } else {
            Vec::new()
        };

        // If we have enough cached results, return them
        if cached_patterns.len() >= limit {
            info!(
                "Returning {} cached patterns for query: {}",
                cached_patterns.len(),
                query
            );
            return Ok(cached_patterns);
        }

        // 2. Perform parallel searches across all databases
        let (semantic_results, graph_results, keyword_results) = tokio::join!(
            self.search_patterns_semantic(query, limit),
            self.search_patterns_graph(pattern_type.clone(), limit),
            self.search_patterns_keyword(query, limit)
        );

        let semantic_patterns = semantic_results?;
        let graph_patterns = graph_results?;
        let keyword_patterns = keyword_results?;

        debug!(
            "Search results - Semantic: {}, Graph: {}, Keyword: {}",
            semantic_patterns.len(),
            graph_patterns.len(),
            keyword_patterns.len()
        );

        // 3. Combine and rank results from all sources
        let mut all_patterns = Vec::new();
        all_patterns.extend(semantic_patterns);
        all_patterns.extend(graph_patterns);
        all_patterns.extend(keyword_patterns);
        all_patterns.extend(cached_patterns);

        // 4. Remove duplicates and rank by relevance
        let unique_patterns = self
            .deduplicate_and_rank_patterns(all_patterns, query)
            .await?;

        // 5. Limit results and return top-k
        let final_patterns: Vec<LearningPattern> =
            unique_patterns.into_iter().take(limit).collect();

        // 6. Cache the results for future searches
        if let Err(e) = self.cache_patterns(query, &final_patterns).await {
            warn!("Failed to cache search results: {}", e);
        }

        info!(
            "Hybrid search completed for query: {}, returning {} patterns",
            query,
            final_patterns.len()
        );

        Ok(final_patterns)
    }

    /// Search for patterns semantically using FAISS
    pub async fn search_patterns_semantic(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<LearningPattern>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("MemorySearchBridge not initialized"));
        }

        debug!("Performing semantic search for: {}", query);

        // Get FAISS connection ID
        let faiss_id = {
            let ids = self.connection_ids.read().await;
            *ids.get(&DatabaseType::FAISS)
                .ok_or_else(|| anyhow::anyhow!("FAISS connection not found"))?
        };

        // Get connection details
        let connection = self
            .database_manager
            .get_connection(faiss_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("FAISS connection not found"))?;

        debug!(
            "Using FAISS connection for semantic search: {} ({})",
            connection.name, connection.id
        );

        // Create FAISS manager from connection
        let faiss_manager = odincode_databases::faiss::FaissManager::new().await?;

        // Convert query to vector embedding (simplified - in real implementation would use embedding model)
        let query_vector = self.text_to_embedding(query).await?;

        // Create search query
        let search_query = odincode_databases::faiss::SearchQuery {
            vector: query_vector,
            k: limit,
            filters: None,
        };

        // Perform FAISS search
        let search_results = faiss_manager.search(search_query).await?;

        // Convert FAISS results to LearningPattern objects
        let mut patterns = Vec::new();
        for result in search_results {
            if let Some(pattern) = self.vector_result_to_pattern(result).await? {
                patterns.push(pattern);
            }
        }

        info!(
            "Semantic search completed for query: {}, found {} patterns",
            query,
            patterns.len()
        );

        Ok(patterns)
    }

    /// Search for patterns using graph traversal in Neo4j
    pub async fn search_patterns_graph(
        &self,
        pattern_type: Option<PatternType>,
        limit: usize,
    ) -> Result<Vec<LearningPattern>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("MemorySearchBridge not initialized"));
        }

        debug!("Performing graph-based search");

        // Get Neo4j connection ID
        let neo4j_id = {
            let ids = self.connection_ids.read().await;
            *ids.get(&DatabaseType::Neo4j)
                .ok_or_else(|| anyhow::anyhow!("Neo4j connection not found"))?
        };

        // Get connection details
        let connection = self
            .database_manager
            .get_connection(neo4j_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Neo4j connection not found"))?;

        debug!(
            "Using Neo4j connection for graph search: {} ({})",
            connection.name, connection.id
        );

        // Create Neo4j manager from connection
        let neo4j_manager = odincode_databases::neo4j::Neo4jManager::new().await?;

        // Build Cypher query based on pattern type
        let query = if let Some(pattern_type) = pattern_type {
            format!(
                "MATCH (p:LearningPattern {{pattern_type: '{}'}})-[r]-(related:LearningPattern) \
                 RETURN p, r, related \
                 LIMIT {}",
                pattern_type, limit
            )
        } else {
            format!(
                "MATCH (p:LearningPattern)-[r]-(related:LearningPattern) \
                 RETURN p, r, related \
                 LIMIT {}",
                limit
            )
        };

        // Execute graph query
        let graph_result = neo4j_manager.execute_query(&query).await?;

        // Convert graph results to LearningPattern objects
        let mut patterns = Vec::new();
        for node in graph_result.nodes {
            if let Some(pattern) = self.neo4j_node_to_pattern(node).await? {
                patterns.push(pattern);
            }
        }

        info!(
            "Graph-based search completed, found {} patterns",
            patterns.len()
        );

        Ok(patterns)
    }

    /// Search for patterns using keyword search in SQLite
    pub async fn search_patterns_keyword(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<LearningPattern>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("MemorySearchBridge not initialized"));
        }

        debug!("Performing keyword search for: {}", query);

        // Get SQLite connection ID
        let sqlite_id = {
            let ids = self.connection_ids.read().await;
            *ids.get(&DatabaseType::SQLite)
                .ok_or_else(|| anyhow::anyhow!("SQLite connection not found"))?
        };

        // Get connection details
        let connection = self
            .database_manager
            .get_connection(sqlite_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("SQLite connection not found"))?;

        debug!(
            "Using SQLite connection for keyword search: {} ({})",
            connection.name, connection.id
        );

        // Create SQLite manager from connection
        let sqlite_manager =
            odincode_databases::sqlite::SQLiteManager::new(&connection.connection_string)?;

        // Perform keyword search using SQLite full-text search
        let sqlite_patterns = sqlite_manager
            .search_learning_patterns(query, limit)
            .await?;

        // Convert SQLite patterns to LTMC patterns
        let mut patterns = Vec::new();
        for sqlite_pattern in sqlite_patterns {
            let pattern = LearningPattern {
                id: uuid::Uuid::parse_str(&sqlite_pattern.id)
                    .unwrap_or_else(|_| uuid::Uuid::new_v4()),
                pattern_type: self.parse_pattern_type(&sqlite_pattern.pattern_type),
                content: sqlite_pattern.pattern_data,
                context: HashMap::new(), // Would be populated from additional metadata
                created: sqlite_pattern.created_at,
                last_accessed: chrono::Utc::now(),
                access_count: 0,
                confidence: sqlite_pattern.confidence as f32,
            };
            patterns.push(pattern);
        }

        info!(
            "Keyword search completed for query: {}, found {} patterns",
            query,
            patterns.len()
        );

        Ok(patterns)
    }

    /// Convert text query to vector embedding (simplified implementation)
    async fn text_to_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // In a real implementation, this would use an embedding model like BERT, OpenAI embeddings, etc.
        // For now, we'll create a simple hash-based embedding as a placeholder
        let words: Vec<&str> = text.split_whitespace().collect();
        let dimension = 768; // Standard embedding dimension
        let mut embedding = vec![0.0; dimension];

        for word in words.iter() {
            let hash = self.simple_hash(word) as usize;
            let index = hash % dimension;
            embedding[index] += 1.0;
        }

        // Normalize the embedding
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }

        Ok(embedding)
    }

    /// Simple hash function for text
    fn simple_hash(&self, text: &str) -> u32 {
        let mut hash: u32 = 5381;
        for byte in text.as_bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(*byte as u32);
        }
        hash
    }

    /// Parse pattern type from string
    fn parse_pattern_type(&self, pattern_type_str: &str) -> PatternType {
        match pattern_type_str.to_lowercase().as_str() {
            "code_pattern" | "code" => PatternType::CodePattern,
            "architectural_decision" | "architecture" => PatternType::ArchitecturalDecision,
            "research_finding" | "research" => PatternType::ResearchFinding,
            "performance_data" | "performance" => PatternType::PerformanceData,
            "error_solution" | "error" => PatternType::ErrorSolution,
            "user_interaction" | "interaction" => PatternType::UserInteraction,
            "sequential_thinking" | "sequential" => PatternType::SequentialThinking,
            "model_training" | "training" => PatternType::ModelTraining,
            _ => PatternType::CodePattern, // Default fallback
        }
    }

    /// Get cached patterns from Redis
    async fn get_cached_patterns(&self, query: &str, limit: usize) -> Result<Vec<LearningPattern>> {
        let redis_id = {
            let ids = self.connection_ids.read().await;
            *ids.get(&DatabaseType::Redis)
                .ok_or_else(|| anyhow::anyhow!("Redis connection not found"))?
        };

        let redis_connection = self
            .database_manager
            .get_connection(redis_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Redis connection not found"))?;

        let redis_config = odincode_databases::redis::RedisConfig {
            url: redis_connection.connection_string.clone(),
            ..Default::default()
        };
        let redis_manager = odincode_databases::redis::RedisManager::new(redis_config)?;

        // Generate cache key
        let cache_key = format!("search:{}", self.simple_hash(query));

        // Try to get cached results
        let cached_result: Option<String> = redis_manager.cache_get("search", &cache_key).await?;
        if let Some(cached_json) = cached_result {
            let cached_patterns: Vec<LearningPattern> = serde_json::from_str(&cached_json)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize cached patterns: {}", e))?;
            Ok(cached_patterns.into_iter().take(limit).collect())
        } else {
            Ok(Vec::new())
        }
    }

    /// Convert FAISS vector search result to LearningPattern
    async fn vector_result_to_pattern(
        &self,
        result: VectorSearchResult,
    ) -> Result<Option<LearningPattern>> {
        // Extract pattern ID from metadata
        let _pattern_id = result
            .metadata
            .get("pattern_id")
            .ok_or_else(|| anyhow::anyhow!("Pattern ID not found in vector metadata"))?;

        // Get SQLite connection to retrieve the full pattern
        let sqlite_id = {
            let ids = self.connection_ids.read().await;
            *ids.get(&DatabaseType::SQLite)
                .ok_or_else(|| anyhow::anyhow!("SQLite connection not found"))?
        };

        let sqlite_connection = self
            .database_manager
            .get_connection(sqlite_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("SQLite connection not found"))?;

        let _sqlite_manager =
            odincode_databases::sqlite::SQLiteManager::new(&sqlite_connection.connection_string)?;

        // For now, return None as we need to implement the actual pattern retrieval
        // This is a placeholder until we have the proper SQLite pattern structure
        Ok(None)
    }

    /// Convert Neo4j node to LearningPattern
    async fn neo4j_node_to_pattern(&self, _node: GraphNode) -> Result<Option<LearningPattern>> {
        // For now, return None as we need to implement the actual conversion
        // This is a placeholder until we have the proper Neo4j node structure
        Ok(None)
    }

    /// Cache search results in Redis
    async fn cache_patterns(&self, query: &str, patterns: &[LearningPattern]) -> Result<()> {
        let redis_id = {
            let ids = self.connection_ids.read().await;
            *ids.get(&DatabaseType::Redis)
                .ok_or_else(|| anyhow::anyhow!("Redis connection not found"))?
        };

        let redis_connection = self
            .database_manager
            .get_connection(redis_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Redis connection not found"))?;

        let redis_config = odincode_databases::redis::RedisConfig {
            url: redis_connection.connection_string.clone(),
            ..Default::default()
        };
        let redis_manager = odincode_databases::redis::RedisManager::new(redis_config)?;

        // Generate cache key
        let cache_key = format!("search:{}", self.simple_hash(query));

        // Serialize patterns
        let patterns_json = serde_json::to_string(patterns)
            .map_err(|e| anyhow::anyhow!("Failed to serialize patterns for caching: {}", e))?;

        // Cache with TTL (1 hour)
        redis_manager
            .cache_set("search", &cache_key, &patterns_json, Some(3600))
            .await?;

        Ok(())
    }

    /// Deduplicate patterns and rank by relevance
    async fn deduplicate_and_rank_patterns(
        &self,
        patterns: Vec<LearningPattern>,
        query: &str,
    ) -> Result<Vec<LearningPattern>> {
        let mut seen_ids = std::collections::HashSet::new();
        let mut unique_patterns = Vec::new();

        // Remove duplicates by ID
        for pattern in patterns {
            if !seen_ids.contains(&pattern.id) {
                seen_ids.insert(pattern.id);
                unique_patterns.push(pattern);
            }
        }

        // Rank patterns by relevance score
        unique_patterns.sort_by(|a, b| {
            let score_a = self.calculate_relevance_score(a, query);
            let score_b = self.calculate_relevance_score(b, query);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(unique_patterns)
    }

    /// Calculate relevance score for a pattern
    fn calculate_relevance_score(&self, pattern: &LearningPattern, query: &str) -> f32 {
        let mut score = 0.0;

        // Base score from confidence
        score += pattern.confidence * 0.3;

        // Content relevance (simple keyword matching)
        let query_lower = query.to_lowercase();
        let content_lower = pattern.content.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        for word in &query_words {
            if content_lower.contains(word) {
                score += 0.2;
            }
        }

        // Recency boost (newer patterns get higher score)
        let days_old = (chrono::Utc::now() - pattern.created).num_days();
        if days_old < 7 {
            score += 0.1;
        } else if days_old < 30 {
            score += 0.05;
        }

        // Access count boost (frequently accessed patterns get higher score)
        if pattern.access_count > 10 {
            score += 0.1;
        } else if pattern.access_count > 5 {
            score += 0.05;
        }

        score
    }
}
