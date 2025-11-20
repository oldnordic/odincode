//! LTMC Manager Module
//!
//! This module contains the main LTMC manager functionality.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::bridges::MemorySearchBridge;
use crate::models::{
    LearningPattern, PatternType, ReasoningType, SequentialThinkingSession, Thought, ThoughtType,
};

/// Main LTMC (Learning Through Meta-Cognition) manager
#[derive(Clone)]
pub struct LTMManager {
    /// In-memory cache for frequently accessed patterns
    pub pattern_cache: Arc<RwLock<HashMap<Uuid, LearningPattern>>>,
    /// In-memory cache for sequential thinking sessions
    pub session_cache: Arc<RwLock<HashMap<Uuid, SequentialThinkingSession>>>,
    /// Memory search bridge for database operations
    pub memory_search_bridge: Option<MemorySearchBridge>,
}

impl Default for LTMManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LTMManager {
    /// Create a new LTMC manager instance
    pub fn new() -> Self {
        Self {
            pattern_cache: Arc::new(RwLock::new(HashMap::new())),
            session_cache: Arc::new(RwLock::new(HashMap::new())),
            memory_search_bridge: None,
        }
    }

    /// Initialize database connections
    pub async fn initialize(
        &mut self,
        database_manager: odincode_databases::DatabaseManager,
    ) -> Result<()> {
        info!("Initializing LTMC databases...");

        // Create and initialize memory search bridge
        let mut bridge = MemorySearchBridge::new(database_manager);
        bridge.initialize().await?;

        self.memory_search_bridge = Some(bridge);

        info!("LTMC databases initialized successfully");
        Ok(())
    }

    /// Store a learning pattern
    pub async fn store_pattern(&self, pattern: LearningPattern) -> Result<Uuid> {
        let id = pattern.id;

        // Store in cache
        let mut cache = self.pattern_cache.write().await;
        cache.insert(id, pattern.clone());
        drop(cache);

        // Store in databases using the bridge if available
        if let Some(bridge) = &self.memory_search_bridge {
            if bridge.is_initialized() {
                match bridge.store_pattern_atomically(pattern).await {
                    Ok(_) => debug!("Stored learning pattern in databases: {}", id),
                    Err(e) => {
                        error!(
                            "Failed to store learning pattern in databases: {} - {}",
                            id, e
                        );
                        // We continue even if database storage fails, relying on cache
                    }
                }
            }
        }

        debug!("Stored learning pattern: {}", id);
        Ok(id)
    }

    /// Retrieve a learning pattern by ID
    pub async fn get_pattern(&self, id: Uuid) -> Result<Option<LearningPattern>> {
        // Check cache first
        {
            let cache = self.pattern_cache.read().await;
            if let Some(pattern) = cache.get(&id) {
                // Update access statistics
                let mut mutable_pattern = pattern.clone();
                mutable_pattern.last_accessed = chrono::Utc::now();
                mutable_pattern.access_count += 1;

                // Update cache with new stats
                let mut cache_write = self.pattern_cache.write().await;
                cache_write.insert(id, mutable_pattern);
                drop(cache_write);

                return Ok(Some(pattern.clone()));
            }
        }

        // Try to get from bridge cache if available
        if let Some(bridge) = &self.memory_search_bridge {
            if bridge.is_initialized() {
                match bridge.get_pattern_from_cache(id).await {
                    Ok(Some(pattern)) => {
                        // Store in local cache
                        let mut cache = self.pattern_cache.write().await;
                        cache.insert(id, pattern.clone());
                        drop(cache);

                        return Ok(Some(pattern));
                    }
                    Ok(None) => {
                        // Pattern not found in bridge cache
                    }
                    Err(e) => {
                        error!("Failed to get pattern from bridge cache: {}", e);
                    }
                }
            }
        }

        // In a real implementation, we would query the databases here
        // For now, return None as the pattern wasn't found in cache
        Ok(None)
    }

