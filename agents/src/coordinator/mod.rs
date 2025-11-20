//! Agents Coordinator Module
//!
//! This module contains the agent coordinator functionality.

use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use crate::ltmc_integration::{AgentExecutionResult, LTMCIntegration, LearningRequest};
use crate::models::{Agent, AgentType};
use odincode_core::{CodeEngine, CodeFile};
use odincode_ltmc::{LTMManager, LearningPattern, PatternType};

pub mod executors;

use crate::coordinator::executors::AgentExecutors;

use std::sync::Arc;

/// Main agent coordinator that manages all agents in the system
#[derive(Clone)]
pub struct AgentCoordinator {
    /// Map of all agents
    pub agents: Arc<RwLock<HashMap<Uuid, Agent>>>,
    /// Reference to the core code engine
    pub core_engine: std::sync::Arc<CodeEngine>,
    /// Reference to the LTMC manager
    pub ltmc_manager: std::sync::Arc<LTMManager>,
    /// LTMC integration for real-time learning
    pub ltmc_integration: std::sync::Arc<LTMCIntegration>,
}

impl AgentCoordinator {
    /// Create a new agent coordinator
    pub fn new(
        core_engine: std::sync::Arc<CodeEngine>,
        ltmc_manager: std::sync::Arc<LTMManager>,
        ltmc_integration: std::sync::Arc<LTMCIntegration>,
    ) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            core_engine,
            ltmc_manager,
            ltmc_integration,
        }
    }

    /// Register a new agent with the coordinator
    pub async fn register_agent(
        &self,
        agent_type: AgentType,
        name: String,
        description: String,
        capabilities: Vec<String>,
        confidence_threshold: f32,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let agent = Agent {
            id,
            agent_type,
            name: name.clone(),
            description,
            created: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            capabilities,
            confidence_threshold,
        };

        let mut agents = self.agents.write().await;
        agents.insert(id, agent);
        drop(agents);

        info!("Registered new agent: {} ({})", name, id);
        Ok(id)
    }

    /// Get an agent by its ID
    pub async fn get_agent(&self, id: Uuid) -> Result<Option<Agent>> {
        let agents = self.agents.read().await;
        Ok(agents.get(&id).cloned())
    }

    /// List all agents of a specific type
    pub async fn list_agents_by_type(&self, agent_type: AgentType) -> Result<Vec<Agent>> {
        let agents = self.agents.read().await;
        let result: Vec<Agent> = agents
            .values()
            .filter(|agent| agent.agent_type == agent_type)
            .cloned()
            .collect();

        Ok(result)
    }

    /// Execute an agent on a specific file
    pub async fn execute_agent_on_file(
        &self,
        agent_id: Uuid,
        file_id: Uuid,
    ) -> Result<Option<Vec<odincode_core::CodeSuggestion>>> {
        let start_time = std::time::Instant::now();

        // Get the agent
        let agent = {
            let agents = self.agents.read().await;
            match agents.get(&agent_id) {
                Some(agent) => agent.clone(),
                None => return Err(anyhow::anyhow!("Agent not found: {}", agent_id)),
            }
        };

        // Update agent's last activity
        {
            let mut agents = self.agents.write().await;
            if let Some(stored_agent) = agents.get_mut(&agent_id) {
                stored_agent.last_activity = chrono::Utc::now();
            }
        }

        debug!(
            "Executing agent {} ({}) on file {}",
            agent.name, agent_id, file_id
        );

        // Get the file from the core engine
        let file = self.core_engine.get_file(file_id).await?;
        if file.is_none() {
            return Err(anyhow::anyhow!("File not found: {}", file_id));
        }
        let file = file.unwrap();

        // Start learning session for this execution
        let learning_session_id = self
            .ltmc_integration
            .start_learning_session(
                agent_id,
                agent.agent_type.clone(),
                format!("Executing {} on file {}", agent.name, file.path),
            )
            .await?;

        // Get learning insights before execution
        let learning_request = LearningRequest {
            agent_id,
            agent_type: agent.agent_type.clone(),
            context: format!("Executing {} on file {}", agent.name, file.path),
            file_id: Some(file_id),
            query: format!("{} analysis for {} file", agent.name, file.language),
            pattern_types: vec![
                PatternType::CodePattern,
                PatternType::UserInteraction,
                PatternType::ErrorSolution,
            ],
        };

        let learning_response = self
            .ltmc_integration
            .process_learning_request(learning_request)
            .await?;

        debug!(
            "Retrieved {} learning patterns for execution",
            learning_response.patterns.len()
        );

        // Execute the appropriate agent logic based on type
        let suggestions = match agent.agent_type {
            AgentType::CodeGenerator => {
                AgentExecutors::execute_code_generation_agent_with_learning(
                    &self.ltmc_integration,
                    &agent,
                    &file,
                    &learning_response,
                )
                .await?
            }
            AgentType::Refactorer => {
                AgentExecutors::execute_refactoring_agent_with_learning(
                    &self.ltmc_integration,
                    &self.core_engine,
                    &agent,
                    &file,
                    &learning_response,
                )
                .await?
            }
            AgentType::BugDetector => {
                AgentExecutors::execute_bug_detection_agent_with_learning(
                    &self.ltmc_integration,
                    &self.core_engine,
                    &agent,
                    &file,
                    &learning_response,
                )
                .await?
            }
            AgentType::Documenter => {
                AgentExecutors::execute_documentation_agent_with_learning(
                    &self.ltmc_integration,
                    &agent,
                    &file,
                    &learning_response,
                )
                .await?
            }
            AgentType::TestGenerator => {
                AgentExecutors::execute_test_generation_agent_with_learning(
                    &self.ltmc_integration,
                    &agent,
                    &file,
                    &learning_response,
                )
                .await?
            }
            AgentType::CodeUnderstanding => {
                AgentExecutors::execute_code_understanding_agent_with_learning(
                    &self.ltmc_integration,
                    &agent,
                    &file,
                    &learning_response,
                )
                .await?
            }
        };

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        // Store the execution result for learning
        let execution_result = AgentExecutionResult {
            description: format!("Executed {} on file {}", agent.name, file.path),
            success: !suggestions.is_empty(),
            suggestions: suggestions.clone(),
            execution_time_ms,
            confidence: learning_response.confidence,
        };

        self.ltmc_integration
            .store_agent_execution(
                agent_id,
                agent.agent_type.clone(),
                Some(file_id),
                &execution_result,
            )
            .await?;

        // Complete the learning session
        self.ltmc_integration
            .complete_learning_session(
                learning_session_id,
                format!("Completed execution with {} suggestions", suggestions.len()),
            )
            .await?;

        // Store the execution in LTMC for backward compatibility
        self.store_agent_execution(&agent, &file, &suggestions)
            .await?;

        Ok(Some(suggestions))
    }

    /// Store agent execution details in LTMC for learning
    async fn store_agent_execution(
        &self,
        agent: &Agent,
        file: &CodeFile,
        suggestions: &[odincode_core::CodeSuggestion],
    ) -> Result<()> {
        let mut context = HashMap::new();
        context.insert("agent_type".to_string(), format!("{:?}", agent.agent_type));
        context.insert("file_path".to_string(), file.path.clone());
        context.insert("language".to_string(), file.language.clone());
        context.insert(
            "suggestion_count".to_string(),
            suggestions.len().to_string(),
        );

        let pattern = LearningPattern {
            id: Uuid::new_v4(),
            pattern_type: PatternType::UserInteraction,
            content: format!(
                "Agent {} executed on file {}, generated {} suggestions",
                agent.name,
                file.path,
                suggestions.len()
            ),
            context,
            created: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 0,
            confidence: agent.confidence_threshold,
        };

        self.ltmc_manager.store_pattern(pattern).await?;
        Ok(())
    }

    /// Get all registered agents
    pub async fn get_all_agents(&self) -> Result<Vec<Agent>> {
        let agents = self.agents.read().await;
        let result: Vec<Agent> = agents.values().cloned().collect();
        Ok(result)
    }
}
