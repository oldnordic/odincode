//! Unified database query interfaces for OdinCode.
//!
//! This module provides a structured and modularized approach to querying the
//! different databases within the OdinCode system. It abstracts the query logic
//! into several sub-modules for better organization and maintainability.

pub mod converters;
pub mod executors;
pub mod interface;
pub mod types;

#[cfg(test)]
mod tests;
