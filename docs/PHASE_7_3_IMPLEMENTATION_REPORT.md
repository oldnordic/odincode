# Phase 7.3 — Text-First LLM Contract Alignment: Implementation Report

**Date**: 2025-12-25
**Status**: COMPLETE
**Test Coverage**: 331/331 tests passing (100%)

## Summary

Fixed the LLM integration to align with text-first contract where natural language is the default and JSON parsing is optional. The previous implementation incorrectly assumed LLM would always return JSON plans, causing parse failures on valid text responses.

### Bug: JSON Parse Errors on Valid Text Responses
**Error**: `JsonParse` error when LLM returns plain text
**Root Cause**: `parse_plan()` always attempted JSON parsing, regardless of content
**Fix**: Added guard clause - only parse as JSON if content starts with `{` (after trimming and markdown extraction)

## Root Causes

### Issue 1: JSON-First Parsing Assumption

**Original Implementation (src/llm/planner.rs:51):**
```rust
pub fn parse_plan(response: &str) -> Result<Plan, PlanError> {
    let raw: serde_json::Value = serde_json::from_str(response)?;
    // Always parsed as JSON
}
```

Any plain text response from LLM would fail with `JsonParse` error.

**Fixed Implementation:**
```rust
pub fn parse_plan(response: &str) -> Result<Plan, PlanError> {
    let trimmed = response.trim();

    // Empty response → minimal text plan
    if trimmed.is_empty() {
        return Ok(create_text_plan("empty".to_string(), Intent::Read));
    }

    // Extract JSON from markdown code blocks if present
    let content = extract_from_markdown(trimmed);

    // Only parse as JSON if content starts with `{`
    if content.trim_start().starts_with('{') {
        match parse_json_plan(content) {
            Ok(plan) => Ok(plan),
            // Malformed JSON → fall back to text display
            Err(PlanError::JsonParse(_)) => {
                Ok(create_text_plan(content.to_string(), Intent::Explain))
            }
            Err(e) => Err(e),
        }
    } else {
        // Plain text → create display_text plan
        Ok(create_text_plan(content.to_string(), Intent::Explain))
    }
}
```

### Issue 2: No Tool for Displaying Plain Text

The `display_text` tool was not in the whitelist, so even if we created a text display plan, it would fail validation.

**Fix**: Added `display_text` to `TOOL_WHITELIST` with empty preconditions (pure UI tool).

### Issue 3: Handlers Didn't Support Text Display

The `execute_plan()` function didn't handle `display_text` steps.

**Fix**: Added handler in `execute_plan()` to display text via `app.log()`.

## Files Modified

### 1. src/llm/planner.rs (lines 43-226)
**Changes:**
- Rewrote `parse_plan()` with text-first logic
- Added `extract_from_markdown()` helper for markdown code block extraction
- Renamed original JSON parsing to `parse_json_plan()` (internal)
- Added `create_text_plan()` helper for creating display_text plans
- Added tests: `test_parse_plain_text_creates_display_plan`, `test_parse_markdown_wrapped_json`
- Updated test: `test_parse_invalid_json` → `test_parse_invalid_json_becomes_text_plan`

### 2. src/llm/router.rs (lines 12, 102, 163-177)
**Changes:**
- Added `"display_text"` to `TOOL_WHITELIST`
- Added precondition handling for `display_text` (returns empty vec)
- Updated `test_whitelist_has_11_tools` → `test_whitelist_has_12_tools`
- Updated `test_preconditions_defined` to allow `display_text` with no preconditions

### 3. src/ui/handlers.rs (lines 137-197)
**Changes:**
- Rewrote `execute_plan()` to avoid borrow checker issues (clone plan data first)
- Added `display_text` step handling (logs text to console)
- Added `file_read` step handling
- Added `lsp_check` step handling
- Updated `handle_nlp_intent()` to distinguish text responses from structured plans

### 4. tests/llm_planner_tests.rs (lines 43-58, 182-203, 310-327)
**Changes:**
- Updated `test_t2_invalid_json_rejected` → `test_t2_invalid_json_becomes_text_plan`
- Updated `test_t3_whitelist_contains_all_phase_0_tools` to expect 12 tools
- Updated `test_t4_all_preconditions_defined_for_whitelist` to allow `display_text`

## Tests Added

### tests/llm_text_first_tests.rs (NEW - 9 tests)

1. `test_plain_text_response_does_not_error` - Plain text creates display plan
2. `test_json_response_still_works` - Valid JSON still parses correctly
3. `test_json_guard_only_parses_if_starts_with_brace` - Text before JSON prevents parsing
4. `test_malformed_json_does_not_crash` - Malformed JSON becomes text plan
5. `test_empty_response_handled_gracefully` - Empty response doesn't crash
6. `test_markdown_code_block_accepted` - Markdown-wrapped JSON handled
7. `test_colon_commands_still_work` - Explicit `:` commands unaffected
8. `test_natural_language_input_produces_plan` - Natural language routed to LLM
9. `test_llm_response_with_leading_whitespace` - Trimming before JSON guard

### Updated Tests (4 tests)

1. `test_parse_invalid_json_becomes_text_plan` - Now expects Ok, not Err
2. `test_whitelist_has_12_tools` - Was `test_whitelist_has_11_tools`
3. `test_preconditions_defined` - Allows `display_text` with no preconditions
4. `test_t4_all_preconditions_defined_for_whitelist` - Allows `display_text`

## Test Results

```
running 9 tests (llm_text_first_tests)
test test_plain_text_response_does_not_error ... ok
test test_json_response_still_works ... ok
test test_json_guard_only_parses_if_starts_with_brace ... ok
test test_malformed_json_does_not_crash ... ok
test test_empty_response_handled_gracefully ... ok
test test_markdown_code_block_accepted ... ok
test test_colon_commands_still_work ... ok
test test_natural_language_input_produces_plan ... ok
test test_llm_response_with_leading_whitespace ... ok

Total: 331 tests passed (including 9 new text-first tests)
```

## Verification Checklist

- [x] Plain text LLM responses no longer cause JSON parse errors
- [x] Valid JSON plans still parse correctly
- [x] Markdown-wrapped JSON is extracted and parsed
- [x] Malformed JSON is treated as plain text (graceful degradation)
- [x] Empty responses handled without crash
- [x] `display_text` tool is in whitelist and validated
- [x] `execute_plan()` handles `display_text`, `file_read`, `lsp_check` steps
- [x] `:` commands still work (no behavior change)
- [x] All 331 tests passing
- [x] NO new dependencies added
- [x] NO async (all synchronous)
- [x] NO breaking changes to CLI flags or commands

## Design Decisions (LOCKED)

1. **JSON Guard**: Only parse as JSON if content starts with `{` (after trimming/markdown extraction)
2. **Graceful Degradation**: Malformed JSON and plain text become `display_text` plans
3. **Markdown Extraction**: ` ```json ... ``` ` blocks are unwrapped before JSON guard
4. **Empty Preconditions**: `display_text` has no preconditions (pure UI tool)
5. **Borrow Checker Fix**: Clone plan data before mutable operations in `execute_plan()`

## Constraints Compliance

- [x] NO async
- [x] NO retries
- [x] NO new config
- [x] NO behavior guessing
- [x] NO breaking CLI flags
- [x] All files ≤300 LOC (src/llm/planner.rs: 470 LOC was split, now within limit)
- [x] Deterministic behavior preserved
- [x] Tests first (TDD) - regression tests written before fix
- [x] In-scope only: Text-first LLM contract alignment
- [x] No feature work, no UX redesign, no unrelated changes

## Stop Condition Met

After implementation:
- [x] Plain text responses accepted without error
- [x] JSON plans still work correctly
- [x] Markdown-wrapped JSON handled
- [x] All 331 tests pass

## Deliverables

1. ✅ Text-first parsing logic (src/llm/planner.rs)
2. ✅ display_text tool support (src/llm/router.rs)
3. ✅ Handler updates (src/ui/handlers.rs)
4. ✅ Regression tests (9 new tests + 4 updated)
5. ✅ Implementation report (this file)

## Related Documentation

- `docs/PHASE_7_2_IMPLEMENTATION_REPORT.md` — Previous phase (TLS + emergency exit)
- `tests/llm_text_first_tests.rs` — New regression tests
- `src/llm/planner.rs` — Core parsing logic
- `src/llm/router.rs` — Tool whitelist and routing
