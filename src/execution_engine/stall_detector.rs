//! Stall detector for tool execution loops
//!
//! Detects when the LLM is stuck in a loop with no meaningful progress:
//! - No state change across N steps (same checksums)
//! - Repeating tool call sequences
//! - Identical tool invocations
//!
//! This prevents infinite loops where the LLM keeps calling the same
//! tools without making progress.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::fmt;

use super::safety_config::SafetyConfig;

/// Reason for stall detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StallReason {
    /// No state change detected across multiple steps
    NoStateChange,
    /// Same tools being called in same order (tool loop)
    ToolLoop,
    /// Identical tool calls (same tool + args)
    IdenticalCalls,
}

impl fmt::Display for StallReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StallReason::NoStateChange => write!(f, "No state change across multiple steps"),
            StallReason::ToolLoop => write!(f, "Repeated tool call sequence detected"),
            StallReason::IdenticalCalls => write!(f, "Identical tool calls detected"),
        }
    }
}

/// Snapshot of execution state at a single step
#[derive(Debug, Clone)]
struct StateSnapshot {
    step_number: usize,
    tools_invoked: Vec<String>,
    files_modified: HashSet<PathBuf>,
    checksum: String,
}

impl StateSnapshot {
    fn new(step_number: usize, tools_invoked: Vec<String>, files_modified: HashSet<PathBuf>) -> Self {
        let checksum = Self::compute_checksum_inner(&tools_invoked, &files_modified);

        Self {
            step_number,
            tools_invoked,
            files_modified,
            checksum,
        }
    }

    /// Compute checksum from relevant state
    fn compute_checksum(&self) -> String {
        Self::compute_checksum_inner(&self.tools_invoked, &self.files_modified)
    }

    fn compute_checksum_inner(tools_invoked: &[String], files_modified: &HashSet<PathBuf>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash tools in order
        for tool in tools_invoked {
            tool.hash(&mut hasher);
        }

        // Hash modified files (sorted for determinism)
        let mut files: Vec<_> = files_modified.iter().collect();
        files.sort();
        for path in files {
            path.hash(&mut hasher);
        }

        format!("{:x}", hasher.finish())
    }
}

/// Stall detector for execution loops
///
/// Tracks recent state snapshots and detects patterns indicating a stall.
pub struct StallDetector {
    snapshots: VecDeque<StateSnapshot>,
    config: SafetyConfig,
    step_number: usize,
    /// Track identical call signatures: (tool, args_hash) -> count
    call_tracker: HashMap<(String, u64), usize>,
}

impl StallDetector {
    /// Create new stall detector with default configuration
    pub fn new() -> Self {
        Self::with_config(SafetyConfig::default())
    }

    /// Create new stall detector with custom configuration
    pub fn with_config(config: SafetyConfig) -> Self {
        Self {
            snapshots: VecDeque::with_capacity(config.stall_threshold + 1),
            config,
            step_number: 0,
            call_tracker: HashMap::new(),
        }
    }

    /// Record a tool execution step
    ///
    /// Returns `Err(StallReason)` if a stall is detected.
    pub fn record_step(
        &mut self,
        tool: &str,
        args: &HashMap<String, String>,
        files_modified: &[PathBuf],
    ) -> Result<(), StallReason> {
        self.step_number += 1;

        // Check for identical calls
        if let Err(reason) = self.check_identical_call(tool, args) {
            return Err(reason);
        }

        // Build snapshot
        let tools_invoked = vec![tool.to_string()];
        let files_modified_set: HashSet<PathBuf> = files_modified.iter().cloned().collect();
        let snapshot = StateSnapshot::new(self.step_number, tools_invoked, files_modified_set);

        // Add to history
        self.snapshots.push_back(snapshot);

        // Trim to threshold + 1
        while self.snapshots.len() > self.config.stall_threshold + 1 {
            self.snapshots.pop_front();
        }

        // Check for stalls (only if we have enough history)
        if self.snapshots.len() >= self.config.stall_threshold {
            if let Some(reason) = self.detect_stall() {
                return Err(reason);
            }
        }

        Ok(())
    }

    /// Check for identical tool calls
    fn check_identical_call(
        &mut self,
        tool: &str,
        args: &HashMap<String, String>,
    ) -> Result<(), StallReason> {
        // Hash arguments for comparison
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        let mut sorted_args: Vec<_> = args.iter().collect();
        sorted_args.sort_by_key(|(k, _)| *k);
        for (key, value) in sorted_args {
            key.hash(&mut hasher);
            value.hash(&mut hasher);
        }
        let args_hash = hasher.finish();

        let key = (tool.to_string(), args_hash);
        let count = self.call_tracker.entry(key).or_insert(0);
        *count += 1;

        if *count > self.config.max_identical_calls {
            return Err(StallReason::IdenticalCalls);
        }

        Ok(())
    }

    /// Detect stall patterns
    fn detect_stall(&self) -> Option<StallReason> {
        // Check for no state change
        if self.detect_no_state_change() {
            return Some(StallReason::NoStateChange);
        }

        // Check for tool loop
        if self.detect_tool_loop() {
            return Some(StallReason::ToolLoop);
        }

        None
    }

