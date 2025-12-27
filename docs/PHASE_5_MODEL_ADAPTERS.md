# Phase 5: Model Adapters (GLM, OpenAI, Ollama)

**Type**: PLANNING ONLY — NO CODE AUTHORIZED
**Status**: PLANNING
**Date**: 2025-12-24
**Precondition**: Phases 4.4 and 4.5 COMPLETE (257/257 tests passing)

---

## 1. Scope & Non-Goals

### IN SCOPE (Phase 5)

1. **Adapter Abstraction Layer**
   - Trait-based interface for LLM providers
   - Normalize request/response formats
   - Provider-specific HTTP clients

2. **Three Provider Adapters**
   - GLM (OpenAI-compatible HTTP API)
   - OpenAI (reference OpenAI API)
   - Ollama (local HTTP API)

3. **Streaming Support**
   - Callback-based streaming (compatible with Phase 4.4)
   - Graceful degradation for non-streaming providers

4. **Error Normalization**
   - Common error type across providers
   - Mapping of provider-specific errors

5. **Configuration Integration**
   - Read existing `config.toml` format
   - Provider selection at startup

### OUT OF SCOPE (Explicitly Deferred)

- ❌ Embeddings
- ❌ RAG (Retrieval Augmented Generation)
- ❌ Tool calling via LLM function APIs
- ❌ Multi-model fallback/routing
- ❌ Model selection heuristics
- ❌ Streaming protocol redesign (use Phase 4.4 interface)
- ❌ UI changes (adapter is transparent to UI)
- ❌ Planner semantics changes
- ❌ Execution engine changes
- ❌ Evidence schema changes

---

## 2. Adapter Architecture

### Design Decision: Trait-Based Abstraction

**Choice**: Trait-based `LlmAdapter` with enum-based provider selection.

**Rationale**:
- Traits allow compile-time polymorphism without enum match sprawl
- Single provider active at a time (no runtime routing)
- Provider selection happens at startup from config
- Easy to add new providers in future

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         UI Layer                             │
│  (AppState, PlanReady, EditingPlan, etc.)                   │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                    LLM Session Layer                        │
│  (propose_plan, propose_plan_streaming, log_plan_*)        │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                   LLM Adapter Trait                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  trait LlmAdapter {                                │   │
│  │      fn generate(&self, prompt: &str)              │   │
│  │          -> Result<String, AdapterError>;          │   │
│  │                                                     │   │
│  │      fn generate_streaming<F>(                     │   │
│  │          &self,                                    │   │
│  │          prompt: &str,                             │   │
│  │          on_chunk: F                               │   │
│  │      ) -> Result<String, AdapterError>             │   │
│  │      where F: FnMut(&str);                         │   │
│  │  }                                                 │   │
│  └─────────────────────────────────────────────────────┘   │
└────────────────────────────┬────────────────────────────────┘
                             │
            ┌────────────────┼────────────────┐
            ▼                ▼                ▼
    ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
    │  GLM Adapter │ │ OpenAI       │ │  Ollama      │
    │              │ │ Adapter      │ │  Adapter     │
    │  HTTP client │ │ HTTP client  │ │  HTTP client │
    │  (ureq)      │ │  (ureq)      │ │  (ureq)      │
    └──────────────┘ └──────────────┘ └──────────────┘
            │                │                │
            └────────────────┼────────────────┘
                             ▼
                    ┌──────────────────┐
                    │  HTTP Transport  │
                    │  (ureq / minreq) │
                    │  Blocking only   │
                    └──────────────────┘
```

### Module Structure

```
src/llm/
├── mod.rs              # (existing, re-exports)
├── types.rs            # (existing, Plan, Intent, etc.)
├── contracts.rs        # (existing, system_prompt, tool_schema)
├── planner.rs          # (existing, parse_plan, validate_plan)
├── router.rs           # (existing, tool whitelist)
├── session.rs          # (existing, propose_plan, logging)
└── adapters/           # NEW in Phase 5
    ├── mod.rs          # Adapter trait, factory, error types (≤300 LOC)
    ├── glm.rs          # GLM adapter implementation (≤300 LOC)
    ├── openai.rs       # OpenAI adapter implementation (≤300 LOC)
    ├── ollama.rs       # Ollama adapter implementation (≤300 LOC)
    └── transport.rs    # HTTP client wrapper (ureq, ≤300 LOC)
