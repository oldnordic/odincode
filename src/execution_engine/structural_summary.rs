//! Lean structural summary builder (Phase 9.7-B)
//!
//! Creates chat-safe summaries for structural tool outputs.
//! Provides count + 3-5 representative entries + pointer to full data.
//!
//! Contract:
//! - List data → count + first N entries
//! - Symbols → name + file
//! - Stats → unchanged (already chat-safe)
//! - Never return empty summaries

use serde_json::Value as JsonValue;

const MAX_EXAMPLES: usize = 5;

/// Build a lean structural summary for chat injection
///
/// Input: structured JSON data from tool execution
/// Output: Human-readable summary with count and examples
pub fn build_structural_summary(tool: &str, data: &JsonValue) -> String {
    match tool {
        "file_search" => summarize_search_results(data),
        "file_glob" => summarize_path_list(data, "files matched"),
        "symbols_in_file" => summarize_symbols(data),
        "references_to_symbol_name" | "references_from_file_to_symbol_name" => {
            summarize_references(data)
        }
        "lsp_check" => summarize_diagnostics(data),
        _ => format!("{}: completed", tool),
    }
}

/// Summarize file_search results
fn summarize_search_results(data: &JsonValue) -> String {
    if let Some(arr) = data.as_array() {
        let count = arr.len();
        if count == 0 {
            return "file_search: no matches found".to_string();
        }

        let mut examples = Vec::new();
        for item in arr.iter().take(MAX_EXAMPLES) {
            if let Some(obj) = item.as_object() {
                let file = obj.get("file").and_then(|v| v.as_str()).unwrap_or("?");
                let line = obj.get("line").and_then(|v| v.as_i64()).unwrap_or(0);
                examples.push(format!("{}:{}", file, line));
            }
        }

        format!(
            "file_search: {} matches found\nExamples:\n  - {}\n(Full results in Explorer)",
            count,
            examples.join("\n  - ")
        )
    } else {
        "file_search: completed".to_string()
    }
}

/// Summarize a list of paths (file_glob, etc.)
fn summarize_path_list(data: &JsonValue, label: &str) -> String {
    if let Some(arr) = data.as_array() {
        let count = arr.len();
        if count == 0 {
            return format!("file_glob: no {}", label);
        }

        let examples: Vec<&str> = arr
            .iter()
            .filter_map(|v| v.as_str())
            .take(MAX_EXAMPLES)
            .collect();

        format!(
            "file_glob: {} {}\nExamples:\n  - {}\n(Full results in Explorer)",
            count,
            label,
            examples.join("\n  - ")
        )
    } else if let Some(str) = data.as_str() {
        // Single string value
        format!("file_glob: {}\n(Full results in Explorer)", str)
    } else {
        format!("file_glob: {}", label)
    }
}

/// Summarize symbol list
fn summarize_symbols(data: &JsonValue) -> String {
    if let Some(arr) = data.as_array() {
        let count = arr.len();
        if count == 0 {
            return "symbols_in_file: no symbols found".to_string();
        }

        let mut examples = Vec::new();
        for item in arr.iter().take(MAX_EXAMPLES) {
            if let Some(obj) = item.as_object() {
                let name = obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let kind = obj
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !kind.is_empty() {
                    examples.push(format!("{} ({})", name, kind));
                } else {
                    examples.push(name.to_string());
                }
            }
        }

        format!(
            "symbols_in_file: {} symbols\nExamples:\n  - {}\n(Full results in Explorer)",
            count,
            examples.join("\n  - ")
        )
    } else {
        "symbols_in_file: completed".to_string()
    }
}

