//! file_write: Atomically overwrite file with fsync
//!
//! Writes file atomically using write+rename pattern.
//! Fails if parent directory does not exist.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during file write operations
#[derive(Error, Debug)]
pub enum FileWriteError {
    #[error("Parent directory does not exist: {0}")]
    ParentNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for file operations
pub type Result<T> = std::result::Result<T, FileWriteError>;

/// Atomically write content to a file
///
/// Uses write+rename pattern for atomicity:
/// 1. Write to temporary file
/// 2. fsync to ensure data is on disk
/// 3. Rename over target (atomic on POSIX)
///
/// # Arguments
/// * `path` - Path to the file to write
/// * `content` - Content to write (UTF-8)
///
/// # Returns
/// * `Ok(())` - Success
/// * `Err(FileWriteError)` - Error if parent dir missing or write fails
///
/// # Examples
/// ```ignore
/// use odincode::file_tools::file_write;
/// use std::path::Path;
///
/// file_write(Path::new("output.txt"), "Hello, World!")?;
/// ```
pub fn file_write(path: &Path, content: &str) -> Result<()> {
    // Check parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(FileWriteError::ParentNotFound(parent.display().to_string()));
        }
    }

    // Create temporary file in same directory as target
    let temp_path = path.with_extension("tmp");

    // Write to temp file
    let mut file = File::create(&temp_path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?; // fsync to ensure data is on disk

    // Atomic rename
    fs::rename(&temp_path, path)?;

    Ok(())
}