```

### Adapter Trait Definition

```rust
/// LLM adapter trait (Phase 5)
///
/// All providers implement this trait.
/// UI layer calls adapters through this uniform interface.
pub trait LlmAdapter: Send + Sync {
    /// Generate completion from prompt (non-streaming)
    ///
    /// Returns full response text.
    fn generate(&self, prompt: &str) -> Result<String, AdapterError>;

    /// Generate completion with streaming callback
    ///
    /// - Calls `on_chunk` for each piece of response
    /// - Returns full response text (concatenated chunks)
    /// - If provider doesn't support streaming, calls once with full response
    fn generate_streaming<F>(
        &self,
        prompt: &str,
        on_chunk: F,
    ) -> Result<String, AdapterError>
    where
        F: FnMut(&str);

    /// Check if adapter supports native streaming
    fn supports_streaming(&self) -> bool;

    /// Get provider name for logging
    fn provider_name(&self) -> &str;
}
```

---

## 3. Provider Breakdown

### 3.1 GLM Adapter

**Provider**: GLM (Zhipu AI)
**API Style**: OpenAI-compatible HTTP
**Base URL Config**: `base_url` in config.toml
**Streaming**: Yes (SSE protocol)
**Authentication**: Bearer token in header

**API Endpoint**:
```
POST {base_url}/chat/completions
```

**Request Format**:
```json
{
  "model": "GLM-4.7",
  "messages": [
    {"role": "system", "content": "You are OdinCode..."},
    {"role": "user", "content": "User Request: ..."}
  ],
  "stream": true
}
```

**Streaming Response** (SSE):
```
data: {"choices":[{"delta":{"content":"..."}}]}

data: [DONE]
```

**GLM-Specific Considerations**:
- OpenAI-compatible but not identical
- SSE format matches OpenAI
- Model name in config (e.g., "GLM-4.7")
- May require custom headers

### 3.2 OpenAI Adapter

**Provider**: OpenAI
**API Style**: OpenAI HTTP API (reference)
**Base URL**: `https://api.openai.com/v1` (or custom via config)
**Streaming**: Yes (SSE protocol)
**Authentication**: Bearer token (API key)

**API Endpoint**:
```
POST https://api.openai.com/v1/chat/completions
```

**Request Format**:
```json
{
  "model": "gpt-4",
  "messages": [
    {"role": "system", "content": "You are OdinCode..."},
    {"role": "user", "content": "..."}
  ],
  "stream": true
}
```

**OpenAI-Specific Considerations**:
- Reference implementation for SSE parsing
- Rate limiting headers may be present
- Standard error response format

### 3.3 Ollama Adapter

**Provider**: Ollama (local)
**API Style**: Custom HTTP API
**Base URL**: `http://{host}:{port}` from config
**Streaming**: Yes (newline-delimited JSON)
**Authentication**: None (local only)

**API Endpoint**:
```
POST {host}:{port}/api/chat
```

**Request Format**:
```json
{
  "model": "codellama",
  "messages": [
    {"role": "system", "content": "You are OdinCode..."},
    {"role": "user", "content": "..."}
  ],
  "stream": true
}
```

**Streaming Response** (NDJSON):
```
{"model":"codellama","created_at":"...","message":{"role":"assistant","content":"..."},"done":false}
{"model":"codellama","created_at":"...","message":{"role":"assistant","content":"..."},"done":false}
{"model":"codellama","created_at":"...","message":{"role":"assistant","content":"..."},"done":true}
```

**Ollama-Specific Considerations**:
- No SSE protocol (NDJSON instead)
- Different response structure
- No authentication
- Local-only (no API key)

---

## 4. Streaming Capability Matrix

| Provider | Native Streaming | Protocol | Adapter Support |
|----------|------------------|----------|-----------------|
| GLM      | Yes              | SSE      | ✅ Native       |
| OpenAI   | Yes              | SSE      | ✅ Native       |
| Ollama   | Yes              | NDJSON   | ✅ Native       |

