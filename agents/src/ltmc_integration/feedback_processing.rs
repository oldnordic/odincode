//! LTMC Feedback Processing
//!
//! This module handles agent feedback processing, event broadcasting, and real-time feedback channels.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::models::AgentType;
use odincode_ltmc::LTMManager;

use super::types::{
    AgentFeedback, EventPriority, FeedbackMetrics, FeedbackProcessor, FeedbackProcessorConfig,
    FeedbackType, LearningEvent, LearningEventData, LearningEventType, ProcessorStats,
    ProcessorType, RealTimeMetrics,
};

/// Feedback processing functionality for LTMC integration
pub struct FeedbackProcessorManager {
    /// Reference to the LTMC manager
    pub ltmc_manager: Arc<LTMManager>,
    /// Real-time feedback channels
    pub feedback_channels:
        Arc<RwLock<HashMap<Uuid, tokio::sync::mpsc::UnboundedSender<AgentFeedback>>>>,
    /// Feedback processors
    pub feedback_processors: Arc<RwLock<HashMap<Uuid, FeedbackProcessor>>>,
    /// Learning event subscribers
    pub event_subscribers: Arc<RwLock<Vec<tokio::sync::mpsc::UnboundedSender<LearningEvent>>>>,
    /// Real-time performance metrics
    pub performance_metrics: Arc<RwLock<RealTimeMetrics>>,
}

impl FeedbackProcessorManager {
    /// Create a new feedback processor manager
    pub fn new(ltmc_manager: Arc<LTMManager>) -> Self {
        Self {
            ltmc_manager,
            feedback_channels: Arc::new(RwLock::new(HashMap::new())),
            feedback_processors: Arc::new(RwLock::new(HashMap::new())),
            event_subscribers: Arc::new(RwLock::new(Vec::new())),
            performance_metrics: Arc::new(RwLock::new(RealTimeMetrics::default())),
        }
    }

