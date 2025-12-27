//! LLM types — Plan, Step, Intent, Evidence Summary, Authorization
//!
//! These types define the structured plan format that the LLM must produce.
//! No inference, no policy, no autonomous execution — just data structures.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// User intent classification
///
/// LLM must classify user input into one of these categories.
/// This is NOT a command; it's a planning hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Intent {
    /// Request information (read files, query symbols)
    Read,

    /// Modify code (splice_patch, file_write)
    Mutate,

    /// Search or inspect (file_search, file_glob)
    Query,

    /// Understand why something happened (explain evidence)
    Explain,
}

/// Internal prompt mode — constrains LLM behavior per intent
///
/// These are INJECTED INTERNAL PROMPTS, not visible to the user.
/// They constrain the LLM to prevent tool spam and runaway exploration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PromptMode {
    /// QUERY MODE — Aggregate/statistical answers only
    /// "How many files?", "Total LOC?", "Count in directory?"
    Query,

    /// EXPLORE MODE — Locate information, bounded search
    /// "Where is X?", "Show me files", "Find symbol"
    Explore,

    /// MUTATION MODE — Single-step edits with validation
    /// "Edit X", "Fix Y", "Refactor Z"
    Mutation,

    /// PRESENTATION MODE — Explain results only, no tools
    /// Final explanation after tools complete
    Presentation,
}

impl PromptMode {
    pub fn to_string(&self) -> &'static str {
        match self {
            PromptMode::Query => "QUERY",
            PromptMode::Explore => "EXPLORE",
            PromptMode::Mutation => "MUTATION",
            PromptMode::Presentation => "PRESENTATION",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PromptMode::Query => "Query Mode",
            PromptMode::Explore => "Explore Mode",
            PromptMode::Mutation => "Mutation Mode",
            PromptMode::Presentation => "Presentation Mode",
        }
    }

    /// Get allowed tools for this mode
    pub fn allowed_tools(&self) -> &'static [&'static str] {
        match self {
            PromptMode::Query => &[
                "count_files", "count_lines", "fs_stats", "wc", "memory_query",
            ],
            PromptMode::Explore => &[
                "file_search", "file_glob", "symbols_in_file",
                "references_to_symbol_name", "references_from_file_to_symbol_name",
                "file_read",
            ],
            PromptMode::Mutation => &[
                "memory_query", "magellan_query", "file_edit", "splice_patch",
                "lsp_check", "bash_exec",
            ],
            PromptMode::Presentation => &[],
        }
    }

    /// Get forbidden tools for this mode
    pub fn forbidden_tools(&self) -> &'static [&'static str] {
        match self {
            PromptMode::Query => &[
                "file_read", "file_search", "symbols_in_file", "references_to_symbol_name",
                "references_from_file_to_symbol_name", "splice_patch", "splice_plan",
                "file_edit", "file_write", "git_status", "git_diff", "git_log", "git_commit",
            ],
            PromptMode::Explore => &[
                "splice_patch", "splice_plan", "file_edit", "file_write", "file_create",
                "git_commit", "bash_exec",
            ],
            PromptMode::Mutation => &[],
            PromptMode::Presentation => &[
                "file_read", "file_write", "file_create", "file_search", "file_glob",
                "symbols_in_file", "references_to_symbol_name", "references_from_file_to_symbol_name",
                "splice_patch", "splice_plan", "file_edit", "lsp_check", "bash_exec",
                "git_status", "git_diff", "git_log", "git_commit", "memory_query",
                "count_files", "count_lines", "fs_stats", "wc",
            ],
        }
    }

    /// Get max tool calls allowed for this mode
    pub fn max_tool_calls(&self) -> usize {
        match self {
            PromptMode::Query => 2,     // count_files + count_lines
            PromptMode::Explore => 3,    // bounded search
            PromptMode::Mutation => 5,   // memory_query + edit + lsp_check + retries
            PromptMode::Presentation => 0, // no tools
        }
    }
}

impl Intent {
    pub fn to_string(&self) -> &'static str {
        match self {
            Intent::Read => "READ",
            Intent::Mutate => "MUTATE",
            Intent::Query => "QUERY",
            Intent::Explain => "EXPLAIN",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Intent> {
        match s {
            "READ" => Some(Intent::Read),
            "MUTATE" => Some(Intent::Mutate),
            "QUERY" => Some(Intent::Query),
            "EXPLAIN" => Some(Intent::Explain),
            _ => None,
        }
    }
}

/// Structured plan produced by LLM
///
/// LLM returns this JSON; system validates before execution.
/// LLM does NOT execute tools directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    /// Unique plan identifier
    pub plan_id: String,

    /// User intent classification
    pub intent: Intent,

    /// Steps to execute (in order)
    pub steps: Vec<Step>,

    /// Evidence queries referenced (Q1-Q8)
    pub evidence_referenced: Vec<String>,
}

/// Single step in a plan
///
/// Each step represents ONE tool invocation.
/// Preconditions checked BEFORE execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    /// Unique step identifier
    pub step_id: String,

    /// Tool name (must be in whitelist)
    pub tool: String,

    /// Tool arguments (validated against schema)
    pub arguments: HashMap<String, String>,

    /// Precondition description (for display)
    pub precondition: String,

    /// Requires user confirmation before execution
    pub requires_confirmation: bool,
}

