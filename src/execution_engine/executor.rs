//! Executor: synchronous step runner for approved plans

use std::time::Instant;

use crate::execution_engine::errors::ExecutionError;
use crate::execution_engine::preconditions::check_precondition;
use crate::execution_engine::result::{ExecutionResult, ExecutionStatus, StepResult};
use crate::execution_engine::tool_mapper::invoke_tool;
use crate::execution_engine::{generate_execution_id, ApprovedPlan};
use crate::execution_tools::ExecutionDb;
use crate::llm::types::Step;
use crate::magellan_tools::MagellanDb;
use serde_json::json;

/// Callback for user confirmation during execution
///
/// Called when step.requires_confirmation == true.
/// Executor BLOCKS until callback returns.
pub trait ConfirmationCallback: Send + Sync {
    /// Request user approval for a step
    ///
    /// Returns true if user approves, false if denied.
    fn request_confirmation(&self, step: &Step) -> bool;
}

/// Callback for progress updates during execution
pub trait ProgressCallback: Send + Sync {
    /// Called before step execution begins
    fn on_step_start(&self, step: &Step);

    /// Called after step completes successfully
    fn on_step_complete(&self, result: &StepResult);

    /// Called after step fails
    fn on_step_failed(&self, result: &StepResult);
}

/// Executor: synchronous step runner for approved plans
pub struct Executor {
    db: ExecutionDb,
    magellan_db: Option<MagellanDb>,
    confirmation_callback: Box<dyn ConfirmationCallback>,
    progress_callback: Box<dyn ProgressCallback>,
}

impl Executor {
    /// Create new executor with database connections and callbacks
    pub fn new(
        db: ExecutionDb,
        magellan_db: Option<MagellanDb>,
        confirmation_callback: Box<dyn ConfirmationCallback>,
        progress_callback: Box<dyn ProgressCallback>,
    ) -> Self {
        Executor {
            db,
            magellan_db,
            confirmation_callback,
            progress_callback,
        }
    }

    /// Execute an approved plan
    ///
    /// Returns ExecutionResult with all completed step results.
    /// Stops immediately on first failure.
    pub fn execute(&mut self, approved: ApprovedPlan) -> Result<ExecutionResult, ExecutionError> {
        // Validate authorization
        if !approved.authorization.is_approved() {
            return Err(ExecutionError::NotAuthorized(
                "Plan not approved by user".to_string(),
            ));
        }

        // Validate plan ID matches authorization
        if approved.plan.plan_id != approved.authorization.plan_id() {
            return Err(ExecutionError::PlanIdMismatch {
                plan: approved.plan.plan_id.clone(),
                auth: approved.authorization.plan_id().to_string(),
            });
        }

        let start_time = Instant::now();
        let mut step_results = Vec::new();

        for step in &approved.plan.steps {
            // Notify progress
            self.progress_callback.on_step_start(step);

            // Check precondition
            if let Err(reason) = check_precondition(step) {
                let step_result = StepResult {
                    step_id: step.step_id.clone(),
                    tool_name: step.tool.clone(),
                    success: false,
                    execution_id: generate_execution_id(),
                    stdout: None,
                    stderr: None,
                    error_message: Some(format!("Precondition failed: {}", reason)),
                    duration_ms: 0,
                    diagnostic_artifacts: vec![],
                };

                self.progress_callback.on_step_failed(&step_result);
                step_results.push(step_result);

                return Ok(ExecutionResult {
                    plan_id: approved.plan.plan_id.clone(),
                    status: ExecutionStatus::Failed,
                    step_results,
                    total_duration_ms: start_time.elapsed().as_millis() as i64,
                });
            }

            // Check confirmation if required
            if step.requires_confirmation {
                let user_approved = self.confirmation_callback.request_confirmation(step);
                if !user_approved {
                    let step_result = StepResult {
                        step_id: step.step_id.clone(),
                        tool_name: step.tool.clone(),
                        success: false,
                        execution_id: generate_execution_id(),
                        stdout: None,
                        stderr: None,
                        error_message: Some("Confirmation denied by user".to_string()),
                        duration_ms: 0,
                        diagnostic_artifacts: vec![],
                    };

                    self.progress_callback.on_step_failed(&step_result);
                    step_results.push(step_result);

                    return Ok(ExecutionResult {
                        plan_id: approved.plan.plan_id.clone(),
                        status: ExecutionStatus::Failed,
                        step_results,
                        total_duration_ms: start_time.elapsed().as_millis() as i64,
                    });
                }
            }

            // Invoke tool
            let step_start = Instant::now();
            let invocation = invoke_tool(step, &self.db, &self.magellan_db, None)?;
            let duration_ms = step_start.elapsed().as_millis() as i64;

            let execution_id = generate_execution_id();

            // Log to execution memory
            self.log_execution(&execution_id, step, duration_ms, &invocation)?;

            let step_result = StepResult {
                step_id: step.step_id.clone(),
                tool_name: step.tool.clone(),
                success: invocation.success,
                execution_id: execution_id.clone(),
                stdout: invocation.stdout,
                stderr: invocation.stderr,
                error_message: invocation.error_message,
                duration_ms,
                diagnostic_artifacts: invocation.diagnostics,
            };

            // Notify progress
            if step_result.success {
                self.progress_callback.on_step_complete(&step_result);
                step_results.push(step_result);
            } else {
                self.progress_callback.on_step_failed(&step_result);
                step_results.push(step_result);

                return Ok(ExecutionResult {
                    plan_id: approved.plan.plan_id.clone(),
                    status: ExecutionStatus::Failed,
                    step_results,
                    total_duration_ms: start_time.elapsed().as_millis() as i64,
                });
            }
        }

        // All steps completed successfully
        Ok(ExecutionResult {
            plan_id: approved.plan.plan_id.clone(),
            status: ExecutionStatus::Completed,
            step_results,
            total_duration_ms: start_time.elapsed().as_millis() as i64,
        })
    }

    /// Log step execution to execution memory
    fn log_execution(
        &self,
        execution_id: &str,
        step: &Step,
        duration_ms: i64,
        invocation: &crate::execution_engine::tool_mapper::ToolInvocation,
    ) -> Result<(), ExecutionError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let arguments_json = json!(step.arguments);

        // Build artifacts
        let mut artifact_values: Vec<serde_json::Value> = Vec::new();
        let mut artifact_types: Vec<&str> = Vec::new();

        if invocation.stdout.is_some() {
            artifact_types.push("stdout");
            artifact_values.push(json!(invocation.stdout));
        }

        if invocation.stderr.is_some() {
            artifact_types.push("stderr");
            artifact_values.push(json!(invocation.stderr));
        }

        if !invocation.diagnostics.is_empty() {
            artifact_types.push("diagnostics");
            artifact_values.push(json!(invocation.diagnostics));
        }

        // Create slice of tuples with references
        let artifacts: Vec<(&str, &serde_json::Value)> = artifact_types
            .iter()
            .zip(artifact_values.iter())
            .map(|(t, v)| (*t, v))
            .collect();

        self.db
            .record_execution_with_artifacts(
                execution_id,
                &step.tool,
                &arguments_json,
                timestamp,
                invocation.success,
                if invocation.success { Some(0) } else { Some(1) },
                Some(duration_ms),
                invocation.error_message.as_deref(),
                &artifacts[..],
            )
            .map_err(|e| ExecutionError::RecordingError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {}
