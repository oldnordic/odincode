//! Memory-aware tool execution tracking
//!
//! Records tool execution outcomes to enable:
//! - Learning from past executions (success/failure patterns)
//! - Causal linking between operations and results
//! - Temporal queries ("has this failed before?")
//! - Adaptive behavior based on history

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Outcome of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionOutcome {
    /// Execution succeeded
    Success {
        duration_ms: u64,
        output_size: usize,
    },
    /// Execution failed
    Failure {
        error: String,
        duration_ms: u64,
    },
    /// Execution timed out
    Timeout {
        duration_ms: u64,
    },
}

impl ExecutionOutcome {
    /// Check if outcome was successful
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionOutcome::Success { .. })
    }

    /// Get duration in milliseconds
    pub fn duration_ms(&self) -> u64 {
        match self {
            ExecutionOutcome::Success { duration_ms, .. } => *duration_ms,
            ExecutionOutcome::Failure { duration_ms, .. } => *duration_ms,
            ExecutionOutcome::Timeout { duration_ms } => *duration_ms,
        }
    }
}

/// Record of a single tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Tool name
    pub tool: String,
    /// Arguments (serialized)
    pub args: String,
    /// Outcome
    pub outcome: ExecutionOutcome,
    /// Files affected (if any)
    pub files_affected: Vec<PathBuf>,
    /// Timestamp (Unix seconds)
    pub timestamp: u64,
    /// Session identifier
    pub session_id: String,
}

impl ExecutionRecord {
    /// Create new execution record
    pub fn new(
        tool: impl Into<String>,
        args: impl Into<String>,
        outcome: ExecutionOutcome,
        session_id: impl Into<String>,
    ) -> Self {
        Self {
            tool: tool.into(),
            args: args.into(),
            outcome,
            files_affected: Vec::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            session_id: session_id.into(),
        }
    }

    /// Add affected files
    pub fn with_files(mut self, files: Vec<PathBuf>) -> Self {
        self.files_affected = files;
        self
    }

    /// Get signature (tool + args) for pattern matching
    pub fn signature(&self) -> String {
        format!("{}:{}", self.tool, self.args)
    }
}

/// Statistics for a tool signature
#[derive(Debug, Clone)]
pub struct ToolStatistics {
    /// Total executions
    pub total: usize,
    /// Successful executions
    pub successes: usize,
    /// Failed executions
    pub failures: usize,
    /// Timeouts
    pub timeouts: usize,
    /// Average duration (milliseconds)
    pub avg_duration_ms: u64,
    /// Last execution timestamp
    pub last_execution: Option<u64>,
}

impl ToolStatistics {
    /// Create new empty statistics
    fn new() -> Self {
        Self {
            total: 0,
            successes: 0,
            failures: 0,
            timeouts: 0,
            avg_duration_ms: 0,
            last_execution: None,
        }
    }

    /// Update statistics with a new record
    fn update(&mut self, record: &ExecutionRecord) {
        self.total += 1;
        match &record.outcome {
            ExecutionOutcome::Success { .. } => self.successes += 1,
            ExecutionOutcome::Failure { .. } => self.failures += 1,
            ExecutionOutcome::Timeout { .. } => self.timeouts += 1,
        }

        // Update average duration
        let duration = record.outcome.duration_ms();
        self.avg_duration_ms =
            (self.avg_duration_ms * (self.total - 1) as u64 + duration) / self.total as u64;

        self.last_execution = Some(record.timestamp);
    }

    /// Get success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            return 1.0;
        }
        self.successes as f64 / self.total as f64
    }

    /// Check if this pattern has high failure rate
    pub fn has_high_failure_rate(&self) -> bool {
        self.total >= 3 && self.success_rate() < 0.5
    }
}

/// Memory-aware tool execution tracker
///
/// Tracks execution history and provides learning capabilities.
pub struct ToolMemory {
    /// All execution records
    records: Vec<ExecutionRecord>,
    /// Statistics per tool signature
    statistics: HashMap<String, ToolStatistics>,
    /// Files affected by tools
    file_impact: HashMap<PathBuf, Vec<usize>>, // file -> record indices
    /// Current session ID
    session_id: String,
    /// Maximum records to keep
    max_records: usize,
}

