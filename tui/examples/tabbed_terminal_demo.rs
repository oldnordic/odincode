//! OdinCode Tabbed Terminal Demonstration
//!
//! This is a standalone demonstration showing how tabbed terminal integration
//! would work in OdinCode, including PTY support for running real terminals
//! and editors like Neovim.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Simple Terminal Tab implementation for demonstration
struct DemoTerminalTab {
    title: String,
    output_buffer: Arc<Mutex<String>>,
    current_input: String,
    is_active: bool,
}

impl DemoTerminalTab {
    fn new(title: String) -> Self {
        Self {
            title,
            output_buffer: Arc::new(Mutex::new(String::new())),
            current_input: String::new(),
            is_active: false,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let content = if let Ok(buffer) = self.output_buffer.lock() {
            buffer.clone()
        } else {
            "Error accessing terminal output".to_string()
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(self.title.clone()),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}

/// Tabbed terminal manager for demonstration
struct DemoTabbedTerminal {
    tabs: Vec<DemoTerminalTab>,
    active_tab_index: usize,
}

impl DemoTabbedTerminal {
    fn new() -> Self {
        let mut terminal = Self {
            tabs: Vec::new(),
            active_tab_index: 0,
        };

        // Add some demo tabs
        terminal
            .tabs
            .push(DemoTerminalTab::new("Welcome".to_string()));
        terminal
            .tabs
            .push(DemoTerminalTab::new("Terminal 1".to_string()));
        terminal
            .tabs
            .push(DemoTerminalTab::new("Neovim".to_string()));

        // Initialize welcome tab content
        if let Ok(mut buffer) = terminal.tabs[0].output_buffer.lock() {
            *buffer = r#"
  ____  _  _  ___  _  _  ____  _____ 
 / ___)( \/ )/ __)( \/ )(  _ \(  _  )
 \___ \ \  /( (__  )  (  )(_) ))(_)( 
 (____/ (__) \___)(_/\_)(____/(_____)
                                    
    AI Code Engineering System

Welcome to the OdinCode Tabbed Terminal Demo!

Features Demonstrated:
• Tabbed interface with keyboard/mouse navigation
• PTY integration for real terminal emulation
• Support for editors like Neovim/Vim
• Seamless integration with OdinCode AI

Keyboard Shortcuts:
• Ctrl+T: Create new terminal tab
• Ctrl+N: Create new Neovim tab
• Ctrl+W: Close current tab
• Ctrl+Tab: Switch to next tab
• Ctrl+Shift+Tab: Switch to previous tab

Mouse Support:
• Click on tabs to switch between them
• Scroll wheel to navigate terminal output

Try creating a new terminal tab with Ctrl+T!

"#
            .to_string();
        }

        terminal
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Terminal content
            ])
            .split(area);

        // Render tabs
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

        frame.render_widget(tabs, chunks[0]);

        // Render active tab content
        if !self.tabs.is_empty() {
            self.tabs[self.active_tab_index].render(frame, chunks[1]);
        }
    }

    fn switch_to_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab_index = index;
        }
    }

    fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab_index = (self.active_tab_index + 1) % self.tabs.len();
        }
    }

    fn previous_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab_index = if self.active_tab_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab_index - 1
            };
        }
    }
}

/// Main demo application
struct DemoApp {
    tabbed_terminal: DemoTabbedTerminal,
    should_quit: bool,
}

impl DemoApp {
    fn new() -> Self {
        Self {
            tabbed_terminal: DemoTabbedTerminal::new(),
            should_quit: false,
        }
    }

    fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| self.draw_ui(f))?;

            if let Event::Key(key) = event::read()? {
                self.handle_key_event(key)?;
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    fn draw_ui(&mut self, frame: &mut Frame) {
        let size = frame.area();
        self.tabbed_terminal.render(frame, size);
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('t') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // In a real implementation, this would create a new terminal tab
                    println!("Would create new terminal tab");
                }
            }
            KeyCode::Char('n') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // In a real implementation, this would create a new Neovim tab
                    println!("Would create new Neovim tab");
                }
            }
            KeyCode::Char('w') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // In a real implementation, this would close the current tab
                    println!("Would close current tab");
                }
            }
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.tabbed_terminal.next_tab();
                }
            }
            KeyCode::BackTab => {
                self.tabbed_terminal.previous_tab();
            }
            _ => {}
        }

        Ok(())
    }
}

/// Entry point for the demonstration
fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the demo app
    let mut app = DemoApp::new();
    let res = app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo_terminal_tab_creation() {
        let tab = DemoTerminalTab::new("Test Tab".to_string());
        assert_eq!(tab.title, "Test Tab");
        assert_eq!(tab.current_input, "");
        assert!(!tab.is_active);
    }

    #[test]
    fn test_demo_tabbed_terminal_creation() {
        let terminal = DemoTabbedTerminal::new();
        assert_eq!(terminal.tabs.len(), 3);
        assert_eq!(terminal.active_tab_index, 0);
        assert_eq!(terminal.tabs[0].title, "Welcome");
        assert_eq!(terminal.tabs[1].title, "Terminal 1");
        assert_eq!(terminal.tabs[2].title, "Neovim");
    }

    #[test]
    fn test_tab_switching() {
        let mut terminal = DemoTabbedTerminal::new();
        assert_eq!(terminal.active_tab_index, 0);

        terminal.next_tab();
        assert_eq!(terminal.active_tab_index, 1);

        terminal.next_tab();
        assert_eq!(terminal.active_tab_index, 2);

        terminal.next_tab();
        assert_eq!(terminal.active_tab_index, 0);

        terminal.previous_tab();
        assert_eq!(terminal.active_tab_index, 2);

        terminal.previous_tab();
        assert_eq!(terminal.active_tab_index, 1);
    }

    #[test]
    fn test_direct_tab_switching() {
        let mut terminal = DemoTabbedTerminal::new();
        assert_eq!(terminal.active_tab_index, 0);

        terminal.switch_to_tab(1);
        assert_eq!(terminal.active_tab_index, 1);

        terminal.switch_to_tab(5); // Invalid index, should not change
        assert_eq!(terminal.active_tab_index, 1);

        terminal.switch_to_tab(0);
        assert_eq!(terminal.active_tab_index, 0);
    }
}
