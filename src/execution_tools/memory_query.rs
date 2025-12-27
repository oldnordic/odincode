//! Memory query tool — Read execution log with filters
//!
//! Provides filtered access to execution history with:
//! - Tool name filter
//! - Session ID filter
//! - Success-only filter
//! - Limit on results
//! - Optional output inclusion
//!
//! Phase 9.7: Temporal grounding — timeline queries for position awareness

use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

use crate::execution_tools::db::ExecutionDb;
use crate::llm::types::{FailureRecord, TimelineEntry, TimelinePosition};

/// Query filters for memory_query
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryQueryArgs {
    /// Filter by tool name (optional)
    pub tool: Option<String>,
    /// Filter by session_id (optional)
    pub session_id: Option<String>,
    /// Only return successful executions (optional)
    pub success_only: Option<bool>,
    /// Maximum number of results (default: 10, max: 100)
    pub limit: Option<usize>,
    /// Include full output in results (default: false)
    pub include_output: Option<bool>,
    /// Start timestamp for range query (optional, milliseconds since epoch)
    pub since: Option<i64>,
    /// End timestamp for range query (optional, milliseconds since epoch)
    pub until: Option<i64>,
}

/// Single execution record returned by memory_query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Execution ID
    pub id: String,
    /// Tool name
    pub tool_name: String,
    /// Timestamp (milliseconds since epoch)
    pub timestamp: i64,
    /// Duration in milliseconds
    pub duration_ms: Option<i64>,
    /// Success flag
    pub success: bool,
    /// Exit code (if any)
    pub exit_code: Option<i32>,
    /// Lean summary (first 200 chars of stdout or error message)
    pub summary: String,
    /// Output preview (first 1000 chars)
    pub output_preview: Option<String>,
    /// Full output (only if include_output=true)
    pub full_output: Option<String>,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Arguments JSON
    pub arguments: serde_json::Value,
}

/// Query result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryResult {
    /// Matching executions
    pub executions: Vec<ExecutionRecord>,
    /// Total count (may exceed returned executions)
    pub total_count: usize,
    /// Query applied
    pub query_applied: MemoryQueryArgs,
}

