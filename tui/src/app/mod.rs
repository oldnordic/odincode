//! TUI App Module
//!
//! This module contains the main TUI application logic.

pub mod key_handlers;
pub mod terminal_integration;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tracing::info;
use uuid::Uuid;

use odincode_agents::{Agent, AgentCoordinator};
use odincode_core::{AnalysisResult, CodeEngine, CodeFile};
use odincode_ltmc::{LTMManager, LearningPattern};
use odincode_tools::ToolManager;

use crate::app::key_handlers::{
    handle_agent_selection_keys, handle_analysis_results_keys, handle_code_editor_keys,
    handle_file_browser_keys, handle_ltmc_view_keys, handle_tool_selection_keys,
};
use crate::app::terminal_integration::TerminalIntegration;
use crate::models::TuiState;

/// Represents the main TUI application
pub struct TuiApp {
    /// Current application state
    pub current_state: TuiState,
    /// List of files
    pub files: Vec<CodeFile>,
    /// Currently selected file index
    pub selected_file_index: usize,
    /// List of agents
    pub agents: Vec<Agent>,
    /// Currently selected agent index
    pub selected_agent_index: Option<usize>,
    /// List of LTMC patterns
    pub ltmc_patterns: Vec<LearningPattern>,
    /// Currently selected LTMC pattern index
    pub selected_pattern_index: Option<usize>,
    /// List of tools
    pub tools: Vec<String>, // Simplified for this example
    /// Currently selected tool index
    pub selected_tool_index: Option<usize>,
    /// Code content for the editor
    pub code_content: String,
    /// Analysis results
    pub analysis_results: Vec<AnalysisResult>,
    /// Current tab index
    pub current_tab: usize,
    /// Application title
    pub title: String,
    /// Enhanced terminal integration
    pub terminal_integration: TerminalIntegration,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new() -> Self {
        Self {
            current_state: TuiState::FileBrowser,
            files: Vec::new(),
            selected_file_index: 0,
            agents: Vec::new(),
            selected_agent_index: None,
            ltmc_patterns: Vec::new(),
            selected_pattern_index: None,
            tools: Vec::new(),
            selected_tool_index: None,
            code_content: String::new(),
            analysis_results: Vec::new(),
            current_tab: 0,
            title: "OdinCode - AI Code Engineering System".to_string(),
            terminal_integration: TerminalIntegration::new(),
        }
    }

    /// Initialize the TUI application
    pub async fn initialize<'a>(
        &mut self,
        core_engine: &std::sync::Arc<CodeEngine>,
        agent_coordinator: &AgentCoordinator,
    ) -> Result<()> {
        info!("Initializing TUI application...");

        // Load files from the core engine
        // In a real implementation, we would query the core engine for loaded files
        // For now, we'll initialize with an empty list

        // Load agents from the agent coordinator
        self.agents = agent_coordinator.get_all_agents().await?;
        info!("Loaded {} agents", self.agents.len());

        // Initialize tools list
        self.tools = vec![
            "Linter".to_string(),
            "Formatter".to_string(),
            "Test Runner".to_string(),
            "Build System".to_string(),
        ];

        info!("TUI application initialized successfully");
        Ok(())
    }

    /// Handle key events
    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<bool> {
        match self.current_state {
            TuiState::FileBrowser => handle_file_browser_keys(self, key_event)?,
            TuiState::CodeEditor => handle_code_editor_keys(self, key_event)?,
            TuiState::AgentSelection => handle_agent_selection_keys(self, key_event)?,
            TuiState::AnalysisResults => handle_analysis_results_keys(self, key_event)?,
            TuiState::LTMCView => handle_ltmc_view_keys(self, key_event)?,
            TuiState::ToolSelection => handle_tool_selection_keys(self, key_event)?,
            TuiState::TerminalIntegration => {
                // Handle terminal integration events
                if let Some(new_state) = self
                    .terminal_integration
                    .handle_event(crossterm::event::Event::Key(key_event))?
                {
                    self.current_state = new_state;
                }
            }
        }

        // Return true if the application should continue running, false to exit
        Ok(key_event.code != KeyCode::Char('q')
            || key_event.modifiers.contains(KeyModifiers::CONTROL))
    }
}
