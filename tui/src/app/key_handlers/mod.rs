//! TUI Key Handlers Module
//!
//! This module contains the key handling logic for the TUI system.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::info;

use crate::app::TuiApp;
use crate::models::TuiState;

/// Handle keys in file browser state
pub fn handle_file_browser_keys(app: &mut TuiApp, key_event: KeyEvent) -> Result<()> {
    match key_event.code {
        KeyCode::Up => {
            if app.files.len() > 0 {
                if app.selected_file_index > 0 {
                    app.selected_file_index -= 1;
                } else {
                    app.selected_file_index = app.files.len() - 1;
                }
            }
        }
        KeyCode::Down => {
            if app.files.len() > 0 {
                if app.selected_file_index < app.files.len() - 1 {
                    app.selected_file_index += 1;
                } else {
                    app.selected_file_index = 0;
                }
            }
        }
        KeyCode::Enter => {
            // Load the selected file into the editor
            if app.selected_file_index < app.files.len() {
                if let Some(file) = app.files.get(app.selected_file_index) {
                    app.code_content = file.content.clone();
                    app.current_state = TuiState::CodeEditor;
                }
            }
        }
        KeyCode::Char('a') => {
            app.current_state = TuiState::AgentSelection;
        }
        KeyCode::Char('t') => {
            app.current_state = TuiState::ToolSelection;
        }
        KeyCode::Char('l') => {
            app.current_state = TuiState::LTMCView;
        }
        _ => {}
    }
    Ok(())
}

/// Handle keys in code editor state
pub fn handle_code_editor_keys(app: &mut TuiApp, key_event: KeyEvent) -> Result<()> {
    match key_event.code {
        KeyCode::Char('b') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_state = TuiState::FileBrowser;
        }
        KeyCode::Char('a') => {
            app.current_state = TuiState::AgentSelection;
        }
        _ => {}
    }
    Ok(())
}

/// Handle keys in agent selection state
pub fn handle_agent_selection_keys(app: &mut TuiApp, key_event: KeyEvent) -> Result<()> {
    match key_event.code {
        KeyCode::Up => {
            if app.agents.len() > 0 {
                if let Some(index) = app.selected_agent_index {
                    if index > 0 {
                        app.selected_agent_index = Some(index - 1);
                    } else {
                        app.selected_agent_index = Some(app.agents.len() - 1);
                    }
                } else {
                    app.selected_agent_index = Some(0);
                }
            }
        }
        KeyCode::Down => {
            if app.agents.len() > 0 {
                if let Some(index) = app.selected_agent_index {
                    if index < app.agents.len() - 1 {
                        app.selected_agent_index = Some(index + 1);
                    } else {
                        app.selected_agent_index = Some(0);
                    }
                } else {
                    app.selected_agent_index = Some(0);
                }
            }
        }
        KeyCode::Enter => {
            // Execute the selected agent on the current file
            if let Some(index) = app.selected_agent_index {
                if index < app.agents.len() {
                    // In a real implementation, we would execute the agent
                    info!("Executing agent: {}", app.agents[index].name);
                }
            }
        }
        KeyCode::Char('b') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_state = TuiState::FileBrowser;
        }
        _ => {}
    }
    Ok(())
}

/// Handle keys in analysis results state
pub fn handle_analysis_results_keys(app: &mut TuiApp, key_event: KeyEvent) -> Result<()> {
    match key_event.code {
        KeyCode::Char('b') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_state = TuiState::FileBrowser;
        }
        _ => {}
    }
    Ok(())
}

/// Handle keys in LTMC view state
pub fn handle_ltmc_view_keys(app: &mut TuiApp, key_event: KeyEvent) -> Result<()> {
    match key_event.code {
        KeyCode::Up => {
            if app.ltmc_patterns.len() > 0 {
                if let Some(index) = app.selected_pattern_index {
                    if index > 0 {
                        app.selected_pattern_index = Some(index - 1);
                    } else {
                        app.selected_pattern_index = Some(app.ltmc_patterns.len() - 1);
                    }
                } else {
                    app.selected_pattern_index = Some(0);
                }
            }
        }
        KeyCode::Down => {
            if app.ltmc_patterns.len() > 0 {
                if let Some(index) = app.selected_pattern_index {
                    if index < app.ltmc_patterns.len() - 1 {
                        app.selected_pattern_index = Some(index + 1);
                    } else {
                        app.selected_pattern_index = Some(0);
                    }
                } else {
                    app.selected_pattern_index = Some(0);
                }
            }
        }
        KeyCode::Char('b') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_state = TuiState::FileBrowser;
        }
        _ => {}
    }
    Ok(())
}

/// Handle keys in tool selection state
pub fn handle_tool_selection_keys(app: &mut TuiApp, key_event: KeyEvent) -> Result<()> {
    match key_event.code {
        KeyCode::Up => {
            if app.tools.len() > 0 {
                if let Some(index) = app.selected_tool_index {
                    if index > 0 {
                        app.selected_tool_index = Some(index - 1);
                    } else {
                        app.selected_tool_index = Some(app.tools.len() - 1);
                    }
                } else {
                    app.selected_tool_index = Some(0);
                }
            }
        }
        KeyCode::Down => {
            if app.tools.len() > 0 {
                if let Some(index) = app.selected_tool_index {
                    if index < app.tools.len() - 1 {
                        app.selected_tool_index = Some(index + 1);
                    } else {
                        app.selected_tool_index = Some(0);
                    }
                } else {
                    app.selected_tool_index = Some(0);
                }
            }
        }
        KeyCode::Enter => {
            // Execute the selected tool
            if let Some(index) = app.selected_tool_index {
                if index < app.tools.len() {
                    info!("Executing tool: {}", app.tools[index]);
                }
            }
        }
        KeyCode::Char('b') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_state = TuiState::FileBrowser;
        }
        _ => {}
    }
    Ok(())
}
