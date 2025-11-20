//! Multi-Edit Tools Module
//!
//! This module provides advanced multi-edit capabilities for the OdinCode system,
//! allowing for complex refactoring and code transformations across multiple files.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use odincode_core::{CodeEngine, CodeFile};

/// Represents a multi-file edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiEditOperation {
    /// Unique identifier for the operation
    pub id: Uuid,
    /// Name of the operation
    pub name: String,
    /// Description of the operation
    pub description: String,
    /// List of edit tasks to perform
    pub tasks: Vec<EditTask>,
    /// Creation timestamp
    pub created: chrono::DateTime<chrono::Utc>,
    /// Status of the operation
    pub status: MultiEditStatus,
}

/// Represents a single edit task within a multi-edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditTask {
    /// Unique identifier for the task
    pub id: Uuid,
    /// File ID to edit
    pub file_id: Uuid,
    /// Type of edit operation
    pub operation_type: EditOperationType,
    /// Start position for the edit (line, column)
    pub start_pos: (usize, usize),
    /// End position for the edit (line, column)
    pub end_pos: (usize, usize),
    /// Content to insert or replace
    pub content: String,
    /// Description of the edit
    pub description: String,
}

/// Type of edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditOperationType {
    /// Insert content at position
    Insert,
    /// Replace content between positions
    Replace,
    /// Delete content between positions
    Delete,
    /// Update content with pattern matching
    PatternReplace,
}

/// Status of a multi-edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultiEditStatus {
    /// Operation is pending
    Pending,
    /// Operation is in progress
    InProgress,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed,
}

/// Main multi-edit manager that handles complex refactoring operations
pub struct MultiEditManager {
    /// Map of all multi-edit operations
    pub operations: RwLock<HashMap<Uuid, MultiEditOperation>>,
    /// Reference to the core code engine
    pub core_engine: std::sync::Arc<CodeEngine>,
}

impl MultiEditManager {
    /// Create a new multi-edit manager
    pub fn new(core_engine: std::sync::Arc<CodeEngine>) -> Self {
        Self {
            operations: RwLock::new(HashMap::new()),
            core_engine,
        }
    }

    /// Create a new multi-edit operation
    pub async fn create_operation(
        &self,
        name: String,
        description: String,
        tasks: Vec<EditTask>,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let operation = MultiEditOperation {
            id,
            name: name.clone(),
            description,
            tasks,
            created: chrono::Utc::now(),
            status: MultiEditStatus::Pending,
        };

        let mut operations = self.operations.write().await;
        operations.insert(id, operation);
        drop(operations);

        info!("Created multi-edit operation: {} ({})", name, id);
        Ok(id)
    }

    /// Execute a multi-edit operation
    pub async fn execute_operation(&self, operation_id: Uuid) -> Result<bool> {
        let operation = {
            let operations = self.operations.read().await;
            match operations.get(&operation_id) {
                Some(op) => op.clone(),
                None => return Err(anyhow::anyhow!("Operation not found: {}", operation_id)),
            }
        };

        // Update operation status to in progress
        {
            let mut operations = self.operations.write().await;
            if let Some(op) = operations.get_mut(&operation_id) {
                op.status = MultiEditStatus::InProgress;
            }
        }

        debug!(
            "Executing multi-edit operation: {} ({})",
            operation.name, operation_id
        );

        // Execute each task in the operation
        let mut all_success = true;
        for task in &operation.tasks {
            match self.execute_edit_task(task).await {
                Ok(success) => {
                    if !success {
                        warn!("Edit task failed: {}", task.id);
                        all_success = false;
                    }
                }
                Err(e) => {
                    warn!("Error executing edit task {}: {}", task.id, e);
                    all_success = false;
                }
            }
        }

        // Update operation status based on result
        {
            let mut operations = self.operations.write().await;
            if let Some(op) = operations.get_mut(&operation_id) {
                op.status = if all_success {
                    MultiEditStatus::Completed
                } else {
                    MultiEditStatus::Failed
                };
            }
        }

        Ok(all_success)
    }