    /// Detect no state change across recent steps
    fn detect_no_state_change(&self) -> bool {
        if self.snapshots.len() < self.config.stall_threshold {
            return false;
        }

        // Get the most recent checksum
        let recent_checksum = &self.snapshots.back().unwrap().checksum;

        // Check if all recent snapshots have the same checksum
        self.snapshots
            .iter()
            .rev()
            .take(self.config.stall_threshold)
            .all(|s| s.checksum == *recent_checksum)
    }

    /// Detect repeating tool call sequence
    ///
    /// Triggers when the same tools are called in the same order across multiple steps.
    /// For example: file_read, file_search, file_read, file_search, file_read, file_search
    fn detect_tool_loop(&self) -> bool {
        if self.snapshots.len() < self.config.stall_threshold {
            return false;
        }

        // Collect the sequence of tools called in each step
        let tool_sequence: Vec<&str> = self
            .snapshots
            .iter()
            .rev()
            .take(self.config.stall_threshold)
            .map(|s| s.tools_invoked.get(0).map(|s| s.as_str()).unwrap_or(""))
            .collect();

        if tool_sequence.is_empty() {
            return false;
        }

        // For a sequence to be a "loop", we need at least 2 distinct tools
        let unique_tools: std::collections::HashSet<_> = tool_sequence.iter().collect();
        if unique_tools.len() < 2 {
            return false;
        }

        // Detect alternating patterns (A, B, A, B, ...)
        // For odd-length sequences ending with A, check if we have A, B, A, B, A
        if tool_sequence.len() >= 3 {
            // Check for 2-cycle: A, B, A, B, A (alternating between 2 tools)
            let first = tool_sequence[0];
            let second = tool_sequence[1];

            if first != second {
                // Check if the sequence alternates between first and second
                let mut alternates = true;
                for (i, &tool) in tool_sequence.iter().enumerate() {
                    let expected = if i % 2 == 0 { first } else { second };
                    if tool != expected {
                        alternates = false;
                        break;
                    }
                }
                if alternates {
                    return true;
                }
            }
        }

        // Detect longer repeating patterns
        for pattern_len in 2..tool_sequence.len() / 2 {
            if tool_sequence.len() % pattern_len != 0 {
                continue;
            }

            let pattern = &tool_sequence[..pattern_len];
            let mut all_match = true;

            for chunk in tool_sequence.chunks(pattern_len) {
                if chunk != pattern {
                    all_match = false;
                    break;
                }
            }

            if all_match {
                return true;
            }
        }

        false
    }

    /// Reset detector state
    ///
    /// Call this when starting a new execution phase or after
    /// user intervention.
    pub fn reset(&mut self) {
        self.snapshots.clear();
        self.step_number = 0;
        self.call_tracker.clear();
    }

    /// Get current step number
    pub fn step_number(&self) -> usize {
        self.step_number
    }

    /// Get number of snapshots in history
    pub fn history_size(&self) -> usize {
        self.snapshots.len()
    }

    /// Check if detector has enough history to detect stalls
    pub fn can_detect(&self) -> bool {
        self.snapshots.len() >= self.config.stall_threshold
    }
}

