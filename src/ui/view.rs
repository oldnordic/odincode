//! Panel rendering for Phase 1 UI (Phase 4.3: NLP-First)
//!
//! Renders 4 panels (Phase 9.6-A: File Explorer removed; Phase 9.7: Code View → Tool Result):
//! - Tool Result Panel (left, shows live tool execution results)
//! - Action Console (bottom)
//! - Evidence Panel (right)
//! - Diagnostics Panel (right)
//!
//! Phase 4.3 additions:
//! - Planning status overlay
//! - LLM error display
//! - Plan approval prompt

use ratatui::style::Stylize;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::ui::state::{App, AppState, ChatRole, Panel};
/// Render the main UI (Phase 10: Chat-focused layout)
///
/// Layout: Chat (main, left) + 3 panels (right, stacked)
/// - Left: Chat transcript (70%)
/// - Right top: Tool result (33%)
/// - Right middle: Tool execution / Trace (33%)
/// - Right bottom: Evidence or Diagnostics (33%)
pub fn render<B: Backend>(terminal: &mut Terminal<B>, app: &App) -> std::io::Result<()> {
    terminal.draw(|f| {
        // Vertical split: main area (top) + input bar (3 lines, bottom)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(f.area());

        // Main area split: chat (left, main) + panels (right, stacked)
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(chunks[0]);

        // Right side panels: stacked vertically (3 equal panels)
        let panel_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(main_chunks[1]);

        // Chat takes the main left area
        render_chat_transcript(f, app, main_chunks[0]);

        // Right side panels (top to bottom)
        render_tool_result_panel(f, app, panel_chunks[0]);

        // Middle panel: tool execution (if active) or trace viewer, otherwise diagnostics
        if app.tool_panel_visible() {
            render_tool_execution_panel(f, app, panel_chunks[1]);
        } else if app.trace_viewer_visible() {
            render_trace_panel(f, app, panel_chunks[1]);
        } else {
            render_diagnostics_panel(f, app, panel_chunks[1]);
        }

        // Bottom panel: evidence
        render_evidence_panel(f, app, panel_chunks[2]);

        // Input bar at bottom
        render_input_bar(f, app, chunks[1]);
    })?;
    Ok(())
}

