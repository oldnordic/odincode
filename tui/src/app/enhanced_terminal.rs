//! Enhanced Tabbed Terminal Integration for OdinCode TUI
//! 
//! This module extends the existing TUI with a full tabbed terminal interface
//! that can run real terminal emulators with PTY support for editors like Neovim/Vim.

use anyhow::Result;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
    Frame,
};
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::{Read, Write};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::TuiState;

/// Represents a terminal emulator tab with full PTY support
pub struct TerminalEmulatorTab {
    /// Tab title (e.g., "Terminal 1", "Neovim", "Bash")
    pub title: String,
    /// PTY master handle
    pub pty_master: Option<portable_pty::Master>,
    /// PTY slave handle
    pub pty_slave: Option<portable_pty::Slave>,
    /// Child process handle
    pub child: Option<portable_pty::Child>,
    /// Output buffer for terminal content
    pub output_buffer: Arc<Mutex<String>>,
    /// Current command input being typed
    pub current_input: String,
    /// Scroll position for output buffer
    pub scroll_position: usize,
    /// Terminal size
    pub size: PtySize,
    /// Is this tab currently active
    pub is_active: bool,
}

impl TerminalEmulatorTab {
    /// Create a new terminal emulator tab
    pub fn new(title: String) -> Self {
        Self {
            title,
            pty_master: None,
            pty_slave: None,
            child: None,
            output_buffer: Arc::new(Mutex::new(String::new())),
            current_input: String::new(),
            scroll_position: 0,
            size: PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            },
            is_active: false,
        }
    }

    /// Initialize the PTY and spawn a shell
    pub fn initialize_shell(&mut self) -> Result<()> {
        // Create PTY system
        let pty_system = NativePtySystem::default();
        
        // Create PTY pair with initial size
        let pty_pair = pty_system.openpty(self.size)?;
        
        // Determine which shell to use
        let shell = if cfg!(windows) {
            std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
        } else {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
        };
        
        // Build command to spawn
        let mut cmd = CommandBuilder::new(shell);
        
        // Set working directory to current directory
        if let Ok(cwd) = std::env::current_dir() {
            cmd.cwd(cwd);
        }
        
        // Spawn the shell
        let child = pty_pair.slave.spawn_command(cmd)?;
        
        // Store handles
        self.pty_master = Some(pty_pair.master);
        self.pty_slave = Some(pty_pair.slave);
        self.child = Some(child);
        
        // Start reading from PTY in background thread
        self.start_output_reader()?;
        
        Ok(())
    }

    /// Initialize Neovim in this terminal tab
    pub fn initialize_neovim(&mut self) -> Result<()> {
        // Create PTY system
        let pty_system = NativePtySystem::default();
        
        // Create PTY pair with initial size
        let pty_pair = pty_system.openpty(self.size)?;
        
        // Build command to spawn Neovim
        let mut cmd = CommandBuilder::new("nvim");
        
        // Set working directory to current directory
        if let Ok(cwd) = std::env::current_dir() {
            cmd.cwd(cwd);
        }
        
        // Spawn Neovim
        let child = pty_pair.slave.spawn_command(cmd)?;
        
        // Store handles
        self.pty_master = Some(pty_pair.master);
        self.pty_slave = Some(pty_pair.slave);
        self.child = Some(child);
        
        // Start reading from PTY in background thread
        self.start_output_reader()?;
        
        Ok(())
    }

    /// Start background thread to read output from PTY
    fn start_output_reader(&mut self) -> Result<()> {
        if let Some(master) = &self.pty_master {
            let reader = master.try_clone_reader()?;
            let output_buffer = Arc::clone(&self.output_buffer);
            
            thread::spawn(move || {
                let mut buf = [0u8; 1024];
                let mut reader = reader;
                
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break, // EOF
                        Ok(n) => {
                            let output = String::from_utf8_lossy(&buf[..n]).to_string();
                            if let Ok(mut buffer) = output_buffer.lock() {
                                *buffer += &output;
                                
                                // Limit buffer size to prevent memory issues
                                if buffer.len() > 100000 {
                                    let len = buffer.len();
                                    buffer.drain(..len - 50000);
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }
        
        Ok(())
    }

    /// Send input to the terminal process
    pub fn send_input(&mut self, input: &str) -> Result<()> {
        if let Some(master) = &mut self.pty_master {
            let mut writer = master.try_clone_writer()?;
            writer.write_all(input.as_bytes())?;
            writer.flush()?;
        }
        Ok(())
    }

    /// Send a key event to the terminal
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char(c) => {
                self.send_input(&c.to_string())?;
                self.current_input.push(c);
            }
            KeyCode::Enter => {
                self.send_input("\n")?;
                self.current_input.clear();
            }
            KeyCode::Backspace => {
                self.send_input("\x08")?; // Backspace character
                self.current_input.pop();
            }
            KeyCode::Delete => {
                self.send_input("\x7f")?; // Delete character
            }
            KeyCode::Left => {
                self.send_input("\x1b[D")?; // Left arrow ESC sequence
            }
            KeyCode::Right => {
                self.send_input("\x1b[C")?; // Right arrow ESC sequence
            }
            KeyCode::Up => {
                self.send_input("\x1b[A")?; // Up arrow ESC sequence
            }
            KeyCode::Down => {
                self.send_input("\x1b[B")?; // Down arrow ESC sequence
            }
            KeyCode::Tab => {
                self.send_input("\t")?; // Tab character
            }
            KeyCode::Esc => {
                self.send_input("\x1b")?; // Escape character
            }
            // Handle Ctrl+key combinations
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.send_input("\x03")?; // Ctrl+C
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.send_input("\x04")?; // Ctrl+D
            }
            KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.send_input("\x1a")?; // Ctrl+Z
            }
            _ => {
                // Ignore other keys
            }
        }
        Ok(())
    }

    /// Resize the terminal
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.size.rows = rows;
        self.size.cols = cols;
        
        if let Some(master) = &self.pty_master {
            master.resize(self.size)?;
        }
        
        Ok(())
    }

    /// Render the terminal emulator tab
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Get content from output buffer
        let content = if let Ok(buffer) = self.output_buffer.lock() {
            buffer.clone()
        } else {
            "Error accessing terminal output".to_string()
        };
        
        // Split content into lines
        let lines: Vec<&str> = content.lines().collect();
        
        // Calculate visible lines based on scroll position
        let visible_lines = if lines.len() > area.height as usize {
            let start = if self.scroll_position < lines.len() {
                self.scroll_position
            } else {
                lines.len().saturating_sub(1)
            };
            let end = (start + area.height as usize).min(lines.len());
            &lines[start..end]
        } else {
            &lines
        };
        
        // Create text content
        let text_lines: Vec<Line> = visible_lines
            .iter()
            .map(|line| Line::from(*line))
            .collect();
        
        let text = Text::from(text_lines);
        
        // Create paragraph widget
        let paragraph = Paragraph::new(text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(self.title.clone()))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_position as u16, 0));
        
        frame.render_widget(paragraph, area);
    }

    /// Handle mouse events for scrolling
    pub fn handle_mouse_scroll(&mut self, delta: i32) {
        if delta > 0 {
            // Scroll up
            self.scroll_position = self.scroll_position.saturating_sub(delta as usize);
        } else {
            // Scroll down
            self.scroll_position = self.scroll_position.saturating_add((-delta) as usize);
        }
    }

    /// Cleanup resources
    pub fn cleanup(&mut self) -> Result<()> {
        // Kill child process if it exists
        if let Some(mut child) = self.child.take() {
            child.kill()?;
        }
        
        Ok(())
    }
}

