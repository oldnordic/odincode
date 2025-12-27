//! file_read: Read entire file into memory
//!
//! Reads file as UTF-8 string. Returns error if file does not exist.

use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during file read operations
#[derive(Error, Debug)]
pub enum FileReadError {
    #[error("File not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid UTF-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
}

/// Result type for file operations
pub type Result<T> = std::result::Result<T, FileReadError>;

/// Read entire file into a UTF-8 string
///
/// # Arguments
/// * `path` - Path to the file to read
///
/// # Returns
/// * `Ok(String)` - File contents as UTF-8 string
/// * `Err(FileReadError)` - Error if file missing or unreadable
///
/// # Examples
/// ```ignore
/// use odincode::file_tools::file_read;
/// use std::path::Path;
///
/// let content = file_read(Path::new("test.txt"))?;
/// ```
pub fn file_read(path: &Path) -> Result<String> {
    if !path.exists() {
        return Err(FileReadError::NotFound(path.display().to_string()));
    }

    let content = fs::read_to_string(path)?;
    Ok(content)
}