/// Summarize reference list
fn summarize_references(data: &JsonValue) -> String {
    if let Some(arr) = data.as_array() {
        let count = arr.len();
        if count == 0 {
            return "references: none found".to_string();
        }

        let mut examples = Vec::new();
        for item in arr.iter().take(MAX_EXAMPLES) {
            if let Some(obj) = item.as_object() {
                let file = obj
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .or_else(|| obj.get("file").and_then(|v| v.as_str()))
                    .unwrap_or("?");
                let symbol = obj
                    .get("symbol_name")
                    .and_then(|v| v.as_str())
                    .or_else(|| obj.get("name").and_then(|v| v.as_str()))
                    .unwrap_or("?");
                examples.push(format!("{} → {}", symbol, file));
            }
        }

        format!(
            "references: {} found\nExamples:\n  - {}\n(Full results in Explorer)",
            count,
            examples.join("\n  - ")
        )
    } else {
        "references: completed".to_string()
    }
}

/// Summarize diagnostics (lsp_check)
fn summarize_diagnostics(data: &JsonValue) -> String {
    if let Some(arr) = data.as_array() {
        let count = arr.len();
        if count == 0 {
            return "lsp_check: no errors - all clean!".to_string();
        }

        let mut examples = Vec::new();
        for item in arr.iter().take(MAX_EXAMPLES) {
            if let Some(obj) = item.as_object() {
                let file = obj
                    .get("file_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let msg = obj
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let line = obj
                    .get("line_start")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                examples.push(format!("{}:{} - {}", file, line, msg));
            }
        }

        format!(
            "lsp_check: {} diagnostics\nExamples:\n  - {}\n(Full results in Explorer)",
            count,
            examples.join("\n  - ")
        )
    } else {
        "lsp_check: completed".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_summarize_search_results_empty() {
        let data = json!([]);
        let summary = summarize_search_results(&data);
        assert!(summary.contains("no matches found"));
    }

    #[test]
    fn test_summarize_search_results_with_data() {
        let data = json!([
            {"file": "src/main.rs", "line": 42, "content": "foo"},
            {"file": "src/lib.rs", "line": 10, "content": "bar"},
            {"file": "src/util.rs", "line": 5, "content": "baz"}
        ]);
        let summary = summarize_search_results(&data);
        assert!(summary.contains("3 matches found"));
        assert!(summary.contains("src/main.rs:42"));
        assert!(summary.contains("src/lib.rs:10"));
        assert!(summary.contains("Explorer"));
    }

    #[test]
    fn test_summarize_path_list() {
        let data = json!(["src/a.rs", "src/b.rs", "src/c.rs"]);
        let summary = summarize_path_list(&data, "files matched");
        assert!(summary.contains("3 files matched"));
        assert!(summary.contains("src/a.rs"));
        assert!(summary.contains("Explorer"));
    }

    #[test]
    fn test_summarize_symbols() {
        let data = json!([
            {"name": "main", "kind": "Function", "id": 1},
            {"name": "Config", "kind": "Struct", "id": 2}
        ]);
        let summary = summarize_symbols(&data);
        assert!(summary.contains("2 symbols"));
        assert!(summary.contains("main (Function)"));
        assert!(summary.contains("Explorer"));
    }

    #[test]
    fn test_summarize_diagnostics_clean() {
        let data = json!([]);
        let summary = summarize_diagnostics(&data);
        assert!(summary.contains("no errors"));
        assert!(summary.contains("all clean"));
    }

    #[test]
    fn test_summarize_diagnostics_with_errors() {
        let data = json!([
            {"file_name": "src/main.rs", "line_start": 10, "message": "unused variable"},
            {"file_name": "src/lib.rs", "line_start": 5, "message": "type mismatch"}
        ]);
        let summary = summarize_diagnostics(&data);
        assert!(summary.contains("2 diagnostics"));
        assert!(summary.contains("src/main.rs:10"));
        assert!(summary.contains("Explorer"));
    }

    #[test]
    fn test_summarize_references() {
        let data = json!([
            {"file_path": "src/a.rs", "symbol_name": "foo"},
            {"file_path": "src/b.rs", "symbol_name": "bar"}
        ]);
        let summary = summarize_references(&data);
        assert!(summary.contains("2 found"));
        assert!(summary.contains("foo → src/a.rs"));
        assert!(summary.contains("Explorer"));
    }
}
