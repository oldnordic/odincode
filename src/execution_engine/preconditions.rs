//! Precondition checking for plan steps

use crate::llm::types::Step;
use std::path::Path;

/// Check if a precondition is satisfied
///
/// Returns Ok(()) if precondition passes, Err(reason) if it fails.
pub fn check_precondition(step: &Step) -> Result<(), String> {
    match step.precondition.as_str() {
        "none" => Ok(()),
        "file exists" => check_file_exists(step),
        "Cargo workspace exists" => check_cargo_workspace(step),
        "Cargo project exists" => check_cargo_workspace(step), // Same as workspace for our purposes
        "codegraph.db exists" => check_codegraph_exists(step),
        "symbol exists" => check_symbol_exists(step),
        "root exists" => check_root_exists(step),
        other if other.starts_with("file exists") => check_file_exists(step),
        _ => Ok(()), // Unknown preconditions pass for now
    }
}

fn check_file_exists(step: &Step) -> Result<(), String> {
    let path = step
        .arguments
        .get("path")
        .ok_or_else(|| "Missing 'path' argument".to_string())?;

    if Path::new(path).exists() {
        Ok(())
    } else {
        Err(format!("File does not exist: {}", path))
    }
}

fn check_cargo_workspace(step: &Step) -> Result<(), String> {
    let path = step
        .arguments
        .get("path")
        .ok_or_else(|| "Missing 'path' argument".to_string())?;

    let cargo_toml = Path::new(path).join("Cargo.toml");
    if cargo_toml.exists() {
        Ok(())
    } else {
        Err(format!("Cargo.toml not found in: {}", path))
    }
}

fn check_codegraph_exists(step: &Step) -> Result<(), String> {
    let db_root = step
        .arguments
        .get("db_root")
        .ok_or_else(|| "Missing 'db_root' argument".to_string())?;

    let codegraph_db = Path::new(db_root).join("codegraph.db");
    if codegraph_db.exists() {
        Ok(())
    } else {
        Err(format!("codegraph.db not found in: {}", db_root))
    }
}

fn check_symbol_exists(_step: &Step) -> Result<(), String> {
    // Symbol existence check requires MagellanDb query
    // For precondition purposes, we'll verify during tool invocation
    Ok(())
}

fn check_root_exists(step: &Step) -> Result<(), String> {
    let root = step
        .arguments
        .get("root")
        .or_else(|| step.arguments.get("path"))
        .ok_or_else(|| "Missing 'root' or 'path' argument".to_string())?;

    if Path::new(root).exists() {
        Ok(())
    } else {
        Err(format!("Root does not exist: {}", root))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_step_with_args(args: HashMap<String, String>, precondition: &str) -> Step {
        Step {
            step_id: "test_step".to_string(),
            tool: "file_read".to_string(),
            arguments: args,
            precondition: precondition.to_string(),
            requires_confirmation: false,
        }
    }

    #[test]
    fn test_none_always_passes() {
        let mut args = HashMap::new();
        args.insert("path".to_string(), "/nonexistent".to_string());

        let step = create_step_with_args(args, "none");
        assert!(check_precondition(&step).is_ok());
    }

    #[test]
    fn test_file_exists_fails_for_missing_file() {
        let mut args = HashMap::new();
        args.insert("path".to_string(), "/nonexistent/file.txt".to_string());

        let step = create_step_with_args(args, "file exists");
        assert!(check_precondition(&step).is_err());
    }

    #[test]
    fn test_root_exists_for_current_dir() {
        let mut args = HashMap::new();
        args.insert("root".to_string(), ".".to_string());

        let step = create_step_with_args(args, "root exists");
        assert!(check_precondition(&step).is_ok());
    }
}
