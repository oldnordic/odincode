//! File edit tools — Patch-based text file editing
//!
//! Provides line-based text editing without requiring AST parsing.
//! Simpler than splice for quick text changes.
//!
//! ## Architecture
//!
//! - `mod.rs` — Module exports and file_edit function

use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Error type for file edit operations
#[derive(Debug, thiserror::Error)]
pub enum FileEditError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Line number out of range: {0} (file has {1} lines)")]
    LineOutOfRange(usize, usize),

    #[error("Pattern not found in file: {0}")]
    PatternNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Read error: {0}")]
    ReadError(String),
}

/// File edit arguments
#[derive(Debug, Clone)]
pub struct FileEditArgs {
    /// Path to the file to edit
    pub file: PathBuf,
    /// Edit to apply
    pub edit: FileEdit,
}

/// File edit operation
#[derive(Debug, Clone)]
pub enum FileEdit {
    /// Replace line at specific line number (1-indexed)
    ReplaceLine {
        line_number: usize,
        new_content: String,
    },
    /// Replace lines matching a pattern
    ReplacePattern {
        pattern: String,
        new_content: String,
        replace_all: bool,
    },
    /// Insert line after specific line number (0 = before first line)
    InsertLine {
        after_line: usize,
        content: String,
    },
    /// Delete line at specific line number
    DeleteLine {
        line_number: usize,
    },
    /// Delete lines matching pattern
    DeletePattern {
        pattern: String,
    },
}

/// File edit result
#[derive(Debug, Clone)]
pub struct FileEditResult {
    /// Path that was edited
    pub path: PathBuf,
    /// Number of lines modified
    pub lines_modified: usize,
    /// Number of lines inserted
    pub lines_inserted: usize,
    /// Number of lines deleted
    pub lines_deleted: usize,
    /// Success flag
    pub success: bool,
}

/// Edit a file using line-based operations
///
/// This is simpler than splice for quick text changes.
/// Does not validate syntax or structure.
pub fn file_edit(args: FileEditArgs) -> Result<FileEditResult> {
    let file = &args.file;

    // Check file exists
    if !file.exists() {
        return Err(FileEditError::FileNotFound(file.display().to_string()).into());
    }

    // Read file content
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    let mut result = FileEditResult {
        path: file.clone(),
        lines_modified: 0,
        lines_inserted: 0,
        lines_deleted: 0,
        success: false,
    };

    match args.edit {
        FileEdit::ReplaceLine { line_number, new_content } => {
            if line_number == 0 || line_number > lines.len() {
                return Err(FileEditError::LineOutOfRange(line_number, lines.len()).into());
            }
            lines[line_number - 1] = new_content;
            result.lines_modified = 1;
        }
        FileEdit::ReplacePattern { pattern, new_content, replace_all } => {
            let mut found = false;
            for line in &mut lines {
                if line.contains(&pattern) {
                    *line = new_content.clone();
                    result.lines_modified += 1;
                    found = true;
                    if !replace_all {
                        break;
                    }
                }
            }
            if !found {
                return Err(FileEditError::PatternNotFound(pattern).into());
            }
        }
        FileEdit::InsertLine { after_line, content } => {
            if after_line > lines.len() {
                return Err(FileEditError::LineOutOfRange(after_line, lines.len()).into());
            }
            lines.insert(after_line, content);
            result.lines_inserted = 1;
        }
        FileEdit::DeleteLine { line_number } => {
            if line_number == 0 || line_number > lines.len() {
                return Err(FileEditError::LineOutOfRange(line_number, lines.len()).into());
            }
            lines.remove(line_number - 1);
            result.lines_deleted = 1;
        }
        FileEdit::DeletePattern { pattern } => {
            let original_len = lines.len();
            lines.retain(|line| !line.contains(&pattern));
            let deleted = original_len - lines.len();
            if deleted == 0 {
                return Err(FileEditError::PatternNotFound(pattern).into());
            }
            result.lines_deleted = deleted;
        }
    }

    // Write back to file
    let new_content = lines.join("\n") + "\n";

    // Atomic write pattern: temp file + rename
    let temp_path = file.with_extension("tmp");
    {
        let mut temp_file = fs::File::create(&temp_path)
            .with_context(|| format!("Failed to create temp file: {}", temp_path.display()))?;
        temp_file.write_all(new_content.as_bytes())?;
        temp_file.sync_all()?;
    }

    // Atomic rename
    fs::rename(&temp_path, file)
        .with_context(|| format!("Failed to rename {} to {}", temp_path.display(), file.display()))?;

    result.success = true;
    Ok(result)
}

