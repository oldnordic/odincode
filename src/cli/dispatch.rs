//! CLI mode dispatch
//!
//! Dispatches to appropriate CLI mode handlers:
//! - TUI: Phase 1 terminal UI
//! - plan: Generate plan from goal
//! - execute: Execute stored plan
//! - evidence: Query evidence database

use crate::cli::bootstrap::{ensure_infrastructure, BootstrapStatus};
use crate::cli::db_root::{resolve_db_root, verify_db_root};
use crate::cli::preflight::{run_llm_preflight, PreflightOutcome};
use crate::cli::{Args, Mode, Result, EXIT_DB_ERROR, EXIT_FAILURE, EXIT_SUCCESS};
use std::path::PathBuf;

/// Exit code wrapper for CLI operations
pub type ExitCode = i32;

/// Run CLI mode and return exit code
///
/// This is the main entry point for CLI mode dispatch.
/// Called from main() after argument parsing.
pub fn run_cli_mode(args: Args) -> ExitCode {
    // Resolve db_root first
    let db_root = match resolve_db_root(args.db_root) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: {}", e);
            return EXIT_DB_ERROR;
        }
    };

    // Verify db_root exists
    if let Err(e) = verify_db_root(&db_root) {
        eprintln!("Error: {}", e);
        return EXIT_DB_ERROR;
    }

    // Phase 6: Ensure infrastructure (non-interactive, but allow prompting for config)
    match ensure_infrastructure(&db_root, false, true, args.no_bootstrap) {
        Ok(BootstrapStatus::Ready) => {
            // Continue to preflight
        }
        Ok(BootstrapStatus::NeedsRestart) => {
            // Config was written, need to restart
            return EXIT_SUCCESS;
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            return EXIT_DB_ERROR;
        }
    }

    // Phase 4.1: Run LLM preflight (after bootstrap, before mode dispatch)
    match run_llm_preflight(&db_root) {
        Ok(PreflightOutcome::Exit) => {
            // Config was written, need to restart
            return EXIT_SUCCESS;
        }
        Ok(PreflightOutcome::Proceed) => {
            // Continue to mode dispatch
        }
        Err(e) => {
            eprintln!("Preflight error: {}", e);
            return EXIT_DB_ERROR;
        }
    }

    // Dispatch based on mode
    let mode = args.mode.unwrap_or(Mode::Tui);

    match run_mode(mode, db_root, args.json_output) {
        Ok(()) => EXIT_SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            // Map error to exit code
            match e {
                crate::cli::Error::Database(_) => EXIT_DB_ERROR,
                _ => EXIT_FAILURE,
            }
        }
    }
}

/// Run specific CLI mode
fn run_mode(mode: Mode, db_root: PathBuf, json_output: bool) -> Result<()> {
    match mode {
        Mode::Tui => {
            // TUI is launched from main() directly, not here
            // This is only for explicit "tui" mode
            run_tui_mode(db_root)?;
        }
        Mode::Plan { goal } => {
            run_plan_mode(db_root, goal, json_output)?;
        }
        Mode::Execute { plan_file } => {
            run_execute_mode(db_root, plan_file, json_output)?;
        }
        Mode::Evidence { query, query_args } => {
            run_evidence_mode(db_root, query, query_args, json_output)?;
        }
    }
    Ok(())
}

/// Run TUI mode
fn run_tui_mode(db_root: PathBuf) -> Result<()> {
    // TUI mode requires codegraph.db
    let codegraph_path = db_root.join("codegraph.db");
    if !codegraph_path.exists() {
        return Err(crate::cli::Error::Database(format!(
            "Symbol navigation is unavailable. To enable code search, run:\n  magellan watch --root . --db {}",
            codegraph_path.display()
        )));
    }

    // TUI is actually launched from main()
    // This just verifies the preconditions
    Ok(())
}

