//! fs_stats tool — Directory statistics
//!
//! Usage:
//! - Get statistics for a directory tree
//!
//! Returns:
//! - file_count: Total number of files
//! - dir_count: Total number of directories
//! - total_bytes: Total size in bytes

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Arguments for fs_stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsStatsArgs {
    /// Root directory to analyze
    pub path: String,
    /// Maximum depth (0 = unlimited)
    #[serde(default)]
    pub max_depth: Option<usize>,
}

/// Result from fs_stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsStatsResult {
    /// Root path analyzed
    pub path: String,
    /// Number of files found
    pub file_count: usize,
    /// Number of directories found
    pub dir_count: usize,
    /// Total bytes
    pub total_bytes: u64,
    /// Optional breakdown by extension
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_extension: Option<Vec<ExtensionStats>>,
}

/// Statistics for a file extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStats {
    /// File extension (e.g., "rs", "txt")
    pub extension: String,
    /// Number of files with this extension
    pub count: usize,
    /// Total bytes for this extension
    pub bytes: u64,
}

/// Get directory statistics
///
/// Pure Rust implementation using std::fs.
pub fn fs_stats(args: FsStatsArgs) -> Result<FsStatsResult, String> {
    let root = Path::new(&args.path);

    if !root.exists() {
        return Err(format!("Path does not exist: {}", args.path));
    }

    let mut file_count = 0;
    let mut dir_count = 0;
    let mut total_bytes = 0;
    let mut ext_map: std::collections::HashMap<String, (usize, u64)> = std::collections::HashMap::new();

    walk_dir(root, args.max_depth, &mut file_count, &mut dir_count, &mut total_bytes, &mut ext_map)?;

    let mut by_extension: Vec<ExtensionStats> = ext_map
        .into_iter()
        .map(|(ext, (count, bytes))| ExtensionStats {
            extension: ext,
            count,
            bytes,
        })
        .collect();

    // Sort by count descending
    by_extension.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(FsStatsResult {
        path: args.path,
        file_count,
        dir_count,
        total_bytes,
        by_extension: if by_extension.is_empty() {
            None
        } else {
            Some(by_extension)
        },
    })
}

/// Walk directory tree recursively
fn walk_dir(
    path: &Path,
    max_depth: Option<usize>,
    file_count: &mut usize,
    dir_count: &mut usize,
    total_bytes: &mut u64,
    ext_map: &mut std::collections::HashMap<String, (usize, u64)>,
) -> Result<(), String> {
    let entries = fs::read_dir(path).map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to get file type: {}", e))?;

        if file_type.is_file() {
            *file_count += 1;

            // Get file size
            let metadata = entry
                .metadata()
                .map_err(|e| format!("Failed to get metadata: {}", e))?;
            let bytes = metadata.len();
            *total_bytes += bytes;

            // Track by extension
            let ext = entry_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("(no extension)")
                .to_string();
            let entry = ext_map.entry(ext).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += bytes;
        } else if file_type.is_dir() {
            *dir_count += 1;

            // Recurse if depth allows
            if max_depth.map(|d| d > 0).unwrap_or(true) {
                let new_max_depth = max_depth.map(|d| d - 1);
                walk_dir(&entry_path, new_max_depth, file_count, dir_count, total_bytes, ext_map)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_fs_stats_basic() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test structure
        let mut f1 = File::create(root.join("file1.txt")).unwrap();
        f1.write_all(b"hello").unwrap();
        let mut f2 = File::create(root.join("file2.rs")).unwrap();
        f2.write_all(b"world").unwrap();
        fs::create_dir(root.join("subdir")).unwrap();
        let mut f3 = File::create(root.join("subdir/file3.txt")).unwrap();
        f3.write_all(b"test").unwrap();

        let args = FsStatsArgs {
            path: root.display().to_string(),
            max_depth: None,
        };

        let result = fs_stats(args).unwrap();
        assert_eq!(result.file_count, 3);
        assert_eq!(result.dir_count, 1); // only subdir is counted (root is the starting point)
        assert!(result.total_bytes > 0);
    }

    #[test]
    fn test_fs_stats_by_extension() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create files with different extensions
        let mut f1 = File::create(root.join("file1.txt")).unwrap();
        f1.write_all(b"hello").unwrap();
        let mut f2 = File::create(root.join("file2.txt")).unwrap();
        f2.write_all(b"world").unwrap();
        File::create(root.join("file3.rs")).unwrap();

        let args = FsStatsArgs {
            path: root.display().to_string(),
            max_depth: None,
        };

        let result = fs_stats(args).unwrap();
        assert!(result.by_extension.is_some());

        let by_ext = result.by_extension.unwrap();
        let txt_entry = by_ext.iter().find(|e| e.extension == "txt").unwrap();
        assert_eq!(txt_entry.count, 2);
        assert_eq!(txt_entry.bytes, 10); // "hello" + "world"
    }

    #[test]
    fn test_fs_stats_max_depth() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create nested structure
        fs::create_dir_all(root.join("subdir/nested")).unwrap();
        File::create(root.join("file1.txt")).unwrap();
        File::create(root.join("subdir/file2.txt")).unwrap();
        File::create(root.join("subdir/nested/file3.txt")).unwrap();

        // Depth 0: only root
        let args = FsStatsArgs {
            path: root.display().to_string(),
            max_depth: Some(0),
        };

        let result = fs_stats(args).unwrap();
        assert_eq!(result.file_count, 1); // Only file1.txt

        // Depth 1: root + subdir
        let args = FsStatsArgs {
            path: root.display().to_string(),
            max_depth: Some(1),
        };

        let result = fs_stats(args).unwrap();
        assert_eq!(result.file_count, 2); // file1.txt + file2.txt
    }

    #[test]
    fn test_fs_stats_nonexistent() {
        let args = FsStatsArgs {
            path: "/nonexistent/path/that/does/not/exist".to_string(),
            max_depth: None,
        };

        let result = fs_stats(args);
        assert!(result.is_err());
    }
}