    /// Execute a single edit task
    async fn execute_edit_task(&self, task: &EditTask) -> Result<bool> {
        debug!("Executing edit task: {} on file {}", task.id, task.file_id);

        // Get the file
        let file = self.core_engine.get_file(task.file_id).await?;
        if file.is_none() {
            return Err(anyhow::anyhow!("File not found: {}", task.file_id));
        }
        let file = file.unwrap();

        // Perform the edit based on operation type
        let new_content = match task.operation_type {
            EditOperationType::Insert => {
                self.insert_content(&file, task.start_pos, &task.content)?
            }
            EditOperationType::Replace => {
                self.replace_content(&file, task.start_pos, task.end_pos, &task.content)?
            }
            EditOperationType::Delete => {
                self.delete_content(&file, task.start_pos, task.end_pos)?
            }
            EditOperationType::PatternReplace => {
                self.pattern_replace_content(&file, &task.content)?
            }
        };

        // Update the file in the core engine
        self.core_engine.update_file(file.id, new_content).await?;

        Ok(true)
    }

    /// Insert content at a specific position
    fn insert_content(
        &self,
        file: &CodeFile,
        pos: (usize, usize),
        content: &str,
    ) -> Result<String> {
        let lines: Vec<&str> = file.content.lines().collect();
        let (line_idx, col_idx) = pos;

        if line_idx >= lines.len() {
            return Err(anyhow::anyhow!("Line index out of bounds"));
        }

        let mut result = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i == line_idx {
                // Insert at the specified column
                if col_idx > line.len() {
                    return Err(anyhow::anyhow!("Column index out of bounds"));
                }
                let (before, after) = line.split_at(col_idx);
                result.push_str(before);
                result.push_str(content);
                result.push_str(after);
            } else {
                result.push_str(line);
            }

            // Add newline if not the last line
            if i < lines.len() - 1 {
                result.push('\n');
            }
        }

        Ok(result)
    }

    /// Replace content between two positions
    fn replace_content(
        &self,
        file: &CodeFile,
        start_pos: (usize, usize),
        end_pos: (usize, usize),
        replacement: &str,
    ) -> Result<String> {
        let lines: Vec<&str> = file.content.lines().collect();
        let (start_line, start_col) = start_pos;
        let (end_line, end_col) = end_pos;

        if start_line >= lines.len() || end_line >= lines.len() {
            return Err(anyhow::anyhow!("Line index out of bounds"));
        }

        let mut result = String::new();

        for (i, line) in lines.iter().enumerate() {
            if i < start_line || i > end_line {
                // Outside the replacement range
                result.push_str(line);
            } else if i == start_line && i == end_line {
                // Same line, replace between columns
                if start_col > line.len() || end_col > line.len() || start_col > end_col {
                    return Err(anyhow::anyhow!("Column index out of bounds"));
                }
                let (before, rest) = line.split_at(start_col);
                let (_, after) = rest.split_at(end_col - start_col);
                result.push_str(before);
                result.push_str(replacement);
                result.push_str(after);
            } else if i == start_line {
                // Start line, replace from start column to end of line
                if start_col > line.len() {
                    return Err(anyhow::anyhow!("Column index out of bounds"));
                }
                let (before, _) = line.split_at(start_col);
                result.push_str(before);
                result.push_str(replacement);
            } else if i == end_line {
                // End line, replace from beginning to end column
                if end_col > line.len() {
                    return Err(anyhow::anyhow!("Column index out of bounds"));
                }
                let (_, after) = line.split_at(end_col);
                result.push_str(after);
            }
            // Lines between start and end are skipped (effectively deleted)

            // Add newline if not the last line
            if i < lines.len() - 1 {
                result.push('\n');
            }
        }

        Ok(result)
    }

    /// Delete content between two positions
    fn delete_content(
        &self,
        file: &CodeFile,
        start_pos: (usize, usize),
        end_pos: (usize, usize),
    ) -> Result<String> {
        self.replace_content(file, start_pos, end_pos, "")
    }

    /// Perform pattern-based replacement
    fn pattern_replace_content(&self, file: &CodeFile, pattern: &str) -> Result<String> {
        // For now, this is a simple implementation
        // In a real implementation, this would use regex or more sophisticated pattern matching
        Ok(file.content.replace(pattern, ""))
    }

    /// Get a multi-edit operation by its ID
    pub async fn get_operation(&self, id: Uuid) -> Result<Option<MultiEditOperation>> {
        let operations = self.operations.read().await;
        Ok(operations.get(&id).cloned())
    }

    /// List all operations
    pub async fn get_all_operations(&self) -> Result<Vec<MultiEditOperation>> {
        let operations = self.operations.read().await;
        let result: Vec<MultiEditOperation> = operations.values().cloned().collect();
        Ok(result)
    }
}
