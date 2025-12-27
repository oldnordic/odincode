//! OpenAI response parsing
//!
//! Public functions for parsing OpenAI JSON and SSE responses.

use crate::llm::adapters::AdapterError;
use serde_json::Value as JsonValue;

/// Parse OpenAI chat completion JSON response
///
/// Public function for testing.
pub fn parse_chat_completion(response: &str) -> Result<String, AdapterError> {
    let json: JsonValue = serde_json::from_str(response)?;

    let content = json["choices"]
        .get(0)
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| {
            AdapterError::InvalidResponse("Missing choices[0].message.content".to_string())
        })?;

    Ok(content.to_string())
}

/// Parse SSE stream from response body
///
/// Public function for testing.
/// Returns concatenated content from all chunks.
pub fn parse_sse_stream<F>(body: &str, mut on_chunk: F) -> Result<String, AdapterError>
where
    F: FnMut(&str),
{
    let mut full_content = String::new();

    for line in body.lines() {
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                break;
            }
            if let Ok(json) = serde_json::from_str::<JsonValue>(data) {
                if let Some(delta) = json["choices"]
                    .get(0)
                    .and_then(|c| c.get("delta"))
                    .and_then(|d| d.get("content"))
                {
                    if let Some(text) = delta.as_str() {
                        on_chunk(text);
                        full_content.push_str(text);
                    }
                }
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
        let json = r#"{"choices":[{"message":{"content":"test content"}}]}"#;
        let result = parse_chat_completion(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test content");
    }

    #[test]
    fn test_parse_chat_completion_missing_choices() {
        let json = r#"{"model":"gpt-4"}"#;
        let result = parse_chat_completion(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_sse_stream() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\
                  data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\
                  data: [DONE]\n";

        let mut chunks = Vec::new();
        let result = parse_sse_stream(sse, |c| chunks.push(c.to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello world");
        assert_eq!(chunks, vec!["Hello", " world"]);
    }

    #[test]
    fn test_parse_sse_stops_at_done() {
        let sse = "data: {\"content\":\"first\"}\n\
                  data: [DONE]\n\
                  data: {\"content\":\"after\"}\n";

        let result = parse_sse_stream(sse, |_| {});
        assert!(result.is_ok());
        assert!(!result.unwrap().contains("after"));
    }

    #[test]
    fn test_extract_content_from_response() {
        let response = r#"{"choices":[{"message":{"content":"test plan"}}]}"#;
        let result = parse_chat_completion(response);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test plan");
    }
}
