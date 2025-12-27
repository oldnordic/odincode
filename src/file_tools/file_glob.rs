//! file_glob: List files matching glob pattern
//!
//! Uses glob crate for pattern matching.
//! Returns deterministically sorted results.

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during glob operations
#[derive(Error, Debug)]
pub enum GlobError {
    #[error("Invalid glob pattern: {0}")]
    InvalidPattern(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for glob operations
pub type Result<T> = std::result::Result<T, GlobError>;

/// List files matching glob pattern under root directory
///
/// Uses glob patterns to match files. Returns results in deterministic
/// (sorted) order.
///
/// # Arguments
/// * `pattern` - Glob pattern (e.g., "*.txt", "**/*.rs")
/// * `root` - Root directory to search in
///
/// # Returns
/// * `Ok(Vec<PathBuf>)` - List of matching files (sorted)
/// * `Err(GlobError)` - Error if pattern invalid
///
/// # Examples
/// ```ignore
/// use odincode::file_tools::file_glob;
/// use std::path::Path;
///
/// let files = file_glob("*.rs", Path::new("./src"))?;
/// for file in files {
///     println!("{}", file.display());
/// }
/// ```
pub fn file_glob(pattern: &str, root: &Path) -> Result<Vec<PathBuf>> {
    // Build full glob pattern
    let full_pattern = root.join(pattern);

    // Compile glob pattern
    let glob_obj = glob::glob(&full_pattern.to_string_lossy())
        .map_err(|e| GlobError::InvalidPattern(e.to_string()))?;

    // Collect matching files (filter out directories and errors)
    let mut files: Vec<PathBuf> = glob_obj
        .filter_map(|entry| match entry {
            Ok(path) if path.is_file() => Some(path),
            _ => None,
        })
        .collect();

    // Sort for deterministic output
    files.sort();

    Ok(files)
}
