//! LTMC Memory Search Bridge
//!
//! This module provides the bridge between the LTMC system and the database stack
//! for memory search operations using SQLite + FAISS + Neo4j with atomic operations.

pub mod core;
pub mod search;
pub mod storage;

pub use core::MemorySearchBridge;
