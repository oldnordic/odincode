//! file_search: Search files using ripgrep
//!
//! Wraps `rg` (ripgrep) via std::process::Command.
//! Returns structured matches with file path, line number, and line text.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use thiserror::Error;

/// A single search match
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchMatch {
    /// File path (absolute or relative to root)
    pub file_path: String,
    /// Line number (1-indexed)
    pub line_number: usize,
    /// Full line text containing match
    pub line: String,
}

/// Errors that can occur during search operations
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("ripgrep not found: is 'rg' installed?")]
    RipgrepNotFound,

    #[error("Search failed: {0}")]
    SearchFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for search operations
pub type Result<T> = std::result::Result<T, SearchError>;

/// Search for pattern in files under root directory
///
/// Uses ripgrep (rg) to perform regex search.
/// Returns structured matches with file path, line number, and line text.
///
/// # Arguments
/// * `pattern` - Regex pattern to search for
/// * `root` - Root directory to search in
///
/// # Returns
/// * `Ok(Vec<SearchMatch>)` - List of matches (empty if none)
/// * `Err(SearchError)` - Error if rg not found or invalid regex
///
/// # Examples
/// ```ignore
/// use odincode::file_tools::file_search;
/// use std::path::Path;
///
/// let matches = file_search("TODO", Path::new("./src"))?;
/// for m in matches {
///     println!("{}:{}: {}", m.file_path, m.line_number, m.line);
/// }
/// ```
pub fn file_search(pattern: &str, root: &Path) -> Result<Vec<SearchMatch>> {
    // Invoke ripgrep with simple line-by-line format
    let output = Command::new("rg")
        .arg("--line-number") // Show line numbers
        .arg("--no-heading") // Don't group by file
        .arg("--no-column") // Don't show column numbers
        .arg(pattern)
        .arg(root)
        .output()
        .map_err(|_| SearchError::RipgrepNotFound)?;

    // Check if ripgrep failed (e.g., invalid regex)
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("parse error") || stderr.contains("invalid regex") {
            return Err(SearchError::SearchFailed(stderr.to_string()));
        }
        // Exit code 1 means no matches found
        if output.status.code() == Some(1) {
            return Ok(Vec::new());
        }
        return Err(SearchError::SearchFailed(stderr.to_string()));
    }

    // Parse ripgrep output
    // Format: "file_path:line_number:line_text"
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut matches = Vec::new();

    for line in stdout.lines() {
        // Find first colon (separates file path from line number)
        let first_colon = match line.find(':') {
            Some(pos) => pos,
            None => continue,
        };

        let file_path = &line[..first_colon];
        let rest = &line[first_colon + 1..];

        // Find second colon (separates line number from line text)
        let second_colon = match rest.find(':') {
            Some(pos) => pos,
            None => continue,
        };

        let line_number_str = &rest[..second_colon];
        let line_text = &rest[second_colon + 1..];

        // Parse line number
        match line_number_str.parse::<usize>() {
            Ok(line_number) => {
                matches.push(SearchMatch {
                    file_path: file_path.to_string(),
                    line_number,
                    line: line_text.to_string(),
                });
            }
            Err(_) => continue,
        }
    }

    Ok(matches)
}
