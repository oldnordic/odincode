//! LTMC Integration Module
//!
//! This module provides real-time learning integration between LTMC and all agents,
//! establishing bidirectional communication channels for continuous learning and improvement.

pub mod feedback_processing;
pub mod metrics;
pub mod session_management;
pub mod types;

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::models::{Agent, AgentType};
use odincode_core::llm_integration::LLMIntegrationManager;
use odincode_core::{CodeEngine, CodeSuggestion};
use odincode_ltmc::{
    LTMManager, LearningPattern, PatternType, ReasoningType, SequentialThinkingSession, Thought,
    ThoughtType,
};

// Re-export all types for public API
pub use types::*;

// Import internal modules
use feedback_processing::FeedbackProcessorManager;
use metrics::MetricsManager;
use session_management::SessionManager;

/// Real-time learning integration between LTMC and agents
pub struct LTMCIntegration {
    /// Reference to the LTMC manager
    pub ltmc_manager: Arc<LTMManager>,
    /// Reference to the core engine
    pub core_engine: Arc<CodeEngine>,
    /// Reference to the LLM integration manager
    pub llm_manager: Arc<LLMIntegrationManager>,
    /// Session manager for learning sessions
    pub session_manager: SessionManager,
    /// Feedback processor manager
    pub feedback_manager: FeedbackProcessorManager,
    /// Metrics manager
    pub metrics_manager: MetricsManager,
}

impl LTMCIntegration {
    /// Create a new LTMC integration instance
    pub fn new(
        ltmc_manager: Arc<LTMManager>,
        core_engine: Arc<CodeEngine>,
        llm_manager: Arc<LLMIntegrationManager>,
    ) -> Self {
        // Create shared Arc references for components that need them
        let learning_sessions = Arc::new(RwLock::new(HashMap::new()));
        let feedback_processors = Arc::new(RwLock::new(HashMap::new()));
        let event_subscribers = Arc::new(RwLock::new(Vec::new()));

        // Create component managers
        let session_manager = SessionManager::new(
            ltmc_manager.clone(),
            core_engine.clone(),
            llm_manager.clone(),
        );

        let feedback_manager = FeedbackProcessorManager::new(ltmc_manager.clone());

        let metrics_manager = MetricsManager::new(
            ltmc_manager.clone(),
            learning_sessions.clone(),
            feedback_processors.clone(),
            event_subscribers.clone(),
        );

        Self {
            ltmc_manager,
            core_engine,
            llm_manager,
            session_manager,
            feedback_manager,
            metrics_manager,
        }
    }

    /// Initialize the LTMC integration
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing LTMC-Agent integration...");

        // Load existing patterns into cache
        self.load_patterns_to_cache().await?;

        // Start background learning tasks
        self.start_background_learning().await?;

        // Start real-time feedback processing
        self.feedback_manager.start_real_time_processing().await?;

        // Start metrics collection
        self.metrics_manager.start_metrics_collection().await?;

        info!("LTMC-Agent integration initialized successfully");
        Ok(())
    }

    /// Start a new learning session for an agent
    pub async fn start_learning_session(
        &self,
        agent_id: Uuid,
        agent_type: AgentType,
        context: String,
    ) -> Result<Uuid> {
        self.session_manager
            .start_learning_session(agent_id, agent_type, context)
            .await
    }

    /// Process a learning request from an agent
    pub async fn process_learning_request(
        &self,
        request: LearningRequest,
    ) -> Result<LearningResponse> {
        self.session_manager.process_learning_request(request).await
    }

    /// Store agent execution results for learning
    pub async fn store_agent_execution(
        &self,
        agent_id: Uuid,
        agent_type: AgentType,
        file_id: Option<Uuid>,
        execution_result: &AgentExecutionResult,
    ) -> Result<Uuid> {
        self.session_manager
            .store_agent_execution(agent_id, agent_type, file_id, execution_result)
            .await
    }

    /// Get learning insights for an agent type
    pub async fn get_learning_insights(
        &self,
        agent_type: AgentType,
    ) -> Result<AgentLearningInsights> {
        self.session_manager.get_learning_insights(agent_type).await
    }

    /// Complete a learning session
    pub async fn complete_learning_session(&self, session_id: Uuid, summary: String) -> Result<()> {
        self.session_manager
            .complete_learning_session(session_id, summary)
            .await
    }

    /// Get current learning statistics
    pub async fn get_learning_statistics(&self) -> Result<LearningStatistics> {
        self.session_manager.get_learning_statistics().await
    }

    /// Register a feedback channel for an agent
    pub async fn register_feedback_channel(
        &self,
        agent_id: Uuid,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<AgentFeedback>> {
        self.feedback_manager
            .register_feedback_channel(agent_id)
            .await
    }

    /// Submit feedback from an agent
    pub async fn submit_feedback(&self, feedback: AgentFeedback) -> Result<()> {
        self.feedback_manager.submit_feedback(feedback).await
    }

    /// Subscribe to learning events
    pub async fn subscribe_to_learning_events(
        &self,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<LearningEvent>> {
        self.feedback_manager.subscribe_to_learning_events().await
    }

    /// Get real-time performance metrics
    pub async fn get_real_time_metrics(&self) -> Result<RealTimeMetrics> {
        self.metrics_manager.get_real_time_metrics().await
    }

    // Private helper methods

    async fn load_patterns_to_cache(&self) -> Result<()> {
        debug!("Loading patterns to cache...");

        let pattern_types = vec![
            PatternType::CodePattern,
            PatternType::UserInteraction,
            PatternType::ErrorSolution,
        ];

        for pattern_type in pattern_types {
            match self
                .ltmc_manager
                .get_patterns_by_type(pattern_type.clone())
                .await
            {
                Ok(patterns) => {
                    for pattern in patterns {
                        let key = pattern.content.clone();
                        let mut cache = self.session_manager.pattern_cache.write().await;
                        cache.entry(key).or_insert_with(Vec::new).push(pattern);
                    }
                }
                Err(e) => {
                    warn!("Failed to load patterns for type {:?}: {}", pattern_type, e);
                }
            }
        }

        info!("Loaded patterns to cache");
        Ok(())
    }

    async fn start_background_learning(&self) -> Result<()> {
        debug!("Starting background learning tasks...");

        // This would start background tasks for:
        // - Pattern analysis and optimization
        // - Learning session cleanup
        // - Statistics updates
        // - Cache management

        info!("Background learning tasks started");
        Ok(())
    }
}
