// LSP diagnostics implementation using cargo check

use std::path::Path;
use std::process::Command;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, LspError>;

#[derive(Debug, Error)]
pub enum LspError {
    #[error("Cargo not found in PATH")]
    CargoNotFound,

    #[error("Failed to execute cargo check: {0}")]
    ExecutionFailed(String),

    #[error("Failed to parse cargo JSON output: {0}")]
    ParseError(String),
}

/// Diagnostic from cargo check
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub level: String,
    pub message: String,
    pub file_name: String,
    pub line_start: usize,
    pub code: Option<String>,
}

/// Run cargo check and parse diagnostics from JSON output
pub fn lsp_check(path: &Path) -> Result<Vec<Diagnostic>> {
    let output = Command::new("cargo")
        .arg("check")
        .arg("--message-format=json")
        .current_dir(path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                LspError::CargoNotFound
            } else {
                LspError::ExecutionFailed(e.to_string())
            }
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut diagnostics = Vec::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| LspError::ParseError(format!("Invalid JSON: {}", e)))?;

        // Only process compiler-message entries
        if value.get("reason").and_then(|r| r.as_str()) != Some("compiler-message") {
            continue;
        }

        let msg = value
            .get("message")
            .ok_or_else(|| LspError::ParseError("Missing 'message' field".to_string()))?;

        let level = msg
            .get("level")
            .and_then(|l| l.as_str())
            .ok_or_else(|| LspError::ParseError("Missing 'level'".to_string()))?
            .to_string();

        let message = msg
            .get("message")
            .and_then(|m| m.as_str())
            .ok_or_else(|| LspError::ParseError("Missing 'message' text".to_string()))?
            .to_string();

        let spans = msg
            .get("spans")
            .and_then(|s| s.as_array())
            .ok_or_else(|| LspError::ParseError("Missing 'spans'".to_string()))?;

        // Skip diagnostics with no spans (e.g., failure-notes)
        let primary_span = match spans.first() {
            Some(span) => span,
            None => continue,
        };

        let file_name = primary_span
            .get("file_name")
            .and_then(|f| f.as_str())
            .ok_or_else(|| LspError::ParseError("Missing 'file_name'".to_string()))?
            .to_string();

        let line_start = primary_span
            .get("line_start")
            .and_then(|l| l.as_u64())
            .ok_or_else(|| LspError::ParseError("Missing 'line_start'".to_string()))?
            as usize;

        let code = msg
            .get("code")
            .and_then(|c| c.get("code"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        diagnostics.push(Diagnostic {
            level,
            message,
            file_name,
            line_start,
            code,
        });
    }

    Ok(diagnostics)
}
