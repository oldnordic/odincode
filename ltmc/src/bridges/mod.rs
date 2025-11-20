//! LTMC Bridges Module
//!
//! This module contains bridges between the LTMC system and external systems,
//! including database connections and other integrations.

pub mod memory_search;

pub use memory_search::MemorySearchBridge;
