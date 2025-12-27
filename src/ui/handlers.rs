//! TUI command handlers — Phase 8.1: Isolated chat lane
//!
//! 3-way router with ISOLATED execution lanes:
//! A) Commands (start with "/") — execute immediately, bypass planner
//! B) Chat (default) — isolated LLM chat, NO plan/workflow system
//! C) Plan (/plan, /apply) — structured plan with approval
//!
//! Phase 8.1 CRITICAL: Chat must NEVER touch plan/workflow system

use crate::ui::input::Command;
use crate::ui::state::App;

/// Handle /find command
pub fn handle_find_command(app: &mut App, pattern: &str) {
    use crate::magellan_tools::MagellanDb;

    let codegraph_path = app.db_root.join("codegraph.db");
    if !codegraph_path.exists() {
        app.log("Symbol navigation is unavailable.".to_string());
        app.log("To enable code search, run:".to_string());
        app.log(format!(
            "  magellan watch --root . --db {}",
            codegraph_path.display()
        ));
        return;
    }

    match MagellanDb::open_readonly(&codegraph_path) {
        Ok(magellan_db) => match magellan_db.symbols_in_file(&format!("%{}%", pattern)) {
            Ok(symbols) => {
                if symbols.is_empty() {
                    app.log(format!("No symbols found matching '{}'", pattern));
                } else {
                    app.log(format!("Found {} symbols:", symbols.len()));
                    for sym in symbols.iter().take(10) {
                        app.log(format!(
                            "  {} ({}) in {}",
                            sym.name, sym.kind, sym.file_path
                        ));
                    }
                    if symbols.len() > 10 {
                        app.log(format!("  ... and {} more", symbols.len() - 10));
                    }
                }
            }
            Err(e) => {
                app.log(format!("Query failed: {}", e));
            }
        },
        Err(e) => {
            app.log(format!("Failed to open codegraph.db: {}", e));
        }
    }
}

/// Handle chat input — Phase 9.6-B1: ChatLoop integration
///
/// CRITICAL INVARIANTS:
/// - Does NOT call propose_plan() or any plan/session functions
/// - Does NOT create Plan objects
/// - Does NOT write to execution DB directly (persistence via events)
/// - Does NOT enter planning state
/// - Shows "Thinking..." immediately
/// - Returns immediately (non-blocking)
/// - Errors route to diagnostics, NOT chat transcript
/// - Streaming chunks update UI via main loop event processing
/// - Phase 9.6-B1: Uses ChatLoop for multi-step tool execution
pub fn handle_chat(app: &mut App, text: &str) {
    use crate::execution_engine::ChatToolRunner;
    use crate::llm::ChatLoop;
    use crate::magellan_tools::MagellanDb;
    use std::sync::mpsc::channel;

    // Clone db_root to avoid borrow checker conflict
    let db_root = app.db_root.clone();

    // Clear any previous error state
    app.clear_chat_error();

    // Add user message to chat transcript
    app.add_user_message(text.to_string());

    // Show "Thinking..." immediately
    app.set_thinking();

    // Create channel for chat events
    let (tx, rx) = channel();

    // Create ChatToolRunner with database access
    let magellan_db = MagellanDb::open_readonly(db_root.join("codegraph.db")).ok();
    let exec_db = crate::execution_tools::ExecutionDb::open(&db_root).ok();
    let tool_runner = ChatToolRunner::new(magellan_db, exec_db);

    // Create and configure ChatLoop
    let mut chat_loop = ChatLoop::new(tool_runner);
    chat_loop.set_sender(tx.clone());

    // Start the loop (spawns initial chat thread)
    match chat_loop.start(text.to_string(), &db_root) {
        Ok(()) => {
            // Store loop, receiver, and sender in app state
            app.set_chat_loop(chat_loop);
            app.chat_event_receiver = Some(rx);
            app.chat_event_sender = Some(tx);
            // UI will render immediately with "Thinking..."
        }
        Err(e) => {
            // Failed to start loop
            app.log(format!("Failed to start chat: {}", e));
            app.chat_messages
                .retain(|m| m.role != crate::ui::state::ChatRole::Thinking);
        }
    }

    // Function returns immediately - main loop processes events
}

