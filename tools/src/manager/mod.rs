//! Tools Manager Module
//!
//! This module contains the tool manager functionality.

use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use crate::tool_models::{ToolIntegration, ToolStatus, ToolType};
use odincode_agents::AgentCoordinator;
use odincode_core::{CodeEngine, CodeFile};
use odincode_ltmc::{LTMManager, LearningPattern, PatternType};

pub mod executors;

use crate::linters::LinterManager;
use crate::manager::executors::ToolExecutors;
use crate::multi_edit::{EditTask, MultiEditManager};
use odincode_core::CodeIssue;

/// Main tool manager that handles all tool integrations
pub struct ToolManager {
    /// Map of all tool integrations
    pub tools: RwLock<HashMap<Uuid, ToolIntegration>>,
    /// Reference to the core code engine
    pub core_engine: CodeEngine,
    /// Reference to the LTMC manager
    pub ltmc_manager: LTMManager,
    /// Reference to the agent coordinator
    pub agent_coordinator: AgentCoordinator,
    /// Multi-edit manager for complex refactoring operations
    pub multi_edit_manager: std::sync::Arc<MultiEditManager>,
    /// Linter manager for code quality checks
    pub linter_manager: std::sync::Arc<LinterManager>,
}

impl ToolManager {
    /// Create a new tool manager
    pub fn new(
        core_engine: CodeEngine,
        ltmc_manager: LTMManager,
        agent_coordinator: AgentCoordinator,
    ) -> Self {
        let multi_edit_manager = std::sync::Arc::new(MultiEditManager::new(std::sync::Arc::new(
            core_engine.clone(),
        )));
        let linter_manager =
            std::sync::Arc::new(LinterManager::new(std::sync::Arc::new(core_engine.clone())));

        Self {
            tools: RwLock::new(HashMap::new()),
            core_engine,
            ltmc_manager,
            agent_coordinator,
            multi_edit_manager,
            linter_manager,
        }
    }

    /// Create a new tool manager from Arc-wrapped components (for API/TUI usage)
    pub fn new_with_arcs(
        core_engine: std::sync::Arc<CodeEngine>,
        ltmc_manager: std::sync::Arc<LTMManager>,
        agent_coordinator: AgentCoordinator,
    ) -> Self {
        let multi_edit_manager =
            std::sync::Arc::new(MultiEditManager::new(std::sync::Arc::clone(&core_engine)));
        let linter_manager =
            std::sync::Arc::new(LinterManager::new(std::sync::Arc::clone(&core_engine)));

        Self {
            tools: RwLock::new(HashMap::new()),
            core_engine: (*core_engine).clone(),
            ltmc_manager: (*ltmc_manager).clone(),
            agent_coordinator,
            multi_edit_manager,
            linter_manager,
        }
    }

    /// Register a new tool integration
    pub async fn register_tool(
        &self,
        name: String,
        description: String,
        tool_type: ToolType,
        config: HashMap<String, String>,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let tool = ToolIntegration {
            id,
            name: name.clone(),
            description,
            tool_type,
            status: ToolStatus::NotConfigured,
            config,
            created: chrono::Utc::now(),
            last_updated: chrono::Utc::now(),
        };

        let mut tools = self.tools.write().await;
        tools.insert(id, tool);
        drop(tools);

        info!("Registered new tool: {} ({})", name, id);
        Ok(id)
    }

    /// Get a tool by its ID
    pub async fn get_tool(&self, id: Uuid) -> Result<Option<ToolIntegration>> {
        let tools = self.tools.read().await;
        Ok(tools.get(&id).cloned())
    }

    /// List all tools of a specific type
    pub async fn list_tools_by_type(&self, tool_type: ToolType) -> Result<Vec<ToolIntegration>> {
        let tools = self.tools.read().await;
        let result: Vec<ToolIntegration> = tools
            .values()
            .filter(|tool| tool.tool_type == tool_type)
            .cloned()
            .collect();

        Ok(result)
    }

    /// Update a tool's status
    pub async fn update_tool_status(&self, id: Uuid, status: ToolStatus) -> Result<bool> {
        let mut tools = self.tools.write().await;
        if let Some(tool) = tools.get_mut(&id) {
            tool.status = status;
            tool.last_updated = chrono::Utc::now();
            drop(tools);

            debug!("Updated tool {} status", id);
            Ok(true)
        } else {
            drop(tools);
            Ok(false)
        }
    }

