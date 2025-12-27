# Changelog

All notable changes to OdinCode are documented in this file.

## [0.1.0] - 2025-12-27

### Added
- **Phase 0: Tool Substrate** (COMPLETE)
  - `file_tools` module — Filesystem operations (read, write, create, search, glob)
  - `splice_tools` module — Splice binary integration for refactoring
  - `magellan_tools` module — SQLiteGraph direct queries
  - `lsp_tools` module — Compiler diagnostics via cargo check
  - `execution_tools` module — Execution logging database
  - `llm` module — Chat loop, frame stack, adapters (OpenAI, Ollama)
  - `adapters` module — LLM provider adapters

### Infrastructure
- Integration test suite (986 tests passing)
  - `file_tools_tests.rs` — 13 tests
  - `splice_tools_tests.rs` — 5 tests
  - `magellan_tools_tests.rs` — 5 tests
  - `lsp_tools_tests.rs` — 4 tests
  - `execution_tools_tests.rs` — 8 tests
  - `adapter_tests.rs` — 4 tests
  - `frame_stack_tests.rs` — 8 tests
  - `chat_tests.rs` — 7 tests
  - `loop_action_tests.rs` — 7 tests
  - `tool_classification_tests.rs` — 10 tests
  - `prompt_mode_tests.rs` — 6 tests
  - Phase 9 tests (8 test files, 906 tests)
  - Phase 10 tests (3 test files, 720 tests)

### Documentation
- `CONTRACT.md` — Phase 0 frozen scope
- `TODO.md` — Phase tracking
- `EXTERNAL_TOOLS_API.md` — External tool APIs
- `TOOL_MANUALS.md` — CLI reference
- `TOOL_RESULT_COMPACTION_V2.md` — Design for tool result compaction

### Design Decisions
- MAX 300 LOC per module constraint enforced
- All tools follow TDD workflow
- No async, no global state, no mocks
- Opaque JSON payloads for SQLiteGraph data

## [Unreleased]

### Planned (Phase 0.5)
- Execution memory & outcome logging
- `memory_query` tool for database retrieval
- Tool result compaction with metadata
- Smart compaction rules for edit workflows
- Error loop prevention with structural enforcement
