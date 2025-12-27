//! Phase 9.5: Tool Execution State Machine
//!
//! Provides explicit state machine for tool execution lifecycle:
//! QUEUED → RUNNING → COMPLETED | FAILED | TIMEOUT | CANCELLED
//!
//! NO silent transitions. Every state change must be explicit and visible.

use std::time::{Duration, Instant};

/// Explicit tool execution state (Phase 9.5)
///
/// State machine lifecycle:
/// Queued → Running → Completed | Failed | Timeout | Cancelled
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolExecutionState {
    /// Tool is queued, waiting to execute
    Queued,
    /// Tool is currently running
    Running { started_at: Instant },
    /// Tool completed successfully
    Completed { duration_ms: u64 },
    /// Tool failed with error
    Failed { error: String },
    /// Tool exceeded timeout limit
    Timeout,
    /// Tool was cancelled by user
    Cancelled,
}

impl ToolExecutionState {
    /// Create a new Running state with current timestamp
    pub fn running() -> Self {
        Self::Running {
            started_at: Instant::now(),
        }
    }

    /// Transition from Running to Completed
    pub fn to_completed(self, duration_ms: u64) -> Self {
        Self::Completed { duration_ms }
    }

    /// Transition from Running to Failed
    pub fn to_failed(self, error: String) -> Self {
        Self::Failed { error }
    }

    /// Check if this state has timed out given the timeout duration
    pub fn check_timeout(&self, timeout: Duration) -> bool {
        match self {
            Self::Running { started_at } => started_at.elapsed() > timeout,
            _ => false,
        }
    }

    /// Get display name for this state
    pub fn display_name(&self) -> &str {
        match self {
            Self::Queued => "QUEUED",
            Self::Running { .. } => "RUNNING",
            Self::Completed { .. } => "COMPLETED",
            Self::Failed { .. } => "FAILED",
            Self::Timeout => "TIMEOUT",
            Self::Cancelled => "CANCELLED",
        }
    }

    /// Check if state is terminal (no further transitions)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::Failed { .. } | Self::Timeout | Self::Cancelled
        )
    }

    /// Check if state is active (can still transition)
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Queued | Self::Running { .. })
    }

    /// Get elapsed time if Running
    pub fn elapsed_ms(&self) -> Option<u64> {
        match self {
            Self::Running { started_at } => Some(started_at.elapsed().as_millis() as u64),
            _ => None,
        }
    }
}

/// Entry in the tool execution queue (Phase 9.5)
#[derive(Debug, Clone)]
pub struct ToolQueueEntry {
    /// Tool name
    pub tool: String,
    /// Step number in the loop
    pub step: usize,
    /// Affected file path (if any)
    pub affected_path: Option<String>,
    /// Current execution state
    pub state: ToolExecutionState,
}

impl ToolQueueEntry {
    /// Create a new queued tool entry
    pub fn new(tool: String, step: usize, affected_path: Option<String>) -> Self {
        Self {
            tool,
            step,
            affected_path,
            state: ToolExecutionState::Queued,
        }
    }

    /// Start tool execution (transition to Running)
    pub fn start(&mut self) {
        self.state = ToolExecutionState::running();
    }

    /// Complete tool execution (transition to Completed)
    pub fn complete(&mut self, duration_ms: u64) {
        self.state = ToolExecutionState::Completed { duration_ms };
    }

    /// Mark tool as failed (transition to Failed)
    pub fn fail(&mut self, error: String) {
        self.state = ToolExecutionState::Failed { error };
    }

    /// Mark tool as timed out (transition to Timeout)
    pub fn timeout(&mut self) {
        self.state = ToolExecutionState::Timeout;
    }

    /// Cancel tool execution (transition to Cancelled)
    pub fn cancel(&mut self) {
        self.state = ToolExecutionState::Cancelled;
    }

    /// Check if this entry has timed out
    pub fn is_timed_out(&self, timeout: Duration) -> bool {
        self.state.check_timeout(timeout)
    }