    /// Register a feedback channel for an agent
    pub async fn register_feedback_channel(
        &self,
        agent_id: Uuid,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<AgentFeedback>> {
        debug!("Registering feedback channel for agent: {}", agent_id);

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut channels = self.feedback_channels.write().await;
        channels.insert(agent_id, sender);
        drop(channels);

        // Create feedback processor for this agent
        let processor = FeedbackProcessor {
            processor_id: Uuid::new_v4(),
            agent_id,
            processor_type: ProcessorType::RealTime,
            config: FeedbackProcessorConfig {
                processing_interval_ms: 1000, // 1 second
                batch_size: 10,
                confidence_threshold: 0.7,
                learning_rate: 0.1,
                max_feedback_history: 100,
            },
            stats: ProcessorStats::default(),
            last_processed: None,
        };

        let mut processors = self.feedback_processors.write().await;
        processors.insert(agent_id, processor);
        drop(processors);

        debug!("Registered feedback channel for agent: {}", agent_id);
        Ok(receiver)
    }

    /// Submit feedback from an agent
    pub async fn submit_feedback(&self, feedback: AgentFeedback) -> Result<()> {
        debug!(
            "Submitting feedback from agent {:?}: {:?}",
            feedback.agent_type, feedback.feedback_type
        );

        // Send to feedback channel if exists
        let channels = self.feedback_channels.read().await;
        if let Some(sender) = channels.get(&feedback.agent_id) {
            if let Err(e) = sender.send(feedback.clone()) {
                warn!("Failed to send feedback to channel: {}", e);
            }
        }
        drop(channels);

        // Process feedback immediately
        self.process_feedback(feedback).await?;

        Ok(())
    }

    /// Subscribe to learning events
    pub async fn subscribe_to_learning_events(
        &self,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<LearningEvent>> {
        debug!("Registering learning event subscriber");

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut subscribers = self.event_subscribers.write().await;
        subscribers.push(sender);
        drop(subscribers);

        debug!("Registered learning event subscriber");
        Ok(receiver)
    }

    /// Get real-time performance metrics
    pub async fn get_real_time_metrics(&self) -> Result<RealTimeMetrics> {
        let metrics = self.performance_metrics.read().await;
        Ok(metrics.clone())
    }

    /// Start real-time feedback processing
    pub async fn start_real_time_processing(&self) -> Result<()> {
        debug!("Starting real-time feedback processing...");

        // Clone the Arc fields for background tasks
        let feedback_processors = self.feedback_processors.clone();
        let event_subscribers = self.event_subscribers.clone();
        let performance_metrics = self.performance_metrics.clone();
        let ltmc_manager = self.ltmc_manager.clone();

        // Start background task for processing feedback
        tokio::spawn(async move {
            if let Err(e) = Self::process_feedback_loop_static(
                feedback_processors,
                event_subscribers,
                performance_metrics,
                ltmc_manager,
            )
            .await
            {
                error!("Error in feedback processing loop: {}", e);
            }
        });

        // Clone the Arc fields for the second task
        let performance_metrics = self.performance_metrics.clone();
        let event_subscribers = self.event_subscribers.clone();

        // Start background task for updating metrics
        tokio::spawn(async move {
            if let Err(e) =
                Self::update_metrics_loop_static(performance_metrics, event_subscribers).await
            {
                error!("Error in metrics update loop: {}", e);
            }
        });

        debug!("Real-time feedback processing started");
        Ok(())
    }

    // Private helper methods

    /// Process feedback and generate learning events
    async fn process_feedback(&self, feedback: AgentFeedback) -> Result<()> {
        debug!("Processing feedback: {:?}", feedback.feedback_type);

        // Update performance metrics
        self.update_performance_metrics(&feedback).await?;

        // Generate learning event
        let event = LearningEvent {
            event_id: Uuid::new_v4(),
            event_type: LearningEventType::FeedbackReceived,
            agent_id: Some(feedback.agent_id),
            agent_type: Some(feedback.agent_type.clone()),
            event_data: LearningEventData::Feedback {
                feedback_id: feedback.feedback_id,
                feedback_type: feedback.feedback_type.clone(),
                metrics: feedback.performance_metrics.clone(),
            },
            timestamp: chrono::Utc::now(),
            priority: self.determine_event_priority(&feedback),
        };

        // Broadcast event to subscribers
        self.broadcast_learning_event(event).await?;

        // Update feedback processor
        self.update_feedback_processor(&feedback).await?;

        // Generate learning patterns from feedback
        self.generate_patterns_from_feedback(&feedback).await?;

        Ok(())
    }

    /// Broadcast learning event to all subscribers
    async fn broadcast_learning_event(&self, event: LearningEvent) -> Result<()> {
        debug!("Broadcasting learning event: {:?}", event.event_type);

        let mut subscribers = self.event_subscribers.write().await;
        let mut dead_subscribers = Vec::new();

        for (i, sender) in subscribers.iter().enumerate() {
            if let Err(e) = sender.send(event.clone()) {
                warn!("Failed to send event to subscriber {}: {}", i, e);
                dead_subscribers.push(i);
            }
        }

        // Remove dead subscribers
        for &i in dead_subscribers.iter().rev() {
            subscribers.remove(i);
        }
        drop(subscribers);

        Ok(())
    }

    /// Update performance metrics based on feedback
    async fn update_performance_metrics(&self, feedback: &AgentFeedback) -> Result<()> {
        let mut metrics = self.performance_metrics.write().await;

        // Update basic metrics
        metrics.total_feedback_processed += 1;
        metrics.last_updated = chrono::Utc::now();

        // Update system load based on execution time
        let load_factor = feedback.performance_metrics.execution_time_ms as f32 / 1000.0;
        metrics.system_load = (metrics.system_load * 0.9 + load_factor * 0.1).min(1.0);

        // Update learning efficiency based on success rate and confidence
        let efficiency = (feedback.performance_metrics.success_rate
            + feedback.performance_metrics.confidence_score)
            / 2.0;
        metrics.learning_efficiency =
            (metrics.learning_efficiency * 0.8 + efficiency * 0.2).min(1.0);

        // Update error rate
        let error_rate = if feedback.performance_metrics.errors_encountered > 0 {
            feedback.performance_metrics.errors_encountered as f32
                / (feedback.performance_metrics.suggestions_generated
                    + feedback.performance_metrics.errors_encountered) as f32
        } else {
            0.0
        };
        metrics.error_rate = (metrics.error_rate * 0.9 + error_rate * 0.1).min(1.0);

        drop(metrics);
        Ok(())
    }

    /// Determine event priority based on feedback
    fn determine_event_priority(&self, feedback: &AgentFeedback) -> EventPriority {
        match feedback.feedback_type {
            FeedbackType::ExecutionFailure => EventPriority::High,
            FeedbackType::Error => EventPriority::Critical,
            FeedbackType::Performance => {
                if feedback.performance_metrics.execution_time_ms > 5000 {
                    EventPriority::High
                } else if feedback.performance_metrics.execution_time_ms > 1000 {
                    EventPriority::Medium
                } else {
                    EventPriority::Low
                }
            }
            FeedbackType::Learning => EventPriority::Medium,
            FeedbackType::ExecutionSuccess => EventPriority::Low,
        }
    }

    /// Update feedback processor statistics
    async fn update_feedback_processor(&self, feedback: &AgentFeedback) -> Result<()> {
        let mut processors = self.feedback_processors.write().await;
        if let Some(processor) = processors.get_mut(&feedback.agent_id) {
            processor.stats.total_processed += 1;
            processor.last_processed = Some(chrono::Utc::now());

            // Update average processing time (simulated)
            let processing_time = feedback.performance_metrics.execution_time_ms as f32;
            processor.stats.avg_processing_time_ms =
                processor.stats.avg_processing_time_ms * 0.9 + processing_time * 0.1;

            // Update success rate
            let success = matches!(feedback.feedback_type, FeedbackType::ExecutionSuccess);
            let current_rate = processor.stats.success_rate;
            processor.stats.success_rate = current_rate * 0.9 + if success { 0.1 } else { 0.0 };
        }
        drop(processors);
        Ok(())
    }

    /// Generate learning patterns from feedback
    async fn generate_patterns_from_feedback(&self, feedback: &AgentFeedback) -> Result<()> {
        let pattern_content = match feedback.feedback_type {
            FeedbackType::ExecutionSuccess => {
                format!(
                    "Agent {:?} executed successfully in {}ms with confidence {:.2}",
                    feedback.agent_type,
                    feedback.performance_metrics.execution_time_ms,
                    feedback.performance_metrics.confidence_score
                )
            }
            FeedbackType::ExecutionFailure => {
                format!(
                    "Agent {:?} execution failed after {}ms with {} errors",
                    feedback.agent_type,
                    feedback.performance_metrics.execution_time_ms,
                    feedback.performance_metrics.errors_encountered
                )
            }
            FeedbackType::Performance => {
                format!(
                    "Agent {:?} performance: {}ms execution, {:.1}% CPU, {:.1}% memory",
                    feedback.agent_type,
                    feedback.performance_metrics.execution_time_ms,
                    feedback.performance_metrics.cpu_usage_percent,
                    feedback.performance_metrics.memory_usage_bytes as f32 / 1024.0 / 1024.0
                )
            }
            FeedbackType::Learning => {
                format!(
                    "Agent {:?} learning update: {:.2} confidence, {} suggestions generated",
                    feedback.agent_type,
                    feedback.performance_metrics.confidence_score,
                    feedback.performance_metrics.suggestions_generated
                )
            }
            FeedbackType::Error => {
                format!(
                    "Agent {:?} encountered error: {}",
                    feedback.agent_type, feedback.content
                )
            }
        };

        let mut context = HashMap::new();
        context.insert("agent_id".to_string(), feedback.agent_id.to_string());
        context.insert(
            "agent_type".to_string(),
            format!("{:?}", feedback.agent_type),
        );
        context.insert(
            "feedback_type".to_string(),
            format!("{:?}", feedback.feedback_type),
        );
        context.insert("feedback_id".to_string(), feedback.feedback_id.to_string());

        // Add performance metrics to context
        context.insert(
            "execution_time_ms".to_string(),
            feedback.performance_metrics.execution_time_ms.to_string(),
        );
        context.insert(
            "success_rate".to_string(),
            feedback.performance_metrics.success_rate.to_string(),
        );
        context.insert(
            "confidence_score".to_string(),
            feedback.performance_metrics.confidence_score.to_string(),
        );

        let pattern = odincode_ltmc::LearningPattern {
            id: Uuid::new_v4(),
            pattern_type: odincode_ltmc::PatternType::UserInteraction,
            content: pattern_content,
            context,
            created: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 0,
            confidence: feedback.performance_metrics.confidence_score,
        };

        let _pattern_id = self.ltmc_manager.store_pattern(pattern).await?;

        Ok(())
    }

    /// Background loop for processing feedback
    async fn process_feedback_loop(&self) -> Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1000));

