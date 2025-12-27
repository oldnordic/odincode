//! Execution budget tracking for sessions
//!
//! Enforces per-session limits on tool execution to prevent:
//! - Resource exhaustion from runaway LLMs
//! - Excessive tool calls in a single session
//! - Cost overruns from paid APIs
//!
//! The budget is reset when starting a new session.

use std::collections::HashMap;

use super::safety_config::SafetyConfig;

/// Execution budget for a tool
#[derive(Debug, Clone, Copy)]
pub struct ToolBudget {
    /// Calls made in current session
    calls: usize,
    /// Calls allowed per session
    limit: usize,
}

impl ToolBudget {
    fn new(limit: usize) -> Self {
        Self { calls: 0, limit }
    }

    /// Remaining calls
    pub fn remaining(&self) -> usize {
        self.limit.saturating_sub(self.calls)
    }

    /// Whether budget is exhausted
    pub fn is_exhausted(&self) -> bool {
        self.calls >= self.limit
    }

    /// Record a tool call
    ///
    /// Returns Ok if call is within budget, Err if exhausted.
    fn record_call(&mut self) -> Result<(), BudgetError> {
        if self.calls >= self.limit {
            return Err(BudgetError::Exhausted {
                limit: self.limit,
                used: self.calls,
            });
        }
        self.calls += 1;
        Ok(())
    }

    /// Get current call count
    pub fn calls(&self) -> usize {
        self.calls
    }
}

/// Budget error
#[derive(Debug, Clone)]
pub enum BudgetError {
    /// Budget exhausted
    Exhausted { limit: usize, used: usize },
    /// Session budget exhausted
    SessionExhausted { limit: usize, used: usize },
}

impl std::fmt::Display for BudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetError::Exhausted { limit, used } => {
                write!(f, "Tool budget exhausted: {}/{} calls used", used, limit)
            }
            BudgetError::SessionExhausted { limit, used } => {
                write!(f, "Session budget exhausted: {}/{} calls used", used, limit)
            }
        }
    }
}

impl std::error::Error for BudgetError {}

/// Per-tool execution budget tracker
#[derive(Debug)]
pub struct ToolBudgetTracker {
    /// Individual tool budgets
    tools: HashMap<String, ToolBudget>,
    /// Total session calls
    session_calls: usize,
    /// Session budget limit
    session_limit: usize,
    /// Maximum calls per turn
    turn_limit: usize,
    /// Calls in current turn
    turn_calls: usize,
}

impl ToolBudgetTracker {
    /// Create new budget tracker with default configuration
    pub fn new() -> Self {
        Self::with_config(SafetyConfig::default())
    }

    /// Create new budget tracker with custom configuration
    pub fn with_config(config: SafetyConfig) -> Self {
        Self {
            tools: HashMap::new(),
            session_calls: 0,
            session_limit: config.session_execution_budget,
            turn_limit: config.max_tool_calls_per_turn,
            turn_calls: 0,
        }
    }

    /// Check if a tool call is within budget
    ///
    /// Returns Ok if the call can proceed, Err if any limit is exceeded.
    pub fn check_call(&mut self, tool: &str) -> Result<(), BudgetError> {
        // Check session budget first
        if self.session_calls >= self.session_limit {
            return Err(BudgetError::SessionExhausted {
                limit: self.session_limit,
                used: self.session_calls,
            });
        }

        // Check turn budget
        if self.turn_calls >= self.turn_limit {
            return Err(BudgetError::Exhausted {
                limit: self.turn_limit,
                used: self.turn_calls,
            });
        }

        // Get or create tool budget
        let budget = self.tools.entry(tool.to_string()).or_insert_with(|| {
            // Default per-tool limit is a fraction of session limit
            ToolBudget::new(self.session_limit.max(10))
        });

        // Check tool budget
        budget.record_call()?;

        Ok(())
    }

    /// Record a tool call (after successful execution)
    ///
    /// Only call this after `check_call` succeeds and the tool executes.
    pub fn record_call(&mut self, _tool: &str) {
        self.session_calls += 1;
        self.turn_calls += 1;
    }

