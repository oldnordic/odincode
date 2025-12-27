//! Phase 9.3: Loop trace viewer
//!
//! Provides:
//! - TraceRow: Struct representing a single execution step
//! - query_last_loop_trace(): Query execution_log.db for recent loop steps

use rusqlite::{params, Connection, Row};
use serde::Deserialize;
use std::collections::HashMap;

/// Single row in the loop trace
#[derive(Debug, Clone, PartialEq)]
pub struct TraceRow {
    /// Execution ID
    pub id: String,
    /// Tool name
    pub tool_name: String,
    /// Execution timestamp (milliseconds since UNIX epoch)
    pub timestamp: i64,
    /// Success flag
    pub success: bool,
    /// Duration in milliseconds (if available)
    pub duration_ms: Option<i64>,
    /// Scope from approval artifact (e.g., "SessionAllGated")
    pub scope: Option<String>,
    /// Affected file path (extracted from arguments)
    pub affected_path: Option<String>,
}

impl TraceRow {
    /// Create TraceRow from database row
    fn from_db_row(
        row: &Row,
        artifacts: &HashMap<String, ApprovalArtifact>,
    ) -> Result<Self, rusqlite::Error> {
        let id: String = row.get("id")?;
        let tool_name: String = row.get("tool_name")?;
        let timestamp: i64 = row.get("timestamp")?;
        let success_i64: i64 = row.get("success")?;
        let duration_ms: Option<i64> = row.get("duration_ms")?;
        let arguments_json: String = row.get("arguments_json")?;

        // Get scope from approval artifact if present
        let scope = artifacts.get(&id).and_then(|a| a.scope.clone());

        // Extract affected path from arguments JSON
        let affected_path = extract_path_from_args(&arguments_json);

        Ok(TraceRow {
            id,
            tool_name,
            timestamp,
            success: success_i64 != 0,
            duration_ms,
            scope,
            affected_path,
        })
    }
}

/// Approval artifact from execution_artifacts table
#[derive(Debug, Clone, Deserialize)]
struct ApprovalArtifact {
    /// Approval scope (e.g., "SessionAllGated")
    scope: Option<String>,
    /// Tool that was approved (reserved for future use)
    #[allow(dead_code)]
    tool: Option<String>,
}

/// Query the last N loop steps from execution_log.db
///
/// Returns executions in reverse chronological order (newest first).
/// Includes approval events (approval_granted, approval_denied) and GATED tool executions.
pub fn query_last_loop_trace(conn: &Connection, limit: i64) -> Result<Vec<TraceRow>, String> {
    // Query executions ordered by timestamp DESC (newest first)
    let query = "
        SELECT id, tool_name, timestamp, success, duration_ms, arguments_json
        FROM executions
        ORDER BY timestamp DESC
        LIMIT ?1
    ";

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    // First, get all execution IDs and fetch their approval artifacts
    let rows: Vec<_> = stmt
        .query_map(params![limit], |row| {
            Ok((
                row.get::<_, String>("id")?,
                row.get::<_, String>("tool_name")?,
            ))
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("Row extraction failed: {}", e))?;

    // Build map of approval artifacts
    let mut artifacts = HashMap::new();
    for (id, _) in &rows {
        if let Ok(artifact) = get_approval_artifact(conn, id) {
            artifacts.insert(id.clone(), artifact);
        }
    }

    // Now build TraceRows with artifact data
    let mut stmt = conn
        .prepare(query)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let trace_rows: Vec<TraceRow> = stmt
        .query_map(params![limit], |row| {
            Ok(TraceRow::from_db_row(row, &artifacts).unwrap())
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("Row extraction failed: {}", e))?;

    Ok(trace_rows)
}

/// Get approval artifact for an execution
fn get_approval_artifact(
    conn: &Connection,
    execution_id: &str,
) -> Result<ApprovalArtifact, rusqlite::Error> {
    let query = "
        SELECT content_json
        FROM execution_artifacts
        WHERE execution_id = ?1
        AND artifact_type IN ('approval_granted', 'approval_denied')
        LIMIT 1
    ";

    let mut stmt = conn.prepare(query)?;

    let artifact_json: Option<String> = stmt.query_row(params![execution_id], |row| row.get(0))?;

    if let Some(json) = artifact_json {
        serde_json::from_str(&json)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    } else {
        // No approval artifact found, return empty
        Ok(ApprovalArtifact {
            scope: None,
            tool: None,
        })
    }
}

/// Extract file path from arguments JSON
fn extract_path_from_args(args_json: &str) -> Option<String> {
    if let Ok(map) = serde_json::from_str::<HashMap<String, serde_json::Value>>(args_json) {
        // Common keys for file paths
        for key in &["path", "file", "file_path", "filePath"] {
            if let Some(value) = map.get(*key) {
                if let Some(s) = value.as_str() {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_path_from_args() {
        let args = r#"{"path":"src/main.rs","line":10}"#;
        assert_eq!(
            extract_path_from_args(args),
            Some("src/main.rs".to_string())
        );

        let args = r#"{"file":"test.txt"}"#;
        assert_eq!(extract_path_from_args(args), Some("test.txt".to_string()));

        let args = r#"{"foo":"bar"}"#;
        assert_eq!(extract_path_from_args(args), None);
    }
}
