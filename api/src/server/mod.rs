//! API Server Module
//!
//! This module contains the server setup functionality for the API system.

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tracing::info;

use odincode_agents::AgentCoordinator;
use odincode_core::CodeEngine;
use odincode_ltmc::LTMManager;
use odincode_tools::ToolManager;

use crate::handlers::{
    analyze_file, create_multi_edit_operation, execute_agent, execute_multi_edit_operation,
    execute_tool, get_file, health_check, lint_file, list_agents, list_tools, load_file,
    register_linter, search_patterns, store_pattern, ApiState,
};
use crate::models::ApiConfig;

use std::sync::Arc;

/// Main API server
pub struct ApiServer {
    /// Server configuration
    config: ApiConfig,
    /// Shared state
    state: std::sync::Arc<ApiState>,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(
        config: ApiConfig,
        core_engine: Arc<CodeEngine>,
        ltmc_manager: Arc<LTMManager>,
        agent_coordinator: Arc<AgentCoordinator>,
        tool_manager: Arc<ToolManager>,
    ) -> Self {
        let state = std::sync::Arc::new(ApiState {
            core_engine,
            ltmc_manager,
            agent_coordinator,
            tool_manager,
        });

        Self { config, state }
    }

    /// Start the API server
    pub async fn start(&self) -> Result<()> {
        info!(
            "Starting OdinCode API server on {}:{}",
            self.config.host, self.config.port
        );

        // Build the application with the shared state
        let app = Router::new()
            // File operations
            .route("/api/files", post(load_file))
            .route("/api/files/:id", get(get_file))
            .route("/api/files/:id/analyze", post(analyze_file))
            // Agent operations
            .route("/api/agents", get(list_agents))
            .route("/api/agents/:id/execute", post(execute_agent))
            // LTMC operations
            .route("/api/ltmc/patterns", get(search_patterns))
            .route("/api/ltmc/patterns", post(store_pattern))
            // Tool operations
            .route("/api/tools", get(list_tools))
            .route("/api/tools/:id/execute", post(execute_tool))
            // Multi-edit operations
            .route(
                "/api/multi-edit/operations",
                post(create_multi_edit_operation),
            )
            .route(
                "/api/multi-edit/operations/:id/execute",
                post(execute_multi_edit_operation),
            )
            // Linter operations
            .route("/api/linters", post(register_linter))
            .route("/api/linters/:file_id/lint", post(lint_file))
            // Health check
            .route("/health", get(health_check))
            .with_state(self.state.clone());

        // Bind to the address
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));
        info!("OdinCode API server listening on {}", addr);

        // Run the server using hyper's TCP listener
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start API server: {}", e))?;

        Ok(())
    }
}