/// Render tool result panel (Phase 9.7: Shows live tool execution results)
fn render_tool_result_panel(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.active_panel == Panel::ToolResult {
        " [Tool Result] "
    } else {
        " Tool Result "
    };

    let border_style = if app.active_panel == Panel::ToolResult {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let content = if let Some(ref result) = app.latest_tool_result {
        let mut lines = Vec::new();

        // Header with tool name and status
        let status_color = if result.success {
            Color::Green
        } else {
            Color::Red
        };
        let status_text = if result.success { "SUCCESS" } else { "FAILED" };

        lines.push(Line::from(vec![
            Span::styled("Tool: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&result.tool, Style::default().fg(Color::Cyan)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Step: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", result.step), Style::default().fg(Color::Yellow)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(status_text, Style::default().fg(status_color)),
        ]));

        if let Some(ref path) = result.affected_path {
            lines.push(Line::from(vec![
                Span::styled("Path: ", Style::default().fg(Color::DarkGray)),
                Span::styled(path, Style::default().fg(Color::Blue)),
            ]));
        }

        if let Some(duration) = result.duration_ms {
            lines.push(Line::from(vec![
                Span::styled("Duration: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}ms", duration), Style::default().fg(Color::Yellow)),
            ]));
        }

        lines.push(Line::from("")); // Empty line separator

        // Show stdout if present
        if let Some(ref stdout) = result.stdout {
            if !stdout.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("stdout:", Style::default().fg(Color::Green).bold()),
                ]));

                // Limit output to fit available space
                let max_lines = area.height as usize - lines.len() - 5;
                for line in stdout.lines().take(max_lines) {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", line),
                        Style::default().fg(Color::White),
                    )));
                }

                if stdout.lines().count() > max_lines {
                    lines.push(Line::from(Span::styled(
                        format!("  ... ({} more lines)", stdout.lines().count() - max_lines),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
        }

        // Show stderr if present
        if let Some(ref stderr) = result.stderr {
            if !stderr.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("stderr:", Style::default().fg(Color::Red).bold()),
                ]));

                let max_lines = area.height as usize - lines.len() - 5;
                for line in stderr.lines().take(max_lines) {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", line),
                        Style::default().fg(Color::LightRed),
                    )));
                }
            }
        }

        // Show error message if present
        if let Some(ref error) = result.error {
            lines.push(Line::from(vec![
                Span::styled("error:", Style::default().fg(Color::Red).bold()),
            ]));
            lines.push(Line::from(Span::styled(
                format!("  {}", error),
                Style::default().fg(Color::Red),
            )));
        }

        // If nothing to show, add placeholder
        if result.stdout.is_none() && result.stderr.is_none() && result.error.is_none() {
            lines.push(Line::from(Span::styled(
                "(no output)",
                Style::default().fg(Color::DarkGray),
            )));
        }

        lines
    } else if let Some(ref current_tool) = app.current_tool {
        // Show tool is running
        let mut lines = Vec::new();
        lines.push(Line::from(vec![
            Span::styled("Running: ", Style::default().fg(Color::Yellow)),
            Span::styled(&current_tool.tool, Style::default().fg(Color::Cyan)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Step: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", current_tool.step), Style::default().fg(Color::Yellow)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Executing...",
            Style::default().fg(Color::Yellow).italic(),
        )));
        lines
    } else {
        // No tool has been executed yet - show timeline position
        let mut lines = vec![
            Line::from(Span::styled(
                "Waiting for tool execution...",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
        ];

        // Add timeline position if available (Phase 9.7)
        if let Some(ref pos) = app.timeline_position {
            lines.push(Line::from(vec![
                Span::styled("Timeline Position:", Style::default().fg(Color::Cyan).bold()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Step: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}", pos.current_step), Style::default().fg(Color::Yellow)),
                Span::styled(" | Executions: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}", pos.total_executions), Style::default().fg(Color::Yellow)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Last: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("#{} ({})", pos.last_execution_id, pos.last_execution_tool),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!(" {}", if pos.last_execution_success { "✓" } else { "✗" }),
                    Style::default().fg(if pos.last_execution_success {
                        Color::Green
                    } else {
                        Color::Red
                    }),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Time since query: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{}ms", pos.time_since_last_query_ms),
                    Style::default().fg(if pos.time_since_last_query_ms > 10_000 {
                        Color::Red
                    } else {
                        Color::Green
                    }),
                ),
                Span::styled(
                    format!(
                        " {}",
                        if pos.time_since_last_query_ms > 10_000 {
                            "(stale - query required)"
                        } else {
                            ""
                        }
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            if pos.pending_failure_count > 0 {
                lines.push(Line::from(vec![
                    Span::styled("  Pending failures: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{}", pos.pending_failure_count),
                        Style::default().fg(Color::Red),
                    ),
                ]));
            }
            lines.push(Line::from(""));
        }

        lines.push(Line::from(Span::styled(
            "Tool results will appear here as they execute.",
            Style::default().fg(Color::DarkGray),
        )));

        lines
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}
/// Render evidence panel
fn render_evidence_panel(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.active_panel == Panel::EvidencePanel {
        " [Evidence] "
    } else {
        " Evidence "
    };

    let border_style = if app.active_panel == Panel::EvidencePanel {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let content = if app.ev_db.is_some() {
        vec![
            Line::from("Evidence DB connected"),
            Line::from("Use :evidence list <tool>"),
            Line::from("  to query executions"),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                "No evidence database",
                Style::default().fg(Color::Yellow),
            )),
            Line::from("(execution_log.db missing)"),
        ]
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Render tool execution panel (Phase 9.5)
/// Always visible during active tool execution, shows current tool state
fn render_tool_execution_panel(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = Vec::new();

    if let Some(ref entry) = app.current_tool {
        // Title with tool state
        let state_name = entry.state.display_name();
        let state_color = match entry.state {
            crate::ui::tool_state::ToolExecutionState::Queued => Color::Yellow,
            crate::ui::tool_state::ToolExecutionState::Running { .. } => Color::Green,
            crate::ui::tool_state::ToolExecutionState::Completed { .. } => Color::Blue,
            crate::ui::tool_state::ToolExecutionState::Failed { .. } => Color::Red,
            crate::ui::tool_state::ToolExecutionState::Timeout => Color::Magenta,
            crate::ui::tool_state::ToolExecutionState::Cancelled => Color::DarkGray,
        };

        lines.push(Line::from(Span::styled(
            format!(" Tool Execution [{}] ", state_name),
            Style::default().fg(state_color).bold(),
        )));

        lines.push(Line::from("")); // Blank line

        // Step number
        lines.push(Line::from(Span::styled(
            format!("Step: {}", entry.step),
            Style::default().fg(Color::Cyan),
        )));

        // Tool name
        lines.push(Line::from(Span::styled(
            format!("Tool: {}", entry.tool),
            Style::default().fg(Color::Yellow),
        )));

        // Elapsed time
        let elapsed = if let Some(ms) = entry.state.elapsed_ms() {
            format!("{}ms", ms)
        } else {
            "-".to_string()
        };
        lines.push(Line::from(Span::styled(
            format!("Elapsed: {}", elapsed),
            Style::default().fg(Color::Blue),
        )));

        // Affected path
        if let Some(ref path) = entry.affected_path {
            // Truncate path if too long
            let display_path = if path.len() > 30 {
                format!("...{}", &path[path.len().saturating_sub(27)..])
            } else {
                path.clone()
            };
            lines.push(Line::from(Span::styled(
                format!("Path: {}", display_path),
                Style::default().fg(Color::Gray),
            )));
        }

        // Show hint for cancel
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Type /cancel to stop",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        // No active tool
        lines.push(Line::from(Span::styled(
            "No Active Tool",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Tool Execution ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Render trace panel (Phase 9.4)
fn render_trace_panel(f: &mut Frame, app: &App, area: Rect) {
    let rows = app.trace_rows();
    let error = app.trace_error();

    // Build title with row count
    let mut title = format!(" Last Loop Trace ({}) ", rows.len());
    if error.is_some() {
        title = " Last Loop Trace (ERROR) ".to_string();
    }

    let mut lines = Vec::new();

    // Show hints in title area
    lines.push(Line::from(Span::styled(
        "[L] close  [R] refresh",
        Style::default().fg(Color::DarkGray),
    )));

    // Show error if any
    if let Some(err) = error {
        lines.push(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )));
        lines.push(Line::from(""));
    }

    if rows.is_empty() && error.is_none() {
        lines.push(Line::from(Span::styled(
            "No loop executions yet",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::styled(
            "Run a tool loop to see activity",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        // Header line
        lines.push(Line::from(Span::styled(
            "Timestamp        Tool            Success  Duration  Scope/Path",
            Style::default().fg(Color::Cyan).bold(),
        )));

        // Trace rows
        let max_lines = area.height.saturating_sub(4) as usize;
        for row in rows.iter().take(max_lines) {
            // Format timestamp (short)
            let timestamp = format_timestamp_ms(row.timestamp);

            // Format tool name (truncate to 14 chars)
            let tool_name = truncate_str(&row.tool_name, 14);

            // Success marker
            let success = if row.success {
                Span::styled("✓", Style::default().fg(Color::Green))
            } else {
                Span::styled("✗", Style::default().fg(Color::Red))
            };

            // Duration
            let duration = if let Some(ms) = row.duration_ms {
                format!("{}ms", ms)
            } else {
                "-".to_string()
            };

            // Scope or path
            let scope_or_path = if let Some(ref scope) = row.scope {
                format!("[{}]", scope)
            } else if let Some(ref path) = row.affected_path {
                truncate_str(path, 30)
            } else {
                "-".to_string()
            };

            // Format string for debugging (not used in display)
            let _line = format!(
                "{:<16} {:<14} {:>8} {:>8} {}",
                timestamp, tool_name, "OK", duration, scope_or_path
            );

            let spans = vec![
                Span::raw(timestamp),
                Span::raw(" "),
                Span::styled(tool_name, Style::default().fg(Color::Yellow)),
                Span::raw("  "),
                success,
                Span::raw("  "),
                Span::styled(duration, Style::default().fg(Color::Blue)),
                Span::raw("  "),
                Span::styled(scope_or_path, Style::default().fg(Color::Gray)),
            ];

            lines.push(Line::from(spans));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Format timestamp (milliseconds since UNIX epoch) to short string
fn format_timestamp_ms(ms: i64) -> String {
    let secs = ms / 1000;
    let mins = secs / 60;
    let hours = mins / 60;

    if hours > 0 {
        format!("{}h{}m", hours, mins % 60)
    } else if mins > 0 {
        format!("{}m{}s", mins, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

/// Truncate string to fit width (with ellipsis if needed)
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Render diagnostics panel
fn render_diagnostics_panel(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.active_panel == Panel::DiagnosticsPanel {
        " [Diagnostics] "
    } else {
        " Diagnostics "
    };

    let border_style = if app.active_panel == Panel::DiagnosticsPanel {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let content = if let Some(err_desc) = app.chat_error_description() {
        // Show chat error in diagnostics
        vec![
            Line::from(Span::styled("Chat Error", Style::default().fg(Color::Red))),
            Line::from(""),
            Line::from(Span::styled(err_desc, Style::default().fg(Color::Red))),
            Line::from(""),
            Line::from(Span::styled(
                "Chat transcript unchanged",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else {
        vec![
            Line::from("No diagnostics"),
            Line::from("Use :lsp to check"),
        ]
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}
/// Render chat transcript panel (separate from console output)
fn render_chat_transcript(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = Vec::new();

    // Phase 9.4: Add loop header if active
    if let Some(header) = app.loop_header_text() {
        lines.push(Line::from(Span::styled(
            header,
            Style::default().fg(Color::Cyan).bold(),
        )));
        lines.push(Line::from("")); // Blank line after header
    }

    for msg in app.chat_messages.iter() {
        match msg.role {
            ChatRole::User => {
                // User messages: cyan, prefixed with "You: "
                lines.push(Line::from(Span::styled(
                    format!("You: {}", msg.content),
                    Style::default().fg(Color::Cyan),
                )));
            }
            ChatRole::Assistant => {
                // Assistant messages: yellow, filter JSON from display
                for content_line in msg.content.lines() {
                    // Skip JSON lines
                    let trimmed = content_line.trim();
                    if trimmed.starts_with("```json") || trimmed.starts_with('`') {
                        continue;
                    }
                    // Skip lines that look like JSON objects
                    if trimmed.starts_with('{') && trimmed.ends_with('}') {
                        continue;
                    }
                    lines.push(Line::from(Span::styled(
                        content_line,
                        Style::default().fg(Color::Yellow),
                    )));
                }
            }
            ChatRole::Thinking => {
                // "Thinking..." indicator: gray, italic-style
                lines.push(Line::from(Span::styled(
                    "Thinking...",
                    Style::default().fg(Color::DarkGray),
                )));
            }
            ChatRole::ToolStatus { .. } => {
                // Tool status: distinct styling (magenta, shows progress)
                // Use the live display with elapsed time
                if let Some(display) = msg.role.tool_status_display() {
                    lines.push(Line::from(Span::styled(
                        display,
                        Style::default().fg(Color::Magenta).bg(Color::DarkGray),
                    )));
                } else {
                    // Fallback to content
                    lines.push(Line::from(Span::styled(
                        msg.content.clone(),
                        Style::default().fg(Color::Magenta).bg(Color::DarkGray),
                    )));
                }
            }
        }
    }

    // If empty, show hint
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "Chat with LLM (no \"/\" prefix needed)",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::styled(
            "Type /help for commands",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Phase 9.3: Use app scroll state
    // scroll_offset: 0 = bottom/latest, higher = further back
    let visible_lines = (area.height as usize).saturating_sub(2);
    let scroll_offset = app.chat_scroll_offset();

    let scroll_start = if lines.len() > visible_lines {
        // Calculate base scroll position (show bottom)
        let base = lines.len() - visible_lines;

        // Apply scroll offset (clamp to available range)
        let max_offset = lines.len() - visible_lines;
        let effective_offset = scroll_offset.min(max_offset);

        // Scroll back from bottom by offset (saturating to avoid underflow)
        base.saturating_sub(effective_offset)
    } else {
        0 // Not enough lines to scroll
    };

    let visible: Vec<_> = lines.into_iter().skip(scroll_start).collect();

    let paragraph = Paragraph::new(visible)
        .block(Block::default().title(" Chat ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Render input bar (Phase 4.3: Shows planning status or input)
fn render_input_bar(f: &mut Frame, app: &App, area: Rect) {
    let content = match app.state() {
        AppState::AwaitingApproval => {
            // Phase 9.2: Show GATED tool approval prompt
            let prompt = app
                .pending_approval()
                .map(|p| p.format_prompt())
                .unwrap_or_else(|| "GATED tool requires approval".to_string());

            // Collect owned strings to avoid lifetime issues
            let prompt_lines: Vec<String> = prompt.lines().map(|l| l.to_string()).collect();

            // Parse the formatted prompt into lines
            let lines: Vec<Line> = prompt_lines
                .iter()
                .map(|l| {
                    let line = l.trim();
                    if line.contains("GATED") {
                        Line::from(Span::styled(
                            line.to_string(),
                            Style::default().fg(Color::Yellow).bold(),
                        ))
                    } else if line.contains("[y=") {
                        Line::from(Span::styled(
                            line.to_string(),
                            Style::default().fg(Color::Green),
                        ))
                    } else if line.contains("file_") {
                        Line::from(Span::styled(
                            line.to_string(),
                            Style::default().fg(Color::Cyan),
                        ))
                    } else {
                        Line::from(Span::styled(
                            line.to_string(),
                            Style::default().fg(Color::White),
                        ))
                    }
                })
                .collect();

            // Take first 3 lines for input bar
            lines.into_iter().take(3).collect::<Vec<_>>()
        }
        AppState::PlanningInProgress => {
            // Show "Planning..." message
            let msg = app.planning_message().unwrap_or("Planning...");
            vec![
                Line::from(Span::styled(msg, Style::default().fg(Color::Yellow))),
                Line::from(""),
                Line::from(Span::styled(
                    "Generating plan from LLM...",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
        AppState::PlanReady => {
            // Show plan approval prompt
            let plan_id = app
                .current_plan()
                .map(|p| p.plan_id.as_str())
                .unwrap_or("unknown");
            vec![
                Line::from(Span::styled(
                    "Plan Ready!",
                    Style::default().fg(Color::Green),
                )),
                Line::from(""),
                Line::from(format!("Plan ID: {}", plan_id)),
                Line::from(Span::styled(
                    "Approve? Type 'y' to execute, 'n' to cancel",
                    Style::default().fg(Color::Yellow),
                )),
            ]
        }
        AppState::PlanError => {
            // Show error message
            let error = app.plan_error().unwrap_or("Unknown error");
            vec![
                Line::from(Span::styled("LLM Error", Style::default().fg(Color::Red))),
                Line::from(""),
                Line::from(Span::styled(error, Style::default().fg(Color::Red))),
                Line::from(""),
                Line::from(Span::styled(
                    "Press Enter to continue",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
        AppState::EditingPlan => {
            // Phase 4.5: Edit mode
            let original_id = app.original_plan_id().unwrap_or("unknown");
            let mut content = vec![
                Line::from(Span::styled(
                    "Editing Plan (original preserved)",
                    Style::default().fg(Color::Cyan),
                )),
                Line::from(""),
                Line::from(format!("Original: {}", original_id)),
                Line::from(""),
            ];
            for line in app.edit_buffer().lines().take(3) {
                content.push(Line::from(if line.len() > 60 {
                    format!("{}...", &line[..60])
                } else {
                    line.to_string()
                }));
            }
            content.push(Line::from(""));
            content.push(Line::from(Span::styled(
                "Ctrl+S: Save | Esc: Discard",
                Style::default().fg(Color::Yellow),
            )));
            content
        }
        _ => {
            // Normal input mode — NO prefix (user types directly)
            vec![Line::from(app.input_buffer.as_str())]
        }
    };

    // Border style based on state
    let border_style = match app.state() {
        AppState::AwaitingApproval => Style::default().fg(Color::Yellow), // Phase 9.2
        AppState::PlanningInProgress => Style::default().fg(Color::Yellow),
        AppState::PlanReady => Style::default().fg(Color::Green),
        AppState::PlanError => Style::default().fg(Color::Red),
        AppState::EditingPlan => Style::default().fg(Color::Cyan), // Phase 4.5
        _ => Style::default(),
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}
