//! file_create: Create file if not exists
//!
//! Creates file and parent directories if needed.
//! Fails if file already exists (no overwrite).

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during file create operations
#[derive(Error, Debug)]
pub enum FileCreateError {
    #[error("File already exists: {0}")]
    AlreadyExists(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for file operations
pub type Result<T> = std::result::Result<T, FileCreateError>;

/// Create a new file with content
///
/// Creates parent directories if they don't exist.
/// Fails if file already exists.
///
/// # Arguments
/// * `path` - Path to the file to create
/// * `content` - Content to write (UTF-8)
///
/// # Returns
/// * `Ok(())` - Success
/// * `Err(FileCreateError)` - Error if file exists or creation fails
///
/// # Examples
/// ```ignore
/// use odincode::file_tools::file_create;
/// use std::path::Path;
///
/// file_create(Path::new("new.txt"), "New content")?;
/// ```
pub fn file_create(path: &Path, content: &str) -> Result<()> {
    // Check if file already exists
    if path.exists() {
        return Err(FileCreateError::AlreadyExists(path.display().to_string()));
    }

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Write file
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?; // fsync to ensure data is on disk

    Ok(())
}