**All three providers support native streaming.**

### Unified Streaming Interface

```rust
// In session.rs - existing function signature unchanged
pub fn propose_plan_streaming<F>(
    context: &SessionContext,
    evidence_summary: &EvidenceSummary,
    mut on_chunk: F,
) -> Result<Plan, SessionError>
where
    F: FnMut(&str),
{
    // 1. Build prompt (existing)
    let prompt = build_user_prompt(...);

    // 2. Call adapter (NEW in Phase 5)
    let adapter = get_active_adapter()?; // From config
    let response_text = adapter.generate_streaming(&prompt, |chunk| {
        on_chunk(chunk); // Forward to UI
    })?;

    // 3. Parse plan (existing)
    parse_plan(&response_text)
        .map_err(SessionError::from)
}
```

### Fallback Behavior

If streaming fails or provider doesn't support it:
1. Log warning to evidence (artifact_type: "adapter_warning")
2. Fall back to non-streaming `generate()`
3. Return full response in single chunk to callback

---

## 5. Error Normalization Strategy

### Common Error Type

```rust
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
    #[error("Rate limited: retry after {seconds}s")]
    RateLimited { seconds: Option<u32> },

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
}
```

### Error Mapping

| Provider Error Type | Maps To |
|---------------------|---------|
| Connection refused | `AdapterError::Network` |
| Timeout | `AdapterError::Network` |
| 401 Unauthorized | `AdapterError::Authentication` |
| 429 Rate Limit | `AdapterError::RateLimited` |
| 500 Server Error | `AdapterError::Http` |
| Invalid JSON | `AdapterError::InvalidResponse` |
| GLM error code | `AdapterError::Provider` |
| OpenAI error code | `AdapterError::Provider` |
| Ollama error | `AdapterError::Provider` |

### Evidence Logging for Errors

All adapter errors are logged to execution_log.db:
- `artifact_type`: "adapter_error"
- `content_json`: `{ provider, error_type, error_details }`

---

## 6. Configuration Schema

### Existing Format (Preserved)

```toml
[llm]
mode = "external"           # | "local" | "disabled"
provider = "glm"            # glm | openai | ollama
base_url = "https://api.z.ai/api/coding/paas/v4"
api_key = "sk-..."          # or "env:VAR_NAME"
model = "GLM-4.7"
```

### Provider-Specific Examples

#### GLM (External)
```toml
[llm]
mode = "external"
provider = "glm"
base_url = "https://api.z.ai/api/coding/paas/v4"
api_key = "633aceaee3ba4aa795ba39668eb54d3c.3viCQWqyN7gnOB7x"
model = "GLM-4.7"
```

#### OpenAI (External)
```toml
[llm]
mode = "external"
provider = "openai"
base_url = "https://api.openai.com/v1"
api_key = "env:OPENAI_API_KEY"
model = "gpt-4"
```

#### Ollama (Local)
```toml
[llm]
mode = "local"
backend = "ollama"
host = "127.0.0.1"
port = "11434"
model = "codellama"
```

### Config Loading (No Changes)

Existing `preflight.rs` already validates this format.
Adapters read from same config structure.

### Adapter Factory

```rust
/// Create adapter from config
pub fn create_adapter_from_config(
    db_root: &Path,
) -> Result<Box<dyn LlmAdapter>, AdapterError> {
    let config_path = db_root.join("config.toml");
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| AdapterError::Configuration(
            format!("Failed to read config: {}", e)
        ))?;

    // Parse provider from config (basic TOML parsing)
    if content.contains("provider = \"glm\"") {
        GLM_ADAPTER.from_config(&content)
    } else if content.contains("provider = \"openai\"") {
        OpenAI_ADAPTER.from_config(&content)
    } else if content.contains("backend = \"ollama\"") {
        Ollama_ADAPTER.from_config(&content)
    } else {
        Err(AdapterError::Configuration(
            "No valid provider found in config".to_string()
        ))
    }
}
```

---

## 7. Determinism & Evidence Guarantees

### Determinism Preserved

1. **No async**: All HTTP calls are blocking (`ureq` or similar)
2. **Single provider**: No runtime model switching
3. **Opaque LLM output**: Treated as text, logged as evidence
4. **Approval unchanged**: User must still approve plans before execution