impl Default for StallDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_args() -> HashMap<String, String> {
        let mut args = HashMap::new();
        args.insert("path".to_string(), ".".to_string());
        args
    }

    #[test]
    fn test_new_detector() {
        let detector = StallDetector::new();
        assert_eq!(detector.step_number(), 0);
        assert_eq!(detector.history_size(), 0);
        assert!(!detector.can_detect());
    }

    #[test]
    fn test_single_step_no_stall() {
        let mut detector = StallDetector::with_config(SafetyConfig {
            stall_threshold: 3,
            ..Default::default()
        });

        let args = make_args();
        let result = detector.record_step("file_read", &args, &[]);
        assert!(result.is_ok());
        assert_eq!(detector.step_number(), 1);
        assert!(!detector.can_detect()); // Not enough history yet
    }

    #[test]
    fn test_no_state_change_detection() {
        let mut detector = StallDetector::with_config(SafetyConfig {
            stall_threshold: 3,
            max_identical_calls: 10, // Allow more than stall_threshold for this test
            ..Default::default()
        });

        let args = make_args();

        // Three identical steps
        let _ = detector.record_step("file_read", &args, &[]);
        assert!(!detector.can_detect());

        let _ = detector.record_step("file_read", &args, &[]);
        assert!(!detector.can_detect());

        let result = detector.record_step("file_read", &args, &[]);
        assert!(matches!(result, Err(StallReason::NoStateChange)));
        assert!(detector.can_detect());
    }

    #[test]
    fn test_different_tools_prevent_stall() {
        let mut detector = StallDetector::with_config(SafetyConfig {
            stall_threshold: 3,
            max_identical_calls: 10, // Allow more for this test
            ..Default::default()
        });

        let args = make_args();

        // Different tools prevent stall
        let _ = detector.record_step("file_read", &args, &[]);
        let _ = detector.record_step("file_search", &args, &[]);
        let result = detector.record_step("file_glob", &args, &[]);

        // Should not stall because tools differ
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_modification_prevents_stall() {
        let mut detector = StallDetector::with_config(SafetyConfig {
            stall_threshold: 3,
            max_identical_calls: 10, // Allow more for this test
            ..Default::default()
        });

        let args = make_args();

        // Same tool but different files modified
        let _ = detector.record_step("file_write", &args, &[PathBuf::from("file1.txt")]);
        let _ = detector.record_step("file_write", &args, &[PathBuf::from("file2.txt")]);
        let result = detector.record_step("file_write", &args, &[PathBuf::from("file3.txt")]);

        // Should not stall because state is changing
        assert!(result.is_ok());
    }

    #[test]
    fn test_identical_call_detection() {
        let mut detector = StallDetector::with_config(SafetyConfig {
            max_identical_calls: 2,
            stall_threshold: 5,
            ..Default::default()
        });

        let args = make_args();

        // First call
        let _ = detector.record_step("file_read", &args, &[]);
        assert!(detector.record_step("file_read", &args, &[]).is_ok());

        // Third identical call should trigger stall
        let result = detector.record_step("file_read", &args, &[]);
        assert!(matches!(result, Err(StallReason::IdenticalCalls)));
    }

    #[test]
    fn test_different_args_allow_same_tool() {
        let mut detector = StallDetector::with_config(SafetyConfig {
            max_identical_calls: 2,
            stall_threshold: 5,
            ..Default::default()
        });

        let mut args1 = HashMap::new();
        args1.insert("path".to_string(), "file1.txt".to_string());

        let mut args2 = HashMap::new();
        args2.insert("path".to_string(), "file2.txt".to_string());

        // Same tool, different args — should not trigger identical call detection
        let _ = detector.record_step("file_read", &args1, &[]);
        let _ = detector.record_step("file_read", &args2, &[]);
        let _ = detector.record_step("file_read", &args1, &[]);
        let _ = detector.record_step("file_read", &args2, &[]);

        // Each unique (tool, args) pair counted separately
        let result = detector.record_step("file_read", &args1, &[]);
        // Now we've called file_read(file1.txt) 3 times
        assert!(matches!(result, Err(StallReason::IdenticalCalls)));
    }

    #[test]
    fn test_reset_clears_state() {
        let mut detector = StallDetector::with_config(SafetyConfig {
            stall_threshold: 3,
            max_identical_calls: 10, // Allow more for this test
            ..Default::default()
        });

        let args = make_args();

        // Create some history
        let _ = detector.record_step("file_read", &args, &[]);
        let _ = detector.record_step("file_read", &args, &[]);
        let _ = detector.record_step("file_read", &args, &[]);

        assert_eq!(detector.step_number(), 3);
        assert!(detector.can_detect()); // 3 >= threshold of 3

        // Reset
        detector.reset();

        assert_eq!(detector.step_number(), 0);
        assert_eq!(detector.history_size(), 0);
        assert!(!detector.can_detect());
    }

    #[test]
    fn test_tool_loop_detection() {
        let mut detector = StallDetector::with_config(SafetyConfig {
            stall_threshold: 3,
            max_identical_calls: 10, // Allow more for this test
            ..Default::default()
        });

        let args = make_args();

        // Create a pattern: A, B, A, B, A, B
        let tools = vec!["file_read", "file_search", "file_read", "file_search", "file_read"];
        for tool in tools {
            let _ = detector.record_step(tool, &args, &[]);
        }

        // Should detect tool loop
        let result = detector.record_step("file_search", &args, &[]);
        assert!(matches!(result, Err(StallReason::ToolLoop)));
    }

    #[test]
    fn test_stall_reason_display() {
        assert_eq!(
            format!("{}", StallReason::NoStateChange),
            "No state change across multiple steps"
        );
        assert_eq!(
            format!("{}", StallReason::ToolLoop),
            "Repeated tool call sequence detected"
        );
        assert_eq!(
            format!("{}", StallReason::IdenticalCalls),
            "Identical tool calls detected"
        );
    }

    #[test]
    fn test_config_respected() {
        let mut detector = StallDetector::with_config(SafetyConfig {
            stall_threshold: 5,  // Higher threshold
            max_identical_calls: 10, // Allow more for this test
            ..Default::default()
        });

        let args = make_args();

        // With threshold of 5, need 5 identical steps
        for _ in 0..4 {
            let _ = detector.record_step("file_read", &args, &[]);
        }
        assert!(!detector.can_detect());

        // Fifth step enables detection
        let _ = detector.record_step("file_read", &args, &[]);
        assert!(detector.can_detect());

        // And should trigger stall
        let result = detector.record_step("file_read", &args, &[]);
        assert!(matches!(result, Err(StallReason::NoStateChange)));
    }

    #[test]
    fn test_empty_step_succeeds() {
        let mut detector = StallDetector::new();
        let args = HashMap::new();
        let result = detector.record_step("file_read", &args, &[]);
        assert!(result.is_ok());
    }
}