impl ExecutionDb {
    /// Query execution log with filters
    ///
    /// Returns executions matching the given filters, ordered by timestamp DESC.
    pub fn memory_query(&self, args: MemoryQueryArgs) -> Result<MemoryQueryResult> {
        let limit = args.limit.unwrap_or(10).min(100);
        let include_output = args.include_output.unwrap_or(false);

        // Build WHERE clause
        let mut where_clauses = vec!["1=1"];
        let mut bind_params: Vec<String> = vec![];

        if let Some(ref tool) = args.tool {
            where_clauses.push("tool_name = ?");
            bind_params.push(tool.clone());
        }

        if let Some(ref session_id) = args.session_id {
            where_clauses.push("id LIKE ?");
            // session_id is prefix of execution_id
            bind_params.push(format!("{}%", session_id));
        }

        if args.success_only.unwrap_or(false) {
            where_clauses.push("success = 1");
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

        // First, get total count
        let count_sql = format!(
            "SELECT COUNT(*) FROM executions WHERE {}",
            where_clause
        );
        let total_count: i64 = self.conn().query_row(
            &count_sql,
            rusqlite::params_from_iter(bind_params.iter()),
            |row| row.get(0),
        )?;
        let total_count = total_count as usize;

        // Get paginated results
        let sql = format!(
            "SELECT id, tool_name, timestamp, duration_ms, success, exit_code,
                    error_message, arguments_json
             FROM executions
             WHERE {}
             ORDER BY timestamp DESC
             LIMIT ?",
            where_clause
        );

        let mut stmt = self.conn().prepare(&sql)?;

        // Build params for query
        let mut query_params: Vec<&dyn rusqlite::ToSql> =
            bind_params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        query_params.push(&limit);

        let executions = stmt
            .query_map(query_params.as_slice(), |row| {
                let id: String = row.get(0)?;
                let tool_name: String = row.get(1)?;
                let timestamp: i64 = row.get(2)?;
                let duration_ms: Option<i64> = row.get(3)?;
                let success: bool = row.get(4)?;
                let exit_code: Option<i32> = row.get(5)?;
                let error_message: Option<String> = row.get(6)?;
                let arguments_json: String = row.get(7)?;

                let arguments: serde_json::Value = serde_json::from_str(&arguments_json)
                    .unwrap_or_else(|_| serde_json::json!({}));

                Ok(ExecutionRecord {
                    id,
                    tool_name,
                    timestamp,
                    duration_ms,
                    success,
                    exit_code,
                    summary: String::new(), // Will be filled below
                    output_preview: None,
                    full_output: None,
                    error_message,
                    arguments,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|mut rec| {
                // Fetch output from artifacts
                if let Ok(artifact) = self.get_stdout_artifact(&rec.id) {
                    rec.summary = summarize_output(&artifact, 200);
                    rec.output_preview = Some(truncate_output(&artifact, 1000));
                    rec.full_output = if include_output {
                        Some(artifact)
                    } else {
                        None
                    };
                } else if let Some(ref err) = rec.error_message {
                    rec.summary = truncate_output(err, 200);
                } else {
                    rec.summary = "(no output)".to_string();
                }
                rec
            })
            .collect();

        Ok(MemoryQueryResult {
            executions,
            total_count,
            query_applied: args,
        })
    }

    /// Get stdout artifact for an execution
    fn get_stdout_artifact(&self, execution_id: &str) -> Result<String> {
        self.conn()
            .query_row(
                "SELECT content_json FROM execution_artifacts
                 WHERE execution_id = ? AND artifact_type = 'stdout'
                 LIMIT 1",
                params![execution_id],
                |row| {
                    let json: String = row.get(0)?;
                    let content: serde_json::Value = serde_json::from_str(&json)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                    Ok(content.as_str().unwrap_or("").to_string())
                },
            )
            .or_else(|_| Ok(String::new()))
    }

    // ========== Phase 9.7: Timeline Query APIs ==========

    /// Get current timeline position
    ///
    /// Returns the current position in the execution timeline,
    /// including last execution, pending failures, and time since last query.
    pub fn get_timeline_position(&self, last_query_time_ms: Option<i64>) -> Result<TimelinePosition> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // Get total execution count
        let total_executions: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM executions",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        // Get last execution
        let (last_id, last_tool, last_success, last_error, last_timestamp) = self.conn().query_row(
            "SELECT id, tool_name, success, error_message, timestamp
             FROM executions
             ORDER BY timestamp DESC
             LIMIT 1",
            [],
            |row| {
                let id: String = row.get(0)?;
                let tool: String = row.get(1)?;
                let success: bool = row.get(2)?;
                let error: Option<String> = row.get(3)?;
                let timestamp: i64 = row.get(4)?;
                Ok((id, tool, success, error, timestamp))
            },
        ).unwrap_or_else(|_| {
            // No executions yet
            (String::from("none"), String::from("none"), true, None, now)
        });

        // Count pending failures (failed executions without successful retry)
        let pending_failure_count: i64 = self.conn().query_row(
            "SELECT COUNT(DISTINCT e1.id) FROM executions e1
             WHERE e1.success = 0
             AND NOT EXISTS (
                 SELECT 1 FROM executions e2
                 WHERE e2.tool_name = e1.tool_name
                 AND e2.timestamp > e1.timestamp
                 AND e2.success = 1
             )",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        // Calculate current step (count of mutation tool executions)
        let current_step: usize = self.conn().query_row(
            "SELECT COUNT(*) FROM executions
             WHERE tool_name IN ('file_edit', 'file_write', 'file_create', 'splice_patch')",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        let time_since_query = if let Some(t) = last_query_time_ms {
            now - t
        } else {
            now // Never queried
        };

        Ok(TimelinePosition {
            current_step,
            total_executions: total_executions as usize,
            last_execution_id: last_id,
            last_execution_tool: last_tool,
            last_execution_success: last_success,
            last_execution_error: last_error,
            last_execution_timestamp: last_timestamp,
            time_since_last_query_ms: time_since_query,
            pending_failure_count: pending_failure_count as usize,
        })
    }

    /// Get recent timeline entries (chronological order)
    ///
    /// Returns the last N executions in chronological order (oldest first).
    pub fn get_recent_timeline(&self, n: usize) -> Result<Vec<TimelineEntry>> {
        let n = n.min(100);

        let sql = "SELECT id, tool_name, timestamp, success, error_message, duration_ms, arguments_json
                   FROM executions
                   ORDER BY timestamp DESC
                   LIMIT ?";

        let mut stmt = self.conn().prepare(sql)?;

        let mut entries: Vec<TimelineEntry> = stmt
            .query_map(params![n], |row| {
                let execution_id: String = row.get(0)?;
                let tool: String = row.get(1)?;
                let timestamp: i64 = row.get(2)?;
                let success: bool = row.get(3)?;
                let error: Option<String> = row.get(4)?;
                let duration_ms: Option<i64> = row.get(5)?;
                let arguments_json: String = row.get(6)?;

                // Extract affected path from arguments if available
                let affected_path = extract_path_from_arguments(&arguments_json);

                Ok(TimelineEntry {
                    execution_id,
                    tool,
                    timestamp,
                    success,
                    affected_path,
                    error_summary: error,
                    duration_ms,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Reverse to get chronological order (oldest first)
        entries.reverse();

        Ok(entries)
    }

    /// Get pending failures (unresolved failed executions)
    ///
    /// Returns failures that have not been followed by a successful retry.
    pub fn get_pending_failures(&self) -> Result<Vec<FailureRecord>> {
        let sql = "SELECT e1.id, e1.tool_name, e1.timestamp, e1.error_message,
                          e1.arguments_json
                   FROM executions e1
                   WHERE e1.success = 0
                   AND NOT EXISTS (
                       SELECT 1 FROM executions e2
                       WHERE e2.tool_name = e1.tool_name
                       AND e2.timestamp > e1.timestamp
                       AND e2.success = 1
                   )
                   ORDER BY e1.timestamp DESC";

        let mut stmt = self.conn().prepare(sql)?;

        let failures = stmt
            .query_map([], |row| {
                let execution_id: String = row.get(0)?;
                let tool: String = row.get(1)?;
                let timestamp: i64 = row.get(2)?;
                let error: String = row.get(3)?;
                let arguments_json: String = row.get(4)?;

                // Extract affected path from arguments
                let affected_path = extract_path_from_arguments(&arguments_json);

                Ok(FailureRecord {
                    execution_id,
                    tool,
                    timestamp,
                    error,
                    affected_path,
                    retried: false,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(failures)
    }

    /// Get timeline context before a specific execution
    ///
    /// Returns executions that occurred before the given execution ID.
    pub fn get_preceding_context(&self, before_id: &str, limit: usize) -> Result<Vec<TimelineEntry>> {
        let limit = limit.min(100);

        // Get timestamp of the reference execution
        let timestamp: i64 = self.conn().query_row(
            "SELECT timestamp FROM executions WHERE id = ?",
            params![before_id],
            |row| row.get(0),
        )?;

        let sql = "SELECT id, tool_name, timestamp, success, error_message, duration_ms, arguments_json
                   FROM executions
                   WHERE timestamp < ?
                   ORDER BY timestamp DESC
                   LIMIT ?";

        let mut stmt = self.conn().prepare(sql)?;

        let mut entries: Vec<TimelineEntry> = stmt
            .query_map(params![timestamp, limit], |row| {
                let execution_id: String = row.get(0)?;
                let tool: String = row.get(1)?;
                let ts: i64 = row.get(2)?;
                let success: bool = row.get(3)?;
                let error: Option<String> = row.get(4)?;
                let duration_ms: Option<i64> = row.get(5)?;
                let arguments_json: String = row.get(6)?;

                let affected_path = extract_path_from_arguments(&arguments_json);

                Ok(TimelineEntry {
                    execution_id,
                    tool,
                    timestamp: ts,
                    success,
                    affected_path,
                    error_summary: error,
                    duration_ms,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Reverse to chronological order
        entries.reverse();

        Ok(entries)
    }
}

/// Extract affected path from arguments JSON
fn extract_path_from_arguments(arguments_json: &str) -> Option<String> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(arguments_json) {
        if let Some(obj) = json.as_object() {
            // Try common path keys
            for key in ["path", "file", "repo_root", "affected_path"] {
                if let Some(Value::String(s)) = obj.get(key) {
                    return Some(s.clone());
                }
            }
        }
    }
    None
}

/// Summarize output to first N characters
fn summarize_output(output: &str, max_len: usize) -> String {
    truncate_output(output, max_len)
}

/// Truncate output with ellipsis if needed
fn truncate_output(output: &str, max_len: usize) -> String {
    if output.len() <= max_len {
        output.to_string()
    } else {
        format!("{}... [{} chars total]", &output[..max_len], output.len())
    }
}

/// Public function for memory_query (used by tool_mapper)
pub fn memory_query(db_root: &Path, args: MemoryQueryArgs) -> Result<MemoryQueryResult> {
    let db = ExecutionDb::open(db_root)?;
    db.memory_query(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_memory_query_args_default() {
        let args = MemoryQueryArgs::default();
        assert!(args.tool.is_none());
        assert!(args.session_id.is_none());
        assert!(args.success_only.is_none());
        assert_eq!(args.limit, None);
        assert!(args.include_output.is_none());
    }

    #[test]
    fn test_memory_query_args_with_tool() {
        let args = MemoryQueryArgs {
            tool: Some("file_read".to_string()),
            ..Default::default()
        };
        assert_eq!(args.tool, Some("file_read".to_string()));
    }

    #[test]
    fn test_truncate_short() {
        let short = "hello";
        assert_eq!(truncate_output(short, 100), "hello");
    }

    #[test]
    fn test_truncate_long() {
        let long = "x".repeat(200);
        let truncated = truncate_output(&long, 50);
        assert!(truncated.len() < 200);
        assert!(truncated.contains("..."));
        assert!(truncated.contains("200 chars total"));
    }

    #[test]
    fn test_execution_record_serialization() {
        let rec = ExecutionRecord {
            id: "test-123".to_string(),
            tool_name: "file_read".to_string(),
            timestamp: 1700000000000,
            duration_ms: Some(50),
            success: true,
            exit_code: Some(0),
            summary: "test summary".to_string(),
            output_preview: Some("preview".to_string()),
            full_output: Some("full output".to_string()),
            error_message: None,
            arguments: json!({"path": "/test/file.txt"}),
        };

        let json = serde_json::to_string(&rec).unwrap();
        let parsed: ExecutionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test-123");
        assert_eq!(parsed.tool_name, "file_read");
    }
}

// ========== Public Timeline Query APIs ==========

/// Get current timeline position
///
/// Public wrapper for ExecutionDb::get_timeline_position.
/// Returns the current position in the execution timeline, including
/// last execution, pending failures, and time since last query.
pub fn get_timeline_position(
    db_root: &Path,
    last_query_time_ms: Option<i64>,
) -> Result<TimelinePosition> {
    let db = ExecutionDb::open(db_root)?;
    db.get_timeline_position(last_query_time_ms)
}

/// Get recent timeline entries (chronological order)
///
/// Public wrapper for ExecutionDb::get_recent_timeline.
/// Returns the last N executions in chronological order (oldest first).
pub fn get_recent_timeline(db_root: &Path, n: usize) -> Result<Vec<TimelineEntry>> {
    let db = ExecutionDb::open(db_root)?;
    db.get_recent_timeline(n)
}

/// Get pending failures (unresolved failed executions)
///
/// Public wrapper for ExecutionDb::get_pending_failures.
/// Returns failures that have not been followed by a successful retry.
pub fn get_pending_failures(db_root: &Path) -> Result<Vec<FailureRecord>> {
    let db = ExecutionDb::open(db_root)?;
    db.get_pending_failures()
}

/// Get timeline context before a specific execution
///
/// Public wrapper for ExecutionDb::get_preceding_context.
/// Returns executions that occurred before the given execution ID.
pub fn get_preceding_context(
    db_root: &Path,
    before_id: &str,
    limit: usize,
) -> Result<Vec<TimelineEntry>> {
    let db = ExecutionDb::open(db_root)?;
    db.get_preceding_context(before_id, limit)
}