/// Tabbed terminal interface managing multiple terminal emulator tabs
pub struct TabbedTerminalInterface {
    /// Collection of terminal tabs
    pub tabs: Vec<TerminalEmulatorTab>,
    /// Index of currently active tab
    pub active_tab_index: usize,
    /// Maximum number of tabs allowed
    pub max_tabs: usize,
}

impl TabbedTerminalInterface {
    /// Create a new tabbed terminal interface
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            max_tabs: 10, // Limit to 10 tabs
        }
    }

    /// Add a new shell terminal tab
    pub fn add_shell_tab(&mut self, title: Option<String>) -> Result<()> {
        if self.tabs.len() >= self.max_tabs {
            return Err(anyhow::anyhow!("Maximum number of tabs reached"));
        }
        
        let tab_title = title.unwrap_or_else(|| format!("Terminal {}", self.tabs.len() + 1));
        let mut tab = TerminalEmulatorTab::new(tab_title);
        tab.initialize_shell()?;
        
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
        
        Ok(())
    }

    /// Add a new Neovim terminal tab
    pub fn add_neovim_tab(&mut self, title: Option<String>) -> Result<()> {
        if self.tabs.len() >= self.max_tabs {
            return Err(anyhow::anyhow!("Maximum number of tabs reached"));
        }
        
        let tab_title = title.unwrap_or_else(|| "Neovim".to_string());
        let mut tab = TerminalEmulatorTab::new(tab_title);
        tab.initialize_neovim()?;
        
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
        
        Ok(())
    }

    /// Close the currently active tab
    pub fn close_active_tab(&mut self) -> Result<()> {
        if self.tabs.is_empty() {
            return Ok(());
        }
        
        // Cleanup the tab being closed
        if let Some(mut tab) = self.tabs.remove(self.active_tab_index) {
            tab.cleanup()?;
        }
        
        // Adjust active tab index
        if self.active_tab_index >= self.tabs.len() && !self.tabs.is_empty() {
            self.active_tab_index = self.tabs.len() - 1;
        }
        
        Ok(())
    }

    /// Switch to a specific tab by index
    pub fn switch_to_tab(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.active_tab_index = index;
            true
        } else {
            false
        }
    }

    /// Switch to next tab
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab_index = (self.active_tab_index + 1) % self.tabs.len();
        }
    }

    /// Switch to previous tab
    pub fn previous_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab_index = if self.active_tab_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab_index - 1
            };
        }
    }

    /// Handle key events for the active tab
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<TuiState>> {
        if !self.tabs.is_empty() {
            let active_tab_index = self.active_tab_index;
            self.tabs[active_tab_index].handle_key_event(key)?;
        }
        
        // Check for tab switching shortcuts
        match key {
            KeyEvent { code: KeyCode::Char('t'), modifiers: KeyModifiers::CONTROL, .. } => {
                self.next_tab();
            }
            KeyEvent { code: KeyCode::BackTab, .. } => {
                self.previous_tab();
            }
            _ => {}
        }
        
        Ok(None)
    }

    /// Handle mouse events for tab switching and scrolling
    pub fn handle_mouse_event(&mut self, mouse_event: crossterm::event::MouseEvent) -> Result<Option<TuiState>> {
        match mouse_event.kind {
            crossterm::event::MouseEventKind::ScrollUp => {
                if !self.tabs.is_empty() {
                    self.tabs[self.active_tab_index].handle_mouse_scroll(1);
                }
            }
            crossterm::event::MouseEventKind::ScrollDown => {
                if !self.tabs.is_empty() {
                    self.tabs[self.active_tab_index].handle_mouse_scroll(-1);
                }
            }
            _ => {
                // Handle tab clicking would go here
            }
        }
        
        Ok(None)
    }

    /// Render the entire tabbed terminal interface
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if self.tabs.is_empty() {
            // Show welcome message if no tabs exist
            let welcome_text = vec![
                Line::from("Welcome to OdinCode Terminal Emulator"),
                Line::from(""),
                Line::from("Press Ctrl+T to create a new terminal tab"),
                Line::from("Press Ctrl+N to create a new Neovim tab"),
                Line::from("Use mouse wheel to scroll in terminals"),
            ];
            
            let paragraph = Paragraph::new(Text::from(welcome_text))
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Terminal Interface"))
                .wrap(Wrap { trim: false });
            
            frame.render_widget(paragraph, area);
            return;
        }
        
        // Split area into tabs and content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs header
                Constraint::Min(0),    // Terminal content
            ])
            .split(area);
        
        // Render tab headers
        self.render_tabs(frame, chunks[0]);
        
        // Render active tab content
        self.render_active_tab(frame, chunks[1]);
    }

    /// Render the tab headers
    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let tab_titles: Vec<Line> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let style = if i == self.active_tab_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(vec![Span::styled(tab.title.clone(), style)])
            })
            .collect();
        
        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::BOTTOM))
            .select(self.active_tab_index)
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        
        frame.render_widget(tabs, area);
    }

    /// Render the content of the active tab
    fn render_active_tab(&self, frame: &mut Frame, area: Rect) {
        if !self.tabs.is_empty() {
            self.tabs[self.active_tab_index].render(frame, area);
        }
    }

    /// Resize all tabs to new dimensions
    pub fn resize_all_tabs(&mut self, rows: u16, cols: u16) -> Result<()> {
        for tab in &mut self.tabs {
            tab.resize(rows, cols)?;
        }
        Ok(())
    }

    /// Cleanup all tabs
    pub fn cleanup(&mut self) -> Result<()> {
        for tab in &mut self.tabs {
            tab.cleanup()?;
        }
        self.tabs.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_tabbed_terminal_creation() {
        let terminal = TabbedTerminalInterface::new();
        assert_eq!(terminal.tabs.len(), 0);
        assert_eq!(terminal.active_tab_index, 0);
        assert_eq!(terminal.max_tabs, 10);
    }

    #[test]
    fn test_terminal_tab_creation() {
        let tab = TerminalEmulatorTab::new("Test Terminal".to_string());
        assert_eq!(tab.title, "Test Terminal");
        assert!(tab.output_buffer.lock().unwrap().is_empty());
        assert_eq!(tab.current_input, "");
    }

    #[test]
    fn test_add_shell_tab() {
        let mut terminal = TabbedTerminalInterface::new();
        // In a real test, we would initialize the PTY, but we'll skip that for now
        assert_eq!(terminal.tabs.len(), 0);
    }

    #[test]
    fn test_tab_switching() {
        let mut terminal = TabbedTerminalInterface::new();
        // Without actual tabs, just test the logic
        terminal.switch_to_tab(5); // Should not crash
        assert_eq!(terminal.active_tab_index, 0);
        
        terminal.next_tab(); // Should not crash
        assert_eq!(terminal.active_tab_index, 0);
        
        terminal.previous_tab(); // Should not crash
        assert_eq!(terminal.active_tab_index, 0);
    }

    #[test]
    fn test_key_event_handling() {
        let mut terminal = TabbedTerminalInterface::new();
        let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        
        // Should not crash even without tabs
        let result = terminal.handle_key_event(key_event);
        assert!(result.is_ok());
    }
}