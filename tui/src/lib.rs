//! OdinCode TUI Module
//!
//! The TUI module provides a rich terminal user interface for the OdinCode system,
//! allowing users to interact with the AI coding assistant directly from the terminal.

pub mod app;
pub mod models;
pub mod ui;

pub use app::TuiApp;
pub use ui::render;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::Terminal;
use std::io;
use tracing::info;

use odincode_agents::AgentCoordinator;
use odincode_core::CodeEngine;
use odincode_ltmc::LTMManager;
use odincode_tools::ToolManager;

/// Main TUI application runner
pub struct TuiRunner {
    /// Shared core engine
    core_engine: std::sync::Arc<CodeEngine>,
    /// Shared LTMC manager
    ltmc_manager: std::sync::Arc<LTMManager>,
    /// Shared agent coordinator
    agent_coordinator: AgentCoordinator,
    /// Shared tool manager
    tool_manager: ToolManager,
}

impl TuiRunner {
    /// Create a new TUI runner
    pub fn new(
        core_engine: std::sync::Arc<CodeEngine>,
        ltmc_manager: std::sync::Arc<LTMManager>,
        agent_coordinator: AgentCoordinator,
        tool_manager: ToolManager,
    ) -> Self {
        Self {
            core_engine,
            ltmc_manager,
            agent_coordinator,
            tool_manager,
        }
    }

    /// Run the TUI application
    pub async fn run(&self) -> Result<()> {
        info!("Starting OdinCode TUI application...");

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = ratatui::backend::CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Create and initialize the application
        let mut app = TuiApp::new();
        app.initialize(&self.core_engine, &self.agent_coordinator)
            .await?;

        // Run the application
        let mut continue_running = true;
        while continue_running {
            // Draw the UI
            terminal.draw(|f| render(&mut app, f))?;

            // Wait for an event
            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    continue_running = app.handle_key_event(key)?;
                }
            }
        }

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        info!("OdinCode TUI application finished");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_app_creation() {
        let app = TuiApp::new();
        assert_eq!(app.title, "OdinCode - AI Code Engineering System");
        // Note: Can't directly match enum variants without PartialEq, so we'll just verify creation
    }
}
