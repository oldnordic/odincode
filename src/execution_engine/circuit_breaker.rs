//! Circuit breaker for tool execution
//!
//! Prevents cascading failures by blocking execution after repeated failures.
//!
//! # State Machine
//!
//! ```text
//!   CLOSED ──(failure_threshold)──▶ OPEN ──(timeout)──▶ HALF_OPEN
//!     ◲                                    │                    │
//!     │                                    (timeout)           │
//!     │                                    │                    │
//!     └────────────(success_threshold)─────◘────────────────────┘
//!                     (from HALF_OPEN)
//! ```
//!
//! - **CLOSED**: Normal operation, all requests execute
//! - **OPEN**: Circuit tripped, requests fail immediately
//! - **HALF_OPEN**: Testing if system has recovered, limited requests execute

use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::fmt;

use super::safety_config::SafetyConfig;

/// Circuit state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation — all requests execute
    Closed,
    /// Circuit tripped — requests fail immediately
    Open,
    /// Testing recovery — limited requests execute
    HalfOpen,
}

impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "CLOSED"),
            CircuitState::Open => write!(f, "OPEN"),
            CircuitState::HalfOpen => write!(f, "HALF_OPEN"),
        }
    }
}

/// Circuit breaker error
#[derive(Debug, Clone)]
pub enum CircuitError {
    /// Circuit is open, blocking execution
    CircuitOpen { tool: String },
    /// Execution failed
    ExecutionFailed { error: String },
}

impl fmt::Display for CircuitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitError::CircuitOpen { tool } => {
                write!(f, "Circuit breaker OPEN for tool '{}'", tool)
            }
            CircuitError::ExecutionFailed { error } => {
                write!(f, "Execution failed: {}", error)
            }
        }
    }
}

impl std::error::Error for CircuitError {}

/// Per-tool circuit breaker state
#[derive(Debug)]
struct ToolCircuitBreaker {
    state: CircuitState,
    failure_count: usize,
    success_count: usize,
    last_failure_time: Option<Instant>,
    half_open_calls: usize,
}

impl ToolCircuitBreaker {
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            half_open_calls: 0,
        }
    }

    fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.failure_count = 0;
        self.success_count = 0;
        self.last_failure_time = None;
        self.half_open_calls = 0;
    }
}

/// Circuit breaker for tool execution
///
/// Tracks failures per-tool and opens the circuit when threshold is exceeded.
pub struct CircuitBreaker {
    /// Per-tool circuit breakers
    tools: HashMap<String, ToolCircuitBreaker>,
    /// Configuration
    config: SafetyConfig,
}

impl CircuitBreaker {
    /// Create new circuit breaker with default configuration
    pub fn new() -> Self {
        Self::with_config(SafetyConfig::default())
    }

    /// Create new circuit breaker with custom configuration
    pub fn with_config(config: SafetyConfig) -> Self {
        Self {
            tools: HashMap::new(),
            config,
        }
    }

    /// Get circuit state for a tool
    pub fn state(&self, tool: &str) -> CircuitState {
        self.tools
            .get(tool)
            .map(|cb| cb.state)
            .unwrap_or(CircuitState::Closed)
    }

    /// Get failure count for a tool
    pub fn failure_count(&self, tool: &str) -> usize {
        self.tools
            .get(tool)
            .map(|cb| cb.failure_count)
            .unwrap_or(0)
    }

    /// Reset circuit breaker for a tool
    ///
    /// Forces the circuit back to CLOSED state, clearing all history.
    pub fn reset(&mut self, tool: &str) {
        if let Some(cb) = self.tools.get_mut(tool) {
            cb.reset();
        }
    }

    /// Reset all circuit breakers
    pub fn reset_all(&mut self) {
        self.tools.clear();
    }

