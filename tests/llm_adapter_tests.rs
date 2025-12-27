//! LLM Adapter integration tests (Phase 5)
//!
//! Tests adapter implementations without live network calls.
//! Uses fixtures for deterministic testing.

use odincode::llm::adapters::LlmAdapter;
use std::path::PathBuf;

// Test helpers
fn load_fixture(name: &str) -> String {
    let path = PathBuf::from("tests/fixtures").join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Failed to load fixture: {}", path.display()))
}

#[allow(dead_code)]
fn create_temp_db_root() -> tempfile::TempDir {
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Create execution_log.db
    let exec_log_path = temp_dir.path().join("execution_log.db");
    let conn = rusqlite::Connection::open(&exec_log_path).unwrap();
    conn.execute(
        "CREATE TABLE executions (
            id TEXT PRIMARY KEY NOT NULL,
            tool_name TEXT NOT NULL,
            arguments_json TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            success BOOLEAN NOT NULL,
            exit_code INTEGER,
            duration_ms INTEGER,
            error_message TEXT
        )",
        [],
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE execution_artifacts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            execution_id TEXT NOT NULL,
            artifact_type TEXT NOT NULL,
            content_json TEXT NOT NULL,
            FOREIGN KEY (execution_id) REFERENCES executions(id) ON DELETE RESTRICT
        )",
        [],
    )
    .unwrap();

    // Create codegraph.db (required by ExecutionDb)
    let codegraph_path = temp_dir.path().join("codegraph.db");
    let conn = rusqlite::Connection::open(&codegraph_path).unwrap();
    conn.execute(
        "CREATE TABLE graph_entities (
            id INTEGER PRIMARY KEY,
            kind TEXT NOT NULL,
            name TEXT,
            file_path TEXT,
            data TEXT
        )",
        [],
    )
    .unwrap();

    temp_dir
}

// =============================================================================
// TEST A: Factory selection from config
// =============================================================================

#[test]
fn test_a_factory_openai_from_config() {
    // Config with provider=openai should create OpenAiAdapter
    let config = r#"[llm]
mode = "external"
provider = "openai"
base_url = "https://api.openai.com/v1"
api_key = "sk-test"
model = "gpt-4"
"#;

    let result = odincode::llm::adapters::create_adapter_from_config_str(config);
    assert!(result.is_ok(), "Factory should create adapter from config");
    let adapter = result.unwrap();
    assert_eq!(adapter.provider_name(), "openai");
}

#[test]
fn test_a_factory_glm_from_config() {
    let config = r#"[llm]
mode = "external"
provider = "glm"
base_url = "https://api.z.ai/api/coding/paas/v4"
api_key = "sk-test"
model = "GLM-4.7"
"#;

    let result = odincode::llm::adapters::create_adapter_from_config_str(config);
    assert!(result.is_ok());
    let adapter = result.unwrap();
    assert_eq!(adapter.provider_name(), "glm");
}

#[test]
fn test_a_factory_ollama_from_config() {
    let config = r#"[llm]
mode = "local"
backend = "ollama"
host = "127.0.0.1"
port = "11434"
model = "codellama"
"#;

    let result = odincode::llm::adapters::create_adapter_from_config_str(config);
    assert!(result.is_ok());
    let adapter = result.unwrap();
    assert_eq!(adapter.provider_name(), "ollama");
}

#[test]
fn test_a_factory_invalid_config() {
    let config = r#"[llm]
mode = "external"
provider = "unknown"
base_url = "http://localhost"
model = "test"
"#;

    let result = odincode::llm::adapters::create_adapter_from_config_str(config);
    assert!(result.is_err(), "Should fail for unknown provider");
}

// =============================================================================
// TEST B: Non-stream parsing (OpenAI-compatible)
// =============================================================================

#[test]
fn test_b_openai_parse_json_response() {
    let fixture = load_fixture("openai_chat_completion.json");

    let result = odincode::llm::adapters::openai::parse_chat_completion(&fixture);
    assert!(result.is_ok(), "Should parse OpenAI JSON response");

    let content = result.unwrap();
    assert!(content.contains("plan_openai_test"));
    assert!(content.contains("\"intent\": \"READ\""));
    assert!(content.contains("\"tool\": \"file_read\""));
}

#[test]
fn test_b_openai_parse_missing_fields() {
    let invalid_json = r#"{"model": "gpt-4"}"#;

    let result = odincode::llm::adapters::openai::parse_chat_completion(invalid_json);
    assert!(result.is_err(), "Should fail with missing choices field");
}

// =============================================================================
// TEST C: SSE streaming (OpenAI-compatible / GLM)
// =============================================================================

#[test]
fn test_c_openai_sse_parse_multiple_chunks() {
    let fixture = load_fixture("openai_chat_completion_sse.txt");

    let mut chunks = Vec::new();
    let result = odincode::llm::adapters::openai::parse_sse_stream(&fixture, |chunk| {
        chunks.push(chunk.to_string());
    });

    assert!(result.is_ok(), "Should parse SSE stream");
    assert!(chunks.len() > 1, "Should emit multiple chunks");

    let full_content = result.unwrap();
    assert!(full_content.contains("plan_sse_test"));
    assert!(full_content.contains("\"intent\": \"READ\""));
}

