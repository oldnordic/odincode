//! Tool mapping from step.tool to Phase 0 function calls

use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};

use crate::execution_engine::errors::ExecutionError;
use crate::execution_engine::output_kind::ToolOutputKind;
use crate::execution_engine::result::DiagnosticArtifact;
use crate::execution_engine::structural_summary;
use crate::execution_tools::{ExecutionDb, ExecutionSummaryArgs, MemoryQueryArgs, execution_summary, memory_query};
use crate::file_edit_tools::{self, FileEdit, FileEditArgs};
use crate::git_tools;
use crate::llm::router::tool_is_allowed;
use crate::llm::types::Step;
use crate::magellan_tools::MagellanDb;
use crate::os_tools::{self, BashExecArgs, WcArgs};

/// Tool invocation result
pub struct ToolInvocation {
    pub success: bool,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub error_message: Option<String>,
    pub diagnostics: Vec<DiagnosticArtifact>,
    #[allow(dead_code)]
    pub duration_ms: i64, // Reserved for future execution timing
    /// Semantic kind of output (Task A: for routing)
    pub kind: ToolOutputKind,
    /// Structured data payload (for UI, not injected into chat)
    pub structured_data: Option<JsonValue>,
}

/// Check if a tool is a mutation tool (requires grounding)
fn is_mutation_tool(tool: &str) -> bool {
    matches!(
        tool,
        "file_edit" | "file_write" | "file_create" | "splice_patch"
    )
}

/// Invoke a tool based on step specification
///
/// Maps step.tool to the corresponding Phase 0 function and executes it.
/// Mutation tools require recent memory_query (within 10 seconds) for grounding.
pub fn invoke_tool(
    step: &Step,
    exec_db: &ExecutionDb,
    magellan_db: &Option<MagellanDb>,
    last_query_time_ms: Option<i64>,
) -> Result<ToolInvocation, ExecutionError> {
    // Verify tool is in whitelist
    if !tool_is_allowed(&step.tool) {
        return Err(ExecutionError::ToolNotFound(step.tool.clone()));
    }

    // Pre-flight grounding check for mutation tools
    if is_mutation_tool(&step.tool) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let time_since_query = if let Some(t) = last_query_time_ms {
            now - t
        } else {
            now // Never queried
        };

        // Require memory query within last 10 seconds
        if time_since_query > 10_000 {
            return Err(ExecutionError::GroundingRequired {
                tool: step.tool.clone(),
                reason: format!(
                    "Last memory_query was {}ms ago (max allowed: 10000ms). \
                    Must call memory_query(timeline_summary) before mutation.",
                    time_since_query
                ),
                required_query: "memory_query(timeline_summary)".to_string(),
            });
        }
    }

    match step.tool.as_str() {
        "file_read" => invoke_file_read(step),
        "file_write" => invoke_file_write(step),
        "file_create" => invoke_file_create(step),
        "file_search" => invoke_file_search(step),
        "file_glob" => invoke_file_glob(step),
        "file_edit" => invoke_file_edit(step),
        "splice_patch" => invoke_splice_patch(step),
        "splice_plan" => invoke_splice_plan(step),
        "symbols_in_file" => invoke_symbols_in_file(step, magellan_db),
        "references_to_symbol_name" => invoke_references_to_symbol_name(step, magellan_db),
        "references_from_file_to_symbol_name" => invoke_references_from_file(step, magellan_db),
        "lsp_check" => invoke_lsp_check(step),
        "memory_query" => invoke_memory_query(step, exec_db),
        "execution_summary" => invoke_execution_summary(step, exec_db),
        "git_status" => invoke_git_status(step),
        "git_diff" => invoke_git_diff(step),
        "git_log" => invoke_git_log(step),
        "wc" => invoke_wc(step),
        "bash_exec" => invoke_bash_exec(step),
        "count_files" => invoke_count_files(step),
        "count_lines" => invoke_count_lines(step),
        "fs_stats" => invoke_fs_stats(step),
        _ => Err(ExecutionError::ToolNotFound(step.tool.clone())),
    }
}

