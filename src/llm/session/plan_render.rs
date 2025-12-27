//! Plan rendering for UI display
//!
//! Pure functions for rendering plans to user-readable format.

use crate::llm::types::Plan;

/// Render plan for UI display
///
/// Pure function, no side effects.
/// UI displays this to user before execution.
pub fn render_plan_for_ui(plan: &Plan) -> String {
    let mut output = format!("Proposed Plan: {}\n", plan.plan_id);
    output.push_str(&format!("Intent: {}\n\n", plan.intent.to_string()));

    for (idx, step) in plan.steps.iter().enumerate() {
        output.push_str(&format!("Step {}: {}\n", idx + 1, step.tool));
        output.push_str(&format!("  Tool: {}\n", step.tool));

        // Display arguments (sorted for determinism)
        let mut args: Vec<_> = step.arguments.iter().collect();
        args.sort_by(|a, b| a.0.cmp(b.0));
        for (key, value) in args {
            output.push_str(&format!("  {}: {}\n", key, value));
        }

        output.push_str(&format!("  Precondition: {}\n", step.precondition));
        if step.requires_confirmation {
            output.push_str("  Requires confirmation: YES\n");
        }
        output.push('\n');
    }

    if !plan.evidence_referenced.is_empty() {
        output.push_str(&format!(
            "Evidence cited: {}\n",
            plan.evidence_referenced.join(", ")
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::{Intent, Step};

    #[test]
    fn test_render_plan_for_ui_basic() {
        let plan = Plan {
            plan_id: "test_plan".to_string(),
            intent: Intent::Read,
            steps: vec![],
            evidence_referenced: vec![],
        };

        let rendered = render_plan_for_ui(&plan);
        assert!(rendered.contains("test_plan"));
        assert!(rendered.contains("READ"));
    }

    #[test]
    fn test_render_plan_with_steps() {
        let step = Step {
            step_id: "step_1".to_string(),
            tool: "file_read".to_string(),
            arguments: vec![("path".to_string(), "src/lib.rs".to_string())]
                .into_iter()
                .collect(),
            precondition: "File exists".to_string(),
            requires_confirmation: false,
        };

        let plan = Plan {
            plan_id: "test_plan".to_string(),
            intent: Intent::Read,
            steps: vec![step],
            evidence_referenced: vec![],
        };

        let rendered = render_plan_for_ui(&plan);
        assert!(rendered.contains("Step 1"));
        assert!(rendered.contains("file_read"));
        assert!(rendered.contains("src/lib.rs"));
        assert!(rendered.contains("File exists"));
    }

    #[test]
    fn test_render_plan_with_confirmation() {
        let step = Step {
            step_id: "step_1".to_string(),
            tool: "file_write".to_string(),
            arguments: vec![].into_iter().collect(),
            precondition: "File exists".to_string(),
            requires_confirmation: true,
        };

        let plan = Plan {
            plan_id: "test_plan".to_string(),
            intent: Intent::Mutate,
            steps: vec![step],
            evidence_referenced: vec![],
        };

        let rendered = render_plan_for_ui(&plan);
        assert!(rendered.contains("Requires confirmation: YES"));
    }

    #[test]
    fn test_render_plan_with_evidence() {
        let plan = Plan {
            plan_id: "test_plan".to_string(),
            intent: Intent::Read,
            steps: vec![],
            evidence_referenced: vec!["symbol:X".to_string(), "file:Y".to_string()],
        };

        let rendered = render_plan_for_ui(&plan);
        assert!(rendered.contains("Evidence cited"));
        assert!(rendered.contains("symbol:X"));
        assert!(rendered.contains("file:Y"));
    }

    #[test]
    fn test_render_plan_deterministic_args() {
        // Add arguments in non-alphabetical order
        let mut args = std::collections::HashMap::new();
        args.insert("zebra".to_string(), "last".to_string());
        args.insert("apple".to_string(), "first".to_string());

        let step = Step {
            step_id: "step_1".to_string(),
            tool: "test_tool".to_string(),
            arguments: args,
            precondition: "".to_string(),
            requires_confirmation: false,
        };

        let plan = Plan {
            plan_id: "test_plan".to_string(),
            intent: Intent::Read,
            steps: vec![step],
            evidence_referenced: vec![],
        };

        let rendered = render_plan_for_ui(&plan);
        // Arguments should be sorted alphabetically
        let apple_pos = rendered.find("apple").unwrap();
        let zebra_pos = rendered.find("zebra").unwrap();
        assert!(apple_pos < zebra_pos, "Arguments should be sorted");
    }
}
