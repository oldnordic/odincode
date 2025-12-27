//! Stats tools (Task B) — OS-style primitive tools
//!
//! Pure, deterministic tools for file/directory statistics.
//! No shell invocation, no OS commands, Rust stdlib only.

mod count_files;
mod count_lines;
mod fs_stats;

// Re-export tool functions
pub use count_files::{count_files, CountFilesArgs, CountFilesResult};
pub use count_lines::{count_lines, CountLinesArgs, CountLinesResult, FileLineCount};
pub use fs_stats::{fs_stats, FsStatsArgs, FsStatsResult};
