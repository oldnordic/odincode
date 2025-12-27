//! Evidence summary rendering
//!
//! Converts EvidenceSummary to deterministic text format for LLM consumption.

use crate::llm::types::EvidenceSummary;

/// Render evidence summary for prompt inclusion
///
/// Converts EvidenceSummary to deterministic text format.
/// LLM receives THIS, not raw DB access.
pub fn render_evidence_summary(summary: &EvidenceSummary) -> String {
    let mut output = String::from("EVIDENCE SUMMARY:\n");

    // Q1: Tool executions
    if !summary.q1_tool_executions.is_empty() {
        output.push_str("Q1 (tool_executions):\n");
        for (tool, total, success, failure) in &summary.q1_tool_executions {
            output.push_str(&format!(
                "  - {}: {} total, {} success, {} failure\n",
                tool, total, success, failure
            ));
        }
    }

    // Q2: Failures
    if !summary.q2_failures.is_empty() {
        output.push_str("Q2 (failures):\n");
        for (tool, ts, err) in &summary.q2_failures {
            output.push_str(&format!("  - {}: {} - \"{}\"\n", tool, ts, err));
        }
    }

    // Q3: Diagnostic executions
    if !summary.q3_diagnostic_executions.is_empty() {
        output.push_str("Q3 (diagnostic_executions):\n");
        for (code, count) in &summary.q3_diagnostic_executions {
            output.push_str(&format!("  - {}: {} occurrences\n", code, count));
        }
    }

    // Q4: File executions
    if !summary.q4_file_executions.is_empty() {
        output.push_str("Q4 (file_executions):\n");
        for (file, count, success) in &summary.q4_file_executions {
            output.push_str(&format!(
                "  - {}: {} executions, last: {}\n",
                file,
                count,
                if *success { "success" } else { "failure" }
            ));
        }
    }

    // Q5: Execution details
    if let Some((eid, tool, ts, success)) = &summary.q5_execution_details {
        output.push_str("Q5 (execution_details):\n");
        output.push_str(&format!(
            "  - {}: tool={}, timestamp={}, success={}\n",
            eid, tool, ts, success
        ));
    }

    // Q6: Latest outcome
    if let Some((file, success, ts)) = &summary.q6_latest_outcome {
        output.push_str("Q6 (latest_outcome):\n");
        output.push_str(&format!(
            "  - {}: {}, timestamp={}\n",
            file,
            if *success { "success" } else { "failure" },
            ts
        ));
    }

    // Q7: Recurring diagnostics
    if !summary.q7_recurring.is_empty() {
        output.push_str("Q7 (recurring):\n");
        for (code, file, count) in &summary.q7_recurring {
            output.push_str(&format!(
                "  - {} in {}: {} occurrences\n",
                code, file, count
            ));
        }
    }

    // Q8: Prior fixes
    if !summary.q8_prior_fixes.is_empty() {
        output.push_str("Q8 (prior_fixes):\n");
        for (code, attempts, gaps) in &summary.q8_prior_fixes {
            output.push_str(&format!(
                "  - {}: {} attempts, gaps: {:?}\n",
                code, attempts, gaps
            ));
        }
    }

    if output == "EVIDENCE SUMMARY:\n" {
        output.push_str("(no evidence available)\n");
    }

    output
}