    /// Get display text for this entry
    pub fn display_text(&self) -> String {
        let state_name = self.state.display_name();
        let elapsed = if let Some(ms) = self.state.elapsed_ms() {
            format!("{}ms", ms)
        } else {
            "-".to_string()
        };

        format!(
            "Step {} | Tool: {} | State: {} | Elapsed: {} | Path: {}",
            self.step,
            self.tool,
            state_name,
            elapsed,
            self.affected_path.as_deref().unwrap_or("-")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_queued_display() {
        assert_eq!(ToolExecutionState::Queued.display_name(), "QUEUED");
    }

    #[test]
    fn test_state_running_display() {
        assert_eq!(ToolExecutionState::running().display_name(), "RUNNING");
    }

    #[test]
    fn test_state_completed_display() {
        let state = ToolExecutionState::Completed { duration_ms: 100 };
        assert_eq!(state.display_name(), "COMPLETED");
    }

    #[test]
    fn test_state_failed_display() {
        let state = ToolExecutionState::Failed {
            error: "test".to_string(),
        };
        assert_eq!(state.display_name(), "FAILED");
    }

    #[test]
    fn test_state_timeout_display() {
        assert_eq!(ToolExecutionState::Timeout.display_name(), "TIMEOUT");
    }

    #[test]
    fn test_state_cancelled_display() {
        assert_eq!(ToolExecutionState::Cancelled.display_name(), "CANCELLED");
    }

    #[test]
    fn test_state_is_terminal() {
        assert!(!ToolExecutionState::Queued.is_terminal());
        assert!(!ToolExecutionState::running().is_terminal());
        assert!(ToolExecutionState::Completed { duration_ms: 100 }.is_terminal());
        assert!(ToolExecutionState::Failed {
            error: "x".to_string()
        }
        .is_terminal());
        assert!(ToolExecutionState::Timeout.is_terminal());
        assert!(ToolExecutionState::Cancelled.is_terminal());
    }

    #[test]
    fn test_state_is_active() {
        assert!(ToolExecutionState::Queued.is_active());
        assert!(ToolExecutionState::running().is_active());
        assert!(!ToolExecutionState::Completed { duration_ms: 100 }.is_active());
        assert!(!ToolExecutionState::Failed {
            error: "x".to_string()
        }
        .is_active());
        assert!(!ToolExecutionState::Timeout.is_active());
        assert!(!ToolExecutionState::Cancelled.is_active());
    }

    #[test]
    fn test_entry_creation() {
        let entry = ToolQueueEntry::new("file_read".to_string(), 1, Some("path".to_string()));
        assert_eq!(entry.tool, "file_read");
        assert_eq!(entry.step, 1);
        assert!(matches!(entry.state, ToolExecutionState::Queued));
    }

    #[test]
    fn test_entry_start_transition() {
        let mut entry = ToolQueueEntry::new("test".to_string(), 1, None);
        entry.start();
        assert!(matches!(entry.state, ToolExecutionState::Running { .. }));
    }

    #[test]
    fn test_entry_complete_transition() {
        let mut entry = ToolQueueEntry::new("test".to_string(), 1, None);
        entry.start();
        entry.complete(100);
        assert!(matches!(entry.state, ToolExecutionState::Completed { .. }));
    }

    #[test]
    fn test_entry_fail_transition() {
        let mut entry = ToolQueueEntry::new("test".to_string(), 1, None);
        entry.start();
        entry.fail("error".to_string());
        assert!(matches!(entry.state, ToolExecutionState::Failed { .. }));
    }

    #[test]
    fn test_entry_cancel_transition() {
        let mut entry = ToolQueueEntry::new("test".to_string(), 1, None);
        entry.cancel();
        assert!(matches!(entry.state, ToolExecutionState::Cancelled));
    }

    #[test]
    fn test_timeout_check() {
        let state = ToolExecutionState::running();
        // Should not timeout immediately with a reasonable timeout
        assert!(!state.check_timeout(Duration::from_secs(10)));
        // Should timeout with zero timeout
        std::thread::sleep(Duration::from_millis(10));
        assert!(state.check_timeout(Duration::from_secs(0)));
    }
}
