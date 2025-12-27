//! Execution summary tool — Aggregate statistics from execution log
//!
//! Provides lean summaries of past executions with:
//! - Tool usage statistics (total, success, failure)
//! - Recent failures
//! - Success rates by tool
//! - Temporal distribution

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::execution_tools::db::ExecutionDb;

/// Summary arguments
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionSummaryArgs {
    /// Filter by tool name (optional)
    pub tool: Option<String>,
    /// Filter by session_id (optional)
    pub session_id: Option<String>,
    /// Time window: only include executions since this timestamp (optional)
    pub since: Option<i64>,
    /// Time window: only include executions until this timestamp (optional)
    pub until: Option<i64>,
}

/// Execution summary result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    /// Tool usage statistics
    pub tool_stats: Vec<ToolStat>,
    /// Recent failures (up to 10)
    pub recent_failures: Vec<FailureInfo>,
    /// Summary metrics
    pub summary: SummaryMetrics,
}

/// Per-tool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStat {
    /// Tool name
    pub tool: String,
    /// Total executions
    pub total_count: i64,
    /// Successful executions
    pub success_count: i64,
    /// Failed executions
    pub failure_count: i64,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Average duration in milliseconds
    pub avg_duration_ms: Option<f64>,
}

/// Failure information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureInfo {
    /// Execution ID
    pub id: String,
    /// Tool name
    pub tool: String,
    /// Timestamp (milliseconds since epoch)
    pub timestamp: i64,
    /// Error message
    pub error_message: Option<String>,
    /// Arguments JSON (truncated)
    pub arguments: serde_json::Value,
}

/// Overall summary metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryMetrics {
    /// Total executions in window
    pub total_executions: i64,
    /// Total successful
    pub total_success: i64,
    /// Total failed
    pub total_failed: i64,
    /// Overall success rate
    pub success_rate: f64,
    /// Unique tools used
    pub unique_tools: i64,
    /// First execution timestamp
    pub first_timestamp: Option<i64>,
    /// Last execution timestamp
    pub last_timestamp: Option<i64>,
}

impl ExecutionDb {
    /// Generate execution summary with filters
    pub fn execution_summary(&self, args: ExecutionSummaryArgs) -> Result<ExecutionSummary> {
        // Build WHERE clause
        let mut where_clauses = vec!["1=1"];
        let mut bind_params: Vec<String> = vec![];

        if let Some(ref tool) = args.tool {
            where_clauses.push("tool_name = ?");
            bind_params.push(tool.clone());
        }

        if let Some(ref session_id) = args.session_id {
            where_clauses.push("id LIKE ?");
            bind_params.push(format!("{}%", session_id));
        }

        if let Some(since) = args.since {
            where_clauses.push("timestamp >= ?");
            bind_params.push(since.to_string());
        }

        if let Some(until) = args.until {
            where_clauses.push("timestamp <= ?");
            bind_params.push(until.to_string());
        }

        let where_clause = where_clauses.join(" AND ");

        // Get summary metrics
        let summary = self.get_summary_metrics(&where_clause, &bind_params)?;

        // Get per-tool statistics
        let tool_stats = self.get_tool_stats(&where_clause, &bind_params)?;

        // Get recent failures
        let recent_failures = self.get_recent_failures(&where_clause, &bind_params, 10)?;

        Ok(ExecutionSummary {
            tool_stats,
            recent_failures,
            summary,
        })
    }

    /// Get overall summary metrics
    fn get_summary_metrics(&self, where_clause: &str, params: &[String]) -> Result<SummaryMetrics> {
        let sql = format!(
            "SELECT
                COUNT(*) as total,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failure_count,
                COUNT(DISTINCT tool_name) as unique_tools,
                MIN(timestamp) as first_ts,
                MAX(timestamp) as last_ts
             FROM executions
             WHERE {}",
            where_clause
        );

        let metrics = self.conn().query_row(
            &sql,
            rusqlite::params_from_iter(params.iter()),
            |row| {
                let total: i64 = row.get(0)?;
                let success_count: i64 = row.get(1)?;
                let failure_count: i64 = row.get(2)?;
                let unique_tools: i64 = row.get(3)?;
                let first_ts: Option<i64> = row.get(4)?;
                let last_ts: Option<i64> = row.get(5)?;

                let success_rate = if total > 0 {
                    success_count as f64 / total as f64
                } else {
                    0.0
                };

                Ok(SummaryMetrics {
                    total_executions: total,
                    total_success: success_count,
                    total_failed: failure_count,
                    success_rate,
                    unique_tools,
                    first_timestamp: first_ts,
                    last_timestamp: last_ts,
                })
            },
        )?;

        Ok(metrics)
    }

