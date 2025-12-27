//! Ollama response parsing
//!
//! Public functions for parsing Ollama JSON and NDJSON responses.

use crate::llm::adapters::AdapterError;
use serde_json::Value as JsonValue;

/// Parse Ollama chat completion JSON response
///
/// Public function for testing.
pub fn parse_chat_completion(response: &str) -> Result<String, AdapterError> {
    let json: JsonValue = serde_json::from_str(response)?;

    let content = json
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| AdapterError::InvalidResponse("Missing message.content".to_string()))?;

    Ok(content.to_string())
}

/// Parse NDJSON stream from response body
///
/// Public function for testing.
/// Returns concatenated content from all chunks.
pub fn parse_ndjson_stream<F>(body: &str, mut on_chunk: F) -> Result<String, AdapterError>
where
    F: FnMut(&str),
{
    let mut full_content = String::new();

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(json) = serde_json::from_str::<JsonValue>(line) {
            // Extract content first (in case this line has done=true)
            if let Some(content) = json
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
            {
                on_chunk(content);
                full_content.push_str(content);
            }

            // Then check if done and stop processing
            if let Some(true) = json.get("done").and_then(|d| d.as_bool()) {
                break;
            }
        }
    }

    Ok(full_content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chat_completion_valid() {
        let json = r#"{"message":{"content":"ollama test"}}"#;
        let result = parse_chat_completion(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "ollama test");
    }

    #[test]
    fn test_parse_chat_completion_missing_message() {
        let json = r#"{"model":"codellama"}"#;
        let result = parse_chat_completion(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ndjson_stream() {
        let ndjson = r#"{"message":{"content":"Hello"},"done":false}
{"message":{"content":" world"},"done":false}
{"message":{"content":"!"},"done":true}
{"message":{"content":"after"},"done":true}"#;

        let mut chunks = Vec::new();
        let result = parse_ndjson_stream(ndjson, |c| chunks.push(c.to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello world!");
        // Should stop at done=true, not include "after"
        assert!(!chunks.join("").contains("after"));
    }

    #[test]
    fn test_parse_ndjson_stops_at_done() {
        let ndjson = r#"{"message":{"content":"first"},"done":false}
{"message":{"content":"second"},"done":true}
{"message":{"content":"third"},"done":true}"#;

        let result = parse_ndjson_stream(ndjson, |_| {});
        assert!(result.is_ok());
        assert!(!result.unwrap().contains("third"));
    }
}