    /// Execute a tool on a specific file
    pub async fn execute_tool_on_file(&self, tool_id: Uuid, file_id: Uuid) -> Result<bool> {
        // Get the tool
        let tool = {
            let tools = self.tools.read().await;
            match tools.get(&tool_id) {
                Some(tool) => tool.clone(),
                None => return Err(anyhow::anyhow!("Tool not found: {}", tool_id)),
            }
        };

        // Check if the tool is connected
        if matches!(tool.status, ToolStatus::Error | ToolStatus::NotConfigured) {
            return Err(anyhow::anyhow!(
                "Tool {} is not ready for execution (status: {:?})",
                tool.name,
                tool.status
            ));
        }

        // Update tool's last activity
        {
            let mut tools = self.tools.write().await;
            if let Some(stored_tool) = tools.get_mut(&tool_id) {
                stored_tool.last_updated = chrono::Utc::now();
            }
        }

        debug!(
            "Executing tool {} ({}) on file {}",
            tool.name, tool_id, file_id
        );

        // Get the file from the core engine
        let file = self.core_engine.get_file(file_id).await?;
        if file.is_none() {
            return Err(anyhow::anyhow!("File not found: {}", file_id));
        }
        let file = file.unwrap();

        // Execute the appropriate tool logic based on type
        let success = match tool.tool_type {
            ToolType::Linter => {
                ToolExecutors::execute_linter(&self.ltmc_manager, &tool, &file).await?
            }
            ToolType::Formatter => {
                ToolExecutors::execute_formatter(&self.ltmc_manager, &tool, &file).await?
            }
            ToolType::TestingFramework => {
                ToolExecutors::execute_test_runner(&self.ltmc_manager, &tool, &file).await?
            }
            ToolType::BuildSystem => {
                ToolExecutors::execute_build_system(&self.ltmc_manager, &tool, &file).await?
            }
            ToolType::VersionControl => {
                ToolExecutors::execute_version_control(&self.ltmc_manager, &tool, &file).await?
            }
            ToolType::Debugger => {
                ToolExecutors::execute_debugger(&self.ltmc_manager, &tool, &file).await?
            }
            ToolType::PackageManager => {
                ToolExecutors::execute_package_manager(&self.ltmc_manager, &tool, &file).await?
            }
            ToolType::IDE => {
                ToolExecutors::execute_ide_integration(&self.ltmc_manager, &tool, &file).await?
            }
        };

        // Store the execution in LTMC for learning
        self.store_tool_execution(&tool, &file, success).await?;

        Ok(success)
    }

    /// Store tool execution details in LTMC for learning
    async fn store_tool_execution(
        &self,
        tool: &ToolIntegration,
        file: &CodeFile,
        success: bool,
    ) -> Result<()> {
        let mut context = HashMap::new();
        context.insert("tool_type".to_string(), format!("{:?}", tool.tool_type));
        context.insert("file_path".to_string(), file.path.clone());
        context.insert("language".to_string(), file.language.clone());
        context.insert("success".to_string(), success.to_string());

        let pattern = LearningPattern {
            id: Uuid::new_v4(),
            pattern_type: PatternType::UserInteraction,
            content: format!(
                "Tool {} executed on file {}, success: {}",
                tool.name, file.path, success
            ),
            context,
            created: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 0,
            confidence: 0.8,
        };

        self.ltmc_manager.store_pattern(pattern).await?;
        Ok(())
    }

    /// Get all registered tools
    pub async fn get_all_tools(&self) -> Result<Vec<ToolIntegration>> {
        let tools = self.tools.read().await;
        let result: Vec<ToolIntegration> = tools.values().cloned().collect();
        Ok(result)
    }

    /// Create a multi-edit operation
    pub async fn create_multi_edit_operation(
        &self,
        name: String,
        description: String,
        tasks: Vec<EditTask>,
    ) -> Result<Uuid> {
        self.multi_edit_manager
            .create_operation(name, description, tasks)
            .await
    }

    /// Execute a multi-edit operation
    pub async fn execute_multi_edit_operation(&self, operation_id: Uuid) -> Result<bool> {
        self.multi_edit_manager
            .execute_operation(operation_id)
            .await
    }

    /// Lint a file using the appropriate linter
    pub async fn lint_file(&self, file_id: Uuid) -> Result<Vec<CodeIssue>> {
        self.linter_manager.lint_file(file_id).await
    }

    /// Register a new linter configuration
    pub async fn register_linter(&self, config: crate::linters::LinterConfig) -> Result<()> {
        self.linter_manager.register_linter(config).await
    }

    /// Get the multi-edit manager
    pub fn get_multi_edit_manager(&self) -> &std::sync::Arc<MultiEditManager> {
        &self.multi_edit_manager
    }

    /// Get the linter manager
    pub fn get_linter_manager(&self) -> &std::sync::Arc<LinterManager> {
        &self.linter_manager
    }
}
