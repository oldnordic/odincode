//! count_files tool — Count files matching a pattern
//!
//! Usage:
//! - By glob pattern: count all files matching a glob pattern
//! - Optionally: group by extension
//!
//! Returns:
//! - total_count: Total number of files
//! - by_extension: Map of extension -> count (optional)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::file_glob;

/// Arguments for count_files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountFilesArgs {
    /// Glob pattern (e.g., "**/*.rs", "**/*")
    pub pattern: String,
    /// Root directory (default: ".")
    #[serde(default)]
    pub root: String,
    /// Group results by extension
    #[serde(default)]
    pub by_extension: bool,
}

/// Result from count_files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountFilesResult {
    /// Total file count
    pub total_count: usize,
    /// Files grouped by extension (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_extension: Option<HashMap<String, usize>>,
}

/// Count files matching a glob pattern
///
/// Pure Rust implementation using crate::file_glob.
pub fn count_files(args: CountFilesArgs) -> Result<CountFilesResult, String> {
    let root = Path::new(&args.root);
    let paths = file_glob(&args.pattern, root).map_err(|e| e.to_string())?;

    let total_count = paths.len();

    let by_extension = if args.by_extension {
        let mut ext_counts: HashMap<String, usize> = HashMap::new();

        for path in &paths {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("(no extension)");
            *ext_counts.entry(ext.to_string()).or_insert(0) += 1;
        }

        Some(ext_counts)
    } else {
        None
    };

    Ok(CountFilesResult {
        total_count,
        by_extension,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn test_count_files_basic() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        File::create(root.join("file1.txt")).unwrap();
        File::create(root.join("file2.txt")).unwrap();
        File::create(root.join("file3.rs")).unwrap();

        let args = CountFilesArgs {
            pattern: "*.txt".to_string(),
            root: root.display().to_string(),
            by_extension: false,
        };

        let result = count_files(args).unwrap();
        assert_eq!(result.total_count, 2);
        assert!(result.by_extension.is_none());
    }

    #[test]
    fn test_count_files_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create nested files
        fs::create_dir_all(root.join("subdir")).unwrap();
        File::create(root.join("file1.txt")).unwrap();
        File::create(root.join("subdir/file2.txt")).unwrap();

        let args = CountFilesArgs {
            pattern: "**/*.txt".to_string(),
            root: root.display().to_string(),
            by_extension: false,
        };

        let result = count_files(args).unwrap();
        assert_eq!(result.total_count, 2);
    }

    #[test]
    fn test_count_files_by_extension() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        File::create(root.join("file1.txt")).unwrap();
        File::create(root.join("file2.txt")).unwrap();
        File::create(root.join("file3.rs")).unwrap();
        File::create(root.join("README")).unwrap();

        let args = CountFilesArgs {
            pattern: "*".to_string(),
            root: root.display().to_string(),
            by_extension: true,
        };

        let result = count_files(args).unwrap();
        assert_eq!(result.total_count, 4);

        let ext_map = result.by_extension.unwrap();
        assert_eq!(ext_map.get("txt"), Some(&2));
        assert_eq!(ext_map.get("rs"), Some(&1));
        assert_eq!(ext_map.get("(no extension)"), Some(&1));
    }

    #[test]
    fn test_count_files_empty() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let args = CountFilesArgs {
            pattern: "*.txt".to_string(),
            root: root.display().to_string(),
            by_extension: false,
        };

        let result = count_files(args).unwrap();
        assert_eq!(result.total_count, 0);
    }
}