### Evidence Logging (Extended)

New artifact types added in Phase 5:

| Artifact Type | Content | Purpose |
|---------------|---------|---------|
| `adapter_call` | `{ provider, model, prompt_length }` | Log each LLM request |
| `adapter_response` | `{ provider, response_length, duration_ms }` | Log each LLM response |
| `adapter_stream_chunk` | `{ chunk_index, chunk_length }` | Log each streaming chunk |
| `adapter_error` | `{ provider, error_type, error_details }` | Log adapter errors |

### Prompt/Response Logging (Full Text)

- **Prompts**: Logged as `llm_prompt` artifact
- **Responses**: Logged as `llm_response` artifact
- **Plan JSON**: Logged as `llm_plan` artifact (existing)

### Audit Trail

Each plan generation creates:
```
execution_id: "llm_plan_{nanoseconds}"
tool_name: "llm_plan"
artifacts:
  - adapter_call      (provider, model, request)
  - adapter_response  (response metadata)
  - llm_prompt        (full prompt text)
  - llm_response      (full raw response)
  - llm_plan          (parsed plan JSON)
```

---

## 8. Test Strategy

### NO Live Network Calls in Tests

All adapter tests use **deterministic mocking**:

1. **HTTP Recording** (preferred):
   - Record actual provider responses once
   - Store as test fixtures (JSON files)
   - Tests replay fixtures instead of calling network

2. **Fake Adapter**:
   - In-memory implementation of `LlmAdapter` trait
   - Returns predetermined responses
   - Used for integration testing

### Test Organization

```
tests/llm_adapter_tests.rs  (NEW)
├── test_adapter_trait_contract      (Trait methods exist)
├── test_glm_request_format          (GLM request structure)
├── test_openai_request_format       (OpenAI request structure)
├── test_ollama_request_format       (Ollama request structure)
├── test_glm_streaming_parse         (GLM SSE parsing)
├── test_openai_streaming_parse      (OpenAI SSE parsing)
├── test_ollama_streaming_parse      (Ollama NDJSON parsing)
├── test_error_normalization         (All error types map correctly)
├── test_fallback_to_non_streaming   (Degrades gracefully)
├── test_config_parsing              (Config → adapter selection)
└── test_evidence_logging            (Adapter calls logged)
```

### Fixture Example

```tests/fixtures/glm_response_chat.json```
```json
{
  "id": "chat-123",
  "choices": [{
    "message": {
      "role": "assistant",
      "content": "{\n  \"plan_id\": \"plan_test\",\n  \"intent\": \"READ\",\n  \"steps\": [],\n  \"evidence_referenced\": []\n}\n"
    }
  }],
  "model": "GLM-4.7"
}
```

### Test Without Network

```rust
#[test]
fn test_glm_adapter_parse_response() {
    // Load fixture
    let fixture = std::fs::read_to_string("tests/fixtures/glm_response_chat.json")
        .unwrap();

    // Parse using adapter internals
    let response: GLMResponse = serde_json::from_str(&fixture).unwrap();

    // Verify structure
    assert_eq!(response.choices.len(), 1);
    assert!(response.choices[0].message.content.contains("plan_id"));
}
```

---

## 9. HTTP Transport Decision

### Library Choice: `minreq` or `ureq`

Both are:
- **Synchronous** (no async)
- **Minimal dependencies**
- **Blocking I/O**

**Recommendation**: `minreq`
- Smaller dependency footprint
- Feature-gated HTTPS (optional)
- Simple API

**Fallback**: `ureq`
- More mature
- Better HTTPS support
- Slightly larger

### Streaming Implementation

Since libraries don't support SSE/NDJSON natively:

```rust
/// Parse SSE stream from response body
fn parse_sse_stream(body: &str) -> Vec<String> {
    body.lines()
        .skip_while(|line| !line.starts_with("data: "))
        .map(|line| line.strip_prefix("data: ").unwrap_or(""))
        .take_while(|data| *data != "[DONE]")
        .map(|data| extract_content_from_sse(data))
        .collect()
}

/// Parse NDJSON stream from Ollama
fn parse_ndjson_stream(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|line| serde_json::from_str::<OllamaChunk>(line).ok())
        .map(|chunk| chunk.message.content)
        .collect()
}
```

