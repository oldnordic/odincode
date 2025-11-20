//! Model Performance Tracker
//!
//! Tracks and analyzes model performance over time for optimization and monitoring.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

use crate::ml_integration::metadata::{ModelMetadata, ModelPerformanceMetrics};

/// Performance tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrackingConfig {
    /// Maximum number of performance records to keep
    pub max_records: usize,
    /// Performance evaluation interval in seconds
    pub evaluation_interval_secs: u64,
    /// Whether to enable real-time monitoring
    pub enable_real_time_monitoring: bool,
    /// Performance thresholds for alerts
    pub alert_thresholds: PerformanceThresholds,
}

/// Performance thresholds for alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// Minimum accuracy threshold
    pub min_accuracy: f64,
    /// Maximum prediction time threshold in milliseconds
    pub max_prediction_time_ms: u64,
    /// Maximum memory usage threshold in MB
    pub max_memory_usage_mb: u64,
    /// Minimum confidence threshold
    pub min_confidence: f64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            min_accuracy: 0.8,
            max_prediction_time_ms: 1000,
            max_memory_usage_mb: 1024,
            min_confidence: 0.7,
        }
    }
}

impl Default for PerformanceTrackingConfig {
    fn default() -> Self {
        Self {
            max_records: 1000,
            evaluation_interval_secs: 300, // 5 minutes
            enable_real_time_monitoring: true,
            alert_thresholds: PerformanceThresholds::default(),
        }
    }
}

/// Performance record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRecord {
    /// Record timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Model ID
    pub model_id: String,
    /// Prediction time in milliseconds
    pub prediction_time_ms: u64,
    /// Memory usage in MB
    pub memory_usage_mb: u64,
    /// Accuracy score (if available)
    pub accuracy: Option<f64>,
    /// Confidence score
    pub confidence: f64,
    /// Number of predictions made
    pub prediction_count: u64,
    /// Error count
    pub error_count: u64,
    /// Additional metrics
    pub additional_metrics: HashMap<String, f64>,
}

/// Performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    /// Model ID
    pub model_id: String,
    /// Total predictions
    pub total_predictions: u64,
    /// Average prediction time in milliseconds
    pub avg_prediction_time_ms: f64,
    /// Average memory usage in MB
    pub avg_memory_usage_mb: f64,
    /// Average accuracy
    pub avg_accuracy: Option<f64>,
    /// Average confidence
    pub avg_confidence: f64,
    /// Error rate
    pub error_rate: f64,
    /// Performance trend (improving, stable, degrading)
    pub performance_trend: PerformanceTrend,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Performance trend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceTrend {
    Improving,
    Stable,
    Degrading,
    Unknown,
}

/// Performance alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    /// Alert ID
    pub alert_id: String,
    /// Model ID
    pub model_id: String,
    /// Alert type
    pub alert_type: AlertType,
    /// Alert message
    pub message: String,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Whether alert is active
    pub active: bool,
}

/// Alert type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    LowAccuracy,
    HighPredictionTime,
    HighMemoryUsage,
    LowConfidence,
    HighErrorRate,
}

/// Alert severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Model Performance Tracker
///
/// Tracks and analyzes model performance over time.
pub struct ModelPerformanceTracker {
    /// Tracking configuration
    config: PerformanceTrackingConfig,
    /// Performance records by model ID
    performance_records: HashMap<String, VecDeque<PerformanceRecord>>,
    /// Active alerts
    active_alerts: HashMap<String, Vec<PerformanceAlert>>,
    /// Last evaluation time
    last_evaluation: HashMap<String, Instant>,
}

impl ModelPerformanceTracker {
    /// Create a new performance tracker
    pub fn new(config: PerformanceTrackingConfig) -> Self {
        Self {
            config,
            performance_records: HashMap::new(),
            active_alerts: HashMap::new(),
            last_evaluation: HashMap::new(),
        }
    }

