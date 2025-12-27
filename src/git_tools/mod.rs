//! Git tools — Version control operations
//!
//! Provides access to git status, diff, and log.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Error type for git operations
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Git command failed: {0}")]
    CommandFailed(String),

    #[error("Not a git repository: {0}")]
    NotARepository(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Git status entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusEntry {
    /// File path
    pub path: String,
    /// Status (M=modified, A=added, D=deleted, ??=untracked, etc.)
    pub status: String,
}

/// Git diff entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffEntry {
    /// File path
    pub path: String,
    /// Number of additions
    pub additions: usize,
    /// Number of deletions
    pub deletions: usize,
    /// Unified diff (if available)
    pub diff: Option<String>,
}

/// Git log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLogEntry {
    /// Commit hash (abbreviated)
    pub hash: String,
    /// Full commit hash
    pub full_hash: String,
    /// Author name
    pub author: String,
    /// Commit date (ISO 8601)
    pub date: String,
    /// Commit message (subject line)
    pub message: String,
}

/// Get git status
///
/// Returns list of changed files with their status.
pub fn git_status(repo_root: &Path) -> Result<Vec<GitStatusEntry>> {
    // Check if we're in a git repo
    if !repo_root.join(".git").exists() {
        // Check parent directories
        let mut current = repo_root;
        loop {
            if current.join(".git").exists() {
                break;
            }
            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                return Err(GitError::NotARepository(repo_root.display().to_string()).into());
            }
        }
    }

    let output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(repo_root)
        .output()
        .context("Failed to execute git status")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(stderr.to_string()).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        // git status --porcelain format: XY PATH
        // X = index status, Y = work tree status
        // For renamed: XY PATH -> NEW_PATH
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            let status = parts[0].to_string();
            let path = parts[1].to_string();
            entries.push(GitStatusEntry { path, status });
        }
    }

    Ok(entries)
}

/// Get git diff
///
/// Returns diff for the working directory vs HEAD.
pub fn git_diff(repo_root: &Path) -> Result<Vec<GitDiffEntry>> {
    let output = Command::new("git")
        .arg("diff")
        .arg("--numstat")
        .current_dir(repo_root)
        .output()
        .context("Failed to execute git diff --numstat")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(stderr.to_string()).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        // git diff --numstat format: "additions\tdeletions\tpath"
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let additions: usize = parts[0].parse().unwrap_or(0);
            let deletions: usize = parts[1].parse().unwrap_or(0);
            let path = parts[2].to_string();
            entries.push(GitDiffEntry {
                path,
                additions,
                deletions,
                diff: None,
            });
        }
    }

    Ok(entries)
}

/// Get git diff for a specific file
///
/// Returns unified diff for a single file.
pub fn git_diff_file(repo_root: &Path, path: &str) -> Result<String> {
    let output = Command::new("git")
        .arg("diff")
        .arg("--")
        .arg(path)
        .current_dir(repo_root)
        .output()
        .context("Failed to execute git diff")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(stderr.to_string()).into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Get git log
///
/// Returns commit history.
/// Returns empty vec if repository has no commits (not an error).
pub fn git_log(repo_root: &Path, limit: Option<usize>) -> Result<Vec<GitLogEntry>> {
    let limit = limit.unwrap_or(20);

    // git log format: --pretty=format:'%H|%an|%ai|%s'
    let format_str = "%H|%an|%ai|%s";

    let output = Command::new("git")
        .arg("log")
        .arg(&format!("--pretty=format:{}", format_str))
        .arg(&format!("-n{}", limit))
        .current_dir(repo_root)
        .output()
        .context("Failed to execute git log")?;

    // If git log fails (e.g., no commits yet), return empty vec
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check for "no commits" error - return empty vec instead of error
        if stderr.contains("does not have any commits yet") {
            return Ok(Vec::new());
        }
        return Err(GitError::CommandFailed(stderr.to_string()).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            let full_hash = parts[0].to_string();
            let hash = full_hash.chars().take(8).collect();
            let author = parts[1].to_string();
            let date = parts[2].to_string();
            let message = parts[3].to_string();
            entries.push(GitLogEntry {
                hash,
                full_hash,
                author,
                date,
                message,
            });
        }
    }

    Ok(entries)
}

/// Check if path is in a git repository
pub fn is_git_repo(repo_root: &Path) -> bool {
    repo_root.join(".git").exists()
        || ancestors(repo_root).any(|p| p.join(".git").exists())
}

fn ancestors(path: &Path) -> impl Iterator<Item = &Path> {
    std::iter::successors(path.parent(), |p| p.parent())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_not_a_git_repo() {
        let temp_dir = TempDir::new().unwrap();
        let result = git_status(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_init_repo_and_status() {
        let temp_dir = TempDir::new().unwrap();

        // Initialize git repo
        Command::new("git")
            .arg("init")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Configure git
        Command::new("git")
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .arg("config")
            .arg("user.name")
            .arg("Test User")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let status = git_status(temp_dir.path()).unwrap();
        assert!(status.is_empty()); // No changes yet
    }

    #[test]
    fn test_is_git_repo() {
        let temp_dir = TempDir::new().unwrap();
        assert!(!is_git_repo(temp_dir.path()));

        // Initialize git repo
        Command::new("git")
            .arg("init")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        assert!(is_git_repo(temp_dir.path()));
    }

    #[test]
    fn test_git_log_empty() {
        let temp_dir = TempDir::new().unwrap();

        // Initialize git repo
        Command::new("git")
            .arg("init")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .arg("config")
            .arg("user.name")
            .arg("Test User")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Empty repo should return empty log
        let log = git_log(temp_dir.path(), Some(10)).unwrap();
        assert!(log.is_empty());
    }

    #[test]
    fn test_git_log_with_commit() {
        let temp_dir = TempDir::new().unwrap();

        // Initialize git repo
        Command::new("git")
            .arg("init")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .arg("config")
            .arg("user.name")
            .arg("Test User")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Create and commit a file
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "test content").unwrap();

        Command::new("git")
            .arg("add")
            .arg("test.txt")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("Test commit")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let log = git_log(temp_dir.path(), Some(10)).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].message, "Test commit");
        assert!(log[0].hash.len() == 8); // Abbreviated hash
    }

    #[test]
    fn test_git_diff() {
        let temp_dir = TempDir::new().unwrap();

        // Initialize git repo
        Command::new("git")
            .arg("init")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .arg("config")
            .arg("user.name")
            .arg("Test User")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Create and commit a file
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "line1\nline2\n").unwrap();

        Command::new("git")
            .arg("add")
            .arg("test.txt")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("Initial commit")
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Modify the file
        std::fs::write(&file_path, "line1\nmodified\nline3\n").unwrap();

        let diff = git_diff(temp_dir.path()).unwrap();
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].path, "test.txt");
        // line2 replaced with "modified" (1 deletion + 1 addition), line3 added (1 addition)
        assert_eq!(diff[0].additions, 2);
        assert_eq!(diff[0].deletions, 1);
    }
}