fn invoke_file_read(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    let path = step
        .arguments
        .get("path")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "path".to_string(),
        })?;

    let result = crate::file_read(Path::new(path));

    match result {
        Ok(content) => Ok(ToolInvocation {
            success: true,
            stdout: Some(content.clone()),
            stderr: None,
            error_message: None,
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::FileContent,
            structured_data: None, // File content stays as text
        }),
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_file_write(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    let path = step
        .arguments
        .get("path")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "path".to_string(),
        })?;

    let contents = step.arguments.get("contents").cloned().unwrap_or_default();

    let result = crate::file_write(Path::new(path), &contents);

    match result {
        Ok(_) => Ok(ToolInvocation {
            success: true,
            stdout: Some(format!("File written: {}", path)),
            stderr: None,
            error_message: None,
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Void,
            structured_data: None,
        }),
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_file_create(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    let path = step
        .arguments
        .get("path")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "path".to_string(),
        })?;

    let contents = step.arguments.get("contents").cloned().unwrap_or_default();

    let result = crate::file_create(Path::new(path), &contents);

    match result {
        Ok(_) => Ok(ToolInvocation {
            success: true,
            stdout: Some(format!("File created: {}", path)),
            stderr: None,
            error_message: None,
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Void,
            structured_data: None,
        }),
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_file_edit(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    let path = step
        .arguments
        .get("path")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "path".to_string(),
        })?;

    // Parse the edit type
    let edit = if let Some(line_number) = step.arguments.get("line_number") {
        // Replace line at specific line number
        let line_number = line_number.parse::<usize>().map_err(|_| {
            ExecutionError::ToolExecutionFailed {
                tool: step.tool.clone(),
                error: "Invalid line_number".to_string(),
            }
        })?;

        let new_content = step
            .arguments
            .get("new_content")
            .cloned()
            .unwrap_or_default();

        FileEdit::ReplaceLine {
            line_number,
            new_content,
        }
    } else if let Some(pattern) = step.arguments.get("pattern") {
        // Replace or delete lines matching pattern
        let replace_all = step
            .arguments
            .get("replace_all")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        if let Some(new_content) = step.arguments.get("new_content") {
            FileEdit::ReplacePattern {
                pattern: pattern.clone(),
                new_content: new_content.clone(),
                replace_all,
            }
        } else {
            FileEdit::DeletePattern {
                pattern: pattern.clone(),
            }
        }
    } else if let Some(after_line) = step.arguments.get("insert_after") {
        // Insert line after specific line number
        let after_line = after_line.parse::<usize>().map_err(|_| {
            ExecutionError::ToolExecutionFailed {
                tool: step.tool.clone(),
                error: "Invalid insert_after value".to_string(),
            }
        })?;

        let content = step
            .arguments
            .get("content")
            .cloned()
            .unwrap_or_default();

        FileEdit::InsertLine {
            after_line,
            content,
        }
    } else if let Some(delete_line) = step.arguments.get("delete_line") {
        // Delete line at specific line number
        let line_number = delete_line.parse::<usize>().map_err(|_| {
            ExecutionError::ToolExecutionFailed {
                tool: step.tool.clone(),
                error: "Invalid delete_line value".to_string(),
            }
        })?;

        FileEdit::DeleteLine { line_number }
    } else {
        return Err(ExecutionError::ToolExecutionFailed {
            tool: step.tool.clone(),
            error: "Must specify one of: line_number, pattern, insert_after, delete_line".to_string(),
        });
    };

    let args = FileEditArgs {
        file: PathBuf::from(path),
        edit,
    };

    match file_edit_tools::file_edit(args) {
        Ok(result) => {
            let summary = format!(
                "file_edit: {} modified ({} lines changed, {} inserted, {} deleted)",
                result.path.display(),
                result.lines_modified,
                result.lines_inserted,
                result.lines_deleted
            );
            Ok(ToolInvocation {
                success: true,
                stdout: Some(summary),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::Textual,
                structured_data: None,
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_file_search(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    let pattern = step
        .arguments
        .get("pattern")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "pattern".to_string(),
        })?;

    let root = step
        .arguments
        .get("root")
        .cloned()
        .unwrap_or_else(|| String::from("."));

    let result = crate::file_search(pattern, Path::new(&root));

    match result {
        Ok(matches) => {
            let json_value = serde_json::to_value(&matches).unwrap_or_default();
            let stdout = structural_summary::build_structural_summary("file_search", &json_value);
            Ok(ToolInvocation {
                success: true,
                stdout: Some(stdout),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::Structural,
                structured_data: Some(json_value),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_file_glob(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    let pattern = step
        .arguments
        .get("pattern")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "pattern".to_string(),
        })?;

    let root = step
        .arguments
        .get("root")
        .or_else(|| step.arguments.get("path"))
        .cloned()
        .unwrap_or_else(|| String::from("."));

    let result = crate::file_glob(pattern, Path::new(&root));

    match result {
        Ok(paths) => {
            let path_strs: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
            let json_value = serde_json::to_value(&path_strs).unwrap_or_default();
            let stdout = structural_summary::build_structural_summary("file_glob", &json_value);
            Ok(ToolInvocation {
                success: true,
                stdout: Some(stdout),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::Structural,
                structured_data: Some(json_value),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_splice_patch(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    use crate::splice_tools::{splice_patch, PatchArgs};
    use std::path::PathBuf;

    let file = step
        .arguments
        .get("file")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "file".to_string(),
        })?;

    let symbol = step
        .arguments
        .get("symbol")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "symbol".to_string(),
        })?;

    let with = step
        .arguments
        .get("with")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "with".to_string(),
        })?;

    let kind = step.arguments.get("kind").cloned();

    let args = PatchArgs {
        file: PathBuf::from(file),
        symbol: symbol.clone(),
        kind,
        with: PathBuf::from(with),
        analyzer: None,
    };

    let result = splice_patch(&args);

    match result {
        Ok(splice_result) => Ok(ToolInvocation {
            success: splice_result.success,
            stdout: Some(splice_result.stdout.clone()),
            stderr: Some(splice_result.stderr.clone()),
            error_message: if splice_result.success {
                None
            } else {
                Some("splice patch failed".to_string())
            },
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Textual,
            structured_data: None,
        }),
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_splice_plan(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    use crate::splice_tools::{splice_plan, PlanArgs};
    use std::path::PathBuf;

    let plan_file =
        step.arguments
            .get("plan_file")
            .ok_or_else(|| ExecutionError::MissingArgument {
                tool: step.tool.clone(),
                argument: "plan_file".to_string(),
            })?;

    let args = PlanArgs {
        file: PathBuf::from(plan_file),
    };

    let result = splice_plan(&args);

    match result {
        Ok(splice_result) => Ok(ToolInvocation {
            success: splice_result.success,
            stdout: Some(splice_result.stdout.clone()),
            stderr: Some(splice_result.stderr.clone()),
            error_message: if splice_result.success {
                None
            } else {
                Some("splice plan failed".to_string())
            },
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Textual,
            structured_data: None,
        }),
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_symbols_in_file(
    step: &Step,
    magellan_db: &Option<MagellanDb>,
) -> Result<ToolInvocation, ExecutionError> {
    let db = magellan_db
        .as_ref()
        .ok_or_else(|| ExecutionError::ToolExecutionFailed {
            tool: step.tool.clone(),
            error: "MagellanDb not available".to_string(),
        })?;

    let file_path =
        step.arguments
            .get("file_path")
            .ok_or_else(|| ExecutionError::MissingArgument {
                tool: step.tool.clone(),
                argument: "file_path".to_string(),
            })?;

    let result = db.symbols_in_file(file_path);

    match result {
        Ok(symbols) => {
            let json_value = serde_json::to_value(&symbols).unwrap_or_default();
            let stdout = structural_summary::build_structural_summary("symbols_in_file", &json_value);
            Ok(ToolInvocation {
                success: true,
                stdout: Some(stdout),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::Structural,
                structured_data: Some(json_value),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_references_to_symbol_name(
    step: &Step,
    magellan_db: &Option<MagellanDb>,
) -> Result<ToolInvocation, ExecutionError> {
    let db = magellan_db
        .as_ref()
        .ok_or_else(|| ExecutionError::ToolExecutionFailed {
            tool: step.tool.clone(),
            error: "MagellanDb not available".to_string(),
        })?;

    let symbol = step
        .arguments
        .get("symbol")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "symbol".to_string(),
        })?;

    let result = db.references_to_symbol_name(symbol);

    match result {
        Ok(refs) => {
            let json_value = serde_json::to_value(&refs).unwrap_or_default();
            let stdout = structural_summary::build_structural_summary("references_to_symbol_name", &json_value);
            Ok(ToolInvocation {
                success: true,
                stdout: Some(stdout),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::Structural,
                structured_data: Some(json_value),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_references_from_file(
    step: &Step,
    magellan_db: &Option<MagellanDb>,
) -> Result<ToolInvocation, ExecutionError> {
    let db = magellan_db
        .as_ref()
        .ok_or_else(|| ExecutionError::ToolExecutionFailed {
            tool: step.tool.clone(),
            error: "MagellanDb not available".to_string(),
        })?;

    let file_path =
        step.arguments
            .get("file_path")
            .ok_or_else(|| ExecutionError::MissingArgument {
                tool: step.tool.clone(),
                argument: "file_path".to_string(),
            })?;

    let symbol = step
        .arguments
        .get("symbol")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "symbol".to_string(),
        })?;

    let result = db.references_from_file_to_symbol_name(file_path, symbol);

    match result {
        Ok(refs) => {
            let json_value = serde_json::to_value(&refs).unwrap_or_default();
            let stdout = structural_summary::build_structural_summary("references_from_file_to_symbol_name", &json_value);
            Ok(ToolInvocation {
                success: true,
                stdout: Some(stdout),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::Structural,
                structured_data: Some(json_value),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_lsp_check(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    let path = step
        .arguments
        .get("path")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "path".to_string(),
        })?;

    let result = crate::lsp_check(Path::new(path));

    match result {
        Ok(diagnostics) => {
            let diag_artifacts: Vec<DiagnosticArtifact> =
                diagnostics.into_iter().map(|d| d.into()).collect();

            let json_value = serde_json::to_value(&diag_artifacts).unwrap_or_default();
            let stdout = structural_summary::build_structural_summary("lsp_check", &json_value);
            Ok(ToolInvocation {
                success: true,
                stdout: Some(stdout),
                stderr: None,
                error_message: None,
                diagnostics: diag_artifacts,
                duration_ms: 0,
                kind: ToolOutputKind::Structural,
                structured_data: Some(json_value),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_memory_query(
    step: &Step,
    _exec_db: &ExecutionDb,
) -> Result<ToolInvocation, ExecutionError> {
    // Build MemoryQueryArgs from step arguments
    // Step.arguments is HashMap<String, String>, so we parse accordingly
    let args = MemoryQueryArgs {
        tool: step.arguments.get("tool").cloned(),
        session_id: step.arguments.get("session_id").cloned(),
        success_only: step
            .arguments
            .get("success_only")
            .and_then(|s| s.parse::<bool>().ok()),
        limit: step
            .arguments
            .get("limit")
            .and_then(|s| s.parse::<usize>().ok()),
        include_output: step
            .arguments
            .get("include_output")
            .and_then(|s| s.parse::<bool>().ok()),
        since: step
            .arguments
            .get("since")
            .and_then(|s| s.parse::<i64>().ok()),
        until: step
            .arguments
            .get("until")
            .and_then(|s| s.parse::<i64>().ok()),
    };

    // Get db_root from ExecutionDb
    // For now, we use a default path. In production, this should come from config.
    let db_root = std::path::Path::new("."); // TODO: Use actual db_root from config

    match memory_query(db_root, args) {
        Ok(result) => {
            let json = serde_json::to_string(&result).unwrap_or_default();
            let summary = format!(
                "memory_query: {} executions found (showing {})",
                result.total_count,
                result.executions.len()
            );
            Ok(ToolInvocation {
                success: true,
                stdout: Some(summary),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::Structural,
                structured_data: serde_json::from_str(&json).ok(),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_execution_summary(
    step: &Step,
    _exec_db: &ExecutionDb,
) -> Result<ToolInvocation, ExecutionError> {
    // Build ExecutionSummaryArgs from step arguments
    let args = ExecutionSummaryArgs {
        tool: step.arguments.get("tool").cloned(),
        session_id: step.arguments.get("session_id").cloned(),
        since: step
            .arguments
            .get("since")
            .and_then(|s| s.parse::<i64>().ok()),
        until: step
            .arguments
            .get("until")
            .and_then(|s| s.parse::<i64>().ok()),
    };

    // Get db_root from ExecutionDb
    // For now, we use a default path. In production, this should come from config.
    let db_root = std::path::Path::new("."); // TODO: Use actual db_root from config

    match execution_summary(db_root, args) {
        Ok(result) => {
            let json = serde_json::to_string(&result).unwrap_or_default();
            let summary = format!(
                "execution_summary: {} total executions ({} success, {} failed, {:.0}% rate)",
                result.summary.total_executions,
                result.summary.total_success,
                result.summary.total_failed,
                result.summary.success_rate * 100.0
            );
            Ok(ToolInvocation {
                success: true,
                stdout: Some(summary),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::Structural,
                structured_data: serde_json::from_str(&json).ok(),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_git_status(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    let repo_root = step
        .arguments
        .get("repo_root")
        .cloned()
        .unwrap_or_else(|| ".".to_string());

    match git_tools::git_status(Path::new(&repo_root)) {
        Ok(entries) => {
            let json = serde_json::to_string(&entries).unwrap_or_default();
            let summary = format!(
                "git_status: {} files changed",
                entries.len()
            );
            Ok(ToolInvocation {
                success: true,
                stdout: Some(summary),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::Structural,
                structured_data: serde_json::from_str(&json).ok(),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_git_diff(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    let repo_root = step
        .arguments
        .get("repo_root")
        .cloned()
        .unwrap_or_else(|| ".".to_string());

    // Check if a specific file is requested
    if let Some(path) = step.arguments.get("path") {
        match git_tools::git_diff_file(Path::new(&repo_root), path) {
            Ok(diff) => {
                Ok(ToolInvocation {
                    success: true,
                    stdout: Some(diff),
                    stderr: None,
                    error_message: None,
                    diagnostics: vec![],
                    duration_ms: 0,
                    kind: ToolOutputKind::Textual,
                    structured_data: None,
                })
            }
            Err(e) => Ok(ToolInvocation {
                success: false,
                stdout: None,
                stderr: None,
                error_message: Some(e.to_string()),
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::Error,
                structured_data: None,
            }),
        }
    } else {
        match git_tools::git_diff(Path::new(&repo_root)) {
            Ok(entries) => {
                let json = serde_json::to_string(&entries).unwrap_or_default();
                let summary = format!(
                    "git_diff: {} files changed ({} additions, {} deletions)",
                    entries.len(),
                    entries.iter().map(|e| e.additions).sum::<usize>(),
                    entries.iter().map(|e| e.deletions).sum::<usize>()
                );
                Ok(ToolInvocation {
                    success: true,
                    stdout: Some(summary),
                    stderr: None,
                    error_message: None,
                    diagnostics: vec![],
                    duration_ms: 0,
                    kind: ToolOutputKind::Structural,
                    structured_data: serde_json::from_str(&json).ok(),
                })
            }
            Err(e) => Ok(ToolInvocation {
                success: false,
                stdout: None,
                stderr: None,
                error_message: Some(e.to_string()),
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::Error,
                structured_data: None,
            }),
        }
    }
}

fn invoke_git_log(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    let repo_root = step
        .arguments
        .get("repo_root")
        .cloned()
        .unwrap_or_else(|| ".".to_string());

    let limit = step
        .arguments
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok());

    match git_tools::git_log(Path::new(&repo_root), limit) {
        Ok(entries) => {
            let json = serde_json::to_string(&entries).unwrap_or_default();
            let summary = format!(
                "git_log: {} commits",
                entries.len()
            );
            Ok(ToolInvocation {
                success: true,
                stdout: Some(summary),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::Structural,
                structured_data: serde_json::from_str(&json).ok(),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_wc(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    let paths_str = step
        .arguments
        .get("paths")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "paths".to_string(),
        })?;

    let paths: Vec<String> = serde_json::from_str(paths_str)
        .unwrap_or_else(|_| paths_str.split(',').map(|s| s.trim().to_string()).collect());

    let lines = step
        .arguments
        .get("lines")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(true);

    let words = step
        .arguments
        .get("words")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(true);

    let chars = step
        .arguments
        .get("chars")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(true);

    let bytes = step
        .arguments
        .get("bytes")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    let args = WcArgs {
        paths,
        lines,
        words,
        chars,
        bytes,
    };

    match os_tools::wc(args) {
        Ok(result) => {
            let json = serde_json::to_string(&result).unwrap_or_default();
            let summary = format!(
                "wc: {} files",
                result.entries.len()
            );
            Ok(ToolInvocation {
                success: true,
                stdout: Some(summary),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::Structural,
                structured_data: serde_json::from_str(&json).ok(),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_bash_exec(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    let command = step
        .arguments
        .get("command")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "command".to_string(),
        })?;

    let timeout_ms = step
        .arguments
        .get("timeout_ms")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30000);

    let working_dir = step.arguments.get("working_dir").cloned();

    let args = BashExecArgs {
        command: command.clone(),
        timeout_ms,
        working_dir,
    };

    match os_tools::bash_exec(args) {
        Ok(result) => {
            let json = serde_json::to_string(&result).unwrap_or_default();
            Ok(ToolInvocation {
                success: result.exit_code == 0,
                stdout: Some(result.stdout.clone()),
                stderr: Some(result.stderr.clone()),
                error_message: if result.exit_code != 0 {
                    Some(format!("Exit code: {}", result.exit_code))
                } else {
                    None
                },
                diagnostics: vec![],
                duration_ms: result.duration_ms as i64,
                kind: ToolOutputKind::Textual,
                structured_data: serde_json::from_str(&json).ok(),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e.to_string()),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_count_files(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    use crate::stats_tools::{count_files, CountFilesArgs};

    let pattern = step
        .arguments
        .get("pattern")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "pattern".to_string(),
        })?;

    let root = step
        .arguments
        .get("root")
        .cloned()
        .unwrap_or_else(|| ".".to_string());

    let by_extension = step
        .arguments
        .get("by_extension")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    let args = CountFilesArgs {
        pattern: pattern.clone(),
        root,
        by_extension,
    };

    match count_files(args) {
        Ok(result) => {
            let json = serde_json::to_string(&result).unwrap_or_default();
            let summary = if let Some(ext_map) = &result.by_extension {
                let parts: Vec<String> = ext_map
                    .iter()
                    .take(5) // Limit to top 5 in summary
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                format!(
                    "count_files: {} files total ({})",
                    result.total_count,
                    parts.join(", ")
                )
            } else {
                format!("count_files: {} files", result.total_count)
            };

            Ok(ToolInvocation {
                success: true,
                stdout: Some(summary),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::NumericSummary,
                structured_data: serde_json::from_str(&json).ok(),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_count_lines(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    use crate::stats_tools::{count_lines, CountLinesArgs};

    let pattern = step
        .arguments
        .get("pattern")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "pattern".to_string(),
        })?;

    let root = step
        .arguments
        .get("root")
        .cloned()
        .unwrap_or_else(|| ".".to_string());

    let args = CountLinesArgs {
        pattern: pattern.clone(),
        root,
    };

    match count_lines(args) {
        Ok(result) => {
            let json = serde_json::to_string(&result).unwrap_or_default();
            let summary = format!(
                "count_lines: {} total lines in {} files",
                result.total_lines, result.file_count
            );

            Ok(ToolInvocation {
                success: true,
                stdout: Some(summary),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::NumericSummary,
                structured_data: serde_json::from_str(&json).ok(),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}

fn invoke_fs_stats(step: &Step) -> Result<ToolInvocation, ExecutionError> {
    use crate::stats_tools::{fs_stats, FsStatsArgs};

    let path = step
        .arguments
        .get("path")
        .ok_or_else(|| ExecutionError::MissingArgument {
            tool: step.tool.clone(),
            argument: "path".to_string(),
        })?;

    let max_depth = step
        .arguments
        .get("max_depth")
        .and_then(|v| v.parse::<usize>().ok());

    let args = FsStatsArgs {
        path: path.clone(),
        max_depth,
    };

    match fs_stats(args) {
        Ok(result) => {
            let json = serde_json::to_string(&result).unwrap_or_default();
            let summary = format!(
                "fs_stats: {} files, {} dirs, {} bytes",
                result.file_count, result.dir_count, result.total_bytes
            );

            Ok(ToolInvocation {
                success: true,
                stdout: Some(summary),
                stderr: None,
                error_message: None,
                diagnostics: vec![],
                duration_ms: 0,
                kind: ToolOutputKind::NumericSummary,
                structured_data: serde_json::from_str(&json).ok(),
            })
        }
        Err(e) => Ok(ToolInvocation {
            success: false,
            stdout: None,
            stderr: None,
            error_message: Some(e),
            diagnostics: vec![],
            duration_ms: 0,
            kind: ToolOutputKind::Error,
            structured_data: None,
        }),
    }
}