    /// Get per-tool statistics
    fn get_tool_stats(&self, where_clause: &str, params: &[String]) -> Result<Vec<ToolStat>> {
        let sql = format!(
            "SELECT
                tool_name,
                COUNT(*) as total,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failure_count,
                AVG(duration_ms) as avg_duration
             FROM executions
             WHERE {}
             GROUP BY tool_name
             ORDER BY total DESC",
            where_clause
        );

        let mut stmt = self.conn().prepare(&sql)?;

        let stats = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                let tool: String = row.get(0)?;
                let total: i64 = row.get(1)?;
                let success_count: i64 = row.get(2)?;
                let failure_count: i64 = row.get(3)?;
                let avg_duration: Option<f64> = row.get(4)?;

                let success_rate = if total > 0 {
                    success_count as f64 / total as f64
                } else {
                    0.0
                };

                Ok(ToolStat {
                    tool,
                    total_count: total,
                    success_count,
                    failure_count,
                    success_rate,
                    avg_duration_ms: avg_duration,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(stats)
    }

    /// Get recent failures
    fn get_recent_failures(
        &self,
        where_clause: &str,
        params: &[String],
        limit: usize,
    ) -> Result<Vec<FailureInfo>> {
        let sql = format!(
            "SELECT
                id,
                tool_name,
                timestamp,
                error_message,
                arguments_json
             FROM executions
             WHERE {} AND success = 0
             ORDER BY timestamp DESC
             LIMIT ?",
            where_clause
        );

        let mut stmt = self.conn().prepare(&sql)?;

        // Build params for query
        let mut query_params: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        query_params.push(&limit);

        let failures = stmt
            .query_map(query_params.as_slice(), |row| {
                let id: String = row.get(0)?;
                let tool_name: String = row.get(1)?;
                let timestamp: i64 = row.get(2)?;
                let error_message: Option<String> = row.get(3)?;
                let arguments_json: String = row.get(4)?;

                let arguments: serde_json::Value = serde_json::from_str(&arguments_json)
                    .unwrap_or_else(|_| serde_json::json!({}));

                Ok(FailureInfo {
                    id,
                    tool: tool_name,
                    timestamp,
                    error_message,
                    arguments,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(failures)
    }
}

/// Public function for execution_summary (used by tool_mapper)
pub fn execution_summary(db_root: &Path, args: ExecutionSummaryArgs) -> Result<ExecutionSummary> {
    let db = ExecutionDb::open(db_root)?;
    db.execution_summary(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_summary_args_default() {
        let args = ExecutionSummaryArgs::default();
        assert!(args.tool.is_none());
        assert!(args.session_id.is_none());
        assert!(args.since.is_none());
        assert!(args.until.is_none());
    }

    #[test]
    fn test_execution_summary_serialization() {
        let summary = ExecutionSummary {
            tool_stats: vec![ToolStat {
                tool: "file_read".to_string(),
                total_count: 100,
                success_count: 95,
                failure_count: 5,
                success_rate: 0.95,
                avg_duration_ms: Some(50.0),
            }],
            recent_failures: vec![],
            summary: SummaryMetrics {
                total_executions: 100,
                total_success: 95,
                total_failed: 5,
                success_rate: 0.95,
                unique_tools: 1,
                first_timestamp: Some(1700000000000),
                last_timestamp: Some(1700000001000),
            },
        };

        let json = serde_json::to_string(&summary).unwrap();
        let parsed: ExecutionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.summary.total_executions, 100);
    }

    #[test]
    fn test_failure_info_serialization() {
        let failure = FailureInfo {
            id: "exec-123".to_string(),
            tool: "file_read".to_string(),
            timestamp: 1700000000000,
            error_message: Some("File not found".to_string()),
            arguments: serde_json::json!({"path": "/nonexistent"}),
        };

        let json = serde_json::to_string(&failure).unwrap();
        let parsed: FailureInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "exec-123");
        assert_eq!(parsed.error_message, Some("File not found".to_string()));
    }

    #[test]
    fn test_tool_stat_success_rate_calculation() {
        let stat = ToolStat {
            tool: "test_tool".to_string(),
            total_count: 100,
            success_count: 75,
            failure_count: 25,
            success_rate: 0.75,
            avg_duration_ms: None,
        };

        assert_eq!(stat.success_rate, 0.75);
        assert_eq!(stat.total_count, stat.success_count + stat.failure_count);
    }
}