---

## 10. Explicit Deferred Work

### What Phase 5 Does NOT Implement

| Feature | Status | Rationale |
|---------|--------|-----------|
| Embeddings API | ❌ Deferred | Out of scope for planning |
| Tool calling (function API) | ❌ Deferred | OdinCode uses its own tool schema |
| Multi-model routing | ❌ Deferred | Single provider only |
| Model selection heuristics | ❌ Deferred | User specifies model in config |
| Retry logic | ❌ Deferred | Fail fast, user retries |
| Caching responses | ❌ Deferred | Determinism > performance |
| Async/await | ❌ Deferred | Constraint: NO async |
| WebSocket support | ❌ Deferred | HTTP only |
| Vision/multimodal | ❌ Deferred | Text-only prompts |

### Future Phases (Not Authorized)

- **Phase 6**: Embeddings & RAG (if authorized)
- **Phase 7**: Multi-model orchestration (if authorized)
- **Phase 8**: Tool calling via LLM function APIs (if authorized)

---

## 11. Dependencies

### New Dependencies (Phase 5)

```toml
[dependencies]
# Existing
thiserror = "2.0"
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }

# NEW in Phase 5
minreq = { version = "2.12", features = ["https"], optional = true }
# OR
ureq = { version = "2.10", features = ["json"], optional = true }
```

### No Breaking Changes

- Existing modules unchanged
- Config format unchanged
- UI behavior unchanged
- Evidence schema extended (not modified)

---

## 12. Implementation Checklist (Future Authorization)

**NOTE**: This is PLANNING ONLY. Implementation requires separate authorization.

### Module: `src/llm/adapters/mod.rs` (≤300 LOC)
- [ ] Define `LlmAdapter` trait
- [ ] Define `AdapterError` enum
- [ ] Create `create_adapter_from_config()` factory
- [ ] Export public types

### Module: `src/llm/adapters/transport.rs` (≤300 LOC)
- [ ] HTTP client wrapper (minreq/ureq)
- [ ] SSE parsing utility
- [ ] NDJSON parsing utility
- [ ] Error handling

### Module: `src/llm/adapters/glm.rs` (≤300 LOC)
- [ ] GLM adapter struct
- [ ] `generate()` implementation
- [ ] `generate_streaming()` implementation
- [ ] Request builder
- [ ] Response parser

### Module: `src/llm/adapters/openai.rs` (≤300 LOC)
- [ ] OpenAI adapter struct
- [ ] `generate()` implementation
- [ ] `generate_streaming()` implementation
- [ ] Request builder
- [ ] Response parser

### Module: `src/llm/adapters/ollama.rs` (≤300 LOC)
- [ ] Ollama adapter struct
- [ ] `generate()` implementation
- [ ] `generate_streaming()` implementation
- [ ] Request builder (NDJSON)
- [ ] Response parser (NDJSON)

### Module: `src/llm/session.rs` (MODIFY)
- [ ] Wire adapter to `propose_plan()`
- [ ] Wire adapter to `propose_plan_streaming()`
- [ ] Log adapter calls to evidence

### Tests: `tests/llm_adapter_tests.rs` (NEW)
- [ ] Trait contract tests
- [ ] Request format tests (x3 providers)
- [ ] Streaming parse tests (x3 providers)
- [ ] Error normalization tests
- [ ] Fallback behavior tests
- [ ] Config parsing tests
- [ ] Evidence logging tests

### Fixtures: `tests/fixtures/` (NEW)
- [ ] `glm_response_chat.json`
- [ ] `glm_response_stream.txt`
- [ ] `openai_response_chat.json`
- [ ] `openai_response_stream.txt`
- [ ] `ollama_response_chat.json`
- [ ] `ollama_response_stream.txt`

---

## SIGN-OFF

**Planning**: COMPLETE
**Authorization Required**: Implementation (separate step)
**Precondition Met**: Phases 4.4 and 4.5 complete (257/257 tests passing)

---

STOP — Phase 5 Model Adapters planning complete; awaiting acceptance or revisions.
