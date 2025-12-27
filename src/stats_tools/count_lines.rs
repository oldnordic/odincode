//! count_lines tool — Count lines in files
//!
//! Usage:
//! - Count lines in a single file
//! - Count lines in multiple files
//!
//! Returns:
//! - total_lines: Total line count across all files
//! - per_file: Breakdown by file

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::file_glob;

/// Arguments for count_lines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountLinesArgs {
    /// Glob pattern for files to count (e.g., "**/*.rs")
    pub pattern: String,
    /// Root directory (default: ".")
    #[serde(default)]
    pub root: String,
}

/// Line count for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLineCount {
    /// File path
    pub path: String,
    /// Line count
    pub lines: usize,
}

/// Result from count_lines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountLinesResult {
    /// Total lines across all files
    pub total_lines: usize,
    /// File count
    pub file_count: usize,
    /// Per-file breakdown
    pub per_file: Vec<FileLineCount>,
}

/// Count lines in files matching a glob pattern
///
/// Pure Rust implementation using std::io::BufRead.
pub fn count_lines(args: CountLinesArgs) -> Result<CountLinesResult, String> {
    let root = Path::new(&args.root);
    let paths = file_glob(&args.pattern, root).map_err(|e| e.to_string())?;

    let mut total_lines = 0;
    let mut per_file = Vec::new();

    for path in &paths {
        match count_lines_in_file(path) {
            Ok(lines) => {
                total_lines += lines;
                per_file.push(FileLineCount {
                    path: path.display().to_string(),
                    lines,
                });
            }
            Err(_e) => {
                // Skip files that can't be read, but include in result as error
                per_file.push(FileLineCount {
                    path: path.display().to_string(),
                    lines: 0,
                });
            }
        }
    }

    // Sort by path for deterministic output
    per_file.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(CountLinesResult {
        total_lines,
        file_count: paths.len(),
        per_file,
    })
}

/// Count lines in a single file
fn count_lines_in_file(path: &Path) -> Result<usize, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = BufReader::new(file);

    let mut count = 0;
    for line in reader.lines() {
        line.map_err(|e| format!("Failed to read line: {}", e))?;
        count += 1;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_count_lines_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test file with known line count
        let mut file = File::create(root.join("test.txt")).unwrap();
        writeln!(file, "line1").unwrap();
        writeln!(file, "line2").unwrap();
        writeln!(file, "line3").unwrap();

        let args = CountLinesArgs {
            pattern: "*.txt".to_string(),
            root: root.display().to_string(),
        };

        let result = count_lines(args).unwrap();
        assert_eq!(result.total_lines, 3);
        assert_eq!(result.file_count, 1);
        assert_eq!(result.per_file.len(), 1);
        assert_eq!(result.per_file[0].lines, 3);
    }

    #[test]
    fn test_count_lines_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        let mut file1 = File::create(root.join("file1.txt")).unwrap();
        for i in 0..5 {
            writeln!(file1, "line{}", i).unwrap();
        }

        let mut file2 = File::create(root.join("file2.txt")).unwrap();
        for i in 0..3 {
            writeln!(file2, "line{}", i).unwrap();
        }

        let args = CountLinesArgs {
            pattern: "*.txt".to_string(),
            root: root.display().to_string(),
        };

        let result = count_lines(args).unwrap();
        assert_eq!(result.total_lines, 8);
        assert_eq!(result.file_count, 2);
        assert_eq!(result.per_file.len(), 2);
    }

    #[test]
    fn test_count_lines_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        File::create(root.join("empty.txt")).unwrap();

        let args = CountLinesArgs {
            pattern: "*.txt".to_string(),
            root: root.display().to_string(),
        };

        let result = count_lines(args).unwrap();
        assert_eq!(result.total_lines, 0);
        assert_eq!(result.per_file[0].lines, 0);
    }

    #[test]
    fn test_count_lines_no_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let args = CountLinesArgs {
            pattern: "*.txt".to_string(),
            root: root.display().to_string(),
        };

        let result = count_lines(args).unwrap();
        assert_eq!(result.total_lines, 0);
        assert_eq!(result.file_count, 0);
        assert_eq!(result.per_file.len(), 0);
    }
}
