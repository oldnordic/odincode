# Phase 9.8: GLM OpenCode Parity

## Overview

This document describes the changes made to OdinCode to achieve GLM adapter parity with OpenCode's OpenAI-compatible behavior.

**Key Changes:**
1. Added `src/llm/adapters/normalize.rs` - Message normalization and streaming event tracking
2. Updated GLM adapter with message array support and accessors
3. Updated OpenAI adapter with `model()` accessor
4. Updated `chat_with_messages_glm()` to use proper message arrays

## Before vs After

### Before: Single-User Message Collapse

**Problem:** GLM chat with tool loop was sending entire conversation as a single "user" message, causing LLM amnesia.

```json
// OLD (WRONG) - All conversation in one user message
{
  "model": "GLM-4.7",
  "messages": [
    {"role": "system", "content": "You can use tools..."},
    {"role": "user", "content": "read file.txt\nI'll read that file.\n[Tool file_read]: OK\nResult: content\nwhat did I just read?"}
  ],
  "stream": true
}
```

### After: Proper Multi-Turn Message Array

**Solution:** Each conversation turn is a separate message with correct role.

```json
// NEW (CORRECT) - Multi-turn message array
{
  "model": "GLM-4.7",
  "messages": [
    {"role": "system", "content": "You can use tools..."},
    {"role": "user", "content": "read file.txt"},
    {"role": "assistant", "content": "I'll read that file."},
    {"role": "user", "content": "[Tool file_read]: OK\nResult: content"},
    {"role": "user", "content": "what did I just read?"}
  ],
  "stream": true
}
```

## Implementation Details

### 1. Message Normalization Module

**File:** `src/llm/adapters/normalize.rs` (286 LOC)

**Purpose:** Normalizes messages for OpenAI-compatible providers and tracks streaming state.

**Key Types:**
```rust
pub struct NormalizedRequest {
    pub body: String,          // JSON request body
    pub message_count: usize,   // For debugging
    pub role_summary: String,   // e.g., "SUAU" = System+User+Assistant+User
}

pub enum StreamingEvent {
    TextDelta(String),
    ToolCallStart { id: String, name: String },
    ToolCallDelta { id: String, args_delta: String },
    ToolCallComplete { id: String, args: String },
    Finish { reason: Option<String> },
    Usage { prompt_tokens: Option<u64>, completion_tokens: Option<u64> },
}

pub struct StreamingState {
    pub assistant_text: String,
    pub tool_calls: Vec<ToolCallState>,
    pub finish_reason: Option<String>,
    pub usage: Option<UsageData>,
}
```

**Functions:**
- `normalize_for_openai_compatible()` - Converts `Vec<LlmMessage>` to JSON with reasoning stripping
- `strip_reasoning_content()` - Removes `<reasoning>`, `[REASONING]`, `Thinking:` prefixes
- `parse_sse_chunk()` - Parses SSE chunks and emits events
- `get_final_tool_calls()` - Extracts completed tool calls from state

### 2. GLM Adapter Updates

**File:** `src/llm/adapters/glm.rs`

**Added Methods:**
```rust
impl GlmAdapter {
    pub fn base_url(&self) -> &str
    pub fn api_key(&self) -> &str
    pub fn transport(&self) -> &Transport
    pub fn build_chat_stream_messages_request(&self, messages: &[LlmMessage]) -> Result<String, AdapterError>
}
```

**GLM Base URL:**
```
https://api.z.ai/api/coding/paas/v4
```

**GLM Endpoint:**
```
https://api.z.ai/api/coding/paas/v4/chat/completions
```

### 3. Chat Path Updates

**File:** `src/llm/chat.rs`

**Function:** `chat_with_messages_glm()`

**Before (incorrect):** Collapsed all messages into single prompt string
```rust
let prompt = messages.iter()
    .filter(|m| matches!(m.role, LlmRole::User | LlmRole::Assistant))
    .map(|m| m.content.clone())
    .join("\n");
adapter.generate_chat_streaming(&prompt, ...)
```

**After (correct):** Uses proper message array via adapter
```rust
let body = adapter.build_chat_stream_messages_request(messages)?;
let url = format!("{}/chat/completions", adapter.base_url().trim_end_matches('/'));
let transport = adapter.transport();
transport.post_stream(&url, &headers, &body, ...)
```

## Reasoning Content Handling

