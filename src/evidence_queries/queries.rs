//! Evidence query implementations (Q1–Q8)
//!
//! All queries are SELECT-only with deterministic ORDER BY clauses.
//! Each query implements the exact SQL specified in PHASE_0_6_EVIDENCE_QUERIES.md.

use super::types::*;
use super::{Error, EvidenceDb};
use anyhow::Result;
use rusqlite::{params, Connection, Row};

impl EvidenceDb {
    /// Q1: ListExecutionsByTool
    ///
    /// Retrieve all executions of a specific tool, optionally filtered by time range.
    ///
    /// ORDER BY: timestamp ASC, id ASC
    pub fn list_executions_by_tool(
        &self,
        tool_name: &str,
        since: Option<i64>,
        until: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<ExecutionSummary>> {
        let base_sql =
            "SELECT id, tool_name, timestamp, success, exit_code, duration_ms, error_message
             FROM executions
             WHERE tool_name = ?1
               AND (?2 IS NULL OR timestamp >= ?2)
               AND (?3 IS NULL OR timestamp < ?3)
             ORDER BY timestamp ASC, id ASC";

        let sql = if let Some(lim) = limit {
            format!("{} LIMIT {}", base_sql, lim)
        } else {
            base_sql.to_string()
        };

        let mut stmt = self.conn().prepare(&sql)?;
        let rows = stmt.query_map(params![tool_name, since, until], |r| {
            row_to_execution_summary(r)
        })?;
        rows.collect::<::std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    /// Q2: ListFailuresByTool
    ///
    /// Retrieve only failed executions of a specific tool.
    ///
    /// ORDER BY: timestamp DESC, id DESC
    pub fn list_failures_by_tool(
        &self,
        tool_name: &str,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<FailureSummary>> {
        let base_sql = "SELECT id, tool_name, timestamp, exit_code, error_message
             FROM executions
             WHERE tool_name = ?1
               AND success = 0
               AND (?2 IS NULL OR timestamp >= ?2)
             ORDER BY timestamp DESC, id DESC";

        let sql = if let Some(lim) = limit {
            format!("{} LIMIT {}", base_sql, lim)
        } else {
            base_sql.to_string()
        };

        let mut stmt = self.conn().prepare(&sql)?;
        let rows = stmt.query_map(params![tool_name, since], row_to_failure_summary)?;
        rows.collect::<::std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    /// Q3: FindExecutionsByDiagnosticCode
    ///
    /// Find all executions that produced a specific diagnostic code.
    ///
    /// ORDER BY: timestamp ASC, id ASC
    pub fn find_executions_by_diagnostic_code(
        &self,
        code: &str,
        limit: Option<usize>,
    ) -> Result<Vec<DiagnosticExecution>> {
        let base_sql = "SELECT DISTINCT
                e.id AS execution_id,
                e.tool_name,
                e.timestamp,
                json_extract(a.value, '$.code') AS diagnostic_code,
                json_extract(a.value, '$.level') AS diagnostic_level,
                json_extract(a.value, '$.message') AS diagnostic_message,
                json_extract(a.value, '$.file_name') AS file_name
             FROM executions e
             JOIN execution_artifacts a ON e.id = a.execution_id,
                  json_each(a.content_json) AS a
             WHERE a.artifact_type = 'diagnostics'
               AND json_extract(a.value, '$.code') = ?1
             ORDER BY e.timestamp ASC, e.id ASC";

        let sql = if let Some(lim) = limit {
            format!("{} LIMIT {}", base_sql, lim)
        } else {
            base_sql.to_string()
        };

        let mut stmt = self.conn().prepare(&sql)?;
        let rows = stmt.query_map(params![code], row_to_diagnostic_execution)?;
        rows.collect::<::std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    /// Q4: FindExecutionsByFile
    ///
    /// Find all executions that operated on a specific file path.
    /// Uses graph relationships if available, falls back to SQLite.
    ///
    /// ORDER BY: timestamp DESC, id DESC
    pub fn find_executions_by_file(
        &self,
        file_path: &str,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<FileExecution>> {
        // Try graph query first
        if let Some(graph_conn) = self.graph_conn() {
            if let Ok(results) =
                self.find_executions_by_file_graph(graph_conn, file_path, since, limit)
            {
                return Ok(results);
            }
        }

        // Fallback to SQLite (parse arguments_json)
        self.find_executions_by_file_fallback(file_path, since, limit)
    }

    /// Graph-based query for Q4
    fn find_executions_by_file_graph(
        &self,
        graph_conn: &Connection,
        file_path: &str,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<FileExecution>> {
        let base_sql = "SELECT DISTINCT
                json_extract(ge_exec.data, '$.execution_id') AS execution_id,
                json_extract(ge_exec.data, '$.tool') AS tool_name,
                json_extract(ge_exec.data, '$.timestamp') AS timestamp,
                json_extract(ge_exec.data, '$.success') AS success,
                edge_type
             FROM graph_entities ge_exec
             JOIN graph_edges e ON ge_exec.id = e.from_id
             JOIN graph_entities ge_file ON e.to_id = ge_file.id
             WHERE ge_exec.kind = 'execution'
               AND ge_file.kind = 'file'
               AND ge_file.file_path = ?1
               AND (?2 IS NULL OR json_extract(ge_exec.data, '$.timestamp') >= ?2)
             ORDER BY json_extract(ge_exec.data, '$.timestamp') DESC, ge_exec.id DESC";

        let sql = if let Some(lim) = limit {
            format!("{} LIMIT {}", base_sql, lim)
        } else {
            base_sql.to_string()
        };

        let mut stmt = graph_conn.prepare(&sql)?;
        let rows = stmt.query_map(params![file_path, since], |r| {
            row_to_file_execution(r, DataSource::Graph)
        })?;
        let result: Result<Vec<_>> = rows
            .collect::<::std::result::Result<Vec<_>, _>>()
            .map_err(Into::into);
        result
    }

    /// Fallback SQLite query for Q4 (when graph missing)
    fn find_executions_by_file_fallback(
        &self,
        file_path: &str,
        since: Option<i64>,
        limit: Option<usize>,
    ) -> Result<Vec<FileExecution>> {
        let base_sql = "SELECT id, tool_name, timestamp, success
             FROM executions
             WHERE tool_name IN ('file_read', 'file_write', 'file_create', 'splice_patch', 'splice_plan', 'lsp_check')
               AND json_extract(arguments_json, '$.file') = ?1
               AND (?2 IS NULL OR timestamp >= ?2)
             ORDER BY timestamp DESC, id DESC";

        let sql = if let Some(lim) = limit {
            format!("{} LIMIT {}", base_sql, lim)
        } else {
            base_sql.to_string()
        };

        let mut stmt = self.conn().prepare(&sql)?;
        let rows = stmt.query_map(params![file_path, since], |r| {
            Ok(FileExecution {
                execution_id: r.get(0)?,
                tool_name: r.get(1)?,
                timestamp: r.get(2)?,
                success: r.get::<_, i64>(3)? == 1,
                edge_type: "UNKNOWN".to_string(),
                data_source: DataSource::Fallback,
            })
        })?;
        rows.collect::<::std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    /// Q5: GetExecutionDetails
    ///
    /// Retrieve complete execution record including all artifacts and graph edges.
    ///
    /// artifacts ORDER BY: artifact_type ASC
    /// graph_edges ORDER BY: edge_type ASC, target_entity_id ASC
    pub fn get_execution_details(&self, execution_id: &str) -> Result<ExecutionDetails> {
        // Get execution record
        let execution = self.get_execution_record(execution_id)?;

        // Get artifacts
        let artifacts = self.get_artifacts(execution_id)?;

        // Get graph entity and edges (if graph available)
        let (graph_entity, graph_edges) = if let Some(graph_conn) = self.graph_conn() {
            let entity = self.get_graph_entity(graph_conn, execution_id)?;
            let edges = if entity.is_some() {
                self.get_graph_edges(graph_conn, execution_id)?
            } else {
                Vec::new()
            };
            (entity, edges)
        } else {
            (None, Vec::new())
        };

        Ok(ExecutionDetails {
            execution,
            artifacts,
            graph_entity,
            graph_edges,
        })
    }

    fn get_execution_record(&self, execution_id: &str) -> Result<ExecutionRecord> {
        let mut stmt = self.conn().prepare(
            "SELECT id, tool_name, arguments_json, timestamp, success, exit_code, duration_ms, error_message
             FROM executions
             WHERE id = ?1"
        )?;

        let result = stmt.query_row(params![execution_id], |r| {
            Ok(ExecutionRecord {
                id: r.get(0)?,
                tool_name: r.get(1)?,
                arguments_json: r.get(2)?,
                timestamp: r.get(3)?,
                success: r.get::<_, i64>(4)? == 1,
                exit_code: r.get(5)?,
                duration_ms: r.get(6)?,
                error_message: r.get(7)?,
            })
        });

        result.map_err(|_| anyhow::anyhow!(Error::ExecutionNotFound(execution_id.to_string())))
    }

    fn get_artifacts(&self, execution_id: &str) -> Result<Vec<ArtifactRecord>> {
        let mut stmt = self.conn().prepare(
            "SELECT artifact_type, content_json
             FROM execution_artifacts
             WHERE execution_id = ?1
             ORDER BY artifact_type ASC",
        )?;

        let rows = stmt.query_map(params![execution_id], |r| {
            Ok(ArtifactRecord {
                artifact_type: r.get(0)?,
                content_json: r.get(1)?,
            })
        })?;

        rows.collect::<::std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn get_graph_entity(
        &self,
        graph_conn: &Connection,
        execution_id: &str,
    ) -> Result<Option<GraphEntityRecord>> {
        let mut stmt = graph_conn.prepare(
            "SELECT id, kind, name, file_path, data
             FROM graph_entities
             WHERE kind = 'execution'
               AND json_extract(data, '$.execution_id') = ?1",
        )?;

        let result = stmt.query_row(params![execution_id], |r| {
            Ok(GraphEntityRecord {
                entity_id: r.get(0)?,
                kind: r.get(1)?,
                name: r.get(2)?,
                file_path: r.get(3)?,
                data: r.get(4)?,
            })
        });

        match result {
            Ok(entity) => Ok(Some(entity)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn get_graph_edges(
        &self,
        graph_conn: &Connection,
        execution_id: &str,
    ) -> Result<Vec<GraphEdgeRecord>> {
        let mut stmt = graph_conn.prepare(
            "SELECT
                e.id AS edge_id,
                e.edge_type,
                e.to_id AS target_entity_id,
                ge.kind AS target_kind,
                ge.name AS target_name
             FROM graph_edges e
             JOIN graph_entities ge ON e.to_id = ge.id
             WHERE e.from_id = (SELECT id FROM graph_entities
                              WHERE kind = 'execution'
                                AND json_extract(data, '$.execution_id') = ?1)
             ORDER BY e.edge_type ASC, e.to_id ASC",
        )?;

        let rows = stmt.query_map(params![execution_id], |r| {
            Ok(GraphEdgeRecord {
                edge_id: r.get(0)?,
                edge_type: r.get(1)?,
                target_entity_id: r.get(2)?,
                target_kind: r.get(3)?,
                target_name: r.get(4)?,
            })
        })?;

        rows.collect::<::std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    /// Q6: GetLatestOutcomeForFile
    ///
    /// Get the most recent execution outcome for a specific file.
    ///
    /// ORDER BY: timestamp DESC LIMIT 1
    pub fn get_latest_outcome_for_file(
        &self,
        file_path: &str,
    ) -> Result<Option<LatestFileOutcome>> {
        // Try graph query first
        if let Some(graph_conn) = self.graph_conn() {
            if let Ok(Some(outcome)) = self.get_latest_outcome_graph(graph_conn, file_path) {
                return Ok(Some(outcome));
            }
        }

        // Fallback to SQLite
        self.get_latest_outcome_fallback(file_path)
    }

    fn get_latest_outcome_graph(
        &self,
        graph_conn: &Connection,
        file_path: &str,
    ) -> Result<Option<LatestFileOutcome>> {
        let mut stmt = graph_conn.prepare(
            "SELECT
                json_extract(ge_exec.data, '$.execution_id') AS execution_id,
                json_extract(ge_exec.data, '$.tool') AS tool_name,
                json_extract(ge_exec.data, '$.timestamp') AS timestamp,
                json_extract(ge_exec.data, '$.success') AS success,
                edge_type
             FROM graph_entities ge_exec
             JOIN graph_edges e ON ge_exec.id = e.from_id
             JOIN graph_entities ge_file ON e.to_id = ge_file.id
             WHERE ge_exec.kind = 'execution'
               AND ge_file.kind = 'file'
               AND ge_file.file_path = ?1
               AND e.edge_type IN ('EXECUTED_ON', 'AFFECTED')
             ORDER BY json_extract(ge_exec.data, '$.timestamp') DESC
             LIMIT 1",
        )?;

        let result = stmt.query_row(params![file_path], |r| {
            Ok(LatestFileOutcome {
                execution_id: r.get(0)?,
                tool_name: r.get(1)?,
                timestamp: r.get(2)?,
                success: r.get::<_, i64>(3)? == 1,
                edge_type: r.get(4)?,
                data_source: DataSource::Graph,
            })
        });

        match result {
            Ok(outcome) => Ok(Some(outcome)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn get_latest_outcome_fallback(&self, file_path: &str) -> Result<Option<LatestFileOutcome>> {
        let mut stmt = self.conn().prepare(
            "SELECT id, tool_name, timestamp, success
             FROM executions
             WHERE tool_name IN ('file_write', 'splice_patch')
               AND json_extract(arguments_json, '$.file') = ?1
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let result = stmt.query_row(params![file_path], |r| {
            Ok(LatestFileOutcome {
                execution_id: r.get(0)?,
                tool_name: r.get(1)?,
                timestamp: r.get(2)?,
                success: r.get::<_, i64>(3)? == 1,
                edge_type: "UNKNOWN".to_string(),
                data_source: DataSource::Fallback,
            })
        });

        match result {
            Ok(outcome) => Ok(Some(outcome)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Q7: GetRecurringDiagnostics
    ///
    /// Find diagnostics that occur repeatedly (grouped by code and file).
    ///
    /// ORDER BY: occurrence_count DESC, diagnostic_code ASC, file_name ASC
    pub fn get_recurring_diagnostics(
        &self,
        threshold: usize,
        since: Option<i64>,
    ) -> Result<Vec<RecurringDiagnostic>> {
        let threshold_val = threshold as i64;
        let mut stmt = self.conn().prepare(
            "SELECT
                json_extract(a.value, '$.code') AS diagnostic_code,
                json_extract(a.value, '$.file_name') AS file_name,
                COUNT(DISTINCT e.id) AS occurrence_count,
                MIN(e.timestamp) AS first_seen,
                MAX(e.timestamp) AS last_seen,
                GROUP_CONCAT(e.id, ',') AS execution_ids
             FROM executions e
             JOIN execution_artifacts a ON e.id = a.execution_id,
                  json_each(a.content_json) AS a
             WHERE a.artifact_type = 'diagnostics'
               AND (?2 IS NULL OR e.timestamp >= ?2)
             GROUP BY diagnostic_code, file_name
             HAVING occurrence_count >= ?1
             ORDER BY occurrence_count DESC, diagnostic_code ASC, file_name ASC",
        )?;

        let rows = stmt.query_map(params![threshold_val, since], |r| {
            let ids_str: String = r.get(5)?;
            let execution_ids: Vec<String> = if ids_str.is_empty() {
                Vec::new()
            } else {
                ids_str.split(',').map(|s| s.to_string()).collect()
            };
            Ok(RecurringDiagnostic {
                diagnostic_code: r.get(0)?,
                file_name: r.get(1)?,
                occurrence_count: r.get(2)?,
                first_seen: r.get(3)?,
                last_seen: r.get(4)?,
                execution_ids,
            })
        })?;

        rows.collect::<::std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    /// Q8: FindPriorFixesForDiagnostic
    ///
    /// Find mutation executions that occurred AFTER diagnostic occurrences.
    ///
    /// EVIDENCE ONLY: Temporal adjacency, NOT causality.
    ///
    /// ORDER BY: diagnostic_timestamp ASC, fix_timestamp ASC
    pub fn find_prior_fixes_for_diagnostic(
        &self,
        code: &str,
        file_path: Option<&str>,
        since: Option<i64>,
    ) -> Result<Vec<PriorFix>> {
        let mut stmt = self.conn().prepare(
            "WITH diagnostics AS (
                SELECT
                    e.id AS diagnostic_execution_id,
                    e.timestamp AS diagnostic_timestamp,
                    json_extract(a.value, '$.file_name') AS file_name
                FROM executions e
                JOIN execution_artifacts a ON e.id = a.execution_id,
                     json_each(a.content_json) AS a
                WHERE a.artifact_type = 'diagnostics'
                  AND json_extract(a.value, '$.code') = ?1
                  AND (?3 IS NULL OR e.timestamp >= ?3)
            ),
            fixes AS (
                SELECT
                    e.id AS fix_execution_id,
                    e.timestamp AS fix_timestamp,
                    e.tool_name,
                    e.success
                FROM executions e
                WHERE e.tool_name IN ('splice_patch', 'file_write')
                  AND (?3 IS NULL OR e.timestamp >= ?3)
            )
            SELECT
                f.fix_execution_id AS execution_id,
                f.tool_name,
                f.fix_timestamp AS timestamp,
                d.diagnostic_execution_id,
                (f.fix_timestamp - d.diagnostic_timestamp) AS temporal_gap_ms,
                f.success
            FROM diagnostics d
            JOIN fixes f ON f.fix_timestamp > d.diagnostic_timestamp
            WHERE (?2 IS NULL OR d.file_name = ?2)
            ORDER BY d.diagnostic_timestamp ASC, f.fix_timestamp ASC",
        )?;

        let rows = stmt.query_map(params![code, file_path, since], |r| {
            Ok(PriorFix {
                execution_id: r.get(0)?,
                tool_name: r.get(1)?,
                timestamp: r.get(2)?,
                diagnostic_execution_id: r.get(3)?,
                temporal_gap_ms: r.get(4)?,
                success: r.get::<_, i64>(5)? == 1,
            })
        })?;

        rows.collect::<::std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
}

// Row conversion helpers

fn row_to_execution_summary(r: &Row) -> rusqlite::Result<ExecutionSummary> {
    Ok(ExecutionSummary {
        execution_id: r.get(0)?,
        tool_name: r.get(1)?,
        timestamp: r.get(2)?,
        success: r.get::<_, i64>(3)? == 1,
        exit_code: r.get(4)?,
        duration_ms: r.get(5)?,
        error_message: r.get(6)?,
    })
}

fn row_to_failure_summary(r: &Row) -> rusqlite::Result<FailureSummary> {
    Ok(FailureSummary {
        execution_id: r.get(0)?,
        tool_name: r.get(1)?,
        timestamp: r.get(2)?,
        exit_code: r.get(3)?,
        error_message: r.get(4)?,
    })
}

fn row_to_diagnostic_execution(r: &Row) -> rusqlite::Result<DiagnosticExecution> {
    Ok(DiagnosticExecution {
        execution_id: r.get(0)?,
        tool_name: r.get(1)?,
        timestamp: r.get(2)?,
        diagnostic_code: r.get(3)?,
        diagnostic_level: r.get(4)?,
        diagnostic_message: r.get(5)?,
        file_name: r.get(6)?,
    })
}

fn row_to_file_execution(r: &Row, data_source: DataSource) -> rusqlite::Result<FileExecution> {
    Ok(FileExecution {
        execution_id: r.get(0)?,
        tool_name: r.get(1)?,
        timestamp: r.get(2)?,
        success: r.get::<_, i64>(3)? == 1,
        edge_type: r.get(4)?,
        data_source,
    })
}
