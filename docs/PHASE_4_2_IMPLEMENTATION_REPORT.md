# Phase 4.2 Implementation Report: Relaxed Local LLM Credentials

**Date**: 2025-12-24
**Status**: COMPLETE
**Test Coverage**: 233/233 tests passing (added 4 new tests)
**Type**: ERGONOMICS PHASE — NOT a security model change

## Overview

Phase 4.2 relaxes the Phase 4.1 constraint that API keys MUST be stored as environment variable references. Users can now store API keys directly in `config.toml` for local single-user development convenience, while preserving the option to use environment variables.

**THIS IS AN ERGONOMICS PHASE — NOT A SECURITY MODEL CHANGE**

### Design Goals

1. **User Choice** — Let users decide how to store credentials
2. **Backward Compatible** — Existing `env:VAR_NAME` configs continue working
3. **Default to Direct** — Direct storage is the default (easier for local dev)
4. **No Lectures** — No security warnings for direct storage choice

## Implementation

### Files Modified

| File | Changes |
|------|---------|
| `src/cli/preflight.rs` | Added storage option prompt in `configure_external_provider()` |
| `tests/cli_preflight_tests.rs` | Updated/added 4 tests for new storage options |

### New User Flow

When user selects "External API provider", the flow is now:

1. Provider type (glm/openai-compatible/other)
2. Base URL
3. **NEW**: Storage option prompt
4. API key OR env var name (depending on choice)
5. Model name

### New Prompt

```
How do you want to store your API key on this machine?
  [1] Store directly in config file [DEFAULT]
  [2] Use environment variable
  [3] Disable LLM

Choice [1]:
```

## Config Format

### Direct Storage (NEW - default)

```toml
[llm]
mode = "external"
provider = "openai-compatible"
base_url = "https://api.openai.com/v1"
api_key = "sk-abc123..."  # Literal value
model = "gpt-4"
```

### Env Var Storage (existing - still supported)

```toml
[llm]
mode = "external"
provider = "openai-compatible"
base_url = "https://api.openai.com/v1"
api_key = "env:ODINCODE_LLM_API_KEY"  # Env reference
model = "gpt-4"
```

## Resolution Logic (Future Implementation)

When the LLM client is implemented in a future phase, it will resolve API keys like this:

```rust
fn resolve_api_key(value: &str) -> String {
    if value.starts_with("env:") {
        let var_name = &value[4..]; // Strip "env:" prefix
        std::env::var(var_name).unwrap_or_default()
    } else {
        value.to_string() // Literal value
    }
}
```

**Note**: This resolution logic is NOT yet implemented. Phase 4.2 only modifies the preflight wizard to support both storage formats.

## Test Coverage

### New Tests Added

| Test | Scenario |
|------|----------|
| `test_missing_config_external_direct_storage` | Direct storage (default, empty input) |
| `test_missing_config_external_env_storage` | Env var storage (option 2) |
| `test_direct_storage_literal_key` | Direct storage writes literal key |
| `test_env_var_storage_no_literal` | Env var storage does NOT write literal |

### Tests Updated

| Test | Change |
|------|--------|
| Test B → Tests B1, B2 | Split into separate direct/env storage tests |
| Test G → Tests G1, G2 | Split into literal/env validation tests |

### Tests Unchanged

- A, C, D, E, F, H, I — No changes (don't involve external provider)

### Full Test Results

```
running 13 tests
test test_direct_storage_literal_key ... ok
test test_disabled_config_no_prompt ... ok
test test_env_var_storage_no_literal ... ok
test test_invalid_config_choose_continue ... ok
test test_missing_config_external_direct_storage ... ok
test test_missing_config_external_env_storage ... ok
test test_missing_config_choose_no_llm ... ok
test test_missing_config_choose_local ... ok
test test_preflight_runs_for_tui_entry ... ok
test test_storage_disable_option ... ok
test test_valid_config_no_prompt ... ok
test test_storage_prompt_shows_options ... ok
test test_preflight_runs_for_cli_entry ... ok

test result: ok. 13 passed; 0 failed; 0 ignored
```

**Total**: 233/233 tests passing (all Phase 4.1 tests + 4 new Phase 4.2 tests)

## Code Changes Summary

### `src/cli/preflight.rs`

**Before** (Phase 4.1):
- Always asked for "API key (will be stored as environment variable reference)"
- Always stored as `api_key = "env:ODINCODE_LLM_API_KEY"`
- Ignored user input, always used default env var name

**After** (Phase 4.2):
- Asks "How do you want to store your API key on this machine?"
- Option [1] (default): Stores literal API key in config
- Option [2]: Stores `api_key = "env:VAR_NAME"`
- Option [3]: Disables LLM (writes `mode = "disabled"`)
- Empty input defaults to option [1]

## Backward Compatibility

✅ **Fully backward compatible**
- Existing `env:VAR_NAME` configs continue to work unchanged
- No changes to config file reading (not implemented yet)
- No breaking changes to API or behavior

## Security Model

**This is NOT a security model change.** The security implications remain the same:
- Config file is readable by the file owner (same as env vars accessible to process)
- Users are responsible for securing their `~/.odincode` directory
- No encryption, no permissions changes, no access control modifications

## Constraints Compliance

| Constraint | Status |
|------------|--------|
| NO async | ✅ All code is synchronous |
| NO network calls | ✅ No HTTP/requests |
| NO provider validation | ✅ No connection testing |
| No breaking changes | ✅ Existing configs work |
| Direct storage default | ✅ Option [1], empty input → direct |
| Empty string invalid | ✅ Rejected for direct storage |
| No warnings/lectures | ✅ No security messages |

## Verification

```bash
$ cargo test
test result: ok. 233 passed; 0 failed; 0 ignored
```

All 233 tests pass, including:
- 13 CLI preflight tests (9 from Phase 4.1 + 4 new)
- All existing tests (no regressions)

## Sign-Off

**Implementation**: Complete
**Tests**: All passing (233/233)
**Documentation**: This report

---

STOP — Phase 4.2 Relaxed Local LLM Credentials implementation complete; awaiting acceptance or revisions.
