# Fix Plan: Chat Loop "Stuck" After Tool Invocation

**Date**: 2025-12-26
**Status**: Planning
**Priority**: HIGH

## Problem Statement

When using OdinCode to chat with Claude:
1. Tool is invoked successfully (e.g., `file_glob`)
2. Tool result is received and displayed
3. System appears "stuck" - no further LLM response

## Investigation Summary

**Current State of Timeout Implementation**:
- ✅ `UreqTransport` HAS a 30-second timeout (line 88)
- ✅ Timeout is applied via `.timeout(Duration::from_secs(self.timeout))`
- ✅ Error events ARE sent when `chat_with_messages()` fails
- ✅ Event handler DOES process `ChatEvent::Error` and returns `LoopAction::LoopError`

**Issues Identified**:

1. **Fallback path has NO timeout** (`src/llm/chat.rs:307-313`)
   ```rust
   let client = ureq::Agent::new();  // ← No timeout configured!
   let response = client.post(&url)...
   ```

2. **Channel send failures are SILENTLY IGNORED**
   ```rust
   let _ = tx.send(ChatEvent::Error { ... });  // ← _ ignores failures
   ```

3. **Session ID mismatch silently drops events** (`src/llm/chat_loop/event_handler.rs:34-36`)
   ```rust
   if event.session_id() != state.session_id {
       return LoopAction::None;  // ← Event dropped silently
   }
   ```

4. **No diagnostic logging** - Cannot determine WHERE execution stops

5. **No watchdog mechanism** - No way to detect if thread is truly hung vs just slow LLM

6. **Error conversion chain** - Multiple layers of error conversion may lose context:
   - `ureq::Error` → `AdapterError` → `ChatError` → `ChatEvent::Error`

## Root Cause Analysis (Updated)

The timeout EXISTS (30 seconds), but there are several failure modes:

1. **Timeout might be too short** for complex LLM queries
2. **Fallback path has no timeout** - if streaming returns empty, we fall back to non-streaming without timeout
3. **Error events might be dropped** due to:
   - Session ID mismatch
   - Channel send failure (silently ignored)
   - Event processing race conditions

4. **No visibility** - Without logging, we cannot determine which of the above is occurring

## Fix Plan

### Phase 1: Diagnostic Logging (Immediate)

**Goal**: Add visibility into chat thread lifecycle

**Changes**:

1. **Add logging to `spawn_chat_thread_with_frame_stack`** (`src/llm/chat_thread.rs:192`)
   ```rust
   log::info!("Spawning chat thread with session_id={}, frames={}",
              session_id, messages.len());
   ```

2. **Add logging inside thread body** (`src/llm/chat_thread.rs:227-256`)
   ```rust
   log::info!("Chat thread {}: calling chat_with_messages", session_id);
   let result = chat::chat_with_messages(&messages, &db_root, |chunk| {
       // ...
   });
   log::info!("Chat thread {}: result={:?}", session_id, result.is_ok());
   ```

3. **Add logging to HTTP transport** (`src/llm/adapters/transport_ureq.rs:87`)
   ```rust
   log::debug!("HTTP POST {} (timeout={}s)", url, self.timeout);
   let response = request.send_string(body)?;
   log::debug!("HTTP response: status={}", response.status());
   ```

4. **Add logging to `execute_tool_and_continue`** (`src/llm/chat_loop/event_handler.rs:188`)
   ```rust
   log::info!("Executing tool '{}' and spawning next LLM call", tool);
   ```

5. **Add logging to event processing** (`src/llm/chat_loop/event_handler.rs:21`)
   ```rust
   log::debug!("Processing event: {:?}", std::mem::discriminant(event));
   ```

**Dependencies**: None (can start immediately)
**Tests**: Add tests to verify logging doesn't break functionality

### Phase 2: Fix Fallback Timeout (High Priority)

**Goal**: Ensure ALL HTTP requests have timeout

**Changes**:

1. **Fix fallback in `chat_with_messages_openai`** (`src/llm/chat.rs:307`)
   ```rust
   // BEFORE (no timeout):
   let client = ureq::Agent::new();

   // AFTER:
   let client = ureq::Agent::new()
       .set_timeout(std::time::Duration::from_secs(30));
   ```

