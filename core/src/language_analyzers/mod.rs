// Core modules
pub mod core;

// Language-specific modules
pub mod compiled;
pub mod functional;
pub mod interpreted;
pub mod specialized;

// Re-export all analyzers for convenience
pub use compiled::*;
pub use core::*;
pub use functional::*;
pub use interpreted::*;
pub use specialized::*;
