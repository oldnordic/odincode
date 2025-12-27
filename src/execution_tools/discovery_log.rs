//! Discovery event logging (Phase 10.7)
//!
//! Logs tool discovery events to execution_log.db for:
//! - Audit trail of which tools were available
//! - Debug "why did LLM have tool X?"
//! - Learn patterns for better trigger definitions

use crate::execution_tools::Error;
use crate::llm::discovery::ToolDiscoveryContext;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Discovery event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryEvent {
    pub id: i64,
    pub session_id: String,
    pub user_query_hash: String,
    pub tools_discovered: Vec<String>,
    pub reason: String,
    pub timestamp: i64,
}

/// Log a discovery event to execution_log.db
///
/// # Arguments
///
/// * `db` — Execution database connection
/// * `session_id` — Session identifier for grouping events
/// * `context` — Discovery context containing user query
/// * `tools_discovered` — Tools that were discovered (sorted)
/// * `reason` — Human-readable reason for discovery (e.g., "keyword: write")
pub fn log_discovery_event(
    db: &crate::execution_tools::ExecutionDb,
    session_id: &str,
    context: &ToolDiscoveryContext,
    tools_discovered: &[String],
    reason: &str,
) -> Result<(), Error> {
    // Compute query hash (using std hash)
    let query_hash = hash_query(&context.user_query);

    // Serialize tools as JSON
    let tools_json = serde_json::to_string(tools_discovered)?;

    // Get current timestamp (milliseconds since UNIX epoch)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e) as Box<dyn std::error::Error + Send + Sync>))?
        .as_millis() as i64;

    // Insert into database
    db.conn().execute(
        "INSERT INTO discovery_events (session_id, user_query_hash, tools_discovered, reason, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (session_id, &query_hash, &tools_json, reason, timestamp),
    )?;

    Ok(())
}

/// Query discovery events by session ID
///
/// Returns all discovery events for a session, sorted by timestamp.
pub fn query_discovery_events(
    db: &crate::execution_tools::ExecutionDb,
    session_id: &str,
) -> Result<Vec<DiscoveryEvent>, Error> {
    let mut stmt = db.conn().prepare(
        "SELECT id, session_id, user_query_hash, tools_discovered, reason, timestamp
         FROM discovery_events
         WHERE session_id = ?1
         ORDER BY timestamp ASC",
    )?;

    let events = stmt.query_map([session_id], |row| {
        Ok(DiscoveryEvent {
            id: row.get(0)?,
            session_id: row.get(1)?,
            user_query_hash: row.get(2)?,
            tools_discovered: serde_json::from_str(row.get::<_, String>(3)?.as_str())
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e) as Box<dyn std::error::Error + Send + Sync>))?,
            reason: row.get(4)?,
            timestamp: row.get(5)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(events)
}

/// Compute hash of user query
///
/// Used to group similar queries and detect patterns.
/// Uses std::collections::hash_map::DefaultHasher.
fn hash_query(query: &str) -> String {
    let mut hasher = DefaultHasher::new();
    query.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_query_is_deterministic() {
        let query = "write a file";
        let hash1 = hash_query(query);
        let hash2 = hash_query(query);

        assert_eq!(hash1, hash2, "Hash should be deterministic");
        assert!(hash1.len() <= 16, "Hash should be reasonable length");
    }

    #[test]
    fn test_hash_query_is_unique() {
        let hash1 = hash_query("write a file");
        let hash2 = hash_query("read a file");

        assert_ne!(hash1, hash2, "Different queries should have different hashes");
    }
}
