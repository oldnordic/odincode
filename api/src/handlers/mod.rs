//! API Handlers Module
//!
//! This module contains the request handlers for the API system.

use axum::{
    debug_handler,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

use odincode_agents::AgentCoordinator;
use odincode_core::CodeEngine;
use odincode_ltmc::{LTMManager, LearningPattern, PatternType};
use odincode_tools::ToolManager;

use crate::models::{ExecuteAgentRequest, ExecuteAgentResponse, FileResponse, LoadFileRequest};
use odincode_tools::EditTask;

use std::sync::Arc;

/// Represents the state of the API server
pub struct ApiState {
    /// Core code engine
    pub core_engine: Arc<CodeEngine>,
    /// LTMC manager
    pub ltmc_manager: Arc<LTMManager>,
    /// Agent coordinator
    pub agent_coordinator: Arc<AgentCoordinator>,
    /// Tool manager
    pub tool_manager: Arc<ToolManager>,
}

/// Health check endpoint
#[debug_handler]
pub async fn health_check() -> Json<HashMap<String, String>> {
    let mut response = HashMap::new();
    response.insert("status".to_string(), "healthy".to_string());
    response.insert("service".to_string(), "odincode-api".to_string());
    Json(response)
}

/// Load a file into the system
#[debug_handler]
pub async fn load_file(
    State(state): State<std::sync::Arc<ApiState>>,
    Json(request): Json<LoadFileRequest>,
) -> Result<Json<FileResponse>, StatusCode> {
    tracing::debug!("Loading file: {}", request.path);

    match state
        .core_engine
        .load_file(request.path, request.content, request.language)
        .await
    {
        Ok(id) => Ok(Json(FileResponse {
            id: id.to_string(),
            success: true,
            message: Some("File loaded successfully".to_string()),
        })),
        Err(e) => {
            tracing::error!("Failed to load file: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get a file by ID
#[debug_handler]
pub async fn get_file(
    State(state): State<std::sync::Arc<ApiState>>,
    Path(file_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    tracing::debug!("Getting file: {}", file_id);

    let uuid = match Uuid::parse_str(&file_id) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    match state.core_engine.get_file(uuid).await {
        Ok(Some(file)) => Ok(Json(
            serde_json::to_value(file).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get file: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Analyze a file
#[debug_handler]
pub async fn analyze_file(
    State(state): State<std::sync::Arc<ApiState>>,
    Path(file_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    tracing::debug!("Analyzing file: {}", file_id);

    let uuid = match Uuid::parse_str(&file_id) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    match state.core_engine.analyze_file(uuid).await {
        Ok(Some(result)) => Ok(Json(
            serde_json::to_value(result).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to analyze file: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List all agents
#[debug_handler]
pub async fn list_agents(
    State(state): State<std::sync::Arc<ApiState>>,
) -> Result<Json<Value>, StatusCode> {
    tracing::debug!("Listing all agents");

    match state.agent_coordinator.get_all_agents().await {
        Ok(agents) => Ok(Json(
            serde_json::to_value(agents).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )),
        Err(e) => {
            tracing::error!("Failed to list agents: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Execute an agent on a file
#[debug_handler]
pub async fn execute_agent(
    State(state): State<std::sync::Arc<ApiState>>,
    Json(request): Json<ExecuteAgentRequest>,
) -> Result<Json<Value>, StatusCode> {
    tracing::debug!(
        "Executing agent {} on file {}",
        request.agent_id,
        request.file_id
    );

    let agent_id = match Uuid::parse_str(&request.agent_id) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let file_id = match Uuid::parse_str(&request.file_id) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let result = state
        .agent_coordinator
        .execute_agent_on_file(agent_id, file_id)
        .await;

    match result {
        Ok(suggestions) => {
            let result_json =
                serde_json::to_value(suggestions).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(result_json))
        }
        Err(e) => {
            tracing::error!("Failed to execute agent: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Search LTMC patterns
#[debug_handler]
pub async fn search_patterns(
    State(state): State<std::sync::Arc<ApiState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    tracing::debug!("Searching LTMC patterns");

    let pattern_type = params.get("type").and_then(|t| match t.as_str() {
        "architectural_decision" => Some(PatternType::ArchitecturalDecision),
        "code_pattern" => Some(PatternType::CodePattern),
        "research_finding" => Some(PatternType::ResearchFinding),
        "performance_data" => Some(PatternType::PerformanceData),
        "error_solution" => Some(PatternType::ErrorSolution),
        "user_interaction" => Some(PatternType::UserInteraction),
        "sequential_thinking" => Some(PatternType::SequentialThinking),
        _ => None,
    });

    let query = params.get("q").cloned().unwrap_or_default();

    match state
        .ltmc_manager
        .search_patterns(pattern_type, &query)
        .await
    {
        Ok(patterns) => Ok(Json(
            serde_json::to_value(patterns).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )),
        Err(e) => {
            tracing::error!("Failed to search patterns: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Store an LTMC pattern
#[debug_handler]
pub async fn store_pattern(
    State(state): State<std::sync::Arc<ApiState>>,
    Json(pattern): Json<LearningPattern>,
) -> Result<Json<HashMap<String, String>>, StatusCode> {
    tracing::debug!("Storing LTMC pattern");

    match state.ltmc_manager.store_pattern(pattern).await {
        Ok(id) => {
            let mut response = HashMap::new();
            response.insert("id".to_string(), id.to_string());
            response.insert("success".to_string(), "true".to_string());
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to store pattern: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List all tools
#[debug_handler]
pub async fn list_tools(
    State(state): State<std::sync::Arc<ApiState>>,
) -> Result<Json<Value>, StatusCode> {
    tracing::debug!("Listing all tools");

    match state.tool_manager.get_all_tools().await {
        Ok(tools) => Ok(Json(
            serde_json::to_value(tools).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )),
        Err(e) => {
            tracing::error!("Failed to list tools: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Execute a tool on a file
#[debug_handler]
pub async fn execute_tool(
    State(state): State<std::sync::Arc<ApiState>>,
    Path(tool_id): Path<String>,
    Json(request): Json<HashMap<String, String>>,
) -> Result<Json<HashMap<String, Value>>, StatusCode> {
    tracing::debug!("Executing tool: {}", tool_id);

    let uuid = match Uuid::parse_str(&tool_id) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let file_id_str = match request.get("file_id") {
        Some(id) => id,
        None => return Err(StatusCode::BAD_REQUEST),
    };

    let file_id = match Uuid::parse_str(file_id_str) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    match state.tool_manager.execute_tool_on_file(uuid, file_id).await {
        Ok(success) => {
            let mut response = HashMap::new();
            response.insert("success".to_string(), Value::Bool(success));
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to execute tool: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Create a multi-edit operation
#[debug_handler]
pub async fn create_multi_edit_operation(
    State(state): State<std::sync::Arc<ApiState>>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<HashMap<String, String>>, StatusCode> {
    tracing::debug!("Creating multi-edit operation");

    // Extract data from the request
    let name = match request.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return Err(StatusCode::BAD_REQUEST),
    };

    let description = match request.get("description").and_then(|v| v.as_str()) {
        Some(d) => d.to_string(),
        None => String::new(),
    };

    let tasks_json = match request.get("tasks") {
        Some(t) => t,
        None => return Err(StatusCode::BAD_REQUEST),
    };

    // Convert JSON tasks to EditTask objects
    let tasks: Vec<EditTask> = match serde_json::from_value(tasks_json.clone()) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to parse tasks: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    match state
        .tool_manager
        .create_multi_edit_operation(name, description, tasks)
        .await
    {
        Ok(id) => {
            let mut response = HashMap::new();
            response.insert("id".to_string(), id.to_string());
            response.insert("success".to_string(), "true".to_string());
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to create multi-edit operation: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Execute a multi-edit operation
#[debug_handler]
pub async fn execute_multi_edit_operation(
    State(state): State<std::sync::Arc<ApiState>>,
    Path(operation_id): Path<String>,
) -> Result<Json<HashMap<String, Value>>, StatusCode> {
    tracing::debug!("Executing multi-edit operation: {}", operation_id);

    let uuid = match Uuid::parse_str(&operation_id) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    match state.tool_manager.execute_multi_edit_operation(uuid).await {
        Ok(success) => {
            let mut response = HashMap::new();
            response.insert("success".to_string(), Value::Bool(success));
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to execute multi-edit operation: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Lint a file
#[debug_handler]
pub async fn lint_file(
    State(state): State<std::sync::Arc<ApiState>>,
    Path(file_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    tracing::debug!("Linting file: {}", file_id);

    let uuid = match Uuid::parse_str(&file_id) {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    match state.tool_manager.lint_file(uuid).await {
        Ok(issues) => Ok(Json(
            serde_json::to_value(issues).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )),
        Err(e) => {
            tracing::error!("Failed to lint file: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Register a linter
#[debug_handler]
pub async fn register_linter(
    State(state): State<std::sync::Arc<ApiState>>,
    Json(config): Json<crate::models::LinterConfig>,
) -> Result<Json<HashMap<String, String>>, StatusCode> {
    tracing::debug!("Registering linter for language: {}", config.language);

    // Convert the API LinterConfig to the internal LinterConfig
    use odincode_core::Severity;
    use odincode_tools::linters::LinterConfig as InternalLinterConfig;

    // Convert string severity overrides to actual Severity enums
    let mut severity_overrides = std::collections::HashMap::new();
    for (rule, severity_str) in config.severity_overrides {
        let severity = match severity_str.as_str() {
            "Low" => Severity::Low,
            "Medium" => Severity::Medium,
            "High" => Severity::High,
            "Critical" => Severity::Critical,
            _ => Severity::Medium, // Default
        };
        severity_overrides.insert(rule, severity);
    }

    let internal_config = InternalLinterConfig {
        language: config.language,
        name: config.name,
        description: config.description,
        enabled_rules: config.enabled_rules,
        disabled_rules: config.disabled_rules,
        severity_overrides,
        custom_params: config.custom_params,
    };

    match state.tool_manager.register_linter(internal_config).await {
        Ok(()) => {
            let mut response = HashMap::new();
            response.insert("success".to_string(), "true".to_string());
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to register linter: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
