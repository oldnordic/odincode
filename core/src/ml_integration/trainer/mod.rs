// ! Model Training Methods
//!
//! This module contains training implementations for various ML algorithms,
//! organized by learning paradigm (supervised vs unsupervised).

pub mod supervised;
pub mod unsupervised;

// Re-export all training functions for convenience
pub use supervised::*;
pub use unsupervised::*;
