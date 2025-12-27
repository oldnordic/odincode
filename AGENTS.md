# Repository Guidelines

## Project Structure & Module Organization
OdinCode is a single Cargo crate: `src/lib.rs` re-exports each deterministic tool module (`file_tools`, `splice_tools`, `magellan_tools`, `lsp_tools`, `execution_tools`, `llm`, `ui`) while `src/main.rs` stays thin so behavior lives in the library layer. Integration tests reside in `tests/*.rs` and mirror that layout (`file_tools_tests.rs`, `splice_tools_tests.rs`, etc.). Architecture notes, constraints, and external-tool specs are under `docs/`; SQLite databases and configs at the repository root are runtime artifacts only.

## Build, Test, and Development Commands
Use Cargo for every workflow: `cargo check` for quick validation, `cargo test` for the full suite, and `cargo test --test magellan_tools_tests` for targeted runs (add `-- --nocapture` for tool logs). Build the release binary with `cargo build --release` so downstream agents exercise the same executable.

## Coding Style & Naming Conventions
Work in Rust 2021, keep files ≤300 LOC, and respect the “zero guessing” rules—no async/await, background threads, or hidden global state. Format with `cargo fmt` and gate merges with `cargo clippy --all-targets --all-features -D warnings` to protect deterministic behavior. Modules expose small structs/enums (e.g., `SpliceResult`, `MagellanDb`) plus `thiserror` enums whose messages explain the corrective action. Follow standard Rust naming: snake_case for functions, UpperCamelCase for types, and SCREAMING_SNAKE_CASE for constant tool identifiers.

## Testing Guidelines
Coverage is integration-only (27 cases) and shells out to `splice`, `magellan`, and `cargo`, so confirm those binaries exist before running `cargo test`. New work starts with a failing test in the matching `*_tests.rs`; keep assertions deterministic by sorting collections and matching SQL ordering to production behavior. Document external prerequisites at the top of each test file.

## Commit & Pull Request Guidelines
Even though this snapshot omits `.git`, follow Conventional Commits `type(scope): subject` (example: `fix(lsp_tools): normalize exit codes`) and add a short body whenever behavior, docs, or tool contracts move. Pull requests should link the motivating issue or plan section, list the exact commands executed (`cargo test --test splice_tools_tests -- --nocapture`, etc.), and attach the relevant stdout/stderr to prove external tool availability. Cite files, commands, and tool versions explicitly—intuition alone violates the zero-guessing contract.

## External Tools & Determinism
`docs/EXTERNAL_TOOLS_API.md` and `CLAUDE.md` define the Magellan, Splice, and Cargo APIs. Keep every interaction synchronous, capture exit codes plus raw output verbatim, and return that data through the public interfaces so downstream agents can trust the substrate. Never infer filesystem or compiler state; rerun the tool you are describing and report exactly what it produced.
