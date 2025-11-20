//! Tools Executors Module
//!
//! This module contains the tool execution logic for the tools system.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::tool_models::ToolIntegration;
use odincode_core::CodeFile;
use odincode_ltmc::{LTMManager, LearningPattern, PatternType};

pub mod subprocess;
use subprocess::SubprocessExecutor;

#[cfg(test)]
mod tests;

/// Tool execution functions
pub struct ToolExecutors;

impl ToolExecutors {
    /// Execute a linter on a file
    pub async fn execute_linter(
        ltmc_manager: &LTMManager,
        tool: &ToolIntegration,
        file: &CodeFile,
    ) -> Result<bool> {
        debug!("Executing linter {} on file: {}", tool.name, file.path);

        // Get the linter command from tool config
        let command = tool.config.get("command").ok_or_else(|| {
            anyhow::anyhow!("Linter command not configured for tool: {}", tool.name)
        })?;

        // Get additional arguments from config
        let args: Vec<&str> = tool
            .config
            .get("args")
            .map(|args_str| args_str.split_whitespace().collect())
            .unwrap_or_default();

        // Get working directory (default to file's directory)
        let file_path = Path::new(&file.path);
        let working_dir = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Cannot determine working directory for file: {}", file.path)
        })?;

        info!("Running linter '{}' on file: {}", command, file.path);

        // Execute the linter
        let result =
            SubprocessExecutor::execute_linter(command, &args, file_path, Some(working_dir)).await;

        match result {
            Ok(execution_result) => {
                // Store the execution in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("success".to_string(), execution_result.success.to_string());
                context.insert(
                    "exit_code".to_string(),
                    execution_result
                        .exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                );
                context.insert(
                    "duration_ms".to_string(),
                    execution_result.duration_ms.to_string(),
                );

                if !execution_result.stdout.is_empty() {
                    context.insert("stdout".to_string(), execution_result.stdout);
                }

                if !execution_result.stderr.is_empty() {
                    context.insert("stderr".to_string(), execution_result.stderr);
                }

                let pattern = LearningPattern {
                    id: Uuid::new_v4(),
                    pattern_type: PatternType::CodePattern,
                    content: format!(
                        "Linter '{}' executed on file: {}, success: {}, duration: {}ms",
                        tool.name,
                        file.path,
                        execution_result.success,
                        execution_result.duration_ms
                    ),
                    context,
                    created: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    confidence: 0.8,
                };

                ltmc_manager.store_pattern(pattern).await?;

                if execution_result.success {
                    info!("Linter '{}' completed successfully", tool.name);
                } else {
                    warn!(
                        "Linter '{}' failed with exit code: {:?}",
                        tool.name, execution_result.exit_code
                    );
                }

                Ok(execution_result.success)
            }
            Err(e) => {
                error!("Failed to execute linter '{}': {}", tool.name, e);

                // Store the error in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("error".to_string(), e.to_string());

                let pattern = LearningPattern {
                    id: Uuid::new_v4(),
                    pattern_type: PatternType::CodePattern,
                    content: format!("Linter '{}' execution failed: {}", tool.name, e),
                    context,
                    created: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    confidence: 0.8,
                };

                ltmc_manager.store_pattern(pattern).await?;

                Ok(false)
            }
        }
    }

    /// Execute a formatter on a file
    pub async fn execute_formatter(
        ltmc_manager: &LTMManager,
        tool: &ToolIntegration,
        file: &CodeFile,
    ) -> Result<bool> {
        debug!("Executing formatter {} on file: {}", tool.name, file.path);

        // Get the formatter command from tool config
        let command = tool.config.get("command").ok_or_else(|| {
            anyhow::anyhow!("Formatter command not configured for tool: {}", tool.name)
        })?;

        // Get additional arguments from config
        let args: Vec<&str> = tool
            .config
            .get("args")
            .map(|args_str| args_str.split_whitespace().collect())
            .unwrap_or_default();

        // Get working directory (default to file's directory)
        let file_path = Path::new(&file.path);
        let working_dir = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Cannot determine working directory for file: {}", file.path)
        })?;

        info!("Running formatter '{}' on file: {}", command, file.path);

        // Execute the formatter
        let result =
            SubprocessExecutor::execute_formatter(command, &args, file_path, Some(working_dir))
                .await;

        match result {
            Ok(execution_result) => {
                // Store the execution in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("success".to_string(), execution_result.success.to_string());
                context.insert(
                    "exit_code".to_string(),
                    execution_result
                        .exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                );
                context.insert(
                    "duration_ms".to_string(),
                    execution_result.duration_ms.to_string(),
                );

                if !execution_result.stdout.is_empty() {
                    context.insert("stdout".to_string(), execution_result.stdout);
                }

                if !execution_result.stderr.is_empty() {
                    context.insert("stderr".to_string(), execution_result.stderr);
                }

                let pattern = LearningPattern {
                    id: Uuid::new_v4(),
                    pattern_type: PatternType::CodePattern,
                    content: format!(
                        "Formatter '{}' executed on file: {}, success: {}, duration: {}ms",
                        tool.name,
                        file.path,
                        execution_result.success,
                        execution_result.duration_ms
                    ),
                    context,
                    created: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    confidence: 0.8,
                };

                ltmc_manager.store_pattern(pattern).await?;

                if execution_result.success {
                    info!("Formatter '{}' completed successfully", tool.name);
                } else {
                    warn!(
                        "Formatter '{}' failed with exit code: {:?}",
                        tool.name, execution_result.exit_code
                    );
                }

                Ok(execution_result.success)
            }
            Err(e) => {
                error!("Failed to execute formatter '{}': {}", tool.name, e);

                // Store the error in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("error".to_string(), e.to_string());

                let pattern = LearningPattern {
                    id: Uuid::new_v4(),
                    pattern_type: PatternType::CodePattern,
                    content: format!("Formatter '{}' execution failed: {}", tool.name, e),
                    context,
                    created: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    confidence: 0.8,
                };

                ltmc_manager.store_pattern(pattern).await?;

                Ok(false)
            }
        }
    }

    /// Execute a test runner on a file
    pub async fn execute_test_runner(
        ltmc_manager: &LTMManager,
        tool: &ToolIntegration,
        file: &CodeFile,
    ) -> Result<bool> {
        debug!("Executing test runner {} on file: {}", tool.name, file.path);

        // Get the test runner command from tool config
        let command = tool.config.get("command").ok_or_else(|| {
            anyhow::anyhow!("Test runner command not configured for tool: {}", tool.name)
        })?;

        // Get additional arguments from config
        let mut args: Vec<&str> = tool
            .config
            .get("args")
            .map(|args_str| args_str.split_whitespace().collect())
            .unwrap_or_default();

        // Add the file path if it's a test file
        if file.path.contains("test") || file.path.contains("spec") {
            args.push(&file.path);
        }

        // Get working directory (default to file's directory)
        let file_path = Path::new(&file.path);
        let working_dir = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Cannot determine working directory for file: {}", file.path)
        })?;

        info!("Running test runner '{}' for file: {}", command, file.path);

        // Execute the test runner
        let result =
            SubprocessExecutor::execute_test_runner(command, &args, Some(working_dir)).await;

        match result {
            Ok(execution_result) => {
                // Store the execution in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("success".to_string(), execution_result.success.to_string());
                context.insert(
                    "exit_code".to_string(),
                    execution_result
                        .exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                );
                context.insert(
                    "duration_ms".to_string(),
                    execution_result.duration_ms.to_string(),
                );

                if !execution_result.stdout.is_empty() {
                    context.insert("stdout".to_string(), execution_result.stdout);
                }

                if !execution_result.stderr.is_empty() {
                    context.insert("stderr".to_string(), execution_result.stderr);
                }

                let pattern = LearningPattern {
                    id: Uuid::new_v4(),
                    pattern_type: PatternType::CodePattern,
                    content: format!(
                        "Test runner '{}' executed for file: {}, success: {}, duration: {}ms",
                        tool.name,
                        file.path,
                        execution_result.success,
                        execution_result.duration_ms
                    ),
                    context,
                    created: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    confidence: 0.8,
                };

                ltmc_manager.store_pattern(pattern).await?;

                if execution_result.success {
                    info!("Test runner '{}' completed successfully", tool.name);
                } else {
                    warn!(
                        "Test runner '{}' failed with exit code: {:?}",
                        tool.name, execution_result.exit_code
                    );
                }

                Ok(execution_result.success)
            }
            Err(e) => {
                error!("Failed to execute test runner '{}': {}", tool.name, e);

                // Store the error in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("error".to_string(), e.to_string());

                let pattern = LearningPattern {
                    id: Uuid::new_v4(),
                    pattern_type: PatternType::CodePattern,
                    content: format!("Test runner '{}' execution failed: {}", tool.name, e),
                    context,
                    created: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    confidence: 0.8,
                };

                ltmc_manager.store_pattern(pattern).await?;

                Ok(false)
            }
        }
    }

    /// Execute a build system on a file
    pub async fn execute_build_system(
        ltmc_manager: &LTMManager,
        tool: &ToolIntegration,
        file: &CodeFile,
    ) -> Result<bool> {
        debug!(
            "Executing build system {} on file: {}",
            tool.name, file.path
        );

        // Get the build command from tool config
        let command = tool.config.get("command").ok_or_else(|| {
            anyhow::anyhow!("Build command not configured for tool: {}", tool.name)
        })?;

        // Get additional arguments from config
        let mut args: Vec<&str> = tool
            .config
            .get("args")
            .map(|args_str| args_str.split_whitespace().collect())
            .unwrap_or_default();

        // Add the file path if it's a build target
        if file.path.ends_with(".rs") || file.path.ends_with(".js") || file.path.ends_with(".py") {
            args.push(&file.path);
        }

        // Get working directory (default to file's directory)
        let file_path = Path::new(&file.path);
        let working_dir = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Cannot determine working directory for file: {}", file.path)
        })?;

        info!("Running build system '{}' for file: {}", command, file.path);

        // Execute the build system
        let result =
            SubprocessExecutor::execute_build_system(command, &args, Some(working_dir)).await;

        match result {
            Ok(execution_result) => {
                // Store the execution in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("success".to_string(), execution_result.success.to_string());
                context.insert(
                    "exit_code".to_string(),
                    execution_result
                        .exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                );
                context.insert(
                    "duration_ms".to_string(),
                    execution_result.duration_ms.to_string(),
                );

                if !execution_result.stdout.is_empty() {
                    context.insert("stdout".to_string(), execution_result.stdout);
                }

                if !execution_result.stderr.is_empty() {
                    context.insert("stderr".to_string(), execution_result.stderr);
                }

                let pattern = LearningPattern {
                    id: Uuid::new_v4(),
                    pattern_type: PatternType::CodePattern,
                    content: format!(
                        "Build system '{}' executed for file: {}, success: {}, duration: {}ms",
                        tool.name,
                        file.path,
                        execution_result.success,
                        execution_result.duration_ms
                    ),
                    context,
                    created: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    confidence: 0.8,
                };

                ltmc_manager.store_pattern(pattern).await?;

                if execution_result.success {
                    info!("Build system '{}' completed successfully", tool.name);
                } else {
                    warn!(
                        "Build system '{}' failed with exit code: {:?}",
                        tool.name, execution_result.exit_code
                    );
                }

                Ok(execution_result.success)
            }
            Err(e) => {
                error!("Failed to execute build system '{}': {}", tool.name, e);

                // Store the error in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("error".to_string(), e.to_string());

                let pattern = LearningPattern {
                    id: Uuid::new_v4(),
                    pattern_type: PatternType::CodePattern,
                    content: format!("Build system '{}' execution failed: {}", tool.name, e),
                    context,
                    created: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    confidence: 0.8,
                };

                ltmc_manager.store_pattern(pattern).await?;

                Ok(false)
            }
        }
    }

    /// Execute version control operations on a file
    pub async fn execute_version_control(
        ltmc_manager: &LTMManager,
        tool: &ToolIntegration,
        file: &CodeFile,
    ) -> Result<bool> {
        debug!(
            "Executing version control {} on file: {}",
            tool.name, file.path
        );

        // Get the VCS command from tool config
        let command = tool
            .config
            .get("command")
            .ok_or_else(|| anyhow::anyhow!("VCS command not configured for tool: {}", tool.name))?;

        // Get additional arguments from config
        let mut args: Vec<&str> = tool
            .config
            .get("args")
            .map(|args_str| args_str.split_whitespace().collect())
            .unwrap_or_default();

        // Add the file path for file-specific operations
        args.push(&file.path);

        // Get working directory (default to file's directory)
        let file_path = Path::new(&file.path);
        let working_dir = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Cannot determine working directory for file: {}", file.path)
        })?;

        info!(
            "Running version control '{}' for file: {}",
            command, file.path
        );

        // Execute the version control command
        let result =
            SubprocessExecutor::execute_version_control(command, &args, Some(working_dir)).await;

        match result {
            Ok(execution_result) => {
                // Store the execution in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("success".to_string(), execution_result.success.to_string());
                context.insert(
                    "exit_code".to_string(),
                    execution_result
                        .exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                );
                context.insert(
                    "duration_ms".to_string(),
                    execution_result.duration_ms.to_string(),
                );

                if !execution_result.stdout.is_empty() {
                    context.insert("stdout".to_string(), execution_result.stdout);
                }

                if !execution_result.stderr.is_empty() {
                    context.insert("stderr".to_string(), execution_result.stderr);
                }

                let pattern =
                    LearningPattern {
                        id: Uuid::new_v4(),
                        pattern_type: PatternType::CodePattern,
                        content: format!(
                        "Version control '{}' executed for file: {}, success: {}, duration: {}ms",
                        tool.name, file.path, execution_result.success, execution_result.duration_ms
                    ),
                        context,
                        created: chrono::Utc::now(),
                        last_accessed: chrono::Utc::now(),
                        access_count: 0,
                        confidence: 0.8,
                    };

                ltmc_manager.store_pattern(pattern).await?;

                if execution_result.success {
                    info!("Version control '{}' completed successfully", tool.name);
                } else {
                    warn!(
                        "Version control '{}' failed with exit code: {:?}",
                        tool.name, execution_result.exit_code
                    );
                }

                Ok(execution_result.success)
            }
            Err(e) => {
                error!("Failed to execute version control '{}': {}", tool.name, e);

                // Store the error in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("error".to_string(), e.to_string());

                let pattern = LearningPattern {
                    id: Uuid::new_v4(),
                    pattern_type: PatternType::CodePattern,
                    content: format!("Version control '{}' execution failed: {}", tool.name, e),
                    context,
                    created: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    confidence: 0.8,
                };

                ltmc_manager.store_pattern(pattern).await?;

                Ok(false)
            }
        }
    }

    /// Execute debugger operations on a file
    pub async fn execute_debugger(
        ltmc_manager: &LTMManager,
        tool: &ToolIntegration,
        file: &CodeFile,
    ) -> Result<bool> {
        debug!("Executing debugger {} on file: {}", tool.name, file.path);

        // Get the debugger command from tool config
        let command = tool.config.get("command").ok_or_else(|| {
            anyhow::anyhow!("Debugger command not configured for tool: {}", tool.name)
        })?;

        // Get additional arguments from config
        let mut args: Vec<&str> = tool
            .config
            .get("args")
            .map(|args_str| args_str.split_whitespace().collect())
            .unwrap_or_default();

        // Add the file path for debugging
        args.push(&file.path);

        // Get working directory (default to file's directory)
        let file_path = Path::new(&file.path);
        let working_dir = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Cannot determine working directory for file: {}", file.path)
        })?;

        info!("Launching debugger '{}' for file: {}", command, file.path);

        // For debuggers, we'll use a shorter timeout since they're typically interactive
        let result = SubprocessExecutor::execute_command(
            command,
            &args,
            Some(working_dir),
            None,
            Some(10000),
        )
        .await;

        match result {
            Ok(execution_result) => {
                // Store the execution in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("success".to_string(), execution_result.success.to_string());
                context.insert(
                    "exit_code".to_string(),
                    execution_result
                        .exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                );
                context.insert(
                    "duration_ms".to_string(),
                    execution_result.duration_ms.to_string(),
                );

                if !execution_result.stdout.is_empty() {
                    context.insert("stdout".to_string(), execution_result.stdout);
                }

                if !execution_result.stderr.is_empty() {
                    context.insert("stderr".to_string(), execution_result.stderr);
                }

                let pattern = LearningPattern {
                    id: Uuid::new_v4(),
                    pattern_type: PatternType::CodePattern,
                    content: format!(
                        "Debugger '{}' launched for file: {}, success: {}, duration: {}ms",
                        tool.name,
                        file.path,
                        execution_result.success,
                        execution_result.duration_ms
                    ),
                    context,
                    created: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    confidence: 0.8,
                };

                ltmc_manager.store_pattern(pattern).await?;

                if execution_result.success {
                    info!("Debugger '{}' launched successfully", tool.name);
                } else {
                    warn!(
                        "Debugger '{}' failed to launch with exit code: {:?}",
                        tool.name, execution_result.exit_code
                    );
                }

                Ok(execution_result.success)
            }
            Err(e) => {
                error!("Failed to launch debugger '{}': {}", tool.name, e);

                // Store the error in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("error".to_string(), e.to_string());

                let pattern = LearningPattern {
                    id: Uuid::new_v4(),
                    pattern_type: PatternType::CodePattern,
                    content: format!("Debugger '{}' launch failed: {}", tool.name, e),
                    context,
                    created: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    confidence: 0.8,
                };

                ltmc_manager.store_pattern(pattern).await?;

                Ok(false)
            }
        }
    }

    /// Execute package manager operations on a file
    pub async fn execute_package_manager(
        ltmc_manager: &LTMManager,
        tool: &ToolIntegration,
        file: &CodeFile,
    ) -> Result<bool> {
        debug!(
            "Executing package manager {} on file: {}",
            tool.name, file.path
        );

        // Get the package manager command from tool config
        let command = tool.config.get("command").ok_or_else(|| {
            anyhow::anyhow!(
                "Package manager command not configured for tool: {}",
                tool.name
            )
        })?;

        // Get additional arguments from config
        let mut args: Vec<&str> = tool
            .config
            .get("args")
            .map(|args_str| args_str.split_whitespace().collect())
            .unwrap_or_default();

        // Add the file path if it's a package file (package.json, Cargo.toml, etc.)
        if file.path.ends_with("package.json")
            || file.path.ends_with("Cargo.toml")
            || file.path.ends_with("requirements.txt")
        {
            args.push(&file.path);
        }

        // Get working directory (default to file's directory)
        let file_path = Path::new(&file.path);
        let working_dir = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Cannot determine working directory for file: {}", file.path)
        })?;

        info!(
            "Running package manager '{}' for file: {}",
            command, file.path
        );

        // Execute the package manager
        let result =
            SubprocessExecutor::execute_package_manager(command, &args, Some(working_dir)).await;

        match result {
            Ok(execution_result) => {
                // Store the execution in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("success".to_string(), execution_result.success.to_string());
                context.insert(
                    "exit_code".to_string(),
                    execution_result
                        .exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                );
                context.insert(
                    "duration_ms".to_string(),
                    execution_result.duration_ms.to_string(),
                );

                if !execution_result.stdout.is_empty() {
                    context.insert("stdout".to_string(), execution_result.stdout);
                }

                if !execution_result.stderr.is_empty() {
                    context.insert("stderr".to_string(), execution_result.stderr);
                }

                let pattern =
                    LearningPattern {
                        id: Uuid::new_v4(),
                        pattern_type: PatternType::CodePattern,
                        content: format!(
                        "Package manager '{}' executed for file: {}, success: {}, duration: {}ms",
                        tool.name, file.path, execution_result.success, execution_result.duration_ms
                    ),
                        context,
                        created: chrono::Utc::now(),
                        last_accessed: chrono::Utc::now(),
                        access_count: 0,
                        confidence: 0.8,
                    };

                ltmc_manager.store_pattern(pattern).await?;

                if execution_result.success {
                    info!("Package manager '{}' completed successfully", tool.name);
                } else {
                    warn!(
                        "Package manager '{}' failed with exit code: {:?}",
                        tool.name, execution_result.exit_code
                    );
                }

                Ok(execution_result.success)
            }
            Err(e) => {
                error!("Failed to execute package manager '{}': {}", tool.name, e);

                // Store the error in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("error".to_string(), e.to_string());

                let pattern = LearningPattern {
                    id: Uuid::new_v4(),
                    pattern_type: PatternType::CodePattern,
                    content: format!("Package manager '{}' execution failed: {}", tool.name, e),
                    context,
                    created: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    confidence: 0.8,
                };

                ltmc_manager.store_pattern(pattern).await?;

                Ok(false)
            }
        }
    }

    /// Execute IDE integration operations on a file
    pub async fn execute_ide_integration(
        ltmc_manager: &LTMManager,
        tool: &ToolIntegration,
        file: &CodeFile,
    ) -> Result<bool> {
        debug!(
            "Executing IDE integration {} on file: {}",
            tool.name, file.path
        );

        // Get the IDE command from tool config
        let command = tool
            .config
            .get("command")
            .ok_or_else(|| anyhow::anyhow!("IDE command not configured for tool: {}", tool.name))?;

        // Get additional arguments from config
        let mut args: Vec<&str> = tool
            .config
            .get("args")
            .map(|args_str| args_str.split_whitespace().collect())
            .unwrap_or_default();

        // Add the file path for IDE operations
        args.push(&file.path);

        // Get working directory (default to file's directory)
        let file_path = Path::new(&file.path);
        let working_dir = file_path.parent().ok_or_else(|| {
            anyhow::anyhow!("Cannot determine working directory for file: {}", file.path)
        })?;

        info!(
            "Running IDE integration '{}' for file: {}",
            command, file.path
        );

        // Execute the IDE integration
        let result = SubprocessExecutor::execute_command(
            command,
            &args,
            Some(working_dir),
            None,
            Some(15000),
        )
        .await;

        match result {
            Ok(execution_result) => {
                // Store the execution in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("success".to_string(), execution_result.success.to_string());
                context.insert(
                    "exit_code".to_string(),
                    execution_result
                        .exit_code
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                );
                context.insert(
                    "duration_ms".to_string(),
                    execution_result.duration_ms.to_string(),
                );

                if !execution_result.stdout.is_empty() {
                    context.insert("stdout".to_string(), execution_result.stdout);
                }

                if !execution_result.stderr.is_empty() {
                    context.insert("stderr".to_string(), execution_result.stderr);
                }

                let pattern =
                    LearningPattern {
                        id: Uuid::new_v4(),
                        pattern_type: PatternType::CodePattern,
                        content: format!(
                        "IDE integration '{}' executed for file: {}, success: {}, duration: {}ms",
                        tool.name, file.path, execution_result.success, execution_result.duration_ms
                    ),
                        context,
                        created: chrono::Utc::now(),
                        last_accessed: chrono::Utc::now(),
                        access_count: 0,
                        confidence: 0.8,
                    };

                ltmc_manager.store_pattern(pattern).await?;

                if execution_result.success {
                    info!("IDE integration '{}' completed successfully", tool.name);
                } else {
                    warn!(
                        "IDE integration '{}' failed with exit code: {:?}",
                        tool.name, execution_result.exit_code
                    );
                }

                Ok(execution_result.success)
            }
            Err(e) => {
                error!("Failed to execute IDE integration '{}': {}", tool.name, e);

                // Store the error in LTMC
                let mut context = HashMap::new();
                context.insert("command".to_string(), command.clone());
                context.insert("file_path".to_string(), file.path.clone());
                context.insert("error".to_string(), e.to_string());

                let pattern = LearningPattern {
                    id: Uuid::new_v4(),
                    pattern_type: PatternType::CodePattern,
                    content: format!("IDE integration '{}' execution failed: {}", tool.name, e),
                    context,
                    created: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    confidence: 0.8,
                };

                ltmc_manager.store_pattern(pattern).await?;

                Ok(false)
            }
        }
    }
}
