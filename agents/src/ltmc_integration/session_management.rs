//! LTMC Session Management
//!
//! This module handles learning session operations including session creation,
//! processing learning requests, and managing agent learning insights.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::models::AgentType;
use odincode_core::llm_integration::LLMIntegrationManager;
use odincode_core::CodeEngine;
use odincode_ltmc::{LTMManager, LearningPattern, PatternType, ReasoningType, ThoughtType};

use super::types::{
    AgentExecutionResult, AgentLearningInsights, AgentLearningSession, LearningRequest,
    LearningResponse, LearningSessionStatus, LearningStatistics,
};

/// Session management functionality for LTMC integration
pub struct SessionManager {
    /// Reference to the LTMC manager
    pub ltmc_manager: Arc<LTMManager>,
    /// Reference to the core engine
    pub core_engine: Arc<CodeEngine>,
    /// Reference to the LLM integration manager
    pub llm_manager: Arc<LLMIntegrationManager>,
    /// Active learning sessions
    pub learning_sessions: Arc<RwLock<HashMap<Uuid, AgentLearningSession>>>,
    /// Pattern matching cache
    pub pattern_cache: Arc<RwLock<HashMap<String, Vec<LearningPattern>>>>,
    /// Learning statistics
    pub learning_stats: Arc<RwLock<LearningStatistics>>,
}

impl SessionManager {
    /// Create a new session manager instance
    pub fn new(
        ltmc_manager: Arc<LTMManager>,
        core_engine: Arc<CodeEngine>,
        llm_manager: Arc<LLMIntegrationManager>,
    ) -> Self {
        Self {
            ltmc_manager,
            core_engine,
            llm_manager,
            learning_sessions: Arc::new(RwLock::new(HashMap::new())),
            pattern_cache: Arc::new(RwLock::new(HashMap::new())),
            learning_stats: Arc::new(RwLock::new(LearningStatistics::default())),
        }
    }

    /// Start a new learning session for an agent
    pub async fn start_learning_session(
        &self,
        agent_id: Uuid,
        agent_type: AgentType,
        context: String,
    ) -> Result<Uuid> {
        let session_id = Uuid::new_v4();
        let session = AgentLearningSession {
            session_id,
            agent_id,
            agent_type: agent_type.clone(),
            context: context.clone(),
            patterns_discovered: Vec::new(),
            started_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            status: LearningSessionStatus::Active,
        };

        // Store session
        let mut sessions = self.learning_sessions.write().await;
        sessions.insert(session_id, session);
        drop(sessions);

        // Start sequential thinking session in LTMC
        let ltmc_session_id = self
            .ltmc_manager
            .start_sequential_thinking_session(
                format!("Agent learning session: {:?} - {}", agent_type, context),
                ReasoningType::ProblemSolving,
            )
            .await?;

        // Add initial thought
        let mut metadata = HashMap::new();
        metadata.insert("agent_id".to_string(), agent_id.to_string());
        metadata.insert("agent_type".to_string(), format!("{:?}", agent_type));
        metadata.insert("integration_session_id".to_string(), session_id.to_string());

        self.ltmc_manager
            .add_thought_to_session(
                ltmc_session_id,
                format!(
                    "Started learning session for {:?} agent with context: {}",
                    agent_type, context
                ),
                ThoughtType::Initial,
                metadata,
            )
            .await?;

        // Update statistics
        let mut stats = self.learning_stats.write().await;
        stats.total_sessions += 1;
        drop(stats);

        info!(
            "Started learning session {} for agent {:?} ({})",
            session_id, agent_type, agent_id
        );
        Ok(session_id)
    }

