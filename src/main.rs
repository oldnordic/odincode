//! OdinCode CLI — Phase 8 routing architecture
//!
//! 3-way input router:
//! - Commands (start with "/") — execute immediately
//! - Chat (default) — conversational LLM
//! - Plan (/plan, /apply) — structured plan with approval
//!
//! EXIT: /quit, /q, /exit work from ANY state, bypass all logic

use std::io::{self};
use std::time::Duration;

use crossterm::{
    event::{
        poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use odincode::cli::bootstrap::{ensure_infrastructure, BootstrapStatus};
use odincode::cli::preflight::{run_llm_preflight, PreflightOutcome};
use odincode::cli::{parse_args, Args, Mode};
use odincode::ui::{handlers, parse_command, App, Command};

fn main() -> io::Result<()> {
    // Parse CLI arguments
    let args: Vec<String> = std::env::args().collect();

    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // Validate db_root exists if explicitly provided
    if let Some(ref db_root_str) = parsed.db_root {
        let db_path = std::path::Path::new(db_root_str);
        if !db_path.exists() {
            eprintln!("Error: db_root '{}' does not exist", db_root_str);
            std::process::exit(2);
        }
    }

    // Handle --version flag
    if parsed.show_version {
        println!("OdinCode v0.0.1");
        println!("Phase 8 — 3-way routing architecture");
        return Ok(());
    }

    // Handle --help flag
    if parsed.show_help {
        print_help();
        return Ok(());
    }

    let mode = parsed.mode.clone().unwrap_or(Mode::Tui);

    // Handle CLI-only modes (non-TUI)
    if matches!(
        mode,
        Mode::Plan { .. } | Mode::Execute { .. } | Mode::Evidence { .. }
    ) {
        let exit_code = odincode::cli::run_cli_mode(parsed);
        std::process::exit(exit_code);
    }

    // TUI mode (default or explicit)
    run_tui_mode(parsed)
}

/// Run TUI mode
fn run_tui_mode(args: Args) -> io::Result<()> {
    use odincode::cli::resolve_db_root;

    // Resolve db_root
    let db_root = match resolve_db_root(args.db_root) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(2);
        }
    };

    // Ensure infrastructure (bootstrap)
    match ensure_infrastructure(&db_root, true, true, args.no_bootstrap) {
        Ok(BootstrapStatus::Ready) => {}
        Ok(BootstrapStatus::NeedsRestart) => {
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Bootstrap error: {}", e);
            std::process::exit(2);
        }
    }

    // Run LLM preflight
    match run_llm_preflight(&db_root) {
        Ok(PreflightOutcome::Exit) => {
            std::process::exit(0);
        }
        Ok(PreflightOutcome::Proceed) => {}
        Err(e) => {
            eprintln!("Preflight error: {}", e);
            std::process::exit(2);
        }
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(db_root);
    app.log("OdinCode — AI-powered editor".to_string());
    app.log("Type to chat, /help for commands, /quit to exit".to_string());

    // Main event loop
    while app.state() != odincode::ui::state::AppState::Quitting {
        // Render
        odincode::ui::render(&mut terminal, &app)?;

        // Block for input (100ms timeout)
        if poll(Duration::from_millis(100))? {
            if let Event::Key(key) = read()? {
                // Phase 8: Ctrl+C exits immediately from any state
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break; // Exit immediately
                }

                handle_key_event(&mut app, key);

                // Phase 8: Check for quit after handling key (immediate exit)
                if app.state() == odincode::ui::state::AppState::Quitting {
                    break;
                }
            }
        }

        // Phase 8.6: Process chat events from background thread
        // Non-blocking: processes all available events, updates UI state
        // Next render() will show streaming chunks as they arrive
        app.process_chat_events();

        // Phase 9.5: Check for stuck tool timeout (60s default)
        const TOOL_TIMEOUT_MS: u64 = 60000; // 60 seconds
        app.handle_tool_timeout(TOOL_TIMEOUT_MS);
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

/// Print help message
fn print_help() {
    println!("OdinCode v0.0.1 - AI-Powered Refactoring Assistant");
    println!();
    println!("USAGE:");
    println!("    odincode [options] [mode] [mode-args]");
    println!();
    println!("MODES:");
    println!("    (none)        TUI mode (default)");
    println!("    tui           TUI mode (explicit)");
    println!("    plan <goal>   Generate plan from natural language goal");
    println!("    execute       Execute stored plan (--plan-file required)");
    println!("    evidence <Q>  Query execution history (Q1-Q8)");
    println!();
    println!("OPTIONS:");
    println!("    --db-root <path>     Database root (default: current directory)");
    println!("    --plan-file <file>   Plan file path (for execute mode)");
    println!("    --json              Output JSON (for scripting)");
    println!("    --no-bootstrap      Skip first-run setup (expert mode)");
    println!("    --version           Show version information");
    println!("    --help              Show this help message");
    println!();
    println!("EXECUTION HISTORY (query results as JSON):");
    println!("    Q1 <tool>     Show recent executions by tool");
    println!("    Q2 <tool>     Show recent failures by tool");
    println!("    Q3 <code>     Find prior executions with error code");
    println!("    Q4 <path>     Find executions affecting a file");
    println!("    Q5 <id>       Get full execution details");
    println!("    Q6 <path>     Get latest result for a file");
    println!("    Q7 <n>        Find recurring errors (>n occurrences)");
    println!("    Q8 <code>     Find prior fixes for an error");
    println!();
    println!("TUI COMMANDS (start with \"/\"):");
    println!("    /quit, /q, /exit     Quit immediately (works from any state)");
    println!("    /open <path>         Open file in code view");
    println!("    /read <path>         Read file contents");
    println!("    /lsp [path]          Run cargo check (default: current dir)");
    println!("    /find <pattern>      Find symbols or files");
    println!("    /plan                Convert chat to structured plan");
    println!("    /apply               Execute pending plan");
    println!("    /help                Show this help message");
    println!();
    println!("CHAT MODE (default):");
    println!("    Type anything to chat with LLM (no \"/\" prefix needed)");
    println!("    Responses shown immediately — no approval for chat");
    println!("    Examples: \"read src/lib.rs\", \"fix the type error\"");
}

/// Handle keyboard input (Phase 8.2: state-aware key routing)
fn handle_key_event(app: &mut App, key: KeyEvent) {
    match key.code {
        // Generic character input — route based on state
        KeyCode::Char(c) => {
            match app.state() {
                odincode::ui::state::AppState::AwaitingApproval => {
                    // Phase 9.2: GATED tool approval mode: y/a/n/q
                    use odincode::ui::ApprovalResponse;
                    let response = match c {
                        'y' | 'Y' => {
                            // Approve once
                            app.pending_approval()
                                .map(|pending| ApprovalResponse::ApproveOnce(pending.tool.clone()))
                        }
                        'a' | 'A' => {
                            // Approve all GATED tools for session
                            Some(ApprovalResponse::ApproveSessionAllGated)
                        }
                        'n' | 'N' => {
                            // Deny
                            app.pending_approval()
                                .map(|pending| ApprovalResponse::Deny(pending.tool.clone()))
                        }
                        'q' | 'Q' => {
                            // Quit
                            Some(ApprovalResponse::Quit)
                        }
                        _ => None,
                    };

                    if let Some(resp) = response {
                        app.send_approval_response(resp);
                        app.log(format!("Approval response: {:?}", c));
                        // Phase 9.4: Refresh trace after approval event
                        if app.trace_viewer_visible() {
                            if let Ok(exec_db) = app.open_exec_db() {
                                app.on_approval_event_refresh_trace(&exec_db, 20);
                            }
                        }
                    }
                }
                odincode::ui::state::AppState::PlanReady => {
                    // Plan approval mode: y/n approve/reject
                    match c {
                        'y' | 'Y' => {
                            handlers::execute_plan(app);
                            app.clear_planning_state();
                        }
                        'n' | 'N' => {
                            app.log("Plan rejected".to_string());
                            app.clear_planning_state();
                        }
                        _ => {
                            // Other keys ignored in PlanReady state
                        }
                    }
                }
                odincode::ui::state::AppState::Running => {
                    // Phase 9.5: NO implicit key bindings for tool control
                    // All tool/trace commands use explicit /commands
                    app.handle_char(c);
                }
                _ => {
                    // Other states: ignore char input
                }
            }
        }
        KeyCode::Backspace => {
            if matches!(app.state(), odincode::ui::state::AppState::Running) {
                app.handle_backspace();
            }
        }
        KeyCode::Enter => {
            // Phase 8: Parse command with 3-way routing
            let input = app.input_buffer.clone();
            let cmd = parse_command(&input);

            // CRITICAL: Check for QUIT commands BEFORE routing to handlers
            // Exit must bypass ALL logic (planner, LLM, memory, approval)
            if matches!(cmd, Command::Quit) {
                app.log("Exiting...".to_string());
                app.quit();
                app.input_buffer.clear();
                return;
            }

            // Route all other commands through handlers
            handlers::execute_command(app, cmd);
            app.input_buffer.clear();
        }
        KeyCode::Esc => {
            // Clear input or exit error state
            if matches!(app.state(), odincode::ui::state::AppState::PlanError) {
                app.clear_planning_state();
            } else {
                app.input_buffer.clear();
            }
        }
        KeyCode::Tab => {
            // Cycle panels (only in normal state)
            if matches!(app.state(), odincode::ui::state::AppState::Running) {
                cycle_panel(app);
            }
        }
        // Chat scroll navigation
        KeyCode::Up => {
            if matches!(app.state(), odincode::ui::state::AppState::Running) {
                app.chat_scroll_up(1);
            }
        }
        KeyCode::Down => {
            if matches!(app.state(), odincode::ui::state::AppState::Running) {
                app.chat_scroll_down(1);
            }
        }
        KeyCode::PageUp => {
            if matches!(app.state(), odincode::ui::state::AppState::Running) {
                app.chat_scroll_up(10); // Scroll 10 lines
            }
        }
        KeyCode::PageDown => {
            if matches!(app.state(), odincode::ui::state::AppState::Running) {
                app.chat_scroll_down(10); // Scroll 10 lines
            }
        }
        KeyCode::Home => {
            if matches!(app.state(), odincode::ui::state::AppState::Running) {
                // Scroll to top (max offset)
                app.chat_scroll_up(1000);
            }
        }
        KeyCode::End => {
            if matches!(app.state(), odincode::ui::state::AppState::Running) {
                app.chat_scroll_to_end();
            }
        }
        _ => {}
    }
}

/// Cycle active panel (Phase 9.6-A: FileExplorer removed, Phase 9.7: CodeView → ToolResult)
fn cycle_panel(app: &mut App) {
    use odincode::ui::state::Panel;
    app.active_panel = match app.active_panel {
        Panel::ToolResult => Panel::ActionConsole,
        Panel::ActionConsole => Panel::EvidencePanel,
        Panel::EvidencePanel => Panel::DiagnosticsPanel,
        Panel::DiagnosticsPanel => Panel::ToolResult,
    };
}