OpenCode strips reasoning chunks from assistant text before displaying to users.

**Patterns Stripped:**
- `<reasoning>...</reasoning>` tags
- `[REASONING]...[/REASONING]` blocks
- Lines starting with `Thinking:`, `Thought:`, `Reasoning:`

**Example:**
```rust
let input = "Response\n<reasoning>This is hidden</reasoning>\nMore text";
let output = strip_reasoning_content(input);
// output: "Response\nMore text" (reasoning removed)
```

## Streaming Event Flow

```
SSE Chunk → parse_sse_chunk() → StreamingEvent
    ├─ Content delta → TextDelta → on_chunk()
    ├─ Tool call id → ToolCallStart (update state)
    ├─ Tool call name → ToolCallStart (update state)
    ├─ Tool call args → ToolCallDelta (update state)
    ├─ Usage data → Usage event
    └─ [DONE] → Finish event
```

## Test Evidence

### All Tests Passing

```bash
$ cargo test --all --all-features
...
test result: ok. 223 passed; 0 failed; 0 ignored; 0 measured
```

### Clippy Clean

```bash
$ cargo clippy --all-targets --all-features -- -D warnings
Finished `dev` profile in 1.25s
```

### Key Test Cases

1. **Message normalization** (`test_normalize_multi_turn`)
   - Input: 4 messages (System, User, Assistant, User with tool result)
   - Output: role_summary = "SUAU" (correct multi-turn structure)

2. **Reasoning stripping** (`test_strip_reasoning_content_tags`)
   - Input: "Hello\n<reasoning>hidden</reasoning>\nWorld"
   - Output: "Hello\nWorld" (reasoning removed)

3. **SSE content parsing** (`test_parse_sse_content_delta`)
   - Input: SSE chunk with content delta
   - Output: TextDelta event + assistant_text updated

4. **SSE usage parsing** (`test_parse_sse_usage`)
   - Input: SSE chunk with usage data
   - Output: Usage event + state.usage populated

5. **Regression test** (`test_regression_multi_turn_message_count`)
   - Ensures >2 messages after tool loop step (prevents collapse to single user message)

## Files Modified

| File | Change |
|------|--------|
| `src/llm/adapters/normalize.rs` | NEW - Message normalization, streaming state |
| `src/llm/adapters/glm.rs` | Added accessors and `build_chat_stream_messages_request()` |
| `src/llm/adapters/openai.rs` | Added `model()` accessor |
| `src/llm/adapters/mod.rs` | Export `normalize` module |
| `src/llm/chat.rs` | Updated `chat_with_messages_glm()` to use message arrays |
| `tests/phase_9_8_adapter_messages_tests.rs` | Fixed clippy warnings |

## Validation Commands

```bash
# Run all tests
cargo test --all --all-features

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Build release binary
cargo build --release

# Run specific normalize tests
cargo test --lib llm::adapters::normalize
```

## GLM Configuration Example

```toml
[llm]
mode = "external"
provider = "glm"
base_url = "https://api.z.ai/api/coding/paas/v4"
api_key = "env:GLM_API_KEY"
model = "GLM-4.7"
```

## OpenCode Parity Checklist

- ✅ Base URL: `https://api.z.ai/api/coding/paas/v4`
- ✅ Endpoint: `/chat/completions`
- ✅ Message arrays (role-per-message, not collapsed)
- ✅ Reasoning content stripping (normalize module)
- ✅ Streaming state tracking (StreamingState)
- ✅ Usage data capture (prompt/completion/total tokens)
- ✅ Tool call event tracking (ToolCallStart/Delta/Complete)
- ✅ Multi-turn conversation support (>2 messages in loop)

## Future Work

1. **Debug logging toggle:** Add optional file-based request/response logging for troubleshooting
2. **Tool call integration:** Connect StreamingEvent tool calls to actual tool execution
3. **GLM-specific tool format:** If GLM uses non-standard tool call format, add adapter
4. **Rate limiting:** Add per-model rate limiting for GLM API
5. **Retry logic:** Add exponential backoff for failed requests

## References

- OpenCode: GLM via OpenAI-compatible API at `https://api.z.ai/api/coding/paas/v4`
- OpenAI Chat Completions API: https://platform.openai.com/docs/api-reference/chat/create
- Phase 9.7: FrameStack for LLM context continuity
- Phase 9.8: Multi-turn message support