    /// Process a learning request from an agent
    pub async fn process_learning_request(
        &self,
        request: LearningRequest,
    ) -> Result<LearningResponse> {
        debug!(
            "Processing learning request from agent {:?}: {}",
            request.agent_type, request.query
        );

        let request_id = Uuid::new_v4();
        let mut patterns = Vec::new();
        let mut suggestions = Vec::new();
        let mut total_confidence = 0.0;
        let mut pattern_count = 0;

        // Search for relevant patterns in LTMC
        for pattern_type in &request.pattern_types {
            match self
                .ltmc_manager
                .search_patterns(Some(pattern_type.clone()), &request.query)
                .await
            {
                Ok(found_patterns) => {
                    for pattern in found_patterns {
                        patterns.push(pattern.clone());
                        total_confidence += pattern.confidence;
                        pattern_count += 1;
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to search patterns for type {:?}: {}",
                        pattern_type, e
                    );
                }
            }
        }

        // Search in cache
        if let Some(cached_patterns) = self.pattern_cache.read().await.get(&request.query) {
            patterns.extend(cached_patterns.clone());
        }

        // Generate learning suggestions using LLM
        if !patterns.is_empty() {
            let avg_confidence = total_confidence / pattern_count as f32;

            let prompt = format!(
                "Based on the following learning patterns and context, generate learning suggestions for a {:?} agent:\n\nContext: {}\nQuery: {}\n\nPatterns:\n{}",
                request.agent_type,
                request.context,
                request.query,
                patterns.iter()
                    .map(|p| format!("- {}: {} (confidence: {:.2})", p.pattern_type, p.content, p.confidence))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            match self.llm_manager.generate_response(&prompt).await {
                Ok(response) => {
                    suggestions = response
                        .split('\n')
                        .filter(|s| !s.trim().is_empty())
                        .map(|s| s.trim().to_string())
                        .collect();
                }
                Err(e) => {
                    warn!("Failed to generate learning suggestions: {}", e);
                }
            }

            // Update pattern cache
            let mut cache = self.pattern_cache.write().await;
            cache.insert(request.query.clone(), patterns.clone());
            drop(cache);
        }

        // Create or update learning session
        let session_id = self.get_or_create_session(&request).await?;

        let response = LearningResponse {
            request_id,
            patterns,
            suggestions,
            confidence: if pattern_count > 0 {
                total_confidence / pattern_count as f32
            } else {
                0.0
            },
            session_id: Some(session_id),
        };

        debug!(
            "Processed learning request with {} patterns and {} suggestions",
            response.patterns.len(),
            response.suggestions.len()
        );
        Ok(response)
    }

    /// Store agent execution results for learning
    pub async fn store_agent_execution(
        &self,
        agent_id: Uuid,
        agent_type: AgentType,
        file_id: Option<Uuid>,
        execution_result: &AgentExecutionResult,
    ) -> Result<Uuid> {
        debug!(
            "Storing execution result for agent {:?} ({})",
            agent_type, agent_id
        );

        let mut context = HashMap::new();
        context.insert("agent_id".to_string(), agent_id.to_string());
        context.insert("agent_type".to_string(), format!("{:?}", agent_type));
        context.insert(
            "execution_success".to_string(),
            execution_result.success.to_string(),
        );
        context.insert(
            "suggestions_generated".to_string(),
            execution_result.suggestions.len().to_string(),
        );
        context.insert(
            "execution_time_ms".to_string(),
            execution_result.execution_time_ms.to_string(),
        );

        if let Some(file_id) = file_id {
            context.insert("file_id".to_string(), file_id.to_string());
        }

        let pattern = LearningPattern {
            id: Uuid::new_v4(),
            pattern_type: PatternType::UserInteraction,
            content: format!(
                "Agent {:?} execution: {} - Success: {}, Suggestions: {}, Time: {}ms",
                agent_type,
                execution_result.description,
                execution_result.success,
                execution_result.suggestions.len(),
                execution_result.execution_time_ms
            ),
            context,
            created: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 0,
            confidence: execution_result.confidence,
        };

        let pattern_id = self.ltmc_manager.store_pattern(pattern).await?;

        // Update learning session if active
        self.update_learning_session(agent_id, &pattern_id).await?;

        // Update statistics
        let mut stats = self.learning_stats.write().await;
        stats.total_agent_executions += 1;
        stats.total_patterns_learned += 1;
        stats.average_confidence = (stats.average_confidence
            * (stats.total_patterns_learned - 1) as f32
            + execution_result.confidence)
            / stats.total_patterns_learned as f32;
        drop(stats);

        info!(
            "Stored execution result for agent {:?} with pattern {}",
            agent_type, pattern_id
        );
        Ok(pattern_id)
    }

    /// Get learning insights for an agent type
    pub async fn get_learning_insights(
        &self,
        agent_type: AgentType,
    ) -> Result<AgentLearningInsights> {
        debug!("Getting learning insights for agent type: {:?}", agent_type);

        // Get patterns for this agent type
        let patterns = self
            .ltmc_manager
            .get_patterns_by_type(PatternType::UserInteraction)
            .await?;

        let mut agent_patterns = Vec::new();
        let mut total_confidence = 0.0;
        let mut successful_executions = 0;
        let mut total_executions = 0;

        for pattern in patterns {
            if let Some(agent_type_str) = pattern.context.get("agent_type") {
                if agent_type_str == &format!("{:?}", agent_type) {
                    agent_patterns.push(pattern.clone());
                    total_confidence += pattern.confidence;
                    total_executions += 1;

                    if let Some(success_str) = pattern.context.get("execution_success") {
                        if success_str == "true" {
                            successful_executions += 1;
                        }
                    }
                }
            }
        }

        let success_rate = if total_executions > 0 {
            successful_executions as f32 / total_executions as f32
        } else {
            0.0
        };

        let average_confidence = if !agent_patterns.is_empty() {
            total_confidence / agent_patterns.len() as f32
        } else {
            0.0
        };

        // Generate insights using LLM
        let insights = if !agent_patterns.is_empty() {
            let prompt = format!(
                "Analyze the following learning patterns for a {:?} agent and provide insights for improvement:\n\nSuccess Rate: {:.2}%\nAverage Confidence: {:.2}\nTotal Executions: {}\n\nRecent Patterns:\n{}",
                agent_type,
                success_rate * 100.0,
                average_confidence,
                total_executions,
                agent_patterns.iter()
                    .take(10) // Limit to recent patterns
                    .map(|p| format!("- {} (confidence: {:.2})", p.content, p.confidence))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            match self.llm_manager.generate_response(&prompt).await {
                Ok(response) => response,
                Err(e) => {
                    warn!("Failed to generate insights: {}", e);
                    "Unable to generate insights at this time.".to_string()
                }
            }
        } else {
            "No learning patterns available for this agent type.".to_string()
        };

        Ok(AgentLearningInsights {
            agent_type,
            total_patterns: agent_patterns.len(),
            success_rate,
            average_confidence,
            insights,
            recent_patterns: agent_patterns.into_iter().take(5).collect(),
        })
    }

    /// Complete a learning session
    pub async fn complete_learning_session(&self, session_id: Uuid, summary: String) -> Result<()> {
        debug!("Completing learning session: {}", session_id);

        let mut sessions = self.learning_sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.status = LearningSessionStatus::Completed;
            session.last_activity = chrono::Utc::now();

            // Store completion in LTMC
            let pattern = LearningPattern {
                id: Uuid::new_v4(),
                pattern_type: PatternType::SequentialThinking,
                content: format!(
                    "Completed learning session for {:?} agent: {}",
                    session.agent_type, summary
                ),
                context: {
                    let mut context = HashMap::new();
                    context.insert("session_id".to_string(), session_id.to_string());
                    context.insert("agent_id".to_string(), session.agent_id.to_string());
                    context.insert(
                        "agent_type".to_string(),
                        format!("{:?}", session.agent_type),
                    );
                    context.insert(
                        "patterns_discovered".to_string(),
                        session.patterns_discovered.len().to_string(),
                    );
                    context.insert(
                        "session_duration_ms".to_string(),
                        ((chrono::Utc::now() - session.started_at).num_milliseconds()).to_string(),
                    );
                    context
                },
                created: chrono::Utc::now(),
                last_accessed: chrono::Utc::now(),
                access_count: 0,
                confidence: 0.9,
            };

            drop(sessions); // Drop lock before storing pattern

            self.ltmc_manager.store_pattern(pattern).await?;
            info!("Completed learning session: {}", session_id);
        } else {
            warn!("Learning session not found: {}", session_id);
        }

        Ok(())
    }

    /// Get current learning statistics
    pub async fn get_learning_statistics(&self) -> Result<LearningStatistics> {
        let stats = self.learning_stats.read().await;
        Ok(stats.clone())
    }

    // Private helper methods

    async fn get_or_create_session(&self, request: &LearningRequest) -> Result<Uuid> {
        // Look for existing active session for this agent
        let sessions = self.learning_sessions.read().await;
        for (session_id, session) in sessions.iter() {
            if session.agent_id == request.agent_id
                && session.status == LearningSessionStatus::Active
                && session.context.contains(&request.context)
            {
                return Ok(*session_id);
            }
        }
        drop(sessions);

        // Create new session
        self.start_learning_session(
            request.agent_id,
            request.agent_type.clone(),
            request.context.clone(),
        )
        .await
    }

    async fn update_learning_session(&self, agent_id: Uuid, pattern_id: &Uuid) -> Result<()> {
        let mut sessions = self.learning_sessions.write().await;
        for session in sessions.values_mut() {
            if session.agent_id == agent_id && session.status == LearningSessionStatus::Active {
                session.last_activity = chrono::Utc::now();

                // Add pattern to session
                if let Ok(pattern) = self.ltmc_manager.get_pattern(*pattern_id).await {
                    if let Some(pattern) = pattern {
                        session.patterns_discovered.push(pattern);
                    }
                }
                break;
            }
        }

        Ok(())
    }
}
