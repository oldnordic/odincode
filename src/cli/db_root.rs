//! db_root resolution
//!
//! Implements Phase 4 db_root resolution priority:
//! 1. --db-root <path> flag (highest priority)
//! 2. $ODINCODE_HOME env var → $ODINCODE_HOME/db
//! 3. Current directory "." (default)

use crate::cli::Result;
use std::path::{Path, PathBuf};

/// Resolve db_root according to Phase 4 specification
///
/// # Resolution Priority
/// 1. Explicit `db_root` from --db-root flag (if Some)
/// 2. $ODINCODE_HOME environment variable → $ODINCODE_HOME/db
/// 3. Current directory "."
///
/// # Arguments
/// * `explicit` - Optional explicit db_root from --db-root flag
///
/// # Returns
/// * `Ok(PathBuf)` - Resolved db_root path
pub fn resolve_db_root(explicit: Option<String>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        // Explicit --db-root flag takes precedence
        let path_buf = PathBuf::from(&path);
        if !path_buf.exists() {
            return Err(crate::cli::Error::InvalidArgs(format!(
                "db_root '{}' does not exist",
                path
            )));
        }
        return Ok(path_buf);
    }

    // Check $ODINCODE_HOME environment variable
    if let Ok(home) = std::env::var("ODINCODE_HOME") {
        let home_path = PathBuf::from(home);
        let db_path = home_path.join("db");

        // Only use $ODINCODE_HOME/db if it exists
        if db_path.exists() {
            return Ok(db_path);
        }

        // If $ODINCODE_HOME is set but db doesn't exist, still use it
        // (it will be created or error will be raised later)
        return Ok(db_path);
    }

    // Default to current directory
    Ok(PathBuf::from("."))
}

/// Get db_root path as string (for display/error messages)
pub fn db_root_display(db_root: &Path) -> String {
    db_root.display().to_string()
}

/// Verify that db_root contains required databases
///
/// # Returns
/// * `Ok(())` - All required databases present
/// * `Err(Error)` - Missing required database
pub fn verify_db_root(db_root: &Path) -> Result<()> {
    // Check that db_root directory exists
    if !db_root.exists() {
        return Err(crate::cli::Error::InvalidArgs(format!(
            "db_root '{}' does not exist",
            db_root.display()
        )));
    }

    // execution_log.db is auto-created by ExecutionDb, so we don't require it here
    // codegraph.db is required for some operations but not all
    // This verification is minimal; actual DB opening will fail if needed

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_explicit_db_root() {
        let temp_dir = TempDir::new().unwrap();
        let explicit_path = temp_dir.path().to_str().unwrap().to_string();

        let resolved = resolve_db_root(Some(explicit_path.clone())).unwrap();
        assert_eq!(resolved, PathBuf::from(explicit_path));
    }

    #[test]
    fn test_resolve_explicit_nonexistent_fails() {
        let result = resolve_db_root(Some("/nonexistent/path/12345".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_defaults_to_current() {
        // Clear ODINCODE_HOME for this test
        std::env::remove_var("ODINCODE_HOME");

        let resolved = resolve_db_root(None).unwrap();
        assert_eq!(resolved, PathBuf::from("."));
    }

    #[test]
    fn test_resolve_env_var() {
        let temp_dir = TempDir::new().unwrap();
        let db_dir = temp_dir.path().join("db");
        fs::create_dir(&db_dir).unwrap();

        std::env::set_var("ODINCODE_HOME", temp_dir.path());
        let resolved = resolve_db_root(None).unwrap();
        std::env::remove_var("ODINCODE_HOME");

        assert_eq!(resolved, db_dir);
    }

    #[test]
    fn test_verify_db_root_exists() {
        let temp_dir = TempDir::new().unwrap();
        assert!(verify_db_root(temp_dir.path()).is_ok());
    }

    #[test]
    fn test_verify_db_root_not_exists() {
        let result = verify_db_root(Path::new("/nonexistent/odincode/12345"));
        assert!(result.is_err());
    }
}