impl ToolMemory {
    /// Create new tool memory with default capacity
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            statistics: HashMap::new(),
            file_impact: HashMap::new(),
            session_id: uuid::Uuid::new_v4().to_string(),
            max_records: 10_000,
        }
    }

    /// Create with custom session ID
    pub fn with_session(session_id: impl Into<String>) -> Self {
        Self {
            records: Vec::new(),
            statistics: HashMap::new(),
            file_impact: HashMap::new(),
            session_id: session_id.into(),
            max_records: 10_000,
        }
    }

    /// Set maximum records to keep
    pub fn with_max_records(mut self, max: usize) -> Self {
        self.max_records = max;
        self
    }

    /// Record a tool execution
    pub fn record(&mut self, record: ExecutionRecord) {
        let signature = record.signature();
        let index = self.records.len();

        // Add to records
        self.records.push(record.clone());

        // Update statistics
        self.statistics
            .entry(signature.clone())
            .or_insert_with(ToolStatistics::new)
            .update(&record);

        // Track file impacts
        for file in &record.files_affected {
            self.file_impact
                .entry(file.clone())
                .or_insert_with(Vec::new)
                .push(index);
        }

        // Prune if too many records
        if self.records.len() > self.max_records {
            self.prune_oldest();
        }
    }

    /// Get statistics for a tool signature
    pub fn statistics(&self, tool: &str, args: &str) -> Option<&ToolStatistics> {
        let signature = format!("{}:{}", tool, args);
        self.statistics.get(&signature)
    }

    /// Get statistics for all calls to a tool
    pub fn tool_statistics(&self, tool: &str) -> ToolStatistics {
        let mut result = ToolStatistics::new();
        for (signature, stats) in &self.statistics {
            if signature.starts_with(&format!("{}:", tool)) {
                result.total += stats.total;
                result.successes += stats.successes;
                result.failures += stats.failures;
                result.timeouts += stats.timeouts;
                // Weighted average for duration
                if stats.total > 0 {
                    result.avg_duration_ms =
                        (result.avg_duration_ms * (result.total - stats.total) as u64
                            + stats.avg_duration_ms * stats.total as u64)
                            / result.total as u64;
                }
                if let Some(last) = stats.last_execution {
                    result.last_execution = Some(match result.last_execution {
                        Some(existing) => existing.max(last),
                        None => last,
                    });
                }
            }
        }
        result
    }

    /// Check if a tool pattern has high failure rate
    pub fn has_high_failure_rate(&self, tool: &str, args: &str) -> bool {
        self.statistics(tool, args)
            .map(|s| s.has_high_failure_rate())
            .unwrap_or(false)
    }

    /// Get recent failures for a tool
    pub fn recent_failures(&self, tool: &str, limit: usize) -> Vec<&ExecutionRecord> {
        self.records
            .iter()
            .rev()
            .filter(|r| r.tool == tool && !r.outcome.is_success())
            .take(limit)
            .collect()
    }

    /// Check if file was recently modified
    pub fn file_modified_recently(&self, file: &PathBuf, within_secs: u64) -> bool {
        if let Some(indices) = self.file_impact.get(file) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            for &index in indices {
                if let Some(record) = self.records.get(index) {
                    if now - record.timestamp <= within_secs {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Get files affected by a tool execution
    pub fn files_affected_by(&self, tool: &str, args: &str) -> Vec<PathBuf> {
        let signature = format!("{}:{}", tool, args);
        let mut files = HashSet::new();

        for record in &self.records {
            if record.signature() == signature {
                files.extend(record.files_affected.iter().cloned());
            }
        }

        files.into_iter().collect()
    }

    /// Get total execution count
    pub fn total_executions(&self) -> usize {
        self.records.len()
    }

    /// Get execution count for current session
    pub fn session_executions(&self) -> usize {
        self.records
            .iter()
            .filter(|r| r.session_id == self.session_id)
            .count()
    }

    /// Start a new session
    pub fn new_session(&mut self) {
        self.session_id = uuid::Uuid::new_v4().to_string();
    }

    /// Get current session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get all records
    pub fn records(&self) -> &[ExecutionRecord] {
        &self.records
    }

    /// Get records for a specific tool
    pub fn records_for_tool(&self, tool: &str) -> Vec<&ExecutionRecord> {
        self.records
            .iter()
            .filter(|r| r.tool == tool)
            .collect()
    }

    /// Clear all records
    pub fn clear(&mut self) {
        self.records.clear();
        self.statistics.clear();
        self.file_impact.clear();
    }

    /// Remove oldest record when at capacity
    fn prune_oldest(&mut self) {
        if let Some(oldest) = self.records.first() {
            let signature = oldest.signature();

            // Remove from statistics
            if let Some(stats) = self.statistics.get_mut(&signature) {
                stats.total = stats.total.saturating_sub(1);
                match &oldest.outcome {
                    ExecutionOutcome::Success { .. } => {
                        stats.successes = stats.successes.saturating_sub(1);
                    }
                    ExecutionOutcome::Failure { .. } => {
                        stats.failures = stats.failures.saturating_sub(1);
                    }
                    ExecutionOutcome::Timeout { .. } => {
                        stats.timeouts = stats.timeouts.saturating_sub(1);
                    }
                }
            }

            // Remove from file impacts
            for file in &oldest.files_affected {
                if let Some(indices) = self.file_impact.get_mut(file) {
                    indices.retain(|i| *i != 0);
                    // Shift remaining indices
                    for i in indices.iter_mut() {
                        *i = i.saturating_sub(1);
                    }
                }
            }

            // Remove record
            self.records.remove(0);

            // Shift all file impact indices
            for indices in self.file_impact.values_mut() {
                indices.retain(|i| *i > 0);
                for i in indices.iter_mut() {
                    *i -= 1;
                }
            }
        }
    }

    /// Recommend whether to execute based on history
    pub fn recommend_execution(&self, tool: &str, args: &str) -> ExecutionRecommendation {
        if let Some(stats) = self.statistics(tool, args) {
            // Check for repeated timeouts first (before Skip check)
            if stats.timeouts > 2 {
                return ExecutionRecommendation::Caution {
                    reason: format!(
                        "Multiple timeouts ({} out of {} attempts)",
                        stats.timeouts, stats.total
                    ),
                };
            }

            // Then check for high failure rate (excluding timeout-only scenarios)
            if stats.total >= 3 && stats.success_rate() < 0.3 {
                return ExecutionRecommendation::Skip {
                    reason: format!(
                        "High failure rate ({:.0}%) - {} failures out of {} attempts",
                        (1.0 - stats.success_rate()) * 100.0,
                        stats.failures,
                        stats.total
                    ),
                };
            }
        }

        ExecutionRecommendation::Proceed
    }
}

impl Default for ToolMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Recommendation for tool execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionRecommendation {
    /// Safe to proceed
    Proceed,
    /// Proceed with caution (may fail or be slow)
    Caution { reason: String },
    /// Skip execution (likely to fail)
    Skip { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_memory_is_empty() {
        let memory = ToolMemory::new();
        assert_eq!(memory.total_executions(), 0);
        assert_eq!(memory.session_executions(), 0);
    }

    #[test]
    fn test_record_execution() {
        let mut memory = ToolMemory::new();
        // Record with the memory's current session ID
        let record = ExecutionRecord::new(
            "file_read",
            "path=test.txt",
            ExecutionOutcome::Success {
                duration_ms: 10,
                output_size: 100,
            },
            memory.session_id(),
        );

        memory.record(record);
        assert_eq!(memory.total_executions(), 1);
        assert_eq!(memory.session_executions(), 1);
    }

    #[test]
    fn test_statistics_tracking() {
        let mut memory = ToolMemory::new();

        // Record success
        memory.record(ExecutionRecord::new(
            "file_read",
            "path=test.txt",
            ExecutionOutcome::Success {
                duration_ms: 100,
                output_size: 200,
            },
            "session1",
        ));

        // Record failure
        memory.record(ExecutionRecord::new(
            "file_read",
            "path=test.txt",
            ExecutionOutcome::Failure {
                error: "not found".to_string(),
                duration_ms: 50,
            },
            "session1",
        ));

        let stats = memory.statistics("file_read", "path=test.txt").unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.failures, 1);
        assert_eq!(stats.success_rate(), 0.5);
    }

    #[test]
    fn test_high_failure_rate_detection() {
        let mut memory = ToolMemory::new();

        // Record 3 failures
        for _ in 0..3 {
            memory.record(ExecutionRecord::new(
                "splice_patch",
                "symbol=foo",
                ExecutionOutcome::Failure {
                    error: "compilation error".to_string(),
                    duration_ms: 1000,
                },
                "session1",
            ));
        }

        assert!(memory.has_high_failure_rate("splice_patch", "symbol=foo"));
    }

    #[test]
    fn test_recent_failures() {
        let mut memory = ToolMemory::new();

        memory.record(ExecutionRecord::new(
            "file_read",
            "path=missing.txt",
            ExecutionOutcome::Failure {
                error: "not found".to_string(),
                duration_ms: 10,
            },
            "session1",
        ));

        memory.record(ExecutionRecord::new(
            "file_read",
            "path=another.txt",
            ExecutionOutcome::Success {
                duration_ms: 10,
                output_size: 100,
            },
            "session1",
        ));

        let failures = memory.recent_failures("file_read", 10);
        assert_eq!(failures.len(), 1);
    }

    #[test]
    fn test_records_for_tool() {
        let mut memory = ToolMemory::new();

        memory.record(ExecutionRecord::new(
            "file_read",
            "path=a.txt",
            ExecutionOutcome::Success {
                duration_ms: 10,
                output_size: 100,
            },
            "session1",
        ));

        memory.record(ExecutionRecord::new(
            "file_write",
            "path=b.txt",
            ExecutionOutcome::Success {
                duration_ms: 20,
                output_size: 0,
            },
            "session1",
        ));

        let read_records = memory.records_for_tool("file_read");
        assert_eq!(read_records.len(), 1);

        let write_records = memory.records_for_tool("file_write");
        assert_eq!(write_records.len(), 1);
    }

    #[test]
    fn test_new_session_resets_count() {
        let mut memory = ToolMemory::new();

        memory.record(ExecutionRecord::new(
            "file_read",
            "path=test.txt",
            ExecutionOutcome::Success {
                duration_ms: 10,
                output_size: 100,
            },
            memory.session_id(),
        ));

        assert_eq!(memory.session_executions(), 1);

        memory.new_session();
        assert_eq!(memory.session_executions(), 0);
        assert_eq!(memory.total_executions(), 1);
    }

    #[test]
    fn test_recommendation_proceed() {
        let memory = ToolMemory::new();
        let rec = memory.recommend_execution("file_read", "path=test.txt");
        assert_eq!(rec, ExecutionRecommendation::Proceed);
    }

    #[test]
    fn test_recommendation_skip_high_failure() {
        let mut memory = ToolMemory::new();

        // Record 3 failures, 0 successes
        for _ in 0..3 {
            memory.record(ExecutionRecord::new(
                "splice_patch",
                "symbol=bad",
                ExecutionOutcome::Failure {
                    error: "error".to_string(),
                    duration_ms: 100,
                },
                "session1",
            ));
        }

        let rec = memory.recommend_execution("splice_patch", "symbol=bad");
        assert!(matches!(rec, ExecutionRecommendation::Skip { .. }));
    }

    #[test]
    fn test_recommendation_caution_timeout() {
        let mut memory = ToolMemory::new();

        // Record some timeouts
        for _ in 0..3 {
            memory.record(ExecutionRecord::new(
                "cargo_check",
                "",
                ExecutionOutcome::Timeout { duration_ms: 30000 },
                "session1",
            ));
        }

        let rec = memory.recommend_execution("cargo_check", "");
        assert!(matches!(rec, ExecutionRecommendation::Caution { .. }));
    }

    #[test]
    fn test_record_signature() {
        let record = ExecutionRecord::new(
            "file_read",
            "path=test.txt",
            ExecutionOutcome::Success {
                duration_ms: 10,
                output_size: 100,
            },
            "session1",
        );

        assert_eq!(record.signature(), "file_read:path=test.txt");
    }

    #[test]
    fn test_execution_outcome_is_success() {
        assert!(ExecutionOutcome::Success {
            duration_ms: 10,
            output_size: 100
        }
        .is_success());

        assert!(!ExecutionOutcome::Failure {
            error: "error".to_string(),
            duration_ms: 10
        }
        .is_success());

        assert!(!ExecutionOutcome::Timeout { duration_ms: 1000 }.is_success());
    }

    #[test]
    fn test_clear_memory() {
        let mut memory = ToolMemory::new();

        memory.record(ExecutionRecord::new(
            "file_read",
            "path=test.txt",
            ExecutionOutcome::Success {
                duration_ms: 10,
                output_size: 100,
            },
            "session1",
        ));

        assert_eq!(memory.total_executions(), 1);

        memory.clear();
        assert_eq!(memory.total_executions(), 0);
    }

    #[test]
    fn test_files_affected_tracking() {
        let mut memory = ToolMemory::new();

        let record = ExecutionRecord::new(
            "file_write",
            "path=test.txt",
            ExecutionOutcome::Success {
                duration_ms: 10,
                output_size: 0,
            },
            "session1",
        )
        .with_files(vec![PathBuf::from("test.txt"), PathBuf::from("other.txt")]);

        memory.record(record);

        let files = memory.files_affected_by("file_write", "path=test.txt");
        assert_eq!(files.len(), 2);
        assert!(files.contains(&PathBuf::from("test.txt")));
        assert!(files.contains(&PathBuf::from("other.txt")));
    }

    #[test]
    fn test_file_modified_recently() {
        let mut memory = ToolMemory::new();

        let record = ExecutionRecord::new(
            "file_write",
            "path=test.txt",
            ExecutionOutcome::Success {
                duration_ms: 10,
                output_size: 0,
            },
            "session1",
        )
        .with_files(vec![PathBuf::from("test.txt")]);

        memory.record(record);

        assert!(memory.file_modified_recently(&PathBuf::from("test.txt"), 10));
        assert!(!memory.file_modified_recently(&PathBuf::from("other.txt"), 10));
    }
}
