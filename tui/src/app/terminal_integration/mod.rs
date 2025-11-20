//! Enhanced Terminal Integration Module
//!
//! This module provides advanced terminal integration features including
//! native terminal commands, shell integration, and enhanced user experience.

#[cfg(test)]
mod auto_completion_tests;
#[cfg(test)]
mod command_execution_tests;
#[cfg(test)]
mod creation_tests;
#[cfg(test)]
mod event_handling_tests;

use anyhow::Result;
use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind},
    execute,
    style::ResetColor,
    terminal::{self, ClearType},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::io::{self, Write};
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::models::{TerminalCommand, TerminalOutput, TuiState};

/// Enhanced terminal integration with native shell support
pub struct TerminalIntegration {
    /// Command history
    command_history: Vec<String>,
    /// Current command input
    current_command: String,
    /// Command history index
    history_index: usize,
    /// Terminal output buffer
    output_buffer: Vec<TerminalOutput>,
    /// Command sender for async execution
    command_sender: mpsc::UnboundedSender<TerminalCommand>,
    /// Command receiver for async execution
    command_receiver: Option<mpsc::UnboundedReceiver<TerminalCommand>>,
    /// Shell integration enabled
    shell_integration: bool,
    /// Auto-completion enabled
    auto_completion: bool,
    /// Syntax highlighting enabled
    syntax_highlighting: bool,
}

