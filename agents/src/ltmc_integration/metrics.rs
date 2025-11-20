//! LTMC Metrics Management
//!
//! This module handles performance metrics, statistics tracking, and background tasks
//! for the LTMC integration system.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, warn};
use uuid::Uuid;

use odincode_ltmc::LTMManager;

use super::types::{
    AgentLearningSession, EventPriority, FeedbackProcessor, LearningEvent, LearningEventData,
    LearningEventType, LearningSessionStatus, ProcessorType, RealTimeMetrics,
};

/// Metrics management functionality for LTMC integration
pub struct MetricsManager {
    /// Reference to the LTMC manager
    pub ltmc_manager: Arc<LTMManager>,
    /// Real-time performance metrics
    pub performance_metrics: Arc<RwLock<RealTimeMetrics>>,
    /// Learning sessions reference for metrics calculation
    pub learning_sessions: Arc<RwLock<HashMap<Uuid, AgentLearningSession>>>,
    /// Feedback processors reference for metrics calculation
    pub feedback_processors: Arc<RwLock<HashMap<Uuid, FeedbackProcessor>>>,
    /// Learning event subscribers
    pub event_subscribers: Arc<RwLock<Vec<tokio::sync::mpsc::UnboundedSender<LearningEvent>>>>,
}

impl MetricsManager {
    /// Create a new metrics manager
    pub fn new(
        ltmc_manager: Arc<LTMManager>,
        learning_sessions: Arc<RwLock<HashMap<Uuid, AgentLearningSession>>>,
        feedback_processors: Arc<RwLock<HashMap<Uuid, FeedbackProcessor>>>,
        event_subscribers: Arc<RwLock<Vec<tokio::sync::mpsc::UnboundedSender<LearningEvent>>>>,
    ) -> Self {
        Self {
            ltmc_manager,
            performance_metrics: Arc::new(RwLock::new(RealTimeMetrics::default())),
            learning_sessions,
            feedback_processors,
            event_subscribers,
        }
    }

    /// Get real-time performance metrics
    pub async fn get_real_time_metrics(&self) -> Result<RealTimeMetrics> {
        let metrics = self.performance_metrics.read().await;
        Ok(metrics.clone())
    }

    /// Update performance metrics based on system activity
    pub async fn update_performance_metrics(&self) -> Result<()> {
        let mut metrics = self.performance_metrics.write().await;
        metrics.last_updated = chrono::Utc::now();

        // Update active sessions count
        let sessions = self.learning_sessions.read().await;
        let active_count = sessions
            .values()
            .filter(|s| s.status == LearningSessionStatus::Active)
            .count() as u32;
        metrics.active_sessions = active_count;
        drop(sessions);

        // Update processor metrics
        let processors = self.feedback_processors.read().await;
        let total_processed: u64 = processors.values().map(|p| p.stats.total_processed).sum();
        let avg_processing_time: f32 = if processors.is_empty() {
            0.0
        } else {
            processors
                .values()
                .map(|p| p.stats.avg_processing_time_ms)
                .sum::<f32>()
                / processors.len() as f32
        };
        metrics.total_feedback_processed = total_processed;
        metrics.avg_processing_time_ms = avg_processing_time;
        drop(processors);

        Ok(())
    }

    /// Start background metrics collection and monitoring
    pub async fn start_metrics_collection(&self) -> Result<()> {
        debug!("Starting background metrics collection...");

        // Clone Arc fields for background tasks
        let performance_metrics = self.performance_metrics.clone();
        let learning_sessions = self.learning_sessions.clone();
        let event_subscribers = self.event_subscribers.clone();

        // Start background task for updating metrics
        tokio::spawn(async move {
            if let Err(e) = Self::update_metrics_loop_static(
                performance_metrics,
                learning_sessions,
                event_subscribers,
            )
            .await
            {
                error!("Error in metrics update loop: {}", e);
            }
        });

        debug!("Background metrics collection started");
        Ok(())
    }

    /// Generate performance alert if thresholds are exceeded
    pub async fn check_performance_thresholds(&self) -> Result<()> {
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

            self.broadcast_learning_event(event).await?;
        }

        Ok(())
    }

    /// Calculate system health score based on various metrics
    pub async fn calculate_system_health(&self) -> Result<f32> {
        let metrics = self.performance_metrics.read().await;

        // Calculate health score based on multiple factors
        let load_health = (1.0 - metrics.system_load).max(0.0);
        let efficiency_health = metrics.learning_efficiency;
        let error_health = (1.0 - metrics.error_rate).max(0.0);

        // Weighted average of health factors
        let health_score = (load_health * 0.4 + efficiency_health * 0.4 + error_health * 0.2)
            .max(0.0)
            .min(1.0);

        drop(metrics);
        Ok(health_score)
    }

    /// Get detailed metrics report
    pub async fn get_metrics_report(&self) -> Result<String> {
        let metrics = self.performance_metrics.read().await;
        let health_score = self.calculate_system_health().await?;

        let report = format!(
            "=== LTMC Integration Metrics Report ===\n\
            \nSystem Health: {:.1}%\n\
            \nPerformance Metrics:\n\
            - Active Sessions: {}\n\
            - Total Feedback Processed: {}\n\
            - Average Processing Time: {:.2}ms\n\
            - System Load: {:.1}%\n\
            - Learning Efficiency: {:.1}%\n\
            - Error Rate: {:.1}%\n\
            \nResource Usage:\n\
            - Memory Usage: {:.1}%\n\
            - CPU Usage: {:.1}%\n\
            \nLast Updated: {}\n\
            =====================================",
            health_score * 100.0,
            metrics.active_sessions,
            metrics.total_feedback_processed,
            metrics.avg_processing_time_ms,
            metrics.system_load * 100.0,
            metrics.learning_efficiency * 100.0,
            metrics.error_rate * 100.0,
            metrics.memory_usage_percent,
            metrics.cpu_usage_percent,
            metrics.last_updated
        );

        drop(metrics);
        Ok(report)
    }

    // Private helper methods

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

    /// Static version of metrics update loop for background tasks
    async fn update_metrics_loop_static(
        performance_metrics: Arc<RwLock<RealTimeMetrics>>,
        learning_sessions: Arc<RwLock<HashMap<Uuid, AgentLearningSession>>>,
        event_subscribers: Arc<RwLock<Vec<tokio::sync::mpsc::UnboundedSender<LearningEvent>>>>,
    ) -> Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(5000));

        loop {
            interval.tick().await;

            // Update active sessions count
            let sessions = learning_sessions.read().await;
            let active_count = sessions
                .values()
                .filter(|s| s.status == LearningSessionStatus::Active)
                .count() as u32;
            drop(sessions);

            let mut metrics = performance_metrics.write().await;
            metrics.active_sessions = active_count;
            metrics.last_updated = chrono::Utc::now();

            // Check if we need to generate performance alert
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
