//! File tools: deterministic filesystem operations
//!
//! All operations use the REAL filesystem — no mocks, no in-memory FS.

mod file_create;
mod file_glob;
mod file_read;
mod file_search;
mod file_write;

// Re-export all file tools
pub use file_create::file_create;
pub use file_glob::file_glob;
pub use file_read::file_read;
pub use file_search::{file_search, SearchMatch};
pub use file_write::file_write;