    /// Try to execute a function through the circuit breaker
    ///
    /// # Errors
    ///
    /// Returns `CircuitError::CircuitOpen` if the circuit is OPEN.
    /// Returns `CircuitError::ExecutionFailed` if the function fails.
    pub fn try_execute<F, R>(
        &mut self,
        tool: &str,
        f: F,
    ) -> Result<R, CircuitError>
    where
        F: FnOnce() -> Result<R, Box<dyn std::error::Error>>,
    {
        let tool_key = tool.to_string();

        // First, check state and get mutable reference separately
        let (initial_state, can_reset) = {
            let cb = self.tools.entry(tool_key.clone()).or_insert_with(ToolCircuitBreaker::new);
            let can_reset = if cb.state == CircuitState::Open {
                if let Some(last_failure) = cb.last_failure_time {
                    let timeout = Duration::from_millis(self.config.circuit_breaker_open_timeout_ms);
                    last_failure.elapsed() >= timeout
                } else {
                    true
                }
            } else {
                false
            };
            (cb.state, can_reset)
        };

        // Handle OPEN state before getting mutable borrow for execution
        if initial_state == CircuitState::Open {
            if can_reset {
                let cb = self.tools.get_mut(&tool_key).unwrap();
                cb.state = CircuitState::HalfOpen;
                cb.success_count = 0;
                cb.half_open_calls = 0;
            } else {
                return Err(CircuitError::CircuitOpen { tool: tool.to_string() });
            }
        }

        // Now execute with proper state handling
        let result = {
            let cb = self.tools.get_mut(&tool_key).unwrap();

            // Check HALF_OPEN limit
            if cb.state == CircuitState::HalfOpen {
                if cb.half_open_calls >= self.config.circuit_breaker_half_open_max_calls {
                    return Err(CircuitError::CircuitOpen { tool: tool.to_string() });
                }
                cb.half_open_calls += 1;
            }

            // Execute
            f()
        };

        // Handle result
        let cb = self.tools.get_mut(&tool_key).unwrap();

        match result {
            Ok(ok) => {
                // Handle success
                cb.failure_count = 0;
                if cb.state == CircuitState::HalfOpen {
                    cb.success_count += 1;
                    // Only transition to CLOSED after success_threshold
                    // Note: half_open_max_calls is checked before execution
                    if cb.success_count >= self.config.circuit_breaker_success_threshold {
                        cb.state = CircuitState::Closed;
                        cb.failure_count = 0;
                        cb.success_count = 0;
                        cb.half_open_calls = 0;
                    }
                }
                Ok(ok)
            }
            Err(err) => {
                // Handle failure
                cb.failure_count += 1;
                cb.last_failure_time = Some(Instant::now());
                cb.success_count = 0;

                // In HALF_OPEN, any failure immediately trips back to OPEN
                let was_half_open = cb.state == CircuitState::HalfOpen;
                if cb.failure_count >= self.config.circuit_breaker_failure_threshold || was_half_open {
                    cb.state = CircuitState::Open;
                    cb.half_open_calls = 0;
                }

                Err(CircuitError::ExecutionFailed {
                    error: err.to_string(),
                })
            }
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_breaker_has_closed_state() {
        let breaker = CircuitBreaker::new();
        assert_eq!(breaker.state("file_read"), CircuitState::Closed);
        assert_eq!(breaker.failure_count("file_read"), 0);
    }

    #[test]
    fn test_success_does_not_open_circuit() {
        let mut breaker = CircuitBreaker::new();
        let result = breaker.try_execute("test_tool", || Ok::<(), Box<dyn std::error::Error>>(()));
        assert!(result.is_ok());
        assert_eq!(breaker.state("test_tool"), CircuitState::Closed);
    }

    #[test]
    fn test_failures_open_circuit() {
        let mut breaker = CircuitBreaker::with_config(SafetyConfig {
            circuit_breaker_failure_threshold: 3,
            ..Default::default()
        });

        // First failure
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error 1".into())
        });
        assert_eq!(breaker.state("test_tool"), CircuitState::Closed);
        assert_eq!(breaker.failure_count("test_tool"), 1);

        // Second failure
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error 2".into())
        });
        assert_eq!(breaker.state("test_tool"), CircuitState::Closed);
        assert_eq!(breaker.failure_count("test_tool"), 2);

        // Third failure trips the circuit
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error 3".into())
        });
        assert_eq!(breaker.state("test_tool"), CircuitState::Open);
        assert_eq!(breaker.failure_count("test_tool"), 3);
    }

    #[test]
    fn test_open_circuit_blocks_execution() {
        let mut breaker = CircuitBreaker::with_config(SafetyConfig {
            circuit_breaker_failure_threshold: 2,
            ..Default::default()
        });

        // Trip the circuit
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error 1".into())
        });
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error 2".into())
        });
        assert_eq!(breaker.state("test_tool"), CircuitState::Open);

        // Next call should be blocked
        let result = breaker.try_execute("test_tool", || {
            Ok::<(), Box<dyn std::error::Error>>(())
        });
        assert!(matches!(result, Err(CircuitError::CircuitOpen { .. })));
    }

    #[test]
    fn test_success_resets_failure_count() {
        let mut breaker = CircuitBreaker::with_config(SafetyConfig {
            circuit_breaker_failure_threshold: 3,
            ..Default::default()
        });

        // Two failures
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        assert_eq!(breaker.failure_count("test_tool"), 2);

        // Success resets count
        let _ = breaker.try_execute("test_tool", || Ok::<(), Box<dyn std::error::Error>>(()));
        assert_eq!(breaker.failure_count("test_tool"), 0);
    }

    #[test]
    fn test_reset_clears_circuit() {
        let mut breaker = CircuitBreaker::with_config(SafetyConfig {
            circuit_breaker_failure_threshold: 2,
            ..Default::default()
        });

        // Trip the circuit
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        assert_eq!(breaker.state("test_tool"), CircuitState::Open);

        // Reset
        breaker.reset("test_tool");
        assert_eq!(breaker.state("test_tool"), CircuitState::Closed);
        assert_eq!(breaker.failure_count("test_tool"), 0);
    }

    #[test]
    fn test_reset_all_clears_everything() {
        let mut breaker = CircuitBreaker::with_config(SafetyConfig {
            circuit_breaker_failure_threshold: 2,
            ..Default::default()
        });

        // Trip circuits for multiple tools
        let _ = breaker.try_execute("tool1", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        let _ = breaker.try_execute("tool1", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        let _ = breaker.try_execute("tool2", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        let _ = breaker.try_execute("tool2", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });

        assert_eq!(breaker.state("tool1"), CircuitState::Open);
        assert_eq!(breaker.state("tool2"), CircuitState::Open);

        breaker.reset_all();
        assert_eq!(breaker.state("tool1"), CircuitState::Closed);
        assert_eq!(breaker.state("tool2"), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_error_display() {
        let err = CircuitError::CircuitOpen {
            tool: "test_tool".to_string(),
        };
        assert_eq!(format!("{}", err), "Circuit breaker OPEN for tool 'test_tool'");

        let err = CircuitError::ExecutionFailed {
            error: "something went wrong".to_string(),
        };
        assert_eq!(format!("{}", err), "Execution failed: something went wrong");
    }

    #[test]
    fn test_half_open_to_closed_on_success() {
        let mut breaker = CircuitBreaker::with_config(SafetyConfig {
            circuit_breaker_failure_threshold: 2,
            circuit_breaker_success_threshold: 2,
            circuit_breaker_open_timeout_ms: 10,
            circuit_breaker_half_open_max_calls: 10,
            ..Default::default()
        });

        // Trip the circuit
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        assert_eq!(breaker.state("test_tool"), CircuitState::Open);

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(20));

        // First success in HALF_OPEN
        let _ = breaker.try_execute("test_tool", || Ok::<(), Box<dyn std::error::Error>>(()));
        assert_eq!(breaker.state("test_tool"), CircuitState::HalfOpen);

        // Second success closes the circuit
        let _ = breaker.try_execute("test_tool", || Ok::<(), Box<dyn std::error::Error>>(()));
        assert_eq!(breaker.state("test_tool"), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_reopens_on_failure() {
        let mut breaker = CircuitBreaker::with_config(SafetyConfig {
            circuit_breaker_failure_threshold: 2,
            circuit_breaker_open_timeout_ms: 10,
            ..Default::default()
        });

        // Trip the circuit
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        assert_eq!(breaker.state("test_tool"), CircuitState::Open);

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(20));

        // First success in HALF_OPEN
        let _ = breaker.try_execute("test_tool", || Ok::<(), Box<dyn std::error::Error>>(()));
        assert_eq!(breaker.state("test_tool"), CircuitState::HalfOpen);

        // Failure reopens the circuit
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error again".into())
        });
        assert_eq!(breaker.state("test_tool"), CircuitState::Open);
    }

    #[test]
    fn test_half_open_respects_max_calls() {
        let mut breaker = CircuitBreaker::with_config(SafetyConfig {
            circuit_breaker_failure_threshold: 2,
            circuit_breaker_success_threshold: 10, // Allow more successes in HALF_OPEN
            circuit_breaker_open_timeout_ms: 10,
            circuit_breaker_half_open_max_calls: 2,
            ..Default::default()
        });

        // Trip the circuit
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        let _ = breaker.try_execute("test_tool", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(20));

        // First call in HALF_OPEN
        let _ = breaker.try_execute("test_tool", || Ok::<(), Box<dyn std::error::Error>>(()));
        assert_eq!(breaker.state("test_tool"), CircuitState::HalfOpen);

        // Second call in HALF_OPEN
        let _ = breaker.try_execute("test_tool", || Ok::<(), Box<dyn std::error::Error>>(()));
        assert_eq!(breaker.state("test_tool"), CircuitState::HalfOpen);

        // Third call exceeds max_calls, should be blocked
        let result = breaker.try_execute("test_tool", || Ok::<(), Box<dyn std::error::Error>>(()));
        assert!(matches!(result, Err(CircuitError::CircuitOpen { .. })));
    }

    #[test]
    fn test_per_tool_isolation() {
        let mut breaker = CircuitBreaker::with_config(SafetyConfig {
            circuit_breaker_failure_threshold: 2,
            ..Default::default()
        });

        // Trip tool1 circuit
        let _ = breaker.try_execute("tool1", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        let _ = breaker.try_execute("tool1", || {
            Err::<(), Box<dyn std::error::Error>>("error".into())
        });
        assert_eq!(breaker.state("tool1"), CircuitState::Open);

        // tool2 should still be closed
        assert_eq!(breaker.state("tool2"), CircuitState::Closed);

        // tool2 should work normally
        let result = breaker.try_execute("tool2", || Ok::<(), Box<dyn std::error::Error>>(()));
        assert!(result.is_ok());
    }
}
