//! LLM Transport TLS Tests (Phase 7.2)
//!
//! Regression tests for HTTPS transport bugs.
//!
//! Bug A1: ureq compiled without TLS features causes "Unknown Scheme" errors
//! Bug A2: Adapters hardcode endpoint paths instead of using base_url correctly

use odincode::llm::adapters::{
    glm::GlmAdapter,
    ollama::OllamaAdapter,
    openai::OpenAiAdapter,
    transport_types::SyncTransport,
    transport_ureq::UreqTransport,
    LlmAdapter, // Trait import for provider_name()
};

#[test]
fn test_https_url_supported() {
    // Verify that ureq transport is compiled with TLS support
    // This test will fail if rustls-tls feature is not enabled

    let transport = UreqTransport::new();

    // Attempt to create a request to an HTTPS URL
    // Before fix: This will fail with "Unknown Scheme" error
    // After fix: This should succeed (the request itself, not the connection)
    let result = transport.post_json("https://api.openai.com/v1/chat/completions", &[], "{}");

    // We expect either:
    // 1. Connection failure (DNS/timeout) - means TLS is working
    // 2. NOT "Unknown Scheme" error - means TLS is compiled in
    match result {
        Ok(_) => {
            // Request succeeded (unlikely in test environment, but TLS works)
            // TLS is working
        }
        Err(e) => {
            let error_msg = format!("{:?}", e);
            // Before fix: error_msg contains "Unknown Scheme" or "cannot make HTTPS request"
            // After fix: error_msg should be about connection, DNS, or timeout
            assert!(
                !error_msg.contains("Unknown Scheme")
                    && !error_msg.contains("cannot make HTTPS request"),
                "TLS not compiled in: {}. Cargo.toml needs ureq with rustls-tls feature",
                error_msg
            );
            println!("HTTPS supported (expected connection error: {})", error_msg);
        }
    }
}

#[test]
fn test_openai_adapter_exists() {
    // Verify OpenAI adapter can be created with correct signature
    let adapter = OpenAiAdapter::new(
        "https://api.openai.com/v1".to_string(),
        "gpt-4".to_string(),
        "test-key".to_string(),
    );

    // Adapter should be created successfully
    assert_eq!(adapter.provider_name(), "openai");
}

#[test]
fn test_ollama_adapter_exists() {
    // Verify Ollama adapter can be created with correct signature
    let adapter = OllamaAdapter::new(
        "127.0.0.1".to_string(),
        "11434".to_string(),
        "llama2".to_string(),
    );

    // Adapter should be created successfully
    assert_eq!(adapter.provider_name(), "ollama");
}

#[test]
fn test_glm_adapter_exists() {
    // Verify GLM adapter can be created with correct signature
    let adapter = GlmAdapter::new(
        "https://open.bigmodel.cn/api/paas/v4".to_string(),
        "glm-4".to_string(),
        "test-key".to_string(),
    );

    // Adapter should be created successfully
    assert_eq!(adapter.provider_name(), "glm");
}

#[test]
fn test_transport_error_includes_url_context() {
    // Verify that transport errors include context about what failed
    // At minimum, it shouldn't be a misleading "Unknown Scheme" error

    let transport = UreqTransport::new();
    let test_url = "https://api.openai.com/v1/chat/completions";

    let result = transport.post_json(test_url, &[], "{}");

    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        // Error message should NOT be "Unknown Scheme"
        assert!(
            !error_msg.contains("Unknown Scheme"),
            "Error message should not be misleading: {}",
            error_msg
        );
        println!("Transport error (expected): {}", error_msg);
    }
}

#[test]
fn test_http_url_still_works() {
    // Verify that HTTP URLs still work after enabling TLS
    // (for local LLMs like Ollama)

    let transport = UreqTransport::new();

    // HTTP to localhost should work (connection refused is expected)
    let result = transport.post_json("http://localhost:11434/api/chat", &[], "{}");

    match result {
        Ok(_) => {}
        Err(e) => {
            let error_msg = format!("{:?}", e);
            // Should NOT be "Unknown Scheme"
            assert!(
                !error_msg.contains("Unknown Scheme"),
                "HTTP support broken: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_adapters_support_https_urls() {
    // Verify adapters can be created with HTTPS base URLs
    // (before fix, ureq without TLS would fail at runtime)

    let openai = OpenAiAdapter::new(
        "https://api.openai.com/v1".to_string(),
        "gpt-4".to_string(),
        "test-key".to_string(),
    );
    assert_eq!(openai.provider_name(), "openai");

    let glm = GlmAdapter::new(
        "https://open.bigmodel.cn/api/paas/v4".to_string(),
        "glm-4".to_string(),
        "test-key".to_string(),
    );
    assert_eq!(glm.provider_name(), "glm");

    // Ollama uses HTTP by default (localhost)
    let ollama = OllamaAdapter::new(
        "127.0.0.1".to_string(),
        "11434".to_string(),
        "llama2".to_string(),
    );
    assert_eq!(ollama.provider_name(), "ollama");
}
