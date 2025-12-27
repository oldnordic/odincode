//! Result types for evidence queries (Phase 0.6)
//!
//! All types are explicit and typed. No HashMaps, no inference.

/// Q1, Q2 output: Execution summary
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionSummary {
    pub execution_id: String,
    pub tool_name: String,
    pub timestamp: i64,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub error_message: Option<String>,
}

/// Q2 output: Failure summary (subset of ExecutionSummary)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureSummary {
    pub execution_id: String,
    pub tool_name: String,
    pub timestamp: i64,
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
}

/// Q3 output: Diagnostic execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticExecution {
    pub execution_id: String,
    pub tool_name: String,
    pub timestamp: i64,
    pub diagnostic_code: String,
    pub diagnostic_level: String,
    pub diagnostic_message: String,
    pub file_name: String,
}

/// Q4 output: File execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileExecution {
    pub execution_id: String,
    pub tool_name: String,
    pub timestamp: i64,
    pub success: bool,
    pub edge_type: String,
    pub data_source: DataSource,
}

/// Data source indicator for Q4, Q6 (best-effort dual-write gap)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSource {
    Graph,
    Fallback,
}

/// Q5 output: Full execution details
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionDetails {
    pub execution: ExecutionRecord,
    pub artifacts: Vec<ArtifactRecord>,
    pub graph_entity: Option<GraphEntityRecord>,
    pub graph_edges: Vec<GraphEdgeRecord>,
}

/// Q5 component: Execution record from SQLite
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRecord {
    pub id: String,
    pub tool_name: String,
    pub arguments_json: String,
    pub timestamp: i64,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub error_message: Option<String>,
}

/// Q5 component: Artifact record
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub artifact_type: String,
    pub content_json: String,
}

/// Q5 component: Graph entity record
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEntityRecord {
    pub entity_id: i64,
    pub kind: String,
    pub name: String,
    pub file_path: Option<String>,
    pub data: String,
}

/// Q5 component: Graph edge record
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdgeRecord {
    pub edge_id: i64,
    pub edge_type: String,
    pub target_entity_id: i64,
    pub target_kind: String,
    pub target_name: String,
}

/// Q6 output: Latest file outcome (Option wrapper)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatestFileOutcome {
    pub execution_id: String,
    pub tool_name: String,
    pub timestamp: i64,
    pub success: bool,
    pub edge_type: String,
    pub data_source: DataSource,
}

/// Q7 output: Recurring diagnostic
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurringDiagnostic {
    pub diagnostic_code: String,
    pub file_name: String,
    pub occurrence_count: i64,
    pub first_seen: i64,
    pub last_seen: i64,
    pub execution_ids: Vec<String>,
}

/// Q8 output: Prior fix (temporal adjacency only, NOT causality)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriorFix {
    pub execution_id: String,
    pub tool_name: String,
    pub timestamp: i64,
    pub diagnostic_execution_id: String,
    pub temporal_gap_ms: i64,
    pub success: bool,
}