    /// Search for patterns by type and content
    pub async fn search_patterns(
        &self,
        pattern_type: Option<PatternType>,
        query: &str,
    ) -> Result<Vec<LearningPattern>> {
        // Try to use the bridge for hybrid search if available
        if let Some(bridge) = &self.memory_search_bridge {
            if bridge.is_initialized() {
                match bridge
                    .search_patterns_hybrid(pattern_type.clone(), query, 10)
                    .await
                {
                    Ok(results) => {
                        debug!("Retrieved {} patterns from hybrid search", results.len());
                        return Ok(results);
                    }
                    Err(e) => {
                        error!("Hybrid search failed: {}", e);
                        // Fall back to cache search
                    }
                }
            }
        }

        // Fall back to cache search
        let cache = self.pattern_cache.read().await;
        let mut results = Vec::new();

        for pattern in cache.values() {
            let matches_type = match &pattern_type {
                Some(t) => &pattern.pattern_type == t,
                None => true,
            };

            let matches_content = pattern
                .content
                .to_lowercase()
                .contains(&query.to_lowercase());

            if matches_type && matches_content {
                results.push(pattern.clone());
            }
        }

        Ok(results)
    }

    /// Start a new sequential thinking session
    pub async fn start_sequential_thinking_session(
        &self,
        context: String,
        reasoning_type: ReasoningType,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let session = SequentialThinkingSession {
            id,
            context,
            reasoning_type,
            thoughts: Vec::new(),
            created: chrono::Utc::now(),
            completed: None,
            summary: None,
        };

        // Store in cache
        let mut cache = self.session_cache.write().await;
        cache.insert(id, session);
        drop(cache);

        debug!("Started sequential thinking session: {}", id);
        Ok(id)
    }

    /// Add a thought to a sequential thinking session
    pub async fn add_thought_to_session(
        &self,
        session_id: Uuid,
        content: String,
        thought_type: ThoughtType,
        metadata: HashMap<String, String>,
    ) -> Result<bool> {
        let mut cache = self.session_cache.write().await;
        if let Some(session) = cache.get_mut(&session_id) {
            let thought_id = Uuid::new_v4();
            let previous_thought_id = session.thoughts.last().map(|t: &Thought| t.id);

            let thought = Thought {
                id: thought_id,
                previous_thought_id,
                content,
                thought_type,
                created: chrono::Utc::now(),
                metadata,
            };

            session.thoughts.push(thought);
            drop(cache);

            debug!("Added thought to session {}: {}", session_id, thought_id);
            Ok(true)
        } else {
            drop(cache);
            Ok(false)
        }
    }

    /// Complete a sequential thinking session
    pub async fn complete_sequential_thinking_session(
        &self,
        session_id: Uuid,
        summary: String,
    ) -> Result<bool> {
        let mut cache = self.session_cache.write().await;
        if let Some(session) = cache.get_mut(&session_id) {
            session.completed = Some(chrono::Utc::now());
            session.summary = Some(summary);
            drop(cache);

            debug!("Completed sequential thinking session: {}", session_id);
            Ok(true)
        } else {
            drop(cache);
            Ok(false)
        }
    }

    /// Get a sequential thinking session by ID
    pub async fn get_sequential_thinking_session(
        &self,
        id: Uuid,
    ) -> Result<Option<SequentialThinkingSession>> {
        let cache = self.session_cache.read().await;
        Ok(cache.get(&id).cloned())
    }

    /// Get all patterns of a specific type
    pub async fn get_patterns_by_type(
        &self,
        pattern_type: PatternType,
    ) -> Result<Vec<LearningPattern>> {
        let cache = self.pattern_cache.read().await;
        let results: Vec<LearningPattern> = cache
            .values()
            .filter(|pattern| pattern.pattern_type == pattern_type)
            .cloned()
            .collect();

        Ok(results)
    }
}