    /// Record a performance event
    pub async fn record_performance(&mut self, record: PerformanceRecord) -> Result<()> {
        debug!("Recording performance for model: {}", record.model_id);

        let records = self
            .performance_records
            .entry(record.model_id.clone())
            .or_insert_with(VecDeque::new);

        // Add new record
        records.push_back(record.clone());

        // Maintain maximum record limit
        if records.len() > self.config.max_records {
            records.pop_front();
        }

        // Check for performance alerts if real-time monitoring is enabled
        if self.config.enable_real_time_monitoring {
            if let Err(e) = self.check_performance_alerts(&record).await {
                warn!("Error checking performance alerts: {}", e);
            }
        }

        Ok(())
    }

    /// Get performance statistics for a model
    pub fn get_performance_stats(&self, model_id: &str) -> Option<PerformanceStats> {
        let records = self.performance_records.get(model_id)?;

        if records.is_empty() {
            return None;
        }

        let total_predictions: u64 = records.iter().map(|r| r.prediction_count).sum();
        let total_errors: u64 = records.iter().map(|r| r.error_count).sum();
        let total_prediction_time: u64 = records.iter().map(|r| r.prediction_time_ms).sum();
        let total_memory_usage: u64 = records.iter().map(|r| r.memory_usage_mb).sum();
        let total_confidence: f64 = records.iter().map(|r| r.confidence).sum();

        let accuracy_values: Vec<f64> = records.iter().filter_map(|r| r.accuracy).collect();
        let avg_accuracy = if accuracy_values.is_empty() {
            None
        } else {
            Some(accuracy_values.iter().sum::<f64>() / accuracy_values.len() as f64)
        };

        let stats = PerformanceStats {
            model_id: model_id.to_string(),
            total_predictions,
            avg_prediction_time_ms: total_prediction_time as f64 / records.len() as f64,
            avg_memory_usage_mb: total_memory_usage as f64 / records.len() as f64,
            avg_accuracy,
            avg_confidence: total_confidence / records.len() as f64,
            error_rate: total_errors as f64 / total_predictions as f64,
            performance_trend: self.calculate_performance_trend(model_id),
            last_updated: records.back().unwrap().timestamp,
        };

        Some(stats)
    }

    /// Get all active alerts for a model
    pub fn get_active_alerts(&self, model_id: &str) -> Vec<PerformanceAlert> {
        self.active_alerts
            .get(model_id)
            .map(|alerts| alerts.clone())
            .unwrap_or_default()
    }

    /// Clear all alerts for a model
    pub fn clear_alerts(&mut self, model_id: &str) {
        self.active_alerts.remove(model_id);
    }

    /// Clear a specific alert
    pub fn clear_alert(&mut self, model_id: &str, alert_id: &str) {
        if let Some(alerts) = self.active_alerts.get_mut(model_id) {
            alerts.retain(|alert| alert.alert_id != alert_id);
        }
    }

