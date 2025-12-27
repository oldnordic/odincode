//! Real HTTP transport using ureq
//!
//! Synchronous blocking HTTP client for LLM adapters.

use crate::llm::adapters::transport_types::{AdapterError, SyncTransport};
use std::fs::OpenOptions;
use std::io::{BufRead, Read, Write};

/// Write to debug log file
fn debug_log(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("/tmp/odincode_debug.log")
    {
        let _ = writeln!(file, "{}", msg);
        let _ = file.flush();
    }
}

/// Real HTTP transport using ureq
#[derive(Debug)]
pub struct UreqTransport {
    /// Timeout in seconds for requests
    timeout: u64,
}

impl UreqTransport {
    /// Create new transport with default timeout (30s)
    pub fn new() -> Self {
        Self { timeout: 30 }
    }

    /// Create transport with custom timeout
    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self {
            timeout: timeout_secs,
        }
    }
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncTransport for UreqTransport {
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, AdapterError> {
        let mut request =
            ureq::request("POST", url).timeout(std::time::Duration::from_secs(self.timeout));

        for (key, value) in headers {
            request = request.set(key, value);
        }

        let response = request.send_string(body)?;

        // Check status code
        let status = response.status();
        if status >= 400 {
            return Err(AdapterError::Http {
                status: status as u16,
                message: format!("HTTP {}", status),
            });
        }

        // Map specific error codes
        if status == 401 {
            return Err(AdapterError::Authentication("Invalid API key".to_string()));
        }
        if status == 429 {
            return Err(AdapterError::RateLimited {
                retry_after: String::new(),
            });
        }

        // Read response body
        let mut reader = response.into_reader();
        let mut body = String::new();
        reader.read_to_string(&mut body)?;
        Ok(body)
    }

    fn post_stream<F>(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
        mut on_line: F,
    ) -> Result<String, AdapterError>
    where
        F: FnMut(&str),
    {
        debug_log(&format!("[HTTP] POST {} (timeout={}s, body_len={})", url, self.timeout, body.len()));
        let mut request =
            ureq::request("POST", url).timeout(std::time::Duration::from_secs(self.timeout));

        for (key, value) in headers {
            request = request.set(key, value);
        }

        let response = request.send_string(body)?;
        debug_log(&format!("[HTTP] response: status={}", response.status()));

        // Check status code
        let status = response.status();
        if status >= 400 {
            return Err(AdapterError::Http {
                status: status as u16,
                message: format!("HTTP {}", status),
            });
        }

        // Read response body line by line
        let reader = response.into_reader();
        let mut buf_reader = std::io::BufReader::new(reader);
        let mut full_body = String::new();
        let mut line_buffer = String::new();

        loop {
            line_buffer.clear();
            let bytes_read = buf_reader.read_line(&mut line_buffer)?;
            if bytes_read == 0 {
                break;
            }
            let line = line_buffer.trim_end();
            on_line(line);
            full_body.push_str(line);
            full_body.push('\n');
        }

        Ok(full_body)
    }
}