/// Read a file and return line count
pub fn file_line_count(file: &Path) -> Result<usize> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;
    Ok(content.lines().count())
}

/// Find line numbers matching a pattern
pub fn find_lines(file: &Path, pattern: &str) -> Result<Vec<usize>> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    let mut matches = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        if line.contains(pattern) {
            matches.push(idx + 1); // 1-indexed
        }
    }
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_replace_line() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "line1\nline2\nline3\n").unwrap();

        let args = FileEditArgs {
            file: file_path.clone(),
            edit: FileEdit::ReplaceLine {
                line_number: 2,
                new_content: "modified".to_string(),
            },
        };

        let result = file_edit(args).unwrap();
        assert!(result.success);
        assert_eq!(result.lines_modified, 1);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "line1\nmodified\nline3\n");
    }

    #[test]
    fn test_replace_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "line1\nline2\nline3\n").unwrap();

        let args = FileEditArgs {
            file: file_path.clone(),
            edit: FileEdit::ReplacePattern {
                pattern: "line".to_string(),
                new_content: "modified".to_string(),
                replace_all: false,
            },
        };

        let result = file_edit(args).unwrap();
        assert!(result.success);
        assert_eq!(result.lines_modified, 1);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "modified\nline2\nline3\n");
    }

    #[test]
    fn test_replace_pattern_all() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "line1\nline2\nline3\n").unwrap();

        let args = FileEditArgs {
            file: file_path.clone(),
            edit: FileEdit::ReplacePattern {
                pattern: "line".to_string(),
                new_content: "modified".to_string(),
                replace_all: true,
            },
        };

        let result = file_edit(args).unwrap();
        assert!(result.success);
        assert_eq!(result.lines_modified, 3);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "modified\nmodified\nmodified\n");
    }

    #[test]
    fn test_insert_line() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "line1\nline2\n").unwrap();

        let args = FileEditArgs {
            file: file_path.clone(),
            edit: FileEdit::InsertLine {
                after_line: 1,
                content: "inserted".to_string(),
            },
        };

        let result = file_edit(args).unwrap();
        assert!(result.success);
        assert_eq!(result.lines_inserted, 1);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "line1\ninserted\nline2\n");
    }

    #[test]
    fn test_delete_line() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "line1\nline2\nline3\n").unwrap();

        let args = FileEditArgs {
            file: file_path.clone(),
            edit: FileEdit::DeleteLine { line_number: 2 },
        };

        let result = file_edit(args).unwrap();
        assert!(result.success);
        assert_eq!(result.lines_deleted, 1);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "line1\nline3\n");
    }

    #[test]
    fn test_delete_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "line1\nkeep\nline3\nkeep\n").unwrap();

        let args = FileEditArgs {
            file: file_path.clone(),
            edit: FileEdit::DeletePattern {
                pattern: "line".to_string(),
            },
        };

        let result = file_edit(args).unwrap();
        assert!(result.success);
        assert_eq!(result.lines_deleted, 2);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "keep\nkeep\n");
    }

    #[test]
    fn test_line_out_of_range() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "line1\nline2\n").unwrap();

        let args = FileEditArgs {
            file: file_path.clone(),
            edit: FileEdit::ReplaceLine {
                line_number: 10,
                new_content: "test".to_string(),
            },
        };

        let result = file_edit(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "line1\nline2\n").unwrap();

        let args = FileEditArgs {
            file: file_path.clone(),
            edit: FileEdit::ReplacePattern {
                pattern: "notfound".to_string(),
                new_content: "test".to_string(),
                replace_all: false,
            },
        };

        let result = file_edit(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_file_line_count() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "line1\nline2\nline3\n").unwrap();

        let count = file_line_count(&file_path).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_find_lines() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "line1\nkeep\nline3\nkeep\n").unwrap();

        let matches = find_lines(&file_path, "keep").unwrap();
        assert_eq!(matches, vec![2, 4]);
    }
}