/// Run plan mode: generate plan from natural language goal
fn run_plan_mode(db_root: PathBuf, goal: String, json_output: bool) -> Result<()> {
    use crate::evidence_queries::EvidenceDb;
    use crate::llm::session::propose_plan;
    use crate::llm::types::{EvidenceSummary, SessionContext};

    // Plan mode requires codegraph.db
    let codegraph_path = db_root.join("codegraph.db");
    if !codegraph_path.exists() {
        return Err(crate::cli::Error::Database(format!(
            "Symbol navigation is unavailable. To enable code search, run:\n  magellan watch --root . --db {}",
            codegraph_path.display()
        )));
    }

    // Open evidence DB for context
    let _ev_db = EvidenceDb::open(&db_root).map_err(|e| {
        crate::cli::Error::Database(format!("Cannot access execution history: {}", e))
    })?;

    // Create session context
    let context = SessionContext {
        user_intent: goal.clone(),
        selected_file: None,
        current_diagnostic: None,
        db_root: db_root.clone(),
    };

    // Create empty evidence summary (for stub implementation)
    let evidence_summary = EvidenceSummary {
        q1_tool_executions: vec![],
        q2_failures: vec![],
        q3_diagnostic_executions: vec![],
        q4_file_executions: vec![],
        q5_execution_details: None,
        q6_latest_outcome: None,
        q7_recurring: vec![],
        q8_prior_fixes: vec![],
    };

    // Propose plan
    let plan = propose_plan(&context, &evidence_summary)
        .map_err(|e| crate::cli::Error::Execution(format!("Failed to generate plan: {}", e)))?;

    // Create plans directory
    let plans_dir = db_root.join("plans");
    std::fs::create_dir_all(&plans_dir).map_err(crate::cli::Error::Io)?;

    // Write plan to file
    let plan_path = plans_dir.join(format!("{}.json", plan.plan_id));
    let plan_json =
        serde_json::to_string_pretty(&plan).map_err(crate::cli::Error::Serialization)?;

    std::fs::write(&plan_path, plan_json).map_err(crate::cli::Error::Io)?;

    // Output result
    if json_output {
        let output = serde_json::json!({
            "plan_id": plan.plan_id,
            "path": format!("plans/{}.json", plan.plan_id),
            "intent": plan.intent.to_string(),
            "step_count": plan.steps.len(),
        });
        println!("{}", output);
    } else {
        println!("Plan written to plans/{}.json", plan.plan_id);
    }

    Ok(())
}

/// Run execute mode: load and execute stored plan
fn run_execute_mode(db_root: PathBuf, plan_file: String, json_output: bool) -> Result<()> {
    use crate::execution_engine::{ApprovedPlan, AutoApprove, Executor, NoopProgress};
    use crate::execution_tools::ExecutionDb;
    use crate::llm::types::{Plan, PlanAuthorization};

    // Resolve plan file path
    let plan_path = if plan_file.is_empty() {
        return Err(crate::cli::Error::InvalidArgs(
            "--plan-file is required for execute mode".to_string(),
        ));
    } else {
        // If relative, resolve against db_root
        let path = PathBuf::from(&plan_file);
        if path.is_absolute() {
            path
        } else {
            db_root.join(&plan_file)
        }
    };

    // Check plan file exists
    if !plan_path.exists() {
        return Err(crate::cli::Error::InvalidArgs(format!(
            "Plan file not found: {}",
            plan_path.display()
        )));
    }

    // Read and parse plan
    let plan_json = std::fs::read_to_string(&plan_path).map_err(crate::cli::Error::Io)?;

    let plan: Plan = serde_json::from_str(&plan_json).map_err(crate::cli::Error::Serialization)?;

    // Validate plan
    crate::llm::planner::validate_plan(&plan)
        .map_err(|e| crate::cli::Error::Execution(format!("Invalid plan: {}", e)))?;

    // Open execution DB
    let exec_db = ExecutionDb::open(&db_root).map_err(|e| {
        crate::cli::Error::Database(format!("Cannot access execution history: {}", e))
    })?;

    // Open MagellanDb (optional, for tools that need it)
    let codegraph_path = db_root.join("codegraph.db");
    let magellan_db = crate::magellan_tools::MagellanDb::open_readonly(&codegraph_path).ok();

    // Create executor with auto-approve (CLI mode)
    let mut executor = Executor::new(
        exec_db,
        magellan_db,
        Box::new(AutoApprove),
        Box::new(NoopProgress),
    );

    // Create approved plan
    let mut auth = PlanAuthorization::new(plan.plan_id.clone());
    auth.approve();
    let approved = ApprovedPlan {
        plan,
        authorization: auth,
    };

    // Execute plan
    let result = executor
        .execute(approved)
        .map_err(|e| crate::cli::Error::Execution(format!("Execution failed: {}", e)))?;

    // Output result
    if json_output {
        let output = serde_json::json!({
            "plan_id": result.plan_id,
            "status": match result.status {
                crate::execution_engine::ExecutionStatus::Completed => "completed",
                crate::execution_engine::ExecutionStatus::Failed => "failed",
                crate::execution_engine::ExecutionStatus::Partial => "partial",
            },
            "step_results": result.step_results.len(),
            "total_duration_ms": result.total_duration_ms,
        });
        println!("{}", output);
    } else {
        let succeeded = result.step_results.iter().filter(|s| s.success).count();
        let failed = result.step_results.len() - succeeded;

        println!(
            "Plan executed: {} steps, {} succeeded, {} failed",
            result.step_results.len(),
            succeeded,
            failed
        );

        // Show step details for failures
        for step in &result.step_results {
            if !step.success {
                if let Some(ref error) = step.error_message {
                    println!(
                        "  Step {} ({}) failed: {}",
                        step.step_id, step.tool_name, error
                    );
                }
            }
        }
    }

    // Return error if execution failed
    if matches!(
        result.status,
        crate::execution_engine::ExecutionStatus::Failed
    ) {
        return Err(crate::cli::Error::Execution(
            "Plan execution failed".to_string(),
        ));
    }

    Ok(())
}

