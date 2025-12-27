//! OdinCode: Factual tool substrate for LLM-powered refactoring
//!
//! This library provides deterministic, boring tools that an LLM MUST use
//! instead of guessing about the filesystem.

pub mod cli;
pub mod evidence_queries;
pub mod execution_engine;
pub mod execution_tools;
pub mod file_edit_tools;
pub mod file_tools;
pub mod git_tools;
pub mod llm;
pub mod lsp_tools;
pub mod magellan_tools;
pub mod os_tools;
pub mod splice_tools;
pub mod stats_tools;
pub mod tools;
pub mod ui;

// Re-export file tools for convenience
pub use file_edit_tools::{file_edit, file_line_count, find_lines, FileEditArgs, FileEdit, FileEditResult, FileEditError};
pub use file_tools::{file_create, file_glob, file_read, file_search, file_write, SearchMatch};

// Re-export splice tools
pub use splice_tools::{splice_patch, splice_plan, PatchArgs, PlanArgs, SpliceResult};

// Re-export magellan tools
pub use magellan_tools::{MagellanDb, ReferenceRow, StatusCounts, SymbolRow};

// Re-export LSP tools
pub use lsp_tools::{lsp_check, Diagnostic};

// Re-export execution tools
pub use execution_tools::{Execution, ExecutionDb};

// Re-export evidence queries
pub use evidence_queries::EvidenceDb;

// Re-export progressive tool discovery (Phase 10)
pub use tools::{ToolCategory, ToolExample, ToolMetadata};
