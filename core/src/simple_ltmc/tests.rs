//! Integration tests for the Simple LTMC system
//! 
//! Tests the complete workflow of task management, pattern storage, 
//! semantic search, and graph operations using the SQLite+FAISS implementation.

use anyhow::Result;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{
    SimpleLTMCManager,
    models::{Task, TaskStatus, TaskPriority, Pattern, PatternType, ProductRequirement},
    storage::StorageManager,
    search::SearchManager,
    graph::GraphManager,
};

#[tokio::test]
async fn test_simple_ltmc_full_workflow() -> Result<()> {
    // Create a temporary directory for the test database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_ltmc.db");
    let db_path_str = db_path.to_str().unwrap();

    // Initialize the Simple LTMC manager
    let ltmc_manager = SimpleLTMCManager::new(db_path_str).await?;

    // Test 1: Create and store a task
    let mut task = Task::new(
        "Implement user authentication".to_string(),
        "Create a secure authentication system with JWT tokens".to_string(),
    );
    task.priority = TaskPriority::High;
    task.estimated_time = Some(120); // 2 hours

    ltmc_manager.storage.create_task(&task).await?;
    
    // Verify the task was stored
    let retrieved_task = ltmc_manager.storage.get_task(task.id).await?;
    assert!(retrieved_task.is_some());
    let retrieved_task = retrieved_task.unwrap();
    assert_eq!(retrieved_task.title, task.title);
    assert_eq!(retrieved_task.description, task.description);
    assert_eq!(retrieved_task.priority, task.priority);
    assert_eq!(retrieved_task.estimated_time, task.estimated_time);

    // Test 2: Create and store a pattern
    let pattern = Pattern::new(
        "JWT Token Pattern".to_string(),
        "Use HS256 algorithm with secure secret rotation for JWT tokens".to_string(),
        PatternType::SecurityPattern,
    );

    ltmc_manager.storage.create_pattern(&pattern).await?;
    
    // Verify the pattern was stored
    let retrieved_pattern = ltmc_manager.storage.get_pattern(pattern.id).await?;
    assert!(retrieved_pattern.is_some());
    let retrieved_pattern = retrieved_pattern.unwrap();
    assert_eq!(retrieved_pattern.title, pattern.title);
    assert_eq!(retrieved_pattern.content, pattern.content);
    assert_eq!(retrieved_pattern.pattern_type, pattern.pattern_type);

    // Test 3: Add pattern to FAISS index
    let embedding = ltmc_manager.search.create_embedding(&pattern.content).await?;
    ltmc_manager.search.add_pattern_to_index(pattern.id, embedding).await?;
    
    // Verify the pattern was added to the index
    let index_size = ltmc_manager.search.get_index_size().await?;
    assert_eq!(index_size, 1);

    // Test 4: Search for similar patterns
    let query = "secure token authentication";
    let similar_patterns = ltmc_manager.search.search_patterns_by_content(query, 5).await?;
    assert!(!similar_patterns.is_empty());
    
    // The first result should be our pattern (since it's the only one)
    let (found_pattern_id, _similarity) = similar_patterns[0];
    assert_eq!(found_pattern_id, pattern.id);

    // Test 5: Create a PRD
    let mut prd = ProductRequirement::new(
        "User Authentication System".to_string(),
        "Implement a complete user authentication and authorization system".to_string(),
    );
    prd.add_goal("Secure user login and registration".to_string());
    prd.add_goal("JWT-based session management".to_string());

    ltmc_manager.storage.create_prd(&prd).await?;
    
    // Verify the PRD was stored
    let retrieved_prd = ltmc_manager.storage.get_prd(prd.id).await?;
    assert!(retrieved_prd.is_some());
    let retrieved_prd = retrieved_prd.unwrap();
    assert_eq!(retrieved_prd.title, prd.title);
    assert_eq!(retrieved_prd.overview, prd.overview);
    assert_eq!(retrieved_prd.goals.len(), 2);

    // Test 6: Associate task with PRD
    task.prd_id = Some(prd.id);
    ltmc_manager.storage.update_task(&task).await?;
    
    // Verify the association
    let prd_tasks = ltmc_manager.storage.get_tasks_by_prd(prd.id).await?;
    assert_eq!(prd_tasks.len(), 1);
    assert_eq!(prd_tasks[0].id, task.id);

    // Test 7: Create child task
    let mut child_task = Task::new(
        "Design database schema".to_string(),
        "Create database tables for users and sessions".to_string(),
    );
    child_task.parent_task_id = Some(task.id);
    child_task.priority = TaskPriority::Normal;
    child_task.estimated_time = Some(60); // 1 hour

    ltmc_manager.storage.create_task(&child_task).await?;
    
    // Verify the child task was stored
    let retrieved_child = ltmc_manager.storage.get_task(child_task.id).await?;
    assert!(retrieved_child.is_some());
    let retrieved_child = retrieved_child.unwrap();
    assert_eq!(retrieved_child.parent_task_id, Some(task.id));

    // Test 8: Get child tasks
    let child_tasks = ltmc_manager.storage.get_child_tasks(task.id).await?;
    assert_eq!(child_tasks.len(), 1);
    assert_eq!(child_tasks[0].id, child_task.id);

    // Test 9: Graph operations - task hierarchy
    let task_hierarchy = ltmc_manager.graph.get_task_hierarchy(task.id).await?;
    assert!(!task_hierarchy.is_empty());
    // Should include both parent and child tasks
    
    // Test 10: Update task status
    task.status = TaskStatus::InProgress;
    task.mark_in_progress();
    ltmc_manager.storage.update_task(&task).await?;
    
    let updated_task = ltmc_manager.storage.get_task(task.id).await?;
    assert!(updated_task.is_some());
    let updated_task = updated_task.unwrap();
    assert_eq!(updated_task.status, TaskStatus::InProgress);
    assert!(updated_task.completed_at.is_none());

    // Test 11: Complete task
    task.status = TaskStatus::Completed;
    task.mark_completed();
    ltmc_manager.storage.update_task(&task).await?;
    
    let completed_task = ltmc_manager.storage.get_task(task.id).await?;
    assert!(completed_task.is_some());
    let completed_task = completed_task.unwrap();
    assert_eq!(completed_task.status, TaskStatus::Completed);
    assert!(completed_task.completed_at.is_some());

    // Test 12: Create another pattern for similarity testing
    let pattern2 = Pattern::new(
        "Password Hashing Pattern".to_string(),
        "Use bcrypt with salt rounds >= 12 for secure password hashing".to_string(),
        PatternType::SecurityPattern,
    );

    ltmc_manager.storage.create_pattern(&pattern2).await?;
    let embedding2 = ltmc_manager.search.create_embedding(&pattern2.content).await?;
    ltmc_manager.search.add_pattern_to_index(pattern2.id, embedding2).await?;

    // Test 13: Find related patterns using semantic search
    let related_patterns = ltmc_manager.graph.find_related_patterns(
        pattern.id,
        &ltmc_manager.search,
        5,
    ).await?;
    
    // Should find at least one related pattern (pattern2)
    assert!(!related_patterns.is_empty());

    // Test 14: Test batch operations
    let patterns_batch = vec![
        (Uuid::new_v4(), vec![0.1; 128]),
        (Uuid::new_v4(), vec![0.2; 128]),
        (Uuid::new_v4(), vec![0.3; 128]),
    ];
    
    ltmc_manager.search.batch_add_patterns(&patterns_batch).await?;
    let batch_index_size = ltmc_manager.search.get_index_size().await?;
    assert_eq!(batch_index_size, 4); // 2 existing + 3 batch patterns

    println!("All Simple LTMC tests passed successfully!");

    Ok(())
}