#[test]
fn test_c_glm_sse_parse() {
    let fixture = load_fixture("glm_chat_completion_sse.txt");

    let mut chunk_count = 0;
    let result = odincode::llm::adapters::glm::parse_sse_stream(&fixture, |_chunk| {
        chunk_count += 1;
    });

    assert!(result.is_ok(), "Should parse GLM SSE stream");
    assert!(chunk_count > 1, "Should emit multiple chunks");
    assert!(result.unwrap().contains("plan_glm_sse"));
}

#[test]
fn test_c_sse_stops_at_done() {
    let sse_with_done =
        "data: {\"content\":\"test\"}\n\ndata: [DONE]\n\ndata: {\"content\":\"after\"}\n";

    let chunks = std::cell::RefCell::new(Vec::new());
    let _result =
        odincode::llm::adapters::openai::parse_sse_stream(sse_with_done, |chunk: &str| {
            chunks.borrow_mut().push(chunk.to_string());
        });

    // Should stop at [DONE] and not include "after"
    let chunks = chunks.borrow();
    assert!(chunks.len() <= 2, "Should stop at [DONE] sentinel");
    let all_text = chunks.join("");
    assert!(
        !all_text.contains("after"),
        "Should not include content after [DONE]"
    );
}

// =============================================================================
// TEST D: NDJSON streaming (Ollama)
// =============================================================================

#[test]
fn test_d_ollama_ndjson_parse() {
    let fixture = load_fixture("ollama_generate_ndjson.txt");

    let mut chunks = Vec::new();
    let result = odincode::llm::adapters::ollama::parse_ndjson_stream(&fixture, |chunk: &str| {
        chunks.push(chunk.to_string());
    });

    assert!(result.is_ok(), "Should parse Ollama NDJSON stream");
    assert!(chunks.len() > 1, "Should emit multiple chunks");

    let full_content = result.unwrap();
    eprintln!("DEBUG: full_content = {}", full_content);
    eprintln!("DEBUG: chunks = {:?}", chunks);
    assert!(
        full_content.contains("plan_ollama_sse"),
        "Content should contain plan_ollama_sse"
    );
    assert!(
        full_content.contains("\"intent\": \"QUERY\""),
        "Content should contain intent: QUERY"
    );
}

#[test]
fn test_d_ollama_parse_non_stream() {
    let fixture = load_fixture("ollama_generate.json");

    let result = odincode::llm::adapters::ollama::parse_chat_completion(&fixture);
    assert!(result.is_ok(), "Should parse Ollama JSON response");

    let content = result.unwrap();
    assert!(content.contains("plan_ollama_test"));
    assert!(content.contains("\"tool\": \"file_search\""));
}

#[test]
fn test_d_ollama_stops_at_done_true() {
    let ndjson = r#"{"message":{"content":"first"},"done":false}
{"message":{"content":"second"},"done":true}
{"message":{"content":"third"},"done":true}"#;

    let mut chunks = Vec::new();
    let _result = odincode::llm::adapters::ollama::parse_ndjson_stream(ndjson, |chunk: &str| {
        chunks.push(chunk.to_string());
    });

    // Should stop at done=true, not include "third"
    let all_text = chunks.join("");
    assert!(all_text.contains("first"), "Should include first chunk");
    assert!(all_text.contains("second"), "Should include second chunk");
    assert!(!all_text.contains("third"), "Should stop at done=true");
}

// =============================================================================
// TEST E: Error normalization
// =============================================================================

#[test]
fn test_e_error_display_stable() {
    let err = odincode::llm::adapters::AdapterError::Network("connection refused".to_string());
    let display = format!("{}", err);
    assert!(display.contains("Network error"));
    assert!(display.contains("connection refused"));
}

#[test]
fn test_e_error_http_401_maps_to_auth() {
    let err = odincode::llm::adapters::AdapterError::Http {
        status: 401,
        message: "Unauthorized".to_string(),
    };
    let display = format!("{}", err);
    assert!(display.contains("401"));
}

#[test]
fn test_e_error_http_429_maps_to_rate_limit() {
    let err = odincode::llm::adapters::AdapterError::RateLimited {
        retry_after: "60s".to_string(),
    };
    let display = format!("{}", err);
    assert!(display.contains("Rate limited"));
    assert!(display.contains("60s"));
}

// =============================================================================
// TEST F: Integration with session types
// =============================================================================

#[test]
fn test_f_adapter_output_integrates_with_planner() {
    // Verify adapter output can be parsed by planner
    let fixture = load_fixture("openai_chat_completion.json");
    let content = odincode::llm::adapters::openai::parse_chat_completion(&fixture)
        .expect("Should parse response");

    // Should be valid plan JSON
    let plan_result = odincode::llm::planner::parse_plan(&content);
    assert!(
        plan_result.is_ok(),
        "Adapter output should be valid plan JSON"
    );

    let plan = plan_result.unwrap();
    assert_eq!(plan.plan_id, "plan_openai_test");
    assert_eq!(plan.intent, odincode::llm::types::Intent::Read);
}
