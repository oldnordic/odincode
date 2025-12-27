//! Read-only queries with deterministic ordering
//!
//! All queries include ORDER BY for reproducible results.

use crate::execution_tools::{db::ExecutionDb, record::Execution};
use anyhow::Result;

impl ExecutionDb {
    /// Query executions by tool name (deterministic: ORDER BY timestamp ASC)
    pub fn query_by_tool(&self, tool_name: &str) -> Result<Vec<Execution>> {
        let mut stmt = self.conn().prepare(
            "SELECT id, tool_name, timestamp, success FROM executions WHERE tool_name = ? ORDER BY timestamp ASC"
        )?;

        let executions = stmt
            .query_map([tool_name], |row| {
                Ok(Execution {
                    id: row.get(0)?,
                    tool_name: row.get(1)?,
                    timestamp: row.get(2)?,
                    success: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(executions)
    }

    /// Query all executions (deterministic: ORDER BY timestamp ASC)
    pub fn query_all(&self) -> Result<Vec<Execution>> {
        let mut stmt = self.conn().prepare(
            "SELECT id, tool_name, timestamp, success FROM executions ORDER BY timestamp ASC",
        )?;

        let executions = stmt
            .query_map([], |row| {
                Ok(Execution {
                    id: row.get(0)?,
                    tool_name: row.get(1)?,
                    timestamp: row.get(2)?,
                    success: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(executions)
    }
}
