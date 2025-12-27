//! Safety configuration for tool execution
//!
//! Defines limits and budgets for tool execution to prevent:
//! - Infinite loops (max identical calls)
//! - Resource exhaustion (per-tool timeout, session budget)
//! - Token spam (output truncation)
//! - Stalls (no state change detection)

use serde::{Deserialize, Serialize};

/// Configuration for safety limits and budgets
///
/// All limits are designed to be failsafe — when exceeded,
/// execution stops with a clear error message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Maximum tool calls per single turn/loop
    pub max_tool_calls_per_turn: usize,

    /// Maximum identical tool calls (same tool + args)
    pub max_identical_calls: usize,

    /// Per-tool timeout in milliseconds
    pub tool_timeout_ms: u64,

    /// Total execution budget for a session
    pub session_execution_budget: usize,

    /// Number of steps without state change to detect stall
    pub stall_threshold: usize,

    /// Maximum output characters before truncation
    pub output_truncate_chars: usize,

    /// Circuit breaker failure threshold
    pub circuit_breaker_failure_threshold: usize,

    /// Circuit breaker success threshold (to close again)
    pub circuit_breaker_success_threshold: usize,

    /// Circuit breaker open timeout in milliseconds
    pub circuit_breaker_open_timeout_ms: u64,

    /// Circuit breaker half-open max calls (for testing)
    pub circuit_breaker_half_open_max_calls: usize,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            max_tool_calls_per_turn: 20,
            max_identical_calls: 2,
            tool_timeout_ms: 30_000,  // 30 seconds
            session_execution_budget: 100,
            stall_threshold: 5,
            output_truncate_chars: 10_000,
            circuit_breaker_failure_threshold: 5,
            circuit_breaker_success_threshold: 2,
            circuit_breaker_open_timeout_ms: 30_000,  // 30 seconds
            circuit_breaker_half_open_max_calls: 3,
        }
    }
}

impl SafetyConfig {
    /// Create configuration with custom values
    ///
    /// # Panics
    ///
    /// Panics if any limit is set to zero (would disable safety).
    pub fn new(
        max_tool_calls_per_turn: usize,
        max_identical_calls: usize,
        tool_timeout_ms: u64,
        session_execution_budget: usize,
    ) -> Self {
        assert!(max_tool_calls_per_turn > 0, "max_tool_calls_per_turn must be > 0");
        assert!(max_identical_calls > 0, "max_identical_calls must be > 0");
        assert!(tool_timeout_ms > 0, "tool_timeout_ms must be > 0");
        assert!(
            session_execution_budget > 0,
            "session_execution_budget must be > 0"
        );

        Self {
            max_tool_calls_per_turn,
            max_identical_calls,
            tool_timeout_ms,
            session_execution_budget,
            ..Default::default()
        }
    }

    /// Create permissive configuration for testing
    #[cfg(test)]
    pub fn permissive() -> Self {
        Self {
            max_tool_calls_per_turn: 1000,
            max_identical_calls: 100,
            tool_timeout_ms: 300_000,  // 5 minutes
            session_execution_budget: 10_000,
            stall_threshold: 100,
            output_truncate_chars: 1_000_000,
            circuit_breaker_failure_threshold: 100,
            circuit_breaker_success_threshold: 1,
            circuit_breaker_open_timeout_ms: 1000,
            circuit_breaker_half_open_max_calls: 10,
        }
    }

    /// Create restrictive configuration for safety-critical contexts
    pub fn restrictive() -> Self {
        Self {
            max_tool_calls_per_turn: 10,
            max_identical_calls: 1,
            tool_timeout_ms: 10_000,  // 10 seconds
            session_execution_budget: 50,
            stall_threshold: 3,
            output_truncate_chars: 5_000,
            circuit_breaker_failure_threshold: 3,
            circuit_breaker_success_threshold: 3,
            circuit_breaker_open_timeout_ms: 60_000,  // 1 minute
            circuit_breaker_half_open_max_calls: 1,
        }
    }

    /// Validate that configuration values are sensible
    ///
    /// Returns Err if any value is out of acceptable range.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_tool_calls_per_turn > 100 {
            return Err(format!(
                "max_tool_calls_per_turn ({}) exceeds recommended maximum (100)",
                self.max_tool_calls_per_turn
            ));
        }
        if self.tool_timeout_ms > 300_000 {
            return Err(format!(
                "tool_timeout_ms ({}) exceeds recommended maximum (300000 = 5 minutes)",
                self.tool_timeout_ms
            ));
        }
        if self.stall_threshold < 2 {
            return Err(format!(
                "stall_threshold ({}) is too small (minimum 2)",
                self.stall_threshold
            ));
        }
        if self.circuit_breaker_failure_threshold < 2 {
            return Err(format!(
                "circuit_breaker_failure_threshold ({}) is too small (minimum 2)",
                self.circuit_breaker_failure_threshold
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SafetyConfig::default();
        assert_eq!(config.max_tool_calls_per_turn, 20);
        assert_eq!(config.max_identical_calls, 2);
        assert_eq!(config.tool_timeout_ms, 30_000);
        assert_eq!(config.session_execution_budget, 100);
        assert_eq!(config.stall_threshold, 5);
        assert_eq!(config.output_truncate_chars, 10_000);
    }

    #[test]
    fn test_new_config() {
        let config = SafetyConfig::new(50, 3, 60_000, 200);
        assert_eq!(config.max_tool_calls_per_turn, 50);
        assert_eq!(config.max_identical_calls, 3);
        assert_eq!(config.tool_timeout_ms, 60_000);
        assert_eq!(config.session_execution_budget, 200);
    }

    #[test]
    #[should_panic(expected = "max_tool_calls_per_turn must be > 0")]
    fn test_new_panics_on_zero_turn_limit() {
        SafetyConfig::new(0, 2, 30_000, 100);
    }

    #[test]
    #[should_panic(expected = "max_identical_calls must be > 0")]
    fn test_new_panics_on_zero_identical() {
        SafetyConfig::new(20, 0, 30_000, 100);
    }

    #[test]
    #[should_panic(expected = "tool_timeout_ms must be > 0")]
    fn test_new_panics_on_zero_timeout() {
        SafetyConfig::new(20, 2, 0, 100);
    }

    #[test]
    #[should_panic(expected = "session_execution_budget must be > 0")]
    fn test_new_panics_on_zero_budget() {
        SafetyConfig::new(20, 2, 30_000, 0);
    }

    #[test]
    fn test_permissive_config() {
        let config = SafetyConfig::permissive();
        assert_eq!(config.max_tool_calls_per_turn, 1000);
        assert_eq!(config.max_identical_calls, 100);
        assert_eq!(config.tool_timeout_ms, 300_000);
        assert_eq!(config.session_execution_budget, 10_000);
    }

    #[test]
    fn test_restrictive_config() {
        let config = SafetyConfig::restrictive();
        assert_eq!(config.max_tool_calls_per_turn, 10);
        assert_eq!(config.max_identical_calls, 1);
        assert_eq!(config.tool_timeout_ms, 10_000);
        assert_eq!(config.session_execution_budget, 50);
    }

    #[test]
    fn test_validate_default() {
        let config = SafetyConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_excessive_turns() {
        let config = SafetyConfig {
            max_tool_calls_per_turn: 101,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_excessive_timeout() {
        let config = SafetyConfig {
            tool_timeout_ms: 300_001,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_too_small_stall_threshold() {
        let config = SafetyConfig {
            stall_threshold: 1,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_too_small_circuit_threshold() {
        let config = SafetyConfig {
            circuit_breaker_failure_threshold: 1,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }
}