/// Run evidence mode: query evidence database
fn run_evidence_mode(
    db_root: PathBuf,
    query: String,
    query_args: Vec<String>,
    _json_output: bool,
) -> Result<()> {
    use crate::evidence_queries::EvidenceDb;

    // Open evidence DB
    let ev_db = EvidenceDb::open(&db_root).map_err(|e| {
        crate::cli::Error::Database(format!("Cannot access execution history: {}", e))
    })?;

    // Dispatch query and output as JSON
    let results_json = match query.as_str() {
        "Q1" => {
            let tool = query_args.first().map(|s| s.as_str()).unwrap_or("");
            let results = ev_db
                .list_executions_by_tool(tool, None, None, None)
                .map_err(|e| crate::cli::Error::Database(format!("Query failed: {}", e)))?;
            // Convert to JSON manually
            format_json_array(&results, |r| {
                format!(
                    r#"{{"execution_id":"{}","tool_name":"{}","timestamp":{},"success":{}}}"#,
                    r.execution_id, r.tool_name, r.timestamp, r.success
                )
            })
        }
        "Q2" => {
            let tool = query_args.first().map(|s| s.as_str()).unwrap_or("");
            let results = ev_db
                .list_failures_by_tool(tool, None, None)
                .map_err(|e| crate::cli::Error::Database(format!("Query failed: {}", e)))?;
            format_json_array(&results, |r| {
                format!(
                    r#"{{"execution_id":"{}","tool_name":"{}","timestamp":{},"exit_code":{},"error_message":"{}"}}"#,
                    r.execution_id,
                    r.tool_name,
                    r.timestamp,
                    r.exit_code.unwrap_or(0),
                    r.error_message
                        .as_ref()
                        .unwrap_or(&"".to_string())
                        .replace('"', r#"\""#)
                )
            })
        }
        "Q4" => {
            let path = query_args.first().map(|s| s.as_str()).unwrap_or("");
            let results = ev_db
                .find_executions_by_file(path, None, None)
                .map_err(|e| crate::cli::Error::Database(format!("Query failed: {}", e)))?;
            format_json_array(&results, |r| {
                format!(
                    r#"{{"execution_id":"{}","tool_name":"{}","timestamp":{},"success":{}}}"#,
                    r.execution_id, r.tool_name, r.timestamp, r.success
                )
            })
        }
        _ => {
            // Unknown query - return empty array
            "[]".to_string()
        }
    };

    // Evidence mode always outputs JSON
    println!("{}", results_json);

    Ok(())
}

/// Helper to format Vec as JSON array
fn format_json_array<T, F>(items: &[T], f: F) -> String
where
    F: Fn(&T) -> String,
{
    if items.is_empty() {
        return "[]".to_string();
    }
    let formatted: Vec<String> = items.iter().map(f).collect();
    format!("[{}]", formatted.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_constants() {
        assert_eq!(EXIT_SUCCESS, 0);
        assert_eq!(EXIT_FAILURE, 1);
        assert_eq!(EXIT_DB_ERROR, 2);
    }

    #[test]
    fn test_format_json_array_empty() {
        let items: Vec<i32> = vec![];
        assert_eq!(format_json_array(&items, |i| i.to_string()), "[]");
    }

    #[test]
    fn test_format_json_array_single() {
        let items = vec![1];
        assert_eq!(format_json_array(&items, |i| i.to_string()), "[1]");
    }

    #[test]
    fn test_format_json_array_multiple() {
        let items = vec![1, 2, 3];
        assert_eq!(format_json_array(&items, |i| i.to_string()), "[1,2,3]");
    }
}