2. **Fix fallback in `chat_with_messages_glm`** (`src/llm/chat.rs:383`)
   ```rust
   let client = ureq::Agent::new()
       .set_timeout(std::time::Duration::from_secs(30));
   ```

**Dependencies**: None
**Tests**: Add unit test for timeout behavior

### Phase 3: Configurable Timeout (Medium Priority)

**Goal**: Allow users to configure timeout based on their LLM

**Changes**:

1. **Add timeout to LLM config** (`src/llm/adapters/factory.rs`)
   ```rust
   pub struct LlmConfig {
       // existing fields...
       pub timeout_secs: Option<u64>,
   }
   ```

2. **Pass timeout through adapter creation**
   - `create_adapter_from_config()` reads timeout
   - `UreqTransport::with_timeout()` uses configured value

3. **Add config file support**
   ```toml
   [llm]
   timeout_secs = 60  # Optional, defaults to 30
   ```

**Dependencies**: None
**Tests**: Add tests for various timeout values

### Phase 4: Channel Error Detection (Medium Priority)

**Goal**: Detect when channel send fails

**Changes**:

1. **Check channel send results** in thread:
   ```rust
   if let Err(e) = tx.send(ChatEvent::Error { ... }) {
       log::error!("Failed to send error event: {}", e);
       // Channel disconnected - cannot notify main thread
       // Thread will exit, but UI won't know why
   }
   ```

2. **Add channel health check** to `ChatThreadHandle`:
   ```rust
   pub fn is_channel_connected(&self) -> bool {
       // Try to send a ping, check if it succeeds
   }
   ```

**Dependencies**: None
**Tests**: Test channel disconnect scenarios

### Phase 5: Session ID Validation (Low Priority)

**Goal**: Detect session ID mismatches early

**Changes**:

1. **Log when events are dropped due to session mismatch**:
   ```rust
   if event.session_id() != state.session_id {
       log::warn!("Dropping event due to session mismatch: expected={}, got={}",
                  state.session_id, event.session_id());
       return LoopAction::None;
   }
   ```

2. **Add session ID generation verification**:
   ```rust
   #[test]
   fn test_session_id_format() {
       let id = generate_session_id();
       assert!(id.starts_with("chat-"));
   }
   ```

**Dependencies**: None
**Tests**: Test session mismatch scenarios

### Phase 6: Thread Watchdog (Future Enhancement)

**Goal**: Detect truly hung threads

**Changes**:

1. **Add thread heartbeat** mechanism:
   - Thread sends `Heartbeat` event every N seconds
   - Main thread tracks last heartbeat time
   - If no heartbeat for M seconds, terminate thread

2. **Add optional timeout wrapper** around `chat_with_messages`:
   ```rust
   pub fn chat_with_messages_timeout(
       messages: &[LlmMessage],
       db_root: &Path,
       timeout: Duration,
       on_chunk: F,
   ) -> Result<String, ChatError>
   ```

**Dependencies**: Requires async/await or thread join timeout
**Tests**: Test timeout cancellation

## Implementation Order

| Phase | Priority | Est. Effort | Dependencies |
|-------|----------|-------------|--------------|
| 1: Diagnostic Logging | HIGH | 2 hours | None |
| 2: Fix Fallback Timeout | HIGH | 1 hour | None |
| 3: Configurable Timeout | MEDIUM | 3 hours | None |
| 4: Channel Error Detection | MEDIUM | 2 hours | None |
| 5: Session ID Validation | LOW | 1 hour | None |
| 6: Thread Watchdog | LOW | 4 hours | Async infrastructure |

## Testing Strategy

### Unit Tests
1. Test timeout configuration
2. Test error event propagation
3. Test session ID matching
4. Test channel send failures

### Integration Tests
1. Test slow LLM response (simulate delay)
2. Test LLM timeout (simulate timeout)
3. Test tool execution → next LLM call flow
4. Test channel disconnect during active chat

### Manual Testing
1. Run with verbose logging enabled
2. Monitor logs for:
   - Thread spawn/completion
   - HTTP request/response
   - Event processing
   - Session ID consistency

## Success Criteria

1. ✅ All HTTP requests have configured timeout
2. ✅ Channel send failures are logged
3. ✅ Session mismatches are logged
4. ✅ Thread lifecycle events are logged
5. ✅ Configurable timeout via config file
6. ✅ All 438 existing tests still pass
7. ✅ New tests for timeout scenarios pass

