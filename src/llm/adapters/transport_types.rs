//! Transport types (Phase 5)
//!
//! Common types shared across transport implementations.

/// Adapter errors (Phase 5)
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    /// Network error (connection refused, timeout, etc.)
    #[error("Network error: {0}")]
    Network(String),

    /// HTTP error (non-2xx status)
    #[error("HTTP error {status}: {message}")]
    Http { status: u16, message: String },

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Rate limited
    #[error("Rate limited{retry_after}")]
    RateLimited { retry_after: String },

    /// Invalid response from provider
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Provider-specific error (see response body)
    #[error("Provider error: {code} - {message}")]
    Provider { code: String, message: String },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Streaming protocol error
    #[error("Streaming error: {0}")]
    Streaming(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(String),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(String),
}

impl From<std::io::Error> for AdapterError {
    fn from(err: std::io::Error) -> Self {
        AdapterError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for AdapterError {
    fn from(err: serde_json::Error) -> Self {
        AdapterError::Json(err.to_string())
    }
}

impl From<ureq::Error> for AdapterError {
    fn from(err: ureq::Error) -> Self {
        match err {
            ureq::Error::Status(code, _response) => AdapterError::Http {
                status: code,
                message: format!("{}", code),
            },
            ureq::Error::Transport(err) => AdapterError::Network(err.to_string()),
        }
    }
}

/// Synchronous HTTP transport (Phase 5)
///
/// Abstraction over HTTP client to enable testing with FakeTransport.
pub trait SyncTransport: Send + Sync {
    /// POST JSON request and return response body
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, AdapterError>;

    /// POST JSON request and process streaming response line-by-line
    ///
    /// Calls `on_line` for each line of the response body.
    /// Returns concatenated response body.
    fn post_stream<F>(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
        on_line: F,
    ) -> Result<String, AdapterError>
    where
        F: FnMut(&str);
}
