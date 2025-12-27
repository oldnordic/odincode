// LSP diagnostics via cargo check
// Uses cargo check --message-format=json for structured output

mod check;

pub use check::{lsp_check, Diagnostic};