/// Evidence summary for LLM input
///
/// Pre-aggregated from Q1-Q8 queries.
/// LLM receives THIS, not raw DB access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSummary {
    /// Q1: Tool executions summary
    /// (tool_name, total_count, success_count, failure_count)
    pub q1_tool_executions: Vec<(String, i64, i64, i64)>,

    /// Q2: Recent failures
    /// (tool_name, timestamp_ms, error_message)
    pub q2_failures: Vec<(String, i64, String)>,

    /// Q3: Diagnostic executions
    /// (diagnostic_code, occurrence_count)
    pub q3_diagnostic_executions: Vec<(String, i64)>,

    /// Q4: File executions
    /// (file_path, execution_count, last_success)
    pub q4_file_executions: Vec<(String, i64, bool)>,

    /// Q5: Execution details (optional, for specific execution)
    /// (execution_id, tool_name, timestamp, success)
    pub q5_execution_details: Option<(String, String, i64, bool)>,

    /// Q6: Latest file outcome
    /// (file_path, success, timestamp_ms)
    pub q6_latest_outcome: Option<(String, bool, i64)>,

    /// Q7: Recurring diagnostics
    /// (diagnostic_code, file_name, occurrence_count)
    pub q7_recurring: Vec<(String, String, i64)>,

    /// Q8: Prior fixes (temporal adjacency, NOT causality)
    /// (diagnostic_code, fix_attempts, temporal_gaps_ms)
    pub q8_prior_fixes: Vec<(String, i64, Vec<i64>)>,
}

/// Plan authorization state
///
/// Tracks user approval status for a plan.
/// LLM cannot execute without authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAuthorization {
    plan_id: String,
    status: AuthorizationStatus,
}

/// Authorization status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationStatus {
    /// Awaiting user decision
    Pending,

    /// User approved execution
    Approved,

    /// User rejected plan
    Rejected,
}

impl PlanAuthorization {
    pub fn new(plan_id: String) -> Self {
        Self {
            plan_id,
            status: AuthorizationStatus::Pending,
        }
    }

    pub fn plan_id(&self) -> &str {
        &self.plan_id
    }

    pub fn status(&self) -> AuthorizationStatus {
        self.status
    }

    pub fn is_approved(&self) -> bool {
        self.status == AuthorizationStatus::Approved
    }

    pub fn approve(&mut self) {
        self.status = AuthorizationStatus::Approved;
    }

    pub fn reject(&mut self) {
        self.status = AuthorizationStatus::Rejected;
    }

    pub fn revoke(&mut self) {
        self.status = AuthorizationStatus::Rejected;
    }
}

/// Session context from UI
///
/// Captures user state without processing.
/// Pure data structure, no side effects.
#[derive(Debug, Clone)]
pub struct SessionContext {
    /// User's natural language intent
    pub user_intent: String,

    /// Currently selected file (if any)
    pub selected_file: Option<String>,

    /// Current diagnostic code (if any)
    pub current_diagnostic: Option<String>,

    /// Database root path
    pub db_root: PathBuf,
}

/// Timeline position — where the LLM is in execution history
///
/// Critical for temporal grounding: the LLM must know where it is
/// before taking any action. This is NOT conversation context —
/// it's the execution database position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePosition {
    /// Current step number in the execution loop
    pub current_step: usize,

    /// Total number of executions in the database
    pub total_executions: usize,

    /// Last execution ID
    pub last_execution_id: String,

    /// Last execution tool name
    pub last_execution_tool: String,

    /// Last execution success status
    pub last_execution_success: bool,

    /// Last execution error message (if failed)
    pub last_execution_error: Option<String>,

    /// Timestamp of last execution (milliseconds since epoch)
    pub last_execution_timestamp: i64,

    /// Time since last memory_query (milliseconds)
    /// Used to enforce pre-flight grounding requirement
    pub time_since_last_query_ms: i64,

    /// Number of pending failures (executions that failed without successful retry)
    pub pending_failure_count: usize,
}

impl TimelinePosition {
    /// Check if grounding is required (mutation tools need fresh memory query)
    pub fn requires_grounding(&self, is_mutation: bool) -> bool {
        // Mutation tools require memory query within last 10 seconds
        is_mutation && self.time_since_last_query_ms > 10_000
    }

    /// Create a "no executions yet" position
    pub fn initial() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Self {
            current_step: 0,
            total_executions: 0,
            last_execution_id: "none".to_string(),
            last_execution_tool: "none".to_string(),
            last_execution_success: true,
            last_execution_error: None,
            last_execution_timestamp: now,
            time_since_last_query_ms: now, // Never queried
            pending_failure_count: 0,
        }
    }

    /// Get display summary for UI
    pub fn display_summary(&self) -> String {
        let status = if self.last_execution_success {
            "SUCCESS"
        } else {
            "FAILED"
        };

        format!(
            "Step {} | Execution #{} | {} | {} | Pending failures: {}",
            self.current_step,
            self.total_executions,
            self.last_execution_tool,
            status,
            self.pending_failure_count
        )
    }
}

/// Timeline entry — single execution in chronological order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    /// Execution ID
    pub execution_id: String,

    /// Tool name
    pub tool: String,

    /// Timestamp (milliseconds since epoch)
    pub timestamp: i64,

    /// Success status
    pub success: bool,

    /// Affected path (if any)
    pub affected_path: Option<String>,

    /// Error summary (if failed)
    pub error_summary: Option<String>,

    /// Duration in milliseconds (if available)
    pub duration_ms: Option<i64>,
}

/// Failure record — for tracking unresolved failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecord {
    /// Execution ID that failed
    pub execution_id: String,

    /// Tool that failed
    pub tool: String,

    /// Timestamp of failure
    pub timestamp: i64,

    /// Error message
    pub error: String,

    /// Affected path (if any)
    pub affected_path: Option<String>,

    /// Whether this failure has been successfully retried
    pub retried: bool,
}
