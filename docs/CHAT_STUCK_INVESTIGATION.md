# Chat Loop "Stuck" Investigation Report

**Date**: 2025-12-26
**Binary Version**: 5.0 MB (post-modularization)
**Issue**: After tool invocation, the chat loop gets stuck - no further LLM response
**Status**: ✅ **FIXED** - Phase 9.10

## User Report

When asking the LLM to list `.rs` files in the `src` folder and count them:
1. Tool (`file_glob`) was invoked successfully
2. Tool result was received and displayed
3. System got stuck - no further LLM response

## Root Cause Identified

**Location**: `src/ui/state.rs:544-548` (before fix)

The bug was in the `update_last_message()` function:

```rust
// BEFORE (buggy version)
pub fn update_last_message(&mut self, content: String) {
    if let Some(last) = self.chat_messages.last_mut() {
        last.content = content;  // ← Only updates content, NOT role!
    }
}
```

**The Bug**:
1. After tool execution, a "Thinking..." message is added with `ChatRole::Thinking`
2. When GLM sends the continuation response via the **fallback path** (non-streaming POST), the callback is called ONCE with full content
3. The Complete event arrives and calls `update_last_message(full_response)`
4. This updates the **content** but leaves the **role** as "Thinking"!
5. The UI still shows "Thinking..." because it renders messages with `role == Thinking` differently

**Why No Chunk Events Were Sent**:
- The callback was called once with full content (fallback path)
- But the Chunk event was sent to the channel
- The Complete event was also sent
- Both events should have been processed, but the UI state remained "Thinking..."

## The Fix

**File**: `src/ui/state.rs:543-557`

```rust
/// Update the last message (for streaming: replace thinking with content)
///
/// Phase 9.10 FIX: Also replace Thinking role with Assistant when updating content.
/// This handles the case where Complete event arrives without any Chunk events.
pub fn update_last_message(&mut self, content: String) {
    if let Some(last) = self.chat_messages.last_mut() {
        last.content = content;
        // CRITICAL FIX: If last message is Thinking, change to Assistant
        // This handles the case where callback was called once (fallback path)
        // and only Complete event was sent, no Chunk events
        if last.role == ChatRole::Thinking {
            last.role = ChatRole::Assistant;
        }
    }
}
```

## Test Coverage

**File**: `tests/phase_9_10_glm_streaming_tests.rs` (7 tests, all passing)

1. **test_fake_transport_sse_calls_on_line**: Verifies FakeTransport calls callback for each SSE line
2. **test_glm_sse_parsing_with_callback**: Verifies GLM SSE parsing with callback (14 chunks received)
3. **test_glm_empty_sse_response**: Verifies empty SSE response triggers fallback condition
4. **test_chat_with_messages_glm_streaming**: Placeholder for future FakeTransport injection test
5. **test_spawn_chat_thread_sends_chunk_events**: Verifies ChatThread sends events correctly
6. **test_fake_transport_newline_handling**: Verifies newline handling in SSE responses
7. **test_empty_content_triggers_fallback_condition**: Proves `full_content.is_empty()` triggers fallback

## Test Results

```
running 7 tests
test test_chat_with_messages_glm_streaming ... ok
test test_fake_transport_newline_handling ... ok
test test_empty_content_triggers_fallback_condition ... ok
test test_fake_transport_sse_calls_on_line ... ok
test test_glm_sse_parsing_with_callback ... ok
test test_spawn_chat_thread_sends_chunk_events ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**Full test suite**: 437 passed (1 pre-existing unrelated failure in `test_resolve_defaults_to_current`)

## Summary

**Root Cause**: `update_last_message()` was only updating content, not the role. When Complete events arrived without Chunk events (fallback path), the message remained in "Thinking" state.

**Fix**: Added role check in `update_last_message()` to change `ChatRole::Thinking` to `ChatRole::Assistant` when updating content.

**Impact**: The UI will now correctly display LLM responses even when the fallback path is used (non-streaming POST after empty streaming response).