/// Execute plan — Phase 8.1: ONLY for /plan and /apply commands
///
/// This function is ONLY called from:
/// - Command::Plan (user explicitly entered /plan)
/// - Command::Apply (user explicitly entered /apply)
/// - User pressed 'y' to approve a plan
///
/// CRITICAL: Chat never calls this function.
pub fn execute_plan(app: &mut App) {
    let plan_data = app
        .current_plan
        .as_ref()
        .map(|p| (p.plan_id.clone(), p.steps.clone()));

    if let Some((plan_id, steps)) = plan_data {
        app.log(format!("Executing: {}", plan_id));

        for step in &steps {
            match step.tool.as_str() {
                "display_text" => {
                    if let Some(text) = step.arguments.get("text") {
                        app.log(format!("> {}", text));
                    }
                }
                "file_read" => {
                    if let Some(path) = step.arguments.get("path") {
                        match app.read_file(path.clone()) {
                            Ok(contents) => {
                                app.log(format!("Read: {} ({} bytes)", path, contents.len()));
                            }
                            Err(e) => {
                                app.log(format!("Read failed: {}", e));
                            }
                        }
                    }
                }
                "lsp_check" => {
                    let path = step
                        .arguments
                        .get("path")
                        .map(|s| s.as_str())
                        .unwrap_or(".");
                    app.log(format!("Running LSP check on: {}", path));
                    match crate::lsp_tools::lsp_check(std::path::Path::new(path)) {
                        Ok(diagnostics) => {
                            if diagnostics.is_empty() {
                                app.log("No diagnostics found".to_string());
                            } else {
                                app.log(format!("Found {} diagnostics", diagnostics.len()));
                                for d in diagnostics.iter().take(5) {
                                    let code = d.code.as_deref().unwrap_or("unknown");
                                    app.log(format!("  {}: {}", code, d.message));
                                }
                            }
                        }
                        Err(e) => {
                            app.log(format!("LSP check failed: {}", e));
                        }
                    }
                }
                _ => {
                    app.log(format!(
                        "Step {}: {} (not yet implemented)",
                        step.step_id, step.tool
                    ));
                }
            }
        }

        app.log("Done".to_string());
    }
}

/// Execute parsed command (Phase 8.1: 3-way routing with isolated chat)
///
/// IMPORTANT: /quit bypasses ALL logic and exits immediately.
/// This function is called AFTER exit check in main loop.
pub fn execute_command(app: &mut App, cmd: Command) {
    match cmd {
        Command::Quit => {
            // Should not reach here — /quit is handled in main loop
            // But if we do, exit immediately
            app.log("Exiting...".to_string());
            app.quit();
        }
        Command::Open(path) => {
            app.open_file(path.clone());
            app.selected_file = Some(path);
        }
        Command::Read(path) => match app.read_file(path.clone()) {
            Ok(_) => {
                app.log(format!("Read: {}", path));
                app.selected_file = Some(path);
            }
            Err(e) => {
                app.log(format!("Read failed: {}", e));
            }
        },
        Command::Lsp(path) => {
            app.log(format!("Running LSP check on: {}", path));
            match crate::lsp_tools::lsp_check(std::path::Path::new(&path)) {
                Ok(diagnostics) => {
                    if diagnostics.is_empty() {
                        app.log("No diagnostics found".to_string());
                    } else {
                        app.log(format!("Found {} diagnostics", diagnostics.len()));
                        for d in diagnostics.iter().take(5) {
                            let code = d.code.as_deref().unwrap_or("unknown");
                            app.log(format!("  {}: {}", code, d.message));
                        }
                    }
                }
                Err(e) => {
                    app.log(format!("LSP check failed: {}", e));
                }
            }
        }
        Command::Help => {
            app.log("OdinCode v0.0.1 — Commands start with \"/\"".to_string());
            app.log("Type to chat, /help for commands, /quit to exit".to_string());
        }
        Command::Find(pattern) => {
            app.log(format!("Finding: {}", pattern));
            handle_find_command(app, &pattern);
        }
        Command::Plan => {
            // Convert chat context to structured plan
            app.log("Converting chat to plan...".to_string());
            // For now, just log that plan mode was triggered
            app.log("(Plan mode: accumulates chat context into structured plan)".to_string());
        }
        Command::Apply => {
            // Execute pending plan
            if app.current_plan().is_some() {
                execute_plan(app);
                app.clear_planning_state();
            } else {
                app.log("No pending plan to apply".to_string());
            }
        }
        // Phase 9.5: Tool execution and trace commands
        Command::TraceOn => {
            if let Ok(exec_db) = app.open_exec_db() {
                app.toggle_trace_viewer(&exec_db, 20);
                if app.trace_viewer_visible() {
                    app.log("Trace viewer opened".to_string());
                }
            } else {
                app.log("Failed to open execution database".to_string());
            }
        }
        Command::TraceOff => {
            if app.trace_viewer_visible() {
                app.hide_trace_viewer();
                app.log("Trace viewer closed".to_string());
            } else {
                app.log("Trace viewer not visible".to_string());
            }
        }
        Command::Cancel => {
            if app.has_active_tool() {
                app.cancel_current_tool();
                app.log("Tool execution cancelled".to_string());
            } else {
                app.log("No active tool to cancel".to_string());
            }
        }
        Command::Status => {
            if let Some(status_text) = app.tool_status_text() {
                app.log(status_text);
            } else {
                app.log("No active tool execution".to_string());
            }
        }
        Command::Continue => {
            // Resume after approval — for now, just acknowledge
            // (Future: will resume paused tool execution)
            app.log("Continuing...".to_string());
        }
        Command::Chat(text) => {
            // Phase 8.1: Isolated chat lane — NO plan/workflow involvement
            handle_chat(app, &text);
        }
        Command::None => {
            // Empty input, ignore
        }
    }
}
