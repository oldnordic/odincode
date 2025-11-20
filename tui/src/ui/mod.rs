//! TUI UI Module
//!
//! This module contains the UI rendering functionality for the TUI system.

use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
    Frame,
};

use crate::app::TuiApp;
use crate::models::TuiState;

/// Render the UI
pub fn render(app: &mut TuiApp, frame: &mut Frame) {
    // Define the main layout
    let size = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(size);

    // Render title bar
    let title = Paragraph::new(app.title.as_str())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[0]);

    // Render tabs
    let tabs = Tabs::new(vec![
        "Files", "Editor", "Agents", "LTMC", "Tools", "Terminal",
    ])
    .block(Block::default().borders(Borders::BOTTOM))
    .select(app.current_tab.min(5))
    .style(Style::default().fg(Color::White))
    .highlight_style(Style::default().fg(Color::Yellow));
    frame.render_widget(tabs, chunks[1]);

    // Render main content based on current state
    match app.current_state {
        TuiState::FileBrowser => render_file_browser(app, frame, chunks[2]),
        TuiState::CodeEditor => render_code_editor(app, frame, chunks[2]),
        TuiState::AgentSelection => render_agent_selection(app, frame, chunks[2]),
        TuiState::AnalysisResults => render_analysis_results(app, frame, chunks[2]),
        TuiState::LTMCView => render_ltmc_view(app, frame, chunks[2]),
        TuiState::ToolSelection => render_tool_selection(app, frame, chunks[2]),
        TuiState::TerminalIntegration => app.terminal_integration.render(frame, chunks[2]),
    }

    // Render status bar
    let status_text = match app.current_state {
        TuiState::FileBrowser => "File Browser - Use ↑↓ to navigate, Enter to open, A for agents, T for tools, L for LTMC",
        TuiState::CodeEditor => "Code Editor - Use Ctrl+B to go back to file browser",
        TuiState::AgentSelection => "Agent Selection - Use ↑↓ to navigate, Enter to execute",
        TuiState::AnalysisResults => "Analysis Results",
        TuiState::LTMCView => "LTMC View - Persistent learning and memory",
        TuiState::ToolSelection => "Tool Selection - Use ↑↓ to navigate, Enter to execute",
        TuiState::TerminalIntegration => "Terminal Integration - Execute shell commands with auto-completion",
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(status, chunks[3]);
}

/// Render file browser view
fn render_file_browser(app: &mut TuiApp, frame: &mut Frame, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .files
        .iter()
        .map(|file| {
            ListItem::new(format!("{} - {}", file.path, file.language))
                .style(Style::default().fg(Color::White))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_file_index));

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Files"))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render code editor view
fn render_code_editor(app: &mut TuiApp, frame: &mut Frame, area: ratatui::layout::Rect) {
    let block = Block::default().borders(Borders::ALL).title("Code Editor");

    let paragraph = Paragraph::new(app.code_content.as_str())
        .block(block)
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}

/// Render agent selection view
fn render_agent_selection(app: &mut TuiApp, frame: &mut Frame, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .agents
        .iter()
        .map(|agent| {
            ListItem::new(format!("{} - {}", agent.name, agent.description))
                .style(Style::default().fg(Color::White))
        })
        .collect();

    let mut state = ListState::default();
    state.select(app.selected_agent_index);

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("AI Agents"))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render analysis results view
fn render_analysis_results(app: &mut TuiApp, frame: &mut Frame, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Analysis Results");

    let content = if app.analysis_results.is_empty() {
        "No analysis results available".to_string()
    } else {
        format!(
            "Found {} issues and suggestions",
            app.analysis_results.len()
        )
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}

/// Render LTMC view
fn render_ltmc_view(app: &mut TuiApp, frame: &mut Frame, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .ltmc_patterns
        .iter()
        .map(|pattern| {
            ListItem::new(format!("{:?}: {}", pattern.pattern_type, pattern.content))
                .style(Style::default().fg(Color::White))
        })
        .collect();

    let mut state = ListState::default();
    state.select(app.selected_pattern_index);

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("LTMC Patterns"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render tool selection view
fn render_tool_selection(app: &mut TuiApp, frame: &mut Frame, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .tools
        .iter()
        .map(|tool| ListItem::new(tool.as_str()).style(Style::default().fg(Color::White)))
        .collect();

    let mut state = ListState::default();
    state.select(app.selected_tool_index);

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Development Tools"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state);
}
