//! wc — Word count tool
//!
//! Executes `wc` command to get line, word, character, and byte counts.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Arguments for wc command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WcArgs {
    /// Path to file(s) to count
    pub paths: Vec<String>,
    /// Count lines (default: true)
    #[serde(default = "default_true")]
    pub lines: bool,
    /// Count words (default: true)
    #[serde(default = "default_true")]
    pub words: bool,
    /// Count characters (default: true)
    #[serde(default = "default_true")]
    pub chars: bool,
    /// Count bytes (default: false)
    #[serde(default)]
    pub bytes: bool,
}

fn default_true() -> bool {
    true
}

/// Word count result for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WcEntry {
    /// File path
    pub path: String,
    /// Line count (if requested)
    pub lines: Option<usize>,
    /// Word count (if requested)
    pub words: Option<usize>,
    /// Character count (if requested)
    pub chars: Option<usize>,
    /// Byte count (if requested)
    pub bytes: Option<usize>,
}

/// Aggregate word count result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WcResult {
    /// Individual file results
    pub entries: Vec<WcEntry>,
    /// Total counts (if multiple files)
    pub total: Option<WcEntry>,
}

/// Execute wc command on given files
///
/// Returns line, word, character, and byte counts for each file.
pub fn wc(args: WcArgs) -> Result<WcResult> {
    if args.paths.is_empty() {
        return Ok(WcResult {
            entries: vec![],
            total: None,
        });
    }

    let mut cmd_args = Vec::new();

    if args.lines {
        cmd_args.push("-l");
    }
    if args.words {
        cmd_args.push("-w");
    }
    if args.chars {
        cmd_args.push("-m");
    }
    if args.bytes {
        cmd_args.push("-c");
    }

    // Convert String paths to &str for Command
    for path in &args.paths {
        cmd_args.push(path.as_str());
    }

    let output = Command::new("wc")
        .args(&cmd_args)
        .output()
        .context("Failed to execute wc command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("wc command failed: {}", stderr);
    }

    parse_wc_output(&String::from_utf8_lossy(&output.stdout), &args.paths)
}

/// Parse wc output into structured result
fn parse_wc_output(stdout: &str, _paths: &[String]) -> Result<WcResult> {
    let mut entries = Vec::new();
    let mut total: Option<WcEntry> = None;

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        // Check if this is a "total" line
        let is_total = parts.last().map_or(false, |p| *p == "total");

        if is_total {
            total = Some(parse_wc_line(line)?);
            continue;
        }

        // Regular file entry
        let entry = parse_wc_line(line)?;
        entries.push(entry);
    }

    Ok(WcResult { entries, total })
}

/// Parse a wc output line
fn parse_wc_line(line: &str) -> Result<WcEntry> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.is_empty() {
        anyhow::bail!("Empty wc line");
    }

    // wc output: lines words chars bytes filename
    // Not all fields present - depends on flags
    // Last field is filename (or "total")

    let mut lines = None;
    let mut words = None;
    let mut chars = None;
    let mut bytes = None;

    // Count numeric fields from start
    let mut numeric_end = 0;
    for part in &parts {
        if part.parse::<usize>().is_ok() || *part == "0" {
            numeric_end += 1;
        } else {
            break;
        }
    }

    // Assign numeric values based on count
    // Standard wc order: lines, words, chars, bytes
    if numeric_end >= 1 {
        lines = parts[0].parse().ok();
    }
    if numeric_end >= 2 {
        words = parts[1].parse().ok();
    }
    if numeric_end >= 3 {
        chars = parts[2].parse().ok();
    }
    if numeric_end >= 4 {
        bytes = parts[3].parse().ok();
    }

    // Filename is after numeric fields
    let path = if numeric_end < parts.len() {
        parts[numeric_end].to_string()
    } else {
        "<unknown>".to_string()
    };

    Ok(WcEntry {
        path,
        lines,
        words,
        chars,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wc_single_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "line1\nline2\nline3\n").unwrap();

        let args = WcArgs {
            paths: vec![test_file.to_str().unwrap().to_string()],
            lines: true,
            words: true,
            chars: false,
            bytes: false,
        };

        let result = wc(args).unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].lines, Some(3));
        assert_eq!(result.entries[0].words, Some(3));
    }

    #[test]
    fn test_wc_empty_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_file = temp_dir.path().join("empty.txt");
        std::fs::write(&test_file, "").unwrap();

        let args = WcArgs {
            paths: vec![test_file.to_str().unwrap().to_string()],
            lines: true,
            words: true,
            chars: true,
            bytes: true,
        };

        let result = wc(args).unwrap();
        assert_eq!(result.entries[0].lines, Some(0));
        assert_eq!(result.entries[0].words, Some(0));
        assert_eq!(result.entries[0].chars, Some(0));
    }

    #[test]
    fn test_wc_parse_output() {
        let output = "  3   6  24 test.txt";
        let result = parse_wc_output(output, &["test.txt".to_string()]).unwrap();

        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].path, "test.txt");
        assert_eq!(result.entries[0].lines, Some(3));
        assert_eq!(result.entries[0].words, Some(6));
        assert_eq!(result.entries[0].chars, Some(24));
    }
}
