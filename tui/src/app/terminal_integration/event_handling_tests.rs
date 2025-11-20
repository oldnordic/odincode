//! Tests for Terminal Integration Event Handling
//!
//! This module contains tests for keyboard and mouse event handling
//! in the TerminalIntegration struct.

use super::*;

#[cfg(test)]
mod event_handling_tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn test_handle_key_event_enter_empty() {
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
    fn test_handle_key_event_multiple_chars() {
        // Test: Handle multiple character inputs
        let mut integration = TerminalIntegration::new();

        // Input 'l'
        let key_event = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
        integration.handle_key_event(key_event).unwrap();

        // Input 's'
        let key_event = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        integration.handle_key_event(key_event).unwrap();

        assert_eq!(integration.get_current_command(), "ls");
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
    fn test_handle_key_event_navigation_keys() {
        // Test: Handle navigation keys (Left, Right, Home, End)
        let mut integration = TerminalIntegration::new();
        integration.current_command = "test".to_string();

        // Test Left key (should not crash)
        let key_event = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        let result = integration.handle_key_event(key_event).unwrap();
        assert!(result.is_none());

        // Test Right key (should not crash)
        let key_event = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        let result = integration.handle_key_event(key_event).unwrap();
        assert!(result.is_none());

        // Test Home key (should not crash)
        let key_event = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
        let result = integration.handle_key_event(key_event).unwrap();
        assert!(result.is_none());

        // Test End key (should not crash)
        let key_event = KeyEvent::new(KeyCode::End, KeyModifiers::NONE);
        let result = integration.handle_key_event(key_event).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_mouse_event_scroll() {
        // Test: Handle mouse scroll events
        let mut integration = TerminalIntegration::new();

        // Test scroll up
        let mouse_event = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        let result = integration.handle_mouse_event(mouse_event).unwrap();
        assert!(result.is_none());

        // Test scroll down
        let mouse_event = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        let result = integration.handle_mouse_event(mouse_event).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_mouse_event_click() {
        // Test: Handle mouse click events
        let mut integration = TerminalIntegration::new();

        let mouse_event = MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 10,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        let result = integration.handle_mouse_event(mouse_event).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_paste_event() {
        // Test: Handle paste events
        let mut integration = TerminalIntegration::new();

        let pasted_text = "pasted content".to_string();
        let result = integration.handle_paste_event(pasted_text).unwrap();

        assert_eq!(integration.get_current_command(), "pasted content");
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_paste_event_append() {
        // Test: Handle paste event with existing content
        let mut integration = TerminalIntegration::new();
        integration.current_command = "existing ".to_string();

        let pasted_text = "content".to_string();
        let result = integration.handle_paste_event(pasted_text).unwrap();

        assert_eq!(integration.get_current_command(), "existing content");
        assert!(result.is_none());
    }
}
