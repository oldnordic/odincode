//! Tests for TerminalIntegration Creation and Initialization
//!
//! This module contains tests for the basic creation and initialization
//! of the TerminalIntegration struct.

use super::*;

#[cfg(test)]
mod creation_tests {
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
    fn test_terminal_integration_initialization() {
        // Test: Initialize terminal integration
        let mut integration = TerminalIntegration::new();

        // This test verifies the initialization method exists and can be called
        // In a test environment, we might skip actual terminal setup
        // to avoid interfering with the test runner's terminal
        let result = integration.initialize();

        // In test environment, we expect initialization to fail
        // This is normal behavior since tests can't modify terminal
        assert!(result.is_err());
    }

    #[test]
    fn test_terminal_integration_cleanup() {
        // Test: Cleanup terminal integration
        let integration = TerminalIntegration::new();

        // This test verifies the cleanup method exists and can be called
        // Similar to initialization, this might be skipped in real test environments
        let result = integration.cleanup();

        assert!(result.is_ok());
    }

    #[test]
    fn test_toggle_features() {
        // Test: Toggle various features
        let mut integration = TerminalIntegration::new();

        // Test shell integration toggle
        assert!(integration.shell_integration);
        integration.toggle_shell_integration();
        assert!(!integration.shell_integration);
        integration.toggle_shell_integration();
        assert!(integration.shell_integration);

        // Test auto-completion toggle
        assert!(integration.auto_completion);
        integration.toggle_auto_completion();
        assert!(!integration.auto_completion);
        integration.toggle_auto_completion();
        assert!(integration.auto_completion);

        // Test syntax highlighting toggle
        assert!(integration.syntax_highlighting);
        integration.toggle_syntax_highlighting();
        assert!(!integration.syntax_highlighting);
        integration.toggle_syntax_highlighting();
        assert!(integration.syntax_highlighting);
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
}