    /// Start a new turn (reset turn counter)
    pub fn new_turn(&mut self) {
        self.turn_calls = 0;
    }

    /// Start a new session (reset all counters)
    pub fn new_session(&mut self) {
        self.tools.clear();
        self.session_calls = 0;
        self.turn_calls = 0;
    }

    /// Get total session calls
    pub fn session_calls(&self) -> usize {
        self.session_calls
    }

    /// Get calls in current turn
    pub fn turn_calls(&self) -> usize {
        self.turn_calls
    }

    /// Get remaining session budget
    pub fn session_remaining(&self) -> usize {
        self.session_limit.saturating_sub(self.session_calls)
    }

    /// Get remaining turn budget
    pub fn turn_remaining(&self) -> usize {
        self.turn_limit.saturating_sub(self.turn_calls)
    }

    /// Get calls for a specific tool
    pub fn tool_calls(&self, tool: &str) -> usize {
        self.tools
            .get(tool)
            .map(|b| b.calls())
            .unwrap_or(0)
    }

    /// Check if session budget is exhausted
    pub fn is_session_exhausted(&self) -> bool {
        self.session_calls >= self.session_limit
    }

    /// Check if turn budget is exhausted
    pub fn is_turn_exhausted(&self) -> bool {
        self.turn_calls >= self.turn_limit
    }
}

