//! Fake transport for testing
//!
//! Uses fixture strings instead of real HTTP calls.

use crate::llm::adapters::transport_types::{AdapterError, SyncTransport};

/// Fake transport for testing (uses fixture strings)
#[derive(Debug)]
pub struct FakeTransport {
    /// Response body to return
    pub response_body: String,
    /// Stream body to return line-by-line
    pub stream_body: String,
    /// Error message to return (if set)
    pub error_message: Option<String>,
}

impl FakeTransport {
    /// Create fake transport with given response
    pub fn new(response: &str) -> Self {
        Self {
            response_body: response.to_string(),
            stream_body: String::new(),
            error_message: None,
        }
    }

    /// Create fake transport with streaming response
    pub fn with_stream(response: &str, stream: &str) -> Self {
        Self {
            response_body: response.to_string(),
            stream_body: stream.to_string(),
            error_message: None,
        }
    }

    /// Create fake transport that returns a network error
    pub fn with_error(msg: &str) -> Self {
        Self {
            response_body: String::new(),
            stream_body: String::new(),
            error_message: Some(msg.to_string()),
        }
    }
}

impl SyncTransport for FakeTransport {
    fn post_json(
        &self,
        _url: &str,
        _headers: &[(&str, &str)],
        _body: &str,
    ) -> Result<String, AdapterError> {
        if let Some(ref msg) = self.error_message {
            return Err(AdapterError::Network(msg.clone()));
        }
        Ok(self.response_body.clone())
    }

    fn post_stream<F>(
        &self,
        _url: &str,
        _headers: &[(&str, &str)],
        _body: &str,
        mut on_line: F,
    ) -> Result<String, AdapterError>
    where
        F: FnMut(&str),
    {
        if let Some(ref msg) = self.error_message {
            return Err(AdapterError::Network(msg.clone()));
        }
        let body = if self.stream_body.is_empty() {
            &self.response_body
        } else {
            &self.stream_body
        };
        let mut full = String::new();
        for line in body.lines() {
            on_line(line);
            full.push_str(line);
            full.push('\n');
        }
        Ok(full)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fake_transport_basic() {
        let transport = FakeTransport::new("test response");
        let result = transport.post_json("http://test", &[], "{}");
        assert_eq!(result.unwrap(), "test response");
    }

    #[test]
    fn test_fake_transport_with_error() {
        let transport = FakeTransport::with_error("test error");
        let result = transport.post_json("http://test", &[], "{}");
        assert!(result.is_err());
    }

    #[test]
    fn test_fake_transport_stream() {
        let transport = FakeTransport::with_stream("response", "line1\nline2\nline3");
        let mut lines = Vec::new();
        let result = transport.post_stream("http://test", &[], "{}", |line| {
            lines.push(line.to_string());
        });
        assert!(result.is_ok());
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line1");
        assert_eq!(lines[1], "line2");
        assert_eq!(lines[2], "line3");
    }

    #[test]
    fn test_adapter_error_display() {
        use crate::llm::adapters::transport_types::AdapterError;

        let err = AdapterError::Network("test".to_string());
        assert_eq!(format!("{}", err), "Network error: test");

        let err = AdapterError::Http {
            status: 404,
            message: "not found".to_string(),
        };
        assert_eq!(format!("{}", err), "HTTP error 404: not found");

        let err = AdapterError::RateLimited {
            retry_after: "60s".to_string(),
        };
        assert!(format!("{}", err).contains("Rate limited"));
        assert!(format!("{}", err).contains("60s"));
    }
}
