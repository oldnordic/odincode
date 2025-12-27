//! OS tools — System-level operations
//!
//! ## Architecture
//!
//! - `wc` — Word count for files (line, word, character, byte counts)
//! - `bash_exec` — Safe bash command execution with timeouts

pub mod wc;
pub mod bash_exec;

pub use wc::{wc, WcArgs, WcResult};
pub use bash_exec::{bash_exec, BashExecArgs, BashExecResult};