impl Default for ToolBudgetTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tracker() {
        let tracker = ToolBudgetTracker::new();
        assert_eq!(tracker.session_calls(), 0);
        assert_eq!(tracker.turn_calls(), 0);
        assert!(!tracker.is_session_exhausted());
        assert!(!tracker.is_turn_exhausted());
    }

    #[test]
    fn test_check_and_record_call() {
        let mut tracker = ToolBudgetTracker::new();

        tracker.check_call("file_read").unwrap();
        tracker.record_call("file_read");

        assert_eq!(tracker.session_calls(), 1);
        assert_eq!(tracker.turn_calls(), 1);
        assert_eq!(tracker.tool_calls("file_read"), 1);
    }

    #[test]
    fn test_turn_limit_enforcement() {
        let mut tracker = ToolBudgetTracker::with_config(SafetyConfig {
            max_tool_calls_per_turn: 2,
            session_execution_budget: 100,
            ..Default::default()
        });

        tracker.check_call("file_read").unwrap();
        tracker.record_call("file_read");

        tracker.check_call("file_search").unwrap();
        tracker.record_call("file_search");

        // Third call should fail turn limit
        let result = tracker.check_call("file_glob");
        assert!(matches!(result, Err(BudgetError::Exhausted { .. })));
    }

    #[test]
    fn test_new_turn_resets_turn_counter() {
        let mut tracker = ToolBudgetTracker::with_config(SafetyConfig {
            max_tool_calls_per_turn: 2,
            session_execution_budget: 100,
            ..Default::default()
        });

        // Use up turn budget
        tracker.check_call("file_read").unwrap();
        tracker.record_call("file_read");
        tracker.check_call("file_search").unwrap();
        tracker.record_call("file_search");

        assert!(tracker.is_turn_exhausted());

        // New turn resets turn counter
        tracker.new_turn();
        assert!(!tracker.is_turn_exhausted());
        assert_eq!(tracker.turn_calls(), 0);

        // Can call again
        tracker.check_call("file_glob").unwrap();
    }

    #[test]
    fn test_session_limit_enforcement() {
        let mut tracker = ToolBudgetTracker::with_config(SafetyConfig {
            max_tool_calls_per_turn: 100,
            session_execution_budget: 3,
            ..Default::default()
        });

        tracker.check_call("file_read").unwrap();
        tracker.record_call("file_read");

        tracker.check_call("file_search").unwrap();
        tracker.record_call("file_search");

        tracker.check_call("file_glob").unwrap();
        tracker.record_call("file_glob");

        // Fourth call should fail session limit
        let result = tracker.check_call("file_read");
        assert!(matches!(result, Err(BudgetError::SessionExhausted { .. })));
    }

    #[test]
    fn test_new_session_resets_all() {
        let mut tracker = ToolBudgetTracker::with_config(SafetyConfig {
            max_tool_calls_per_turn: 5,
            session_execution_budget: 10,
            ..Default::default()
        });

        // Use some budget
        tracker.check_call("file_read").unwrap();
        tracker.record_call("file_read");
        tracker.check_call("file_search").unwrap();
        tracker.record_call("file_search");

        assert_eq!(tracker.session_calls(), 2);
        assert_eq!(tracker.tool_calls("file_read"), 1);
        assert_eq!(tracker.tool_calls("file_search"), 1);

        // New session resets
        tracker.new_session();
        assert_eq!(tracker.session_calls(), 0);
        assert_eq!(tracker.turn_calls(), 0);
        assert_eq!(tracker.tool_calls("file_read"), 0);
        assert_eq!(tracker.tool_calls("file_search"), 0);
    }

    #[test]
    fn test_remaining_calculations() {
        let mut tracker = ToolBudgetTracker::with_config(SafetyConfig {
            max_tool_calls_per_turn: 10,
            session_execution_budget: 50,
            ..Default::default()
        });

        assert_eq!(tracker.session_remaining(), 50);
        assert_eq!(tracker.turn_remaining(), 10);

        tracker.check_call("file_read").unwrap();
        tracker.record_call("file_read");

        assert_eq!(tracker.session_remaining(), 49);
        assert_eq!(tracker.turn_remaining(), 9);
    }

    #[test]
    fn test_multiple_tools_tracked_separately() {
        let mut tracker = ToolBudgetTracker::new();

        tracker.check_call("file_read").unwrap();
        tracker.record_call("file_read");

        tracker.check_call("file_search").unwrap();
        tracker.record_call("file_search");

        tracker.check_call("file_read").unwrap();
        tracker.record_call("file_read");

        assert_eq!(tracker.tool_calls("file_read"), 2);
        assert_eq!(tracker.tool_calls("file_search"), 1);
        assert_eq!(tracker.session_calls(), 3);
    }

    #[test]
    fn test_budget_error_display() {
        let err = BudgetError::Exhausted {
            limit: 10,
            used: 10,
        };
        assert_eq!(format!("{}", err), "Tool budget exhausted: 10/10 calls used");

        let err = BudgetError::SessionExhausted {
            limit: 100,
            used: 100,
        };
        assert_eq!(format!("{}", err), "Session budget exhausted: 100/100 calls used");
    }

    #[test]
    fn test_turn_counter_independent_of_session() {
        let mut tracker = ToolBudgetTracker::with_config(SafetyConfig {
            max_tool_calls_per_turn: 2,
            session_execution_budget: 10,
            ..Default::default()
        });

        // Fill turn budget
        tracker.check_call("tool1").unwrap();
        tracker.record_call("tool1");
        tracker.check_call("tool2").unwrap();
        tracker.record_call("tool2");

        assert!(tracker.is_turn_exhausted());
        assert!(!tracker.is_session_exhausted());

        // New turn allows more calls
        tracker.new_turn();
        tracker.check_call("tool3").unwrap();
        tracker.record_call("tool3");
    }

    #[test]
    fn test_check_without_record_doesnt_count() {
        let mut tracker = ToolBudgetTracker::new();

        // Check but don't record
        tracker.check_call("file_read").unwrap();

        // Shouldn't count yet
        assert_eq!(tracker.session_calls(), 0);

        // Now record
        tracker.record_call("file_read");
        assert_eq!(tracker.session_calls(), 1);
    }

    #[test]
    fn test_tool_budget_independent() {
        let mut tracker = ToolBudgetTracker::with_config(SafetyConfig {
            max_tool_calls_per_turn: 100, // High enough for this test
            session_execution_budget: 100,
            ..Default::default()
        });

        // Each tool gets its own budget (fraction of session)
        for _ in 0..50 {
            tracker.check_call("file_read").unwrap();
            tracker.record_call("file_read");
        }

        assert_eq!(tracker.tool_calls("file_read"), 50);

        // Other tools still have budget
        tracker.check_call("file_search").unwrap();
        assert_eq!(tracker.tool_calls("file_search"), 1);
    }
}