## Rollback Plan

If issues arise:
1. Each phase can be reverted independently
2. Logging additions are non-breaking (just add info)
3. Timeout changes are backward compatible (default 30s)
4. No breaking changes to public APIs

## Implementation Status (Updated 2025-12-26)

| Phase | Status | Date Completed |
|-------|--------|----------------|
| 1: Diagnostic Logging | ✅ COMPLETE | 2025-12-26 |
| 2: Fix Fallback Timeout | ✅ COMPLETE | 2025-12-26 |
| 3: Configurable Timeout | ⏳ PENDING | - |
| 4: Channel Error Detection | ⏳ PENDING | - |
| 5: Session ID Validation | ✅ COMPLETE (partial) | 2025-12-26 |
| 6: Thread Watchdog | ⏳ PENDING | - |

## Changes Made (2025-12-26)

### Phase 1: Diagnostic Logging ✅

Added `eprintln!` logging with prefixed tags:

1. **`src/llm/chat_thread.rs`** (lines 218-282)
   - `[CHAT_THREAD]` prefixed logs for thread lifecycle
   - Logs when thread spawns, starts, calls LLM, succeeds/fails, exits
   - Includes session_id, frame count, message count, response length

2. **`src/llm/adapters/transport_ureq.rs`** (lines 87-96)
   - `[HTTP]` prefixed logs for HTTP requests
   - Logs URL, timeout, body length on request
   - Logs response status on completion

3. **`src/llm/chat_loop/event_handler.rs`** (lines 35-40, 201-211)
   - `[CHAT_LOOP]` prefixed logs for event processing
   - Logs session mismatches (was silently dropping)
   - Logs tool execution and results

### Phase 2: Fix Fallback Timeout ✅

Fixed critical bug in `src/llm/chat.rs` (lines 305-315, 383-393):

**BEFORE** (no timeout):
```rust
let client = ureq::Agent::new();
let response = client.post(&url)...
```

**AFTER** (30-second timeout):
```rust
let response = ureq::request("POST", &url)
    .timeout(std::time::Duration::from_secs(30))
    .set("Authorization", &auth_header)...
```

### Test Results

- ✅ All 970 tests passing
- ✅ Binary builds successfully (5.0 MB)
- ✅ 7 warnings (pre-existing, not related to changes)
- ✅ No breaking changes

## Logging Format

When the chat loop runs, you'll now see output like:

```
[CHAT_THREAD] Spawning: session_id=chat-123abc, frames=3, messages=3
[CHAT_THREAD] chat-123abc: started
[CHAT_THREAD] chat-123abc: calling chat_with_messages with 3 messages
[HTTP] POST https://api.openai.com/v1/chat/completions (timeout=30s, body_len=1234)
[HTTP] response: status=200
[CHAT_THREAD] chat-123abc: SUCCESS, response length=1234 chars
[CHAT_THREAD] chat-123abc: exiting
```

## Next Steps

1. ✅ Review and approve this plan
2. ✅ Implement Phase 1 (Diagnostic Logging) - COMPLETE
3. ✅ Implement Phase 2 (Fix Fallback Timeout) - COMPLETE
4. ⏳ Test with real LLM to verify logging works
5. ⏳ Based on logs, implement remaining phases (3, 4, 6)

## Files to Modify (Updated)

| File | Changes | LOC Impact | Status |
|------|---------|------------|--------|
| `src/llm/chat_thread.rs` | Add logging | +15 | ✅ Done |
| `src/llm/adapters/transport_ureq.rs` | Add logging | +5 | ✅ Done |
| `src/llm/chat.rs` | Fix fallback timeout | +2 | ✅ Done |
| `src/llm/chat_loop/event_handler.rs` | Add logging | +10 | ✅ Done |
| `src/llm/adapters/factory.rs` | Add timeout config | +15 | ⏳ Pending |
| `src/llm/adapters/mod.rs` | Update types | +5 | ⏳ Pending |
| `tests/chat_timeout_tests.rs` | New tests | +100 | ⏳ Pending |

**Completed**: ~42 LOC added
**Remaining**: ~120 LOC for phases 3-6

---

**Status**: Phase 1-2 COMPLETE, remaining phases pending
**Last Updated**: 2025-12-26