    /// Check for performance alerts
    async fn check_performance_alerts(&mut self, record: &PerformanceRecord) -> Result<()> {
        let mut alerts = Vec::new();

        // Check accuracy threshold
        if let Some(accuracy) = record.accuracy {
            if accuracy < self.config.alert_thresholds.min_accuracy {
                alerts.push(PerformanceAlert {
                    alert_id: uuid::Uuid::new_v4().to_string(),
                    model_id: record.model_id.clone(),
                    alert_type: AlertType::LowAccuracy,
                    message: format!(
                        "Accuracy {:.2}% below threshold {:.2}%",
                        accuracy * 100.0,
                        self.config.alert_thresholds.min_accuracy * 100.0
                    ),
                    severity: AlertSeverity::Warning,
                    timestamp: chrono::Utc::now(),
                    active: true,
                });
            }
        }

        // Check prediction time threshold
        if record.prediction_time_ms > self.config.alert_thresholds.max_prediction_time_ms {
            alerts.push(PerformanceAlert {
                alert_id: uuid::Uuid::new_v4().to_string(),
                model_id: record.model_id.clone(),
                alert_type: AlertType::HighPredictionTime,
                message: format!(
                    "Prediction time {}ms above threshold {}ms",
                    record.prediction_time_ms, self.config.alert_thresholds.max_prediction_time_ms
                ),
                severity: AlertSeverity::Warning,
                timestamp: chrono::Utc::now(),
                active: true,
            });
        }

        // Check memory usage threshold
        if record.memory_usage_mb > self.config.alert_thresholds.max_memory_usage_mb {
            alerts.push(PerformanceAlert {
                alert_id: uuid::Uuid::new_v4().to_string(),
                model_id: record.model_id.clone(),
                alert_type: AlertType::HighMemoryUsage,
                message: format!(
                    "Memory usage {}MB above threshold {}MB",
                    record.memory_usage_mb, self.config.alert_thresholds.max_memory_usage_mb
                ),
                severity: AlertSeverity::Error,
                timestamp: chrono::Utc::now(),
                active: true,
            });
        }

        // Check confidence threshold
        if record.confidence < self.config.alert_thresholds.min_confidence {
            alerts.push(PerformanceAlert {
                alert_id: uuid::Uuid::new_v4().to_string(),
                model_id: record.model_id.clone(),
                alert_type: AlertType::LowConfidence,
                message: format!(
                    "Confidence {:.2} below threshold {:.2}",
                    record.confidence, self.config.alert_thresholds.min_confidence
                ),
                severity: AlertSeverity::Info,
                timestamp: chrono::Utc::now(),
                active: true,
            });
        }

        // Add alerts to the model's alert list
        if !alerts.is_empty() {
            let alert_count = alerts.len();
            let model_alerts = self
                .active_alerts
                .entry(record.model_id.clone())
                .or_insert_with(Vec::new);
            model_alerts.extend(alerts);

            warn!(
                "Generated {} performance alerts for model: {}",
                alert_count, record.model_id
            );
        }

        Ok(())
    }

    /// Calculate performance trend
    fn calculate_performance_trend(&self, model_id: &str) -> PerformanceTrend {
        let records = match self.performance_records.get(model_id) {
            Some(records) => records,
            None => return PerformanceTrend::Unknown,
        };

        if records.len() < 10 {
            return PerformanceTrend::Unknown;
        }

        // Simple trend analysis based on recent vs older performance
        let recent_count = records.len() / 3;
        let recent_records: Vec<_> = records.iter().rev().take(recent_count).collect();
        let older_records: Vec<_> = records.iter().take(recent_count).collect();

        let recent_avg_accuracy: f64 = recent_records
            .iter()
            .filter_map(|r| r.accuracy)
            .sum::<f64>()
            / recent_records.len().max(1) as f64;

        let older_avg_accuracy: f64 = older_records.iter().filter_map(|r| r.accuracy).sum::<f64>()
            / older_records.len().max(1) as f64;

        let accuracy_diff = recent_avg_accuracy - older_avg_accuracy;

        if accuracy_diff > 0.05 {
            PerformanceTrend::Improving
        } else if accuracy_diff < -0.05 {
            PerformanceTrend::Degrading
        } else {
            PerformanceTrend::Stable
        }
    }

    /// Get performance records for a time range
    pub fn get_records_in_range(
        &self,
        model_id: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Vec<PerformanceRecord> {
        self.performance_records
            .get(model_id)
            .map(|records| {
                records
                    .iter()
                    .filter(|r| r.timestamp >= start && r.timestamp <= end)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Update tracking configuration
    pub fn update_config(&mut self, config: PerformanceTrackingConfig) {
        self.config = config;
    }

    /// Get tracking statistics
    pub fn get_tracking_stats(&self) -> HashMap<String, usize> {
        self.performance_records
            .iter()
            .map(|(model_id, records)| (model_id.clone(), records.len()))
            .collect()
    }
}

impl Default for ModelPerformanceTracker {
    fn default() -> Self {
        Self::new(PerformanceTrackingConfig::default())
    }
}
