//! API endpoints for the Simple LTMC system
//!
//! Provides HTTP endpoints for task management, pattern storage, and semantic search
//! using the local SQLite+FAISS implementation.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{
    graph::GraphManager,
    models::{
        EntityType, Pattern, ProductRequirement, RelatedPattern, Relationship, RelationshipType,
        Task, TaskPriority, TaskStatus,
    },
    search::SearchManager,
    storage::StorageManager,
};

/// State containing all the managers needed for the API
pub struct SimpleLTMCState {
    pub storage: StorageManager,
    pub search: Arc<RwLock<SearchManager>>,
    pub graph: GraphManager,
}

/// Request body for creating a new task
#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
    pub parent_task_id: Option<Uuid>,
    pub prd_id: Option<Uuid>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub estimated_time: Option<u32>,
    pub related_files: Option<Vec<String>>,
}

/// Request body for updating a task
#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub estimated_time: Option<u32>,
    pub actual_time: Option<u32>,
    pub related_files: Option<Vec<String>>,
}

/// Request body for creating a new pattern
#[derive(Deserialize)]
pub struct CreatePatternRequest {
    pub title: String,
    pub content: String,
    pub pattern_type: String, // Will be converted to PatternType
    pub context: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Request body for creating a new PRD
#[derive(Deserialize)]
pub struct CreatePRDRequest {
    pub title: String,
    pub overview: String,
}

/// Response wrapper for API results
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

/// Create the router with all simple LTMC endpoints
pub fn create_router(state: SimpleLTMCState) -> Router {
    Router::new()
        // Task endpoints
        .route("/tasks", post(create_task))
        .route(
            "/tasks/:id",
            get(get_task).put(update_task).delete(delete_task),
        )
        .route("/tasks", get(list_tasks))
        .route("/tasks/:id/children", get(get_child_tasks))
        .route("/tasks/:id/dependencies", get(get_task_dependencies))
        .route("/tasks/:id/dependents", get(get_dependent_tasks))
        // Pattern endpoints
        .route("/patterns", post(create_pattern))
        .route(
            "/patterns/:id",
            get(get_pattern).put(update_pattern).delete(delete_pattern),
        )
        .route("/patterns", get(list_patterns))
        .route("/patterns/search", post(search_patterns))
        // PRD endpoints
        .route("/prds", post(create_prd))
        .route("/prds/:id", get(get_prd).put(update_prd))
        .route("/prds", get(list_prds))
        .route("/prds/:id/tasks", get(get_prd_tasks))
        // Relationship endpoints
        .route("/relationships", post(create_relationship))
        .route(
            "/relationships/:id",
            get(get_relationship).delete(delete_relationship),
        )
        .route("/entities/:id/relationships", get(get_entity_relationships))
        // Graph operations
        .route("/graph/traverse/:id", get(traverse_graph))
        .route("/graph/related/:id", get(get_related_entities))
        .with_state(Arc::new(state))
}

/// Create a new task
async fn create_task(
    State(state): State<Arc<SimpleLTMCState>>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<Json<ApiResponse<Task>>, StatusCode> {
    let mut task = Task::new(request.title, request.description);

    task.parent_task_id = request.parent_task_id;
    task.prd_id = request.prd_id;
    if let Some(status) = request.status {
        task.status = status;
    }
    if let Some(priority) = request.priority {
        task.priority = priority;
    }
    task.estimated_time = request.estimated_time;
    if let Some(files) = request.related_files {
        task.related_files = files;
    }

    state.storage.create_task(&task).await.map_err(|e| {
        eprintln!("Error creating task: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(task),
        message: Some("Task created successfully".to_string()),
    }))
}

/// Get a task by ID
async fn get_task(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(task_id): Path<String>,
) -> Result<Json<ApiResponse<Task>>, StatusCode> {
    let task_id = Uuid::parse_str(&task_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let task = state.storage.get_task(task_id).await.map_err(|e| {
        eprintln!("Error getting task: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match task {
        Some(task) => Ok(Json(ApiResponse {
            success: true,
            data: Some(task),
            message: None,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Update a task
async fn update_task(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(task_id): Path<String>,
    Json(request): Json<UpdateTaskRequest>,
) -> Result<Json<ApiResponse<Task>>, StatusCode> {
    let task_id = Uuid::parse_str(&task_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut task = state
        .storage
        .get_task(task_id)
        .await
        .map_err(|e| {
            eprintln!("Error getting task: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(title) = request.title {
        task.title = title;
    }
    if let Some(description) = request.description {
        task.description = description;
    }
    if let Some(status) = request.status {
        task.status = status;
    }
    if let Some(priority) = request.priority {
        task.priority = priority;
    }
    if let Some(estimated_time) = request.estimated_time {
        task.estimated_time = Some(estimated_time);
    }
    if let Some(actual_time) = request.actual_time {
        task.actual_time = Some(actual_time);
    }
    if let Some(related_files) = request.related_files {
        task.related_files = related_files;
    }

    if task.status == TaskStatus::Completed {
        task.mark_completed();
    } else if task.status == TaskStatus::InProgress {
        task.mark_in_progress();
    }

    state.storage.update_task(&task).await.map_err(|e| {
        eprintln!("Error updating task: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(task),
        message: Some("Task updated successfully".to_string()),
    }))
}

/// Delete a task
async fn delete_task(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(task_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let task_id = Uuid::parse_str(&task_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    state.storage.delete_task(task_id).await.map_err(|e| {
        eprintln!("Error deleting task: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(StatusCode::OK)
}

/// Get all tasks
async fn list_tasks(
    State(state): State<Arc<SimpleLTMCState>>,
) -> Result<Json<ApiResponse<Vec<Task>>>, StatusCode> {
    let tasks = state.storage.list_tasks().await.map_err(|e| {
        eprintln!("Error listing tasks: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(tasks.clone()),
        message: Some(format!("Found {} tasks", tasks.len())),
    }))
}

/// Get child tasks of a parent task
async fn get_child_tasks(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(task_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Task>>>, StatusCode> {
    let task_id = Uuid::parse_str(&task_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let child_tasks = state.storage.get_child_tasks(task_id).await.map_err(|e| {
        eprintln!("Error getting child tasks: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(child_tasks.clone()),
        message: Some(format!("Found {} child tasks", child_tasks.len())),
    }))
}

/// Create a new pattern
async fn create_pattern(
    State(state): State<Arc<SimpleLTMCState>>,
    Json(request): Json<CreatePatternRequest>,
) -> Result<Json<ApiResponse<Pattern>>, StatusCode> {
    // Map string pattern_type to enum (simplified mapping)
    let pattern_type = match request.pattern_type.to_lowercase().as_str() {
        "architecturaldecision" | "architectural_decision" => {
            super::models::PatternType::ArchitecturalDecision
        }
        "codepattern" | "code_pattern" => super::models::PatternType::CodePattern,
        "researchfinding" | "research_finding" => super::models::PatternType::ResearchFinding,
        "performancedata" | "performance_data" => super::models::PatternType::PerformanceData,
        "errorsolution" | "error_solution" => super::models::PatternType::ErrorSolution,
        "userinteraction" | "user_interaction" => super::models::PatternType::UserInteraction,
        "sequentialthinking" | "sequential_thinking" => {
            super::models::PatternType::SequentialThinking
        }
        "modeltraining" | "model_training" => super::models::PatternType::ModelTraining,
        "bestpractice" | "best_practice" => super::models::PatternType::BestPractice,
        "antipattern" | "anti_pattern" => super::models::PatternType::AntiPattern,
        "securitypattern" | "security_pattern" => super::models::PatternType::SecurityPattern,
        _ => super::models::PatternType::CodePattern, // Default
    };

    let mut pattern = Pattern::new(request.title, request.content, pattern_type);

    if let Some(context) = request.context {
        pattern.context = context;
    }

    state.storage.create_pattern(&pattern).await.map_err(|e| {
        eprintln!("Error creating pattern: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Add the pattern to the FAISS index for similarity search
    let embedding = state
        .search
        .read()
        .await
        .create_embedding(&pattern.content)
        .await
        .map_err(|e| {
            eprintln!("Error creating embedding: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    state
        .search
        .write()
        .await
        .add_pattern_to_index(pattern.id, embedding)
        .await
        .map_err(|e| {
            eprintln!("Error adding pattern to search index: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(pattern),
        message: Some("Pattern created successfully".to_string()),
    }))
}

/// Get a pattern by ID
async fn get_pattern(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(pattern_id): Path<String>,
) -> Result<Json<ApiResponse<Pattern>>, StatusCode> {
    let pattern_id = Uuid::parse_str(&pattern_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let pattern = state.storage.get_pattern(pattern_id).await.map_err(|e| {
        eprintln!("Error getting pattern: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match pattern {
        Some(pattern) => Ok(Json(ApiResponse {
            success: true,
            data: Some(pattern),
            message: None,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Update an existing pattern
async fn update_pattern(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(pattern_id): Path<String>,
    Json(request): Json<CreatePatternRequest>,
) -> Result<Json<ApiResponse<Pattern>>, StatusCode> {
    let pattern_id = Uuid::parse_str(&pattern_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Get existing pattern
    let mut pattern = state
        .storage
        .get_pattern(pattern_id)
        .await
        .map_err(|e| {
            eprintln!("Error getting pattern: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Update pattern fields
    pattern.title = request.title;
    pattern.content = request.content;
    if let Some(context) = request.context {
        pattern.context = context;
    }
    pattern.last_accessed = chrono::Utc::now();

    state.storage.update_pattern(&pattern).await.map_err(|e| {
        eprintln!("Error updating pattern: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(pattern),
        message: Some("Pattern updated successfully".to_string()),
    }))
}

/// Delete a pattern by ID
async fn delete_pattern(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(pattern_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let pattern_id = Uuid::parse_str(&pattern_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    state
        .storage
        .delete_pattern(pattern_id)
        .await
        .map_err(|e| {
            eprintln!("Error deleting pattern: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(StatusCode::OK)
}

/// Get all patterns
async fn list_patterns(
    State(state): State<Arc<SimpleLTMCState>>,
) -> Result<Json<ApiResponse<Vec<Pattern>>>, StatusCode> {
    let patterns = state.storage.list_patterns().await.map_err(|e| {
        eprintln!("Error listing patterns: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(patterns.clone()),
        message: Some(format!("Found {} patterns", patterns.len())),
    }))
}

/// Search for patterns
async fn search_patterns(
    State(state): State<Arc<SimpleLTMCState>>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<Pattern>>>, StatusCode> {
    let search_results = state
        .search
        .read()
        .await
        .search_patterns_by_content(&request.query, request.limit.unwrap_or(10))
        .await
        .map_err(|e| {
            eprintln!("Error searching patterns: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get the full pattern details for each result
    let mut patterns = Vec::new();
    for (pattern_id, _similarity) in search_results {
        if let Some(pattern) = state.storage.get_pattern(pattern_id).await.map_err(|e| {
            eprintln!("Error getting pattern: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })? {
            patterns.push(pattern);
        }
    }

    Ok(Json(ApiResponse {
        success: true,
        data: Some(patterns.clone()),
        message: Some(format!("Found {} patterns", patterns.len())),
    }))
}

/// Create a new PRD
async fn create_prd(
    State(state): State<Arc<SimpleLTMCState>>,
    Json(request): Json<CreatePRDRequest>,
) -> Result<Json<ApiResponse<ProductRequirement>>, StatusCode> {
    let prd = ProductRequirement::new(request.title, request.overview);

    state.storage.create_prd(&prd).await.map_err(|e| {
        eprintln!("Error creating PRD: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(prd),
        message: Some("PRD created successfully".to_string()),
    }))
}

/// Get a PRD by ID
async fn get_prd(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(prd_id): Path<String>,
) -> Result<Json<ApiResponse<ProductRequirement>>, StatusCode> {
    let prd_id = Uuid::parse_str(&prd_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let prd = state.storage.get_prd(prd_id).await.map_err(|e| {
        eprintln!("Error getting PRD: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match prd {
        Some(prd) => Ok(Json(ApiResponse {
            success: true,
            data: Some(prd),
            message: None,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Update an existing PRD
async fn update_prd(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(prd_id): Path<String>,
    Json(request): Json<CreatePRDRequest>,
) -> Result<Json<ApiResponse<ProductRequirement>>, StatusCode> {
    let prd_id = Uuid::parse_str(&prd_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Get existing PRD
    let mut prd = state
        .storage
        .get_prd(prd_id)
        .await
        .map_err(|e| {
            eprintln!("Error getting PRD: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Update PRD fields
    prd.title = request.title;
    prd.overview = request.overview;
    prd.updated_at = chrono::Utc::now();

    state.storage.update_prd(&prd).await.map_err(|e| {
        eprintln!("Error updating PRD: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(prd),
        message: Some("PRD updated successfully".to_string()),
    }))
}

/// Get all PRDs
async fn list_prds(
    State(state): State<Arc<SimpleLTMCState>>,
) -> Result<Json<ApiResponse<Vec<ProductRequirement>>>, StatusCode> {
    let prds = state.storage.list_prds().await.map_err(|e| {
        eprintln!("Error listing PRDs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(prds.clone()),
        message: Some(format!("Found {} PRDs", prds.len())),
    }))
}

/// Get tasks for a PRD
async fn get_prd_tasks(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(prd_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Task>>>, StatusCode> {
    let prd_id = Uuid::parse_str(&prd_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let tasks = state.storage.get_tasks_by_prd(prd_id).await.map_err(|e| {
        eprintln!("Error getting PRD tasks: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(tasks.clone()),
        message: Some(format!("Found {} tasks for PRD", tasks.len())),
    }))
}

/// Get task dependencies
async fn get_task_dependencies(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(task_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Task>>>, StatusCode> {
    let task_id = Uuid::parse_str(&task_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let dependencies = state
        .graph
        .get_task_dependencies(task_id)
        .await
        .map_err(|e| {
            eprintln!("Error getting task dependencies: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(dependencies.clone()),
        message: Some(format!("Found {} dependencies", dependencies.len())),
    }))
}

/// Get dependent tasks
async fn get_dependent_tasks(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(task_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Task>>>, StatusCode> {
    let task_id = Uuid::parse_str(&task_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let dependents = state
        .graph
        .get_dependent_tasks(task_id)
        .await
        .map_err(|e| {
            eprintln!("Error getting dependent tasks: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(dependents.clone()),
        message: Some(format!("Found {} dependent tasks", dependents.len())),
    }))
}

/// Create a new relationship
async fn create_relationship(
    State(state): State<Arc<SimpleLTMCState>>,
    Json(request): Json<CreateRelationshipRequest>,
) -> Result<Json<ApiResponse<Relationship>>, StatusCode> {
    let from_id = Uuid::parse_str(&request.from_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let to_id = Uuid::parse_str(&request.to_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    state
        .graph
        .create_relationship(from_id, to_id, request.relationship_type)
        .await
        .map_err(|e| {
            eprintln!("Error creating relationship: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Create a relationship object to return
    let relationship = Relationship {
        id: Uuid::new_v4(),
        from_id,
        from_type: request.from_type,
        to_id,
        to_type: request.to_type,
        relationship_type: request.relationship_type,
        metadata: std::collections::HashMap::new(),
        created_at: chrono::Utc::now().naive_utc(),
    };

    Ok(Json(ApiResponse {
        success: true,
        data: Some(relationship),
        message: Some("Relationship created successfully".to_string()),
    }))
}

/// Get a relationship by ID
async fn get_relationship(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(relationship_id): Path<String>,
) -> Result<Json<ApiResponse<Relationship>>, StatusCode> {
    let relationship_id = Uuid::parse_str(&relationship_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let relationship = state
        .storage
        .get_relationship(relationship_id)
        .await
        .map_err(|e| {
            eprintln!("Error getting relationship: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match relationship {
        Some(relationship) => Ok(Json(ApiResponse {
            success: true,
            data: Some(relationship),
            message: None,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Delete a relationship by ID
async fn delete_relationship(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(relationship_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let relationship_id = Uuid::parse_str(&relationship_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    state
        .storage
        .delete_relationship(relationship_id)
        .await
        .map_err(|e| {
            eprintln!("Error deleting relationship: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(StatusCode::OK)
}

/// Get all relationships for an entity
async fn get_entity_relationships(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(entity_id): Path<String>,
    Json(entity_type): Json<EntityType>,
) -> Result<Json<ApiResponse<Vec<Relationship>>>, StatusCode> {
    let entity_id = Uuid::parse_str(&entity_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let relationships = state
        .storage
        .get_relationships(entity_id, entity_type)
        .await
        .map_err(|e| {
            eprintln!("Error getting entity relationships: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(relationships.clone()),
        message: Some(format!("Found {} relationships", relationships.len())),
    }))
}

/// Traverse the graph starting from an entity
async fn traverse_graph(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(entity_id): Path<String>,
    Json(request): Json<TraverseGraphRequest>,
) -> Result<Json<ApiResponse<Vec<Uuid>>>, StatusCode> {
    let entity_id = Uuid::parse_str(&entity_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let connected_entities = state
        .graph
        .find_connected_entities(
            entity_id,
            request.max_depth.unwrap_or(3),
            request.entity_type,
            request.relationship_type,
        )
        .await
        .map_err(|e| {
            eprintln!("Error traversing graph: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Extract just the UUIDs from the result
    let entity_ids: Vec<Uuid> = connected_entities
        .into_iter()
        .map(|(id, _, _)| id)
        .collect();

    Ok(Json(ApiResponse {
        success: true,
        data: Some(entity_ids),
        message: Some(format!("Found {} connected entities", entity_ids.len())),
    }))
}

/// Get related entities for a specific entity
async fn get_related_entities(
    State(state): State<Arc<SimpleLTMCState>>,
    Path(entity_id): Path<String>,
    Json(entity_type): Json<EntityType>,
) -> Result<Json<ApiResponse<RelatedEntitiesResponse>>, StatusCode> {
    let entity_id = Uuid::parse_str(&entity_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Get related patterns
    let related_patterns_result = state
        .graph
        .find_related_patterns(
            entity_id,
            &state.search_manager,
            10, // limit
        )
        .await
        .map_err(|e| {
            eprintln!("Error finding related patterns: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Convert (Pattern, f32) to RelatedPattern
    let related_patterns: Vec<RelatedPattern> = related_patterns_result
        .into_iter()
        .map(|(pattern, similarity)| RelatedPattern {
            pattern_id: pattern.id,
            relationship_type: RelationshipType::RelatedTo,
            similarity_score: similarity,
            context: format!("Semantic similarity: {:.2}", similarity),
        })
        .collect();

    // Get task hierarchy if it's a task
    let related_tasks = if matches!(entity_type, EntityType::Task) {
        state
            .graph
            .get_task_hierarchy(entity_id)
            .await
            .map_err(|e| {
                eprintln!("Error getting task hierarchy: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        Vec::new()
    };

    let response = RelatedEntitiesResponse {
        patterns: related_patterns,
        tasks: related_tasks,
    };

    Ok(Json(ApiResponse {
        success: true,
        data: Some(response),
        message: Some("Retrieved related entities".to_string()),
    }))
}

/// Query parameters for search
#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
}

/// Request body for creating relationships
#[derive(Deserialize)]
pub struct CreateRelationshipRequest {
    pub from_id: String,
    pub from_type: EntityType,
    pub to_id: String,
    pub to_type: EntityType,
    pub relationship_type: RelationshipType,
}

/// Request body for graph traversal
#[derive(Deserialize)]
pub struct TraverseGraphRequest {
    pub entity_type: EntityType,
    pub relationship_type: Option<RelationshipType>,
    pub max_depth: Option<usize>,
}

/// Response for related entities
#[derive(Serialize)]
pub struct RelatedEntitiesResponse {
    pub patterns: Vec<RelatedPattern>,
    pub tasks: Vec<Task>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simple_ltmc::search::SearchManager;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        ServiceExt,
    };
    use serde_json::json;
    use sqlx::SqlitePool;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_create_task_endpoint() {
        // Set up test database
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let storage = StorageManager::new(pool.clone());
        storage.init_db().await.unwrap();

        let index = SearchManager::create_faiss_index().unwrap();
        let search = Arc::new(RwLock::new(SearchManager::new(Arc::new(RwLock::new(
            index,
        )))));
        let graph = GraphManager::new(pool.clone());

        let state = SimpleLTMCState {
            storage,
            search,
            graph,
        };

        let app = create_router(state);

        let create_task_request = json!({
            "title": "Test Task",
            "description": "This is a test task"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&create_task_request).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