        loop {
            interval.tick().await;

            // Process any pending feedback from processors
            let processors = self.feedback_processors.read().await;
            for (agent_id, processor) in processors.iter() {
                if processor.processor_type == ProcessorType::Batch {
                    // Process batch feedback
                    if let Err(e) = self.process_batch_feedback(*agent_id).await {
                        error!(
                            "Error processing batch feedback for agent {}: {}",
                            agent_id, e
                        );
                    }
                }
            }
            drop(processors);
        }
    }

    /// Background loop for updating metrics
    async fn update_metrics_loop(&self) -> Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(5000));

        loop {
            interval.tick().await;

            // Check if we need to generate performance alert
            let metrics = self.performance_metrics.read().await;
            let should_alert = metrics.system_load > 0.8;
            let system_load = metrics.system_load;
            drop(metrics);

            // Generate performance alert if needed
            if should_alert {
                let event = LearningEvent {
                    event_id: Uuid::new_v4(),
                    event_type: LearningEventType::PerformanceAlert,
                    agent_id: None,
                    agent_type: None,
                    event_data: LearningEventData::Performance {
                        metric_name: "system_load".to_string(),
                        metric_value: system_load,
                        threshold_exceeded: true,
                    },
                    timestamp: chrono::Utc::now(),
                    priority: EventPriority::High,
                };

                if let Err(e) = self.broadcast_learning_event(event).await {
                    error!("Error broadcasting performance alert: {}", e);
                }
            }
        }
    }

    /// Process batch feedback for an agent
    async fn process_batch_feedback(&self, agent_id: Uuid) -> Result<()> {
        // This would process accumulated feedback in batches
        // For now, we'll simulate batch processing
        debug!("Processing batch feedback for agent: {}", agent_id);

        // Update processor stats
        let mut processors = self.feedback_processors.write().await;
        if let Some(processor) = processors.get_mut(&agent_id) {
            processor.stats.total_processed += 1;
            processor.last_processed = Some(chrono::Utc::now());
        }
        drop(processors);

        Ok(())
    }

    /// Static version of feedback processing loop for background tasks
    async fn process_feedback_loop_static(
        feedback_processors: Arc<RwLock<HashMap<Uuid, FeedbackProcessor>>>,
        _event_subscribers: Arc<RwLock<Vec<tokio::sync::mpsc::UnboundedSender<LearningEvent>>>>,
        _performance_metrics: Arc<RwLock<RealTimeMetrics>>,
        _ltmc_manager: Arc<LTMManager>,
    ) -> Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1000));

        loop {
            interval.tick().await;

            // Collect batch processor IDs first
            let batch_agent_ids = {
                let processors = feedback_processors.read().await;
                processors
                    .iter()
                    .filter(|(_, processor)| processor.processor_type == ProcessorType::Batch)
                    .map(|(agent_id, _)| *agent_id)
                    .collect::<Vec<_>>()
            };

            // Process each batch processor
            for agent_id in batch_agent_ids {
                debug!("Processing batch feedback for agent: {}", agent_id);

                let mut processors = feedback_processors.write().await;
                if let Some(processor) = processors.get_mut(&agent_id) {
                    processor.stats.total_processed += 1;
                    processor.last_processed = Some(chrono::Utc::now());
                }
                drop(processors);
            }
        }
    }

    /// Static version of metrics update loop for background tasks
    async fn update_metrics_loop_static(
        performance_metrics: Arc<RwLock<RealTimeMetrics>>,
        event_subscribers: Arc<RwLock<Vec<tokio::sync::mpsc::UnboundedSender<LearningEvent>>>>,
    ) -> Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(5000));

        loop {
            interval.tick().await;

            // Check if we need to generate performance alert
            let metrics = performance_metrics.read().await;
            let should_alert = metrics.system_load > 0.8;
            let system_load = metrics.system_load;
            drop(metrics);

            // Generate performance alert if needed
            if should_alert {
                let event = LearningEvent {
                    event_id: Uuid::new_v4(),
                    event_type: LearningEventType::PerformanceAlert,
                    agent_id: None,
                    agent_type: None,
                    event_data: LearningEventData::Performance {
                        metric_name: "system_load".to_string(),
                        metric_value: system_load,
                        threshold_exceeded: true,
                    },
                    timestamp: chrono::Utc::now(),
                    priority: EventPriority::High,
                };

                // Broadcast event to subscribers
                let mut subscribers = event_subscribers.write().await;
                let mut dead_subscribers = Vec::new();

                for (i, sender) in subscribers.iter().enumerate() {
                    if let Err(e) = sender.send(event.clone()) {
                        warn!("Failed to send event to subscriber {}: {}", i, e);
                        dead_subscribers.push(i);
                    }
                }

                // Remove dead subscribers
                for &i in dead_subscribers.iter().rev() {
                    subscribers.remove(i);
                }
                drop(subscribers);
            }
        }
    }
}