impl TerminalIntegration {
    /// Create a new terminal integration instance
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            command_history: Vec::new(),
            current_command: String::new(),
            history_index: 0,
            output_buffer: Vec::new(),
            command_sender: sender,
            command_receiver: Some(receiver),
            shell_integration: true,
            auto_completion: true,
            syntax_highlighting: true,
        }
    }

    /// Initialize the terminal integration
    pub fn initialize(&mut self) -> Result<()> {
        info!("Initializing enhanced terminal integration...");

        // Setup terminal for enhanced features
        terminal::enable_raw_mode()?;
        execute!(
            io::stdout(),
            terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture,
            crossterm::event::EnableBracketedPaste
        )?;

        // Clear screen and set up initial state
        execute!(
            io::stdout(),
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        info!("Terminal integration initialized successfully");
        Ok(())
    }

    /// Handle terminal events
    pub fn handle_event(&mut self, event: Event) -> Result<Option<TuiState>> {
        match event {
            Event::Key(key_event) => self.handle_key_event(key_event),
            Event::Mouse(mouse_event) => self.handle_mouse_event(mouse_event),
            Event::Paste(pasted_text) => self.handle_paste_event(pasted_text),
            _ => Ok(None),
        }
    }

    /// Handle keyboard events
    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<Option<TuiState>> {
        match key_event.code {
            KeyCode::Enter => {
                if !self.current_command.trim().is_empty() {
                    self.execute_command()?;
                }
                Ok(None)
            }
            KeyCode::Char(c) => {
                self.current_command.push(c);
                Ok(None)
            }
            KeyCode::Backspace => {
                self.current_command.pop();
                Ok(None)
            }
            KeyCode::Delete => {
                if !self.current_command.is_empty() {
                    self.current_command.clear();
                }
                Ok(None)
            }
            KeyCode::Up => {
                self.navigate_history(-1);
                Ok(None)
            }
            KeyCode::Down => {
                self.navigate_history(1);
                Ok(None)
            }
            KeyCode::Tab => {
                if self.auto_completion {
                    self.handle_auto_completion()?;
                }
                Ok(None)
            }
            KeyCode::Left => {
                // Handle cursor movement (simplified)
                Ok(None)
            }
            KeyCode::Right => {
                // Handle cursor movement (simplified)
                Ok(None)
            }
            KeyCode::Home => {
                // Move to beginning of line
                Ok(None)
            }
            KeyCode::End => {
                // Move to end of line
                Ok(None)
            }
            KeyCode::Esc => {
                // Return to previous state
                Ok(Some(TuiState::FileBrowser))
            }
            _ => Ok(None),
        }
    }

    /// Handle mouse events
    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) -> Result<Option<TuiState>> {
        match mouse_event.kind {
            MouseEventKind::ScrollUp => {
                // Scroll output buffer up
                Ok(None)
            }
            MouseEventKind::ScrollDown => {
                // Scroll output buffer down
                Ok(None)
            }
            MouseEventKind::Down(_button) => {
                // Handle mouse click
                debug!(
                    "Mouse click at: ({}, {})",
                    mouse_event.column, mouse_event.row
                );
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Handle paste events
    fn handle_paste_event(&mut self, pasted_text: String) -> Result<Option<TuiState>> {
        self.current_command.push_str(&pasted_text);
        Ok(None)
    }

    /// Execute a terminal command
    fn execute_command(&mut self) -> Result<()> {
        let command = self.current_command.trim().to_string();
        if command.is_empty() {
            // Clear current command even if it's just whitespace
            self.current_command.clear();
            return Ok(());
        }

        // Add to history
        self.command_history.push(command.clone());
        self.history_index = self.command_history.len();

        // Create command for execution
        let terminal_command = TerminalCommand {
            id: uuid::Uuid::new_v4(),
            command: command.clone(),
            timestamp: chrono::Utc::now(),
        };

        // Send command for async execution
        self.command_sender.send(terminal_command.clone())?;

        // Add command to output buffer
        self.output_buffer.push(TerminalOutput {
            id: uuid::Uuid::new_v4(),
            content: format!("$ {}", command),
            output_type: "command".to_string(),
            timestamp: chrono::Utc::now(),
        });

        // Clear current command
        self.current_command.clear();

        info!("Executed terminal command: {}", command);
        Ok(())
    }

    /// Navigate command history
    fn navigate_history(&mut self, direction: isize) {
        if self.command_history.is_empty() {
            return;
        }

        let new_index = (self.history_index as isize + direction) as usize;

        if new_index < self.command_history.len() {
            self.history_index = new_index;
            self.current_command = self.command_history[self.history_index].clone();
        } else if new_index == self.command_history.len() {
            self.history_index = new_index;
            self.current_command.clear();
        }
    }

    /// Handle auto-completion
    fn handle_auto_completion(&mut self) -> Result<()> {
        if self.current_command.is_empty() {
            return Ok(());
        }

        let current_word = self.current_command.split_whitespace().last().unwrap_or("");

        // Don't auto-complete if current word is empty (whitespace only)
        if current_word.is_empty() {
            return Ok(());
        }

        // Priority-based auto-completion with specific mappings
        let completion_map = vec![
            ("gi", "git"),
            ("car", "cargo"),
            ("np", "npm"),
            ("py", "python"),
            ("ru", "rustc"),
            ("l", "ls"),
            ("c", "cargo"),
            ("mk", "mkdir"),
            ("r", "rustc"),
            ("cp", "cp"),
            ("mv", "mv"),
            ("ca", "cat"),
            ("gr", "grep"),
            ("f", "find"),
            ("ma", "make"),
            ("cm", "cmake"),
            ("doc", "docker"),
            ("kub", "kubectl"),
        ];

        // Check for exact prefix matches first
        for (prefix, completion) in &completion_map {
            if current_word == *prefix {
                self.current_command
                    .push_str(&completion[current_word.len()..]);
                return Ok(());
            }
        }

        // Fallback to simple auto-completion for common commands
        let common_commands = vec![
            "git", "cargo", "cat", "npm", "python", "rustc", "ls", "cd", "mkdir", "rm", "cp", "mv",
            "grep", "find", "make", "cmake", "docker", "kubectl",
        ];

        for command in common_commands {
            if command.starts_with(current_word) && command != current_word {
                let completion = command[current_word.len()..].to_string();
                self.current_command.push_str(&completion);
                break;
            }
        }

        Ok(())
    }

    /// Render the terminal interface
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // Output area
                Constraint::Length(3), // Command input area
            ])
            .split(area);

        // Render output area
        self.render_output_area(frame, chunks[0]);

        // Render command input area
        self.render_command_input(frame, chunks[1]);
    }

    /// Render the output area
    fn render_output_area(&self, frame: &mut Frame, area: Rect) {
        let output_text: Vec<Line> = self
            .output_buffer
            .iter()
            .map(|output| {
                let style = match output.output_type.as_str() {
                    "command" => Style::default()
                        .fg(ratatui::style::Color::Green)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                    "error" => Style::default().fg(ratatui::style::Color::Red),
                    "success" => Style::default().fg(ratatui::style::Color::Cyan),
                    _ => Style::default().fg(ratatui::style::Color::White),
                };

                Line::from(Span::styled(output.content.clone(), style))
            })
            .collect();

        let paragraph = Paragraph::new(Text::from(output_text))
            .block(
                Block::default()
                    .title("Terminal Output")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render the command input area
    fn render_command_input(&self, frame: &mut Frame, area: Rect) {
        let input_text = format!("$ {}", self.current_command);

        let paragraph = Paragraph::new(input_text)
            .block(
                Block::default()
                    .title("Command Input")
                    .borders(Borders::ALL),
            )
            .style(Style::default().fg(ratatui::style::Color::Yellow));

        frame.render_widget(paragraph, area);
    }

    /// Process command execution results
    pub fn process_command_results(&mut self) -> Result<()> {
        if let Some(mut receiver) = self.command_receiver.take() {
            while let Ok(command) = receiver.try_recv() {
                // Execute command and capture output
                let output = self.execute_shell_command(&command.command)?;

                // Add output to buffer
                self.output_buffer.push(TerminalOutput {
                    id: uuid::Uuid::new_v4(),
                    content: output,
                    output_type: "output".to_string(),
                    timestamp: chrono::Utc::now(),
                });
            }
            self.command_receiver = Some(receiver);
        }
        Ok(())
    }

    /// Execute shell command and capture output
    fn execute_shell_command(&self, command: &str) -> Result<String> {
        use std::process::Command;

        debug!("Executing shell command: {}", command);

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(&["/C", command]).output()?
        } else {
            Command::new("sh").args(&["-c", command]).output()?
        };

        let result = if output.status.success() {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            format!("Error: {}", error)
        };

        debug!("Command output: {}", result);
        Ok(result)
    }

    /// Get command history
    pub fn get_command_history(&self) -> &[String] {
        &self.command_history
    }

    /// Get current command
    pub fn get_current_command(&self) -> &str {
        &self.current_command
    }

    /// Clear output buffer
    pub fn clear_output(&mut self) {
        self.output_buffer.clear();
    }

    /// Toggle shell integration
    pub fn toggle_shell_integration(&mut self) {
        self.shell_integration = !self.shell_integration;
        info!("Shell integration: {}", self.shell_integration);
    }

    /// Toggle auto-completion
    pub fn toggle_auto_completion(&mut self) {
        self.auto_completion = !self.auto_completion;
        info!("Auto-completion: {}", self.auto_completion);
    }

    /// Toggle syntax highlighting
    pub fn toggle_syntax_highlighting(&mut self) {
        self.syntax_highlighting = !self.syntax_highlighting;
        info!("Syntax highlighting: {}", self.syntax_highlighting);
    }

    /// Cleanup terminal integration
    pub fn cleanup(&self) -> Result<()> {
        execute!(
            io::stdout(),
            terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
            crossterm::event::DisableBracketedPaste,
            ResetColor,
            cursor::Show
        )?;
        terminal::disable_raw_mode()?;
        info!("Terminal integration cleaned up");
        Ok(())
    }
}

// Import all test modules
#[cfg(test)]
mod tests {
    // Import all test modules
    pub use super::auto_completion_tests::*;
    pub use super::command_execution_tests::*;
    pub use super::creation_tests::*;
    pub use super::event_handling_tests::*;
}
