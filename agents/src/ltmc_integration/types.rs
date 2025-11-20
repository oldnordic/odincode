//! LTMC Integration Types
//!
//! This module contains all data structures and enums used by the LTMC integration system.

use crate::models::AgentType;
use odincode_core::CodeSuggestion;
use odincode_ltmc::{LearningPattern, PatternType};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents an active learning session for an agent
#[derive(Debug, Clone)]
pub struct AgentLearningSession {
    /// Unique session identifier
    pub session_id: Uuid,
    /// Agent ID
    pub agent_id: Uuid,
    /// Agent type
    pub agent_type: AgentType,
    /// Session context
    pub context: String,
    /// Learning patterns discovered
    pub patterns_discovered: Vec<LearningPattern>,
    /// Session start time
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Last activity time
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// Session status
    pub status: LearningSessionStatus,
}

/// Learning session status
#[derive(Debug, Clone, PartialEq)]
pub enum LearningSessionStatus {
    /// Session is active
    Active,
    /// Session is paused
    Paused,
    /// Session is completed
    Completed,
    /// Session failed
    Failed,
}

/// Learning statistics
#[derive(Debug, Clone, Default)]
pub struct LearningStatistics {
    /// Total learning sessions
    pub total_sessions: u64,
    /// Total patterns learned
    pub total_patterns_learned: u64,
    /// Total agent executions
    pub total_agent_executions: u64,
    /// Average learning confidence
    pub average_confidence: f32,
    /// Most successful agent type
    pub most_successful_agent: Option<AgentType>,
    /// Learning efficiency score
    pub learning_efficiency: f32,
}

/// Learning pattern request from agent
pub struct LearningRequest {
    /// Agent ID
    pub agent_id: Uuid,
    /// Agent type
    pub agent_type: AgentType,
    /// Request context
    pub context: String,
    /// File being processed (if any)
    pub file_id: Option<Uuid>,
    /// Learning query
    pub query: String,
    /// Pattern types to search for
    pub pattern_types: Vec<PatternType>,
}

/// Learning response to agent
pub struct LearningResponse {
    /// Request ID
    pub request_id: Uuid,
    /// Relevant patterns found
    pub patterns: Vec<LearningPattern>,
    /// Learning suggestions
    pub suggestions: Vec<String>,
    /// Confidence score
    pub confidence: f32,
    /// Session ID for continuous learning
    pub session_id: Option<Uuid>,
}

/// Agent execution result for learning
#[derive(Debug, Clone)]
pub struct AgentExecutionResult {
    /// Execution description
    pub description: String,
    /// Whether execution was successful
    pub success: bool,
    /// Generated suggestions
    pub suggestions: Vec<CodeSuggestion>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Confidence score
    pub confidence: f32,
}

/// Learning insights for an agent type
#[derive(Debug, Clone)]
pub struct AgentLearningInsights {
    /// Agent type
    pub agent_type: AgentType,
    /// Total learning patterns
    pub total_patterns: usize,
    /// Success rate
    pub success_rate: f32,
    /// Average confidence
    pub average_confidence: f32,
    /// Generated insights
    pub insights: String,
    /// Recent patterns
    pub recent_patterns: Vec<LearningPattern>,
}

/// Real-time feedback from agent execution
#[derive(Debug, Clone)]
pub struct AgentFeedback {
    /// Feedback ID
    pub feedback_id: Uuid,
    /// Agent ID
    pub agent_id: Uuid,
    /// Agent type
    pub agent_type: AgentType,
    /// Feedback type
    pub feedback_type: FeedbackType,
    /// Feedback content
    pub content: String,
    /// Performance metrics
    pub performance_metrics: FeedbackMetrics,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Context information
    pub context: HashMap<String, String>,
}

/// Feedback type
#[derive(Debug, Clone, PartialEq)]
pub enum FeedbackType {
    /// Execution success feedback
    ExecutionSuccess,
    /// Execution failure feedback
    ExecutionFailure,
    /// Performance feedback
    Performance,
    /// Learning feedback
    Learning,
    /// Error feedback
    Error,
}

/// Performance metrics for feedback
#[derive(Debug, Clone, Default)]
pub struct FeedbackMetrics {
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f32,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f32,
    /// Confidence score (0.0 to 1.0)
    pub confidence_score: f32,
    /// Suggestions generated
    pub suggestions_generated: u32,
    /// Errors encountered
    pub errors_encountered: u32,
}

