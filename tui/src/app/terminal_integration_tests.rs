//! Tests for Enhanced Terminal Integration
//! 
//! This module contains comprehensive tests for the TerminalIntegration struct
//! following Test-Driven Development (TDD) approach.

use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[cfg(test)]
mod terminal_integration_tests {
    use super::*;

    #[test]
    fn test_terminal_integration_creation() {
        // Test: Create a new TerminalIntegration instance
        let integration = TerminalIntegration::new();
        
        // Verify initial state
        assert_eq!(integration.get_current_command(), "");
        assert_eq!(integration.get_command_history().len(), 0);
        assert_eq!(integration.history_index, 0);
        assert_eq!(integration.output_buffer.len(), 0);
        assert!(integration.shell_integration);
        assert!(integration.auto_completion);
        assert!(integration.syntax_highlighting);
    }

    #[test]
    fn test_handle_key_event_enter() {
        // Test: Handle Enter key with empty command
        let mut integration = TerminalIntegration::new();
        let key_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        
        let result = integration.handle_key_event(key_event).unwrap();
        
        // Should not change state and should not add empty command to history
        assert_eq!(integration.get_current_command(), "");
        assert_eq!(integration.get_command_history().len(), 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_key_event_enter_with_command() {
        // Test: Handle Enter key with non-empty command
        let mut integration = TerminalIntegration::new();
        integration.current_command = "ls".to_string();
        let key_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        
        let result = integration.handle_key_event(key_event).unwrap();
        
        // Should clear current command and add to history
        assert_eq!(integration.get_current_command(), "");
        assert_eq!(integration.get_command_history().len(), 1);
        assert_eq!(integration.get_command_history()[0], "ls");
        assert_eq!(integration.history_index, 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_key_event_char_input() {
        // Test: Handle character input
        let mut integration = TerminalIntegration::new();
        let key_event = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
        
        let result = integration.handle_key_event(key_event).unwrap();
        
        // Should add character to current command
        assert_eq!(integration.get_current_command(), "l");
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_key_event_backspace() {
        // Test: Handle backspace key
        let mut integration = TerminalIntegration::new();
        integration.current_command = "ls".to_string();
        let key_event = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        
        let result = integration.handle_key_event(key_event).unwrap();
        
        // Should remove last character
        assert_eq!(integration.get_current_command(), "l");
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_key_event_backspace_empty() {
        // Test: Handle backspace key on empty command
        let mut integration = TerminalIntegration::new();
        let key_event = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        
        let result = integration.handle_key_event(key_event).unwrap();
        
        // Should not crash and remain empty
        assert_eq!(integration.get_current_command(), "");
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_key_event_delete() {
        // Test: Handle delete key
        let mut integration = TerminalIntegration::new();
        integration.current_command = "ls".to_string();
        let key_event = KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE);
        
        let result = integration.handle_key_event(key_event).unwrap();
        
        // Should clear entire command
        assert_eq!(integration.get_current_command(), "");
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_key_event_up_arrow() {
        // Test: Handle up arrow key with history
        let mut integration = TerminalIntegration::new();
        integration.command_history.push("ls".to_string());
        integration.command_history.push("cd".to_string());
        integration.history_index = 2;
        
        let key_event = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let result = integration.handle_key_event(key_event).unwrap();
        
        // Should navigate to previous command
        assert_eq!(integration.get_current_command(), "cd");
        assert_eq!(integration.history_index, 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_key_event_down_arrow() {
        // Test: Handle down arrow key with history
        let mut integration = TerminalIntegration::new();
        integration.command_history.push("ls".to_string());
        integration.command_history.push("cd".to_string());
        integration.history_index = 1;
        
        let key_event = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let result = integration.handle_key_event(key_event).unwrap();
        
        // Should navigate to next command (empty)
        assert_eq!(integration.get_current_command(), "");
        assert_eq!(integration.history_index, 2);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_key_event_tab_completion() {
        // Test: Handle tab key with auto-completion enabled
        let mut integration = TerminalIntegration::new();
        integration.current_command = "gi".to_string();
        integration.auto_completion = true;
        
        let key_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let result = integration.handle_key_event(key_event).unwrap();
        
        // Should complete to "git"
        assert_eq!(integration.get_current_command(), "git");
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_key_event_tab_no_completion() {
        // Test: Handle tab key with auto-completion disabled
        let mut integration = TerminalIntegration::new();
        integration.current_command = "gi".to_string();
        integration.auto_completion = false;
        
        let key_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let result = integration.handle_key_event(key_event).unwrap();
        
        // Should not complete
        assert_eq!(integration.get_current_command(), "gi");
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_key_event_escape() {
        // Test: Handle escape key
        let mut integration = TerminalIntegration::new();
        let key_event = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        
        let result = integration.handle_key_event(key_event).unwrap();
        
        // Should return FileBrowser state
        assert_eq!(result, Some(TuiState::FileBrowser));
    }

    #[test]
    fn test_execute_command_empty() {
        // Test: Execute empty command
        let mut integration = TerminalIntegration::new();
        integration.current_command = "".to_string();
        
        let result = integration.execute_command();
        
        // Should not add to history
        assert!(result.is_ok());
        assert_eq!(integration.get_command_history().len(), 0);
        assert_eq!(integration.get_current_command(), "");
    }

    #[test]
    fn test_execute_command_valid() {
        // Test: Execute valid command
        let mut integration = TerminalIntegration::new();
        integration.current_command = "ls".to_string();
        
        let result = integration.execute_command();
        
        // Should add to history and clear current command
        assert!(result.is_ok());
        assert_eq!(integration.get_command_history().len(), 1);
        assert_eq!(integration.get_command_history()[0], "ls");
        assert_eq!(integration.get_current_command(), "");
        assert_eq!(integration.history_index, 1);
    }

    #[test]
    fn test_navigate_history_up() {
        // Test: Navigate history up
        let mut integration = TerminalIntegration::new();
        integration.command_history.push("ls".to_string());
        integration.command_history.push("cd".to_string());
        integration.history_index = 2;
        
        integration.navigate_history(-1);
        
        assert_eq!(integration.get_current_command(), "cd");
        assert_eq!(integration.history_index, 1);
    }

    #[test]
    fn test_navigate_history_down() {
        // Test: Navigate history down
        let mut integration = TerminalIntegration::new();
        integration.command_history.push("ls".to_string());
        integration.command_history.push("cd".to_string());
        integration.history_index = 1;
        
        integration.navigate_history(1);
        
        assert_eq!(integration.get_current_command(), "");
        assert_eq!(integration.history_index, 2);
    }

    #[test]
    fn test_navigate_history_empty() {
        // Test: Navigate history when empty
        let mut integration = TerminalIntegration::new();
        
        integration.navigate_history(-1);
        integration.navigate_history(1);
        
        // Should not crash
        assert_eq!(integration.get_current_command(), "");
        assert_eq!(integration.history_index, 0);
    }

    #[test]
    fn test_handle_auto_completion_git() {
        // Test: Auto-complete git command
        let mut integration = TerminalIntegration::new();
        integration.current_command = "gi".to_string();
        
        let result = integration.handle_auto_completion();
        
        assert!(result.is_ok());
        assert_eq!(integration.get_current_command(), "git");
    }

    #[test]
    fn test_handle_auto_completion_cargo() {
        // Test: Auto-complete cargo command
        let mut integration = TerminalIntegration::new();
        integration.current_command = "car".to_string();
        
        let result = integration.handle_auto_completion();
        
        assert!(result.is_ok());
        assert_eq!(integration.get_current_command(), "cargo");
    }

    #[test]
    fn test_handle_auto_completion_no_match() {
        // Test: Auto-complete with no match
        let mut integration = TerminalIntegration::new();
        integration.current_command = "xyz".to_string();
        
        let result = integration.handle_auto_completion();
        
        assert!(result.is_ok());
        assert_eq!(integration.get_current_command(), "xyz");
    }

    #[test]
    fn test_handle_auto_completion_empty() {
        // Test: Auto-complete with empty command
        let mut integration = TerminalIntegration::new();
        integration.current_command = "".to_string();
        
        let result = integration.handle_auto_completion();
        
        assert!(result.is_ok());
        assert_eq!(integration.get_current_command(), "");
    }

    #[test]
    fn test_clear_output() {
        // Test: Clear output buffer
        let mut integration = TerminalIntegration::new();
        integration.output_buffer.push(TerminalOutput {
            id: uuid::Uuid::new_v4(),
            content: "test output".to_string(),
            output_type: "test".to_string(),
            timestamp: chrono::Utc::now(),
        });
        
        assert_eq!(integration.output_buffer.len(), 1);
        
        integration.clear_output();
        
        assert_eq!(integration.output_buffer.len(), 0);
    }

    #[test]
    fn test_toggle_shell_integration() {
        // Test: Toggle shell integration
        let mut integration = TerminalIntegration::new();
        
        assert!(integration.shell_integration);
        
        integration.toggle_shell_integration();
        
        assert!(!integration.shell_integration);
        
        integration.toggle_shell_integration();
        
        assert!(integration.shell_integration);
    }

    #[test]
    fn test_toggle_auto_completion() {
        // Test: Toggle auto-completion
        let mut integration = TerminalIntegration::new();
        
        assert!(integration.auto_completion);
        
        integration.toggle_auto_completion();
        
        assert!(!integration.auto_completion);
        
        integration.toggle_auto_completion();
        
        assert!(integration.auto_completion);
    }

    #[test]
    fn test_toggle_syntax_highlighting() {
        // Test: Toggle syntax highlighting
        let mut integration = TerminalIntegration::new();
        
        assert!(integration.syntax_highlighting);
        
        integration.toggle_syntax_highlighting();
        
        assert!(!integration.syntax_highlighting);
        
        integration.toggle_syntax_highlighting();
        
        assert!(integration.syntax_highlighting);
    }

    #[test]
    fn test_execute_shell_command_echo() {
        // Test: Execute shell command with echo
        let integration = TerminalIntegration::new();
        
        let result = integration.execute_shell_command("echo 'test'");
        
        assert!(result.is_ok());
        assert!(result.unwrap().contains("test"));
    }

    #[test]
    fn test_execute_shell_command_invalid() {
        // Test: Execute invalid shell command
        let integration = TerminalIntegration::new();
        
        let result = integration.execute_shell_command("invalid_command_12345");
        
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Error"));
    }
}