#[tokio::test]
async fn test_storage_layer_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_storage.db");
    let db_path_str = db_path.to_str().unwrap();

    let pool = super::storage::init_db(db_path_str).await?;
    let storage = StorageManager::new(pool);

    // Test CRUD operations for tasks
    let task = Task::new(
        "Storage Test Task".to_string(),
        "Test storage layer operations".to_string(),
    );

    // Create
    storage.create_task(&task).await?;
    
    // Read
    let retrieved = storage.get_task(task.id).await?;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().title, task.title);

    // Update
    let mut updated_task = task.clone();
    updated_task.title = "Updated Storage Test Task".to_string();
    storage.update_task(&updated_task).await?;
    
    let retrieved_updated = storage.get_task(task.id).await?;
    assert!(retrieved_updated.is_some());
    assert_eq!(retrieved_updated.unwrap().title, "Updated Storage Test Task");

    // Delete
    storage.delete_task(task.id).await?;
    
    let deleted_task = storage.get_task(task.id).await?;
    assert!(deleted_task.is_none());

    Ok(())
}

#[tokio::test]
async fn test_search_functionality() -> Result<()> {
    let index = SearchManager::create_faiss_index()?;
    let search_manager = SearchManager::new(Arc::new(RwLock::new(index)));

    // Test embedding creation
    let text = "This is a test pattern for semantic search";
    let embedding = search_manager.create_embedding(text).await?;
    assert_eq!(embedding.len(), 128); // Should match the dimension in create_faiss_index

    // Test adding to index
    let pattern_id = Uuid::new_v4();
    search_manager.add_pattern_to_index(pattern_id, embedding.clone()).await?;
    
    let index_size = search_manager.get_index_size().await?;
    assert_eq!(index_size, 1);

    // Test searching
    let results = search_manager.search_similar_patterns(&embedding, 5).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, pattern_id);

    // Test content-based search
    let content_results = search_manager.search_patterns_by_content("test pattern", 5).await?;
    assert!(!content_results.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_graph_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_graph.db");
    let db_path_str = db_path.to_str().unwrap();

    let pool = super::storage::init_db(db_path_str).await?;
    let graph = GraphManager::new(pool.clone());
    let storage = StorageManager::new(pool);

    // Create test tasks
    let task1 = Task::new(
        "Parent Task".to_string(),
        "Parent task for testing".to_string(),
    );
    let task2 = Task::new(
        "Child Task".to_string(),
        "Child task for testing".to_string(),
    );

    storage.create_task(&task1).await?;
    storage.create_task(&task2).await?;

    // Test creating relationships would require the relationships table
    // which isn't fully implemented in our simplified version
    
    // Test getting task hierarchy
    let hierarchy = graph.get_task_hierarchy(task1.id).await?;
    // Should at least return the root task
    assert!(!hierarchy.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_errors.db");
    let db_path_str = db_path.to_str().unwrap();

    // Test with wrong embedding dimensions
    let index = SearchManager::create_faiss_index()?;
    let search_manager = SearchManager::new(Arc::new(RwLock::new(index)));
    
    let wrong_embedding = vec![0.1; 64]; // Wrong dimension (should be 128)
    let result = search_manager.add_pattern_to_index(Uuid::new_v4(), wrong_embedding).await;
    assert!(result.is_err());

    Ok(())

    Ok(())
}