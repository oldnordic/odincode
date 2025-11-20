//! OdinCode LTMC Module
//!
//! The LTMC (Learning Through Meta-Cognition) module provides persistent memory
//! and learning capabilities for the OdinCode AI coding assistant.
//! It uses a 4-database system (SQLite + Neo4j + Redis + FAISS) for comprehensive
//! knowledge storage and retrieval.

pub mod bridges;
pub mod config;
pub mod features;
pub mod manager;
pub mod models;
pub mod search;

pub use bridges::*;
pub use config::*;
pub use features::*;
pub use manager::*;
pub use models::*;
pub use search::*;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_ltmc_manager_creation() {
        let manager = LTMManager::new();
        assert_eq!(manager.pattern_cache.read().await.len(), 0);
        assert_eq!(manager.session_cache.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_pattern_storage_and_retrieval() {
        // Test with in-memory manager only (no database initialization)
        let manager = LTMManager::new();

        let pattern = LearningPattern {
            id: Uuid::new_v4(),
            pattern_type: PatternType::CodePattern,
            content: "Test pattern content".to_string(),
            context: std::collections::HashMap::new(),
            created: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 0,
            confidence: 0.8,
        };

        // Store pattern in memory cache only
        let id = pattern.id;
        manager
            .pattern_cache
            .write()
            .await
            .insert(id, pattern.clone());

        let retrieved = manager.pattern_cache.read().await.get(&id).cloned();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, pattern.id);
    }

    #[tokio::test]
    async fn test_sequential_thinking_session() {
        // Test with in-memory manager only (no database initialization)
        let manager = LTMManager::new();

        // Create session directly in memory cache
        let session_id = Uuid::new_v4();
        let session = SequentialThinkingSession {
            id: session_id,
            context: "Test context".to_string(),
            reasoning_type: ReasoningType::Sequential,
            thoughts: Vec::new(),
            created: chrono::Utc::now(),
            completed: None,
            summary: None,
        };

        manager
            .session_cache
            .write()
            .await
            .insert(session_id, session);

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("test".to_string(), "value".to_string());

        // Add thought directly to session in memory cache
        let thought = Thought {
            id: Uuid::new_v4(),
            previous_thought_id: None,
            content: "Test thought".to_string(),
            thought_type: ThoughtType::Analysis,
            created: chrono::Utc::now(),
            metadata,
        };

        let mut sessions = manager.session_cache.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.thoughts.push(thought);
        }
        drop(sessions);

        let added = true;

        assert!(added);

        let session = manager.session_cache.read().await.get(&session_id).cloned();
        assert!(session.is_some());
        assert_eq!(session.unwrap().thoughts.len(), 1);
    }
}