/// Feedback processor for analyzing agent feedback
#[derive(Debug, Clone)]
pub struct FeedbackProcessor {
    /// Processor ID
    pub processor_id: Uuid,
    /// Agent ID this processor is for
    pub agent_id: Uuid,
    /// Processor type
    pub processor_type: ProcessorType,
    /// Processing configuration
    pub config: FeedbackProcessorConfig,
    /// Processing statistics
    pub stats: ProcessorStats,
    /// Last processed timestamp
    pub last_processed: Option<chrono::DateTime<chrono::Utc>>,
}

/// Feedback processor type
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessorType {
    /// Real-time processor
    RealTime,
    /// Batch processor
    Batch,
    /// Adaptive processor
    Adaptive,
}

/// Feedback processor configuration
#[derive(Debug, Clone)]
pub struct FeedbackProcessorConfig {
    /// Processing interval in milliseconds
    pub processing_interval_ms: u64,
    /// Batch size for batch processing
    pub batch_size: usize,
    /// Confidence threshold
    pub confidence_threshold: f32,
    /// Learning rate
    pub learning_rate: f32,
    /// Maximum feedback history
    pub max_feedback_history: usize,
}

/// Processor statistics
#[derive(Debug, Clone, Default)]
pub struct ProcessorStats {
    /// Total feedback processed
    pub total_processed: u64,
    /// Average processing time
    pub avg_processing_time_ms: f32,
    /// Success rate
    pub success_rate: f32,
    /// Last error
    pub last_error: Option<String>,
}

/// Learning event for real-time notifications
#[derive(Debug, Clone)]
pub struct LearningEvent {
    /// Event ID
    pub event_id: Uuid,
    /// Event type
    pub event_type: LearningEventType,
    /// Agent ID (if applicable)
    pub agent_id: Option<Uuid>,
    /// Agent type (if applicable)
    pub agent_type: Option<AgentType>,
    /// Event data
    pub event_data: LearningEventData,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Priority
    pub priority: EventPriority,
}

/// Learning event type
#[derive(Debug, Clone, PartialEq)]
pub enum LearningEventType {
    /// Pattern discovered
    PatternDiscovered,
    /// Session started
    SessionStarted,
    /// Session completed
    SessionCompleted,
    /// Feedback received
    FeedbackReceived,
    /// Insight generated
    InsightGenerated,
    /// Performance alert
    PerformanceAlert,
    /// Error occurred
    ErrorOccurred,
}

/// Learning event data
#[derive(Debug, Clone)]
pub enum LearningEventData {
    /// Pattern discovered data
    PatternDiscovered {
        /// Pattern ID
        pattern_id: Uuid,
        /// Pattern type
        pattern_type: PatternType,
        /// Confidence score
        confidence: f32,
    },
    /// Session data
    Session {
        /// Session ID
        session_id: Uuid,
        /// Session status
        status: LearningSessionStatus,
    },
    /// Feedback data
    Feedback {
        /// Feedback ID
        feedback_id: Uuid,
        /// Feedback type
        feedback_type: FeedbackType,
        /// Performance metrics
        metrics: FeedbackMetrics,
    },
    /// Insight data
    Insight {
        /// Agent type
        agent_type: AgentType,
        /// Insight content
        insight: String,
        /// Confidence score
        confidence: f32,
    },
    /// Performance data
    Performance {
        /// Metric name
        metric_name: String,
        /// Metric value
        metric_value: f32,
        /// Threshold exceeded
        threshold_exceeded: bool,
    },
    /// Error data
    Error {
        /// Error message
        error_message: String,
        /// Error context
        error_context: HashMap<String, String>,
    },
}

/// Event priority
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum EventPriority {
    /// Low priority
    Low,
    /// Medium priority
    Medium,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Real-time performance metrics
#[derive(Debug, Clone, Default)]
pub struct RealTimeMetrics {
    /// Current active sessions
    pub active_sessions: u32,
    /// Total feedback processed
    pub total_feedback_processed: u64,
    /// Average processing time
    pub avg_processing_time_ms: f32,
    /// System load (0.0 to 1.0)
    pub system_load: f32,
    /// Memory usage percentage
    pub memory_usage_percent: f32,
    /// CPU usage percentage
    pub cpu_usage_percent: f32,
    /// Learning efficiency score
    pub learning_efficiency: f32,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f32,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}
