//! HTTP Transport for LLM Adapters (Phase 5)
//!
//! Provides synchronous HTTP client with streaming support.
//! Uses ureq for blocking I/O.

// Import sibling modules (declared in adapters/mod.rs)
pub use crate::llm::adapters::transport_fake::FakeTransport;
pub use crate::llm::adapters::transport_types::{AdapterError, SyncTransport};
pub use crate::llm::adapters::transport_ureq::UreqTransport;

/// Concrete transport enum (Phase 5)
///
/// Wraps all transport types, avoiding dyn compatibility issues.
#[derive(Debug)]
pub enum Transport {
    Real(UreqTransport),
    Fake(FakeTransport),
}

impl SyncTransport for Transport {
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, AdapterError> {
        match self {
            Transport::Real(t) => t.post_json(url, headers, body),
            Transport::Fake(t) => t.post_json(url, headers, body),
        }
    }

    fn post_stream<F>(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
        on_line: F,
    ) -> Result<String, AdapterError>
    where
        F: FnMut(&str),
    {
        match self {
            Transport::Real(t) => t.post_stream(url, headers, body, on_line),
            Transport::Fake(t) => t.post_stream(url, headers, body, on_line),
        }
    }
}

impl Default for Transport {
    fn default() -> Self {
        Transport::Real(UreqTransport::new())
    }
}
