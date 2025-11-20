//! Tests for Terminal Integration Command Execution
//!
//! This module contains tests for command execution, history management,
//! and shell command functionality.

use super::*;

#[cfg(test)]
mod command_execution_tests {
    use super::*;

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
    fn test_execute_command_multiple() {
        // Test: Execute multiple commands
        let mut integration = TerminalIntegration::new();

        // Execute first command
        integration.current_command = "ls".to_string();
        integration.execute_command().unwrap();

        // Execute second command
        integration.current_command = "cd".to_string();
        integration.execute_command().unwrap();

        // Execute third command
        integration.current_command = "pwd".to_string();
        integration.execute_command().unwrap();

        // Verify all commands are in history
        assert_eq!(integration.get_command_history().len(), 3);
        assert_eq!(integration.get_command_history()[0], "ls");
        assert_eq!(integration.get_command_history()[1], "cd");
        assert_eq!(integration.get_command_history()[2], "pwd");
        assert_eq!(integration.history_index, 3);
    }

    #[test]
    fn test_execute_command_whitespace() {
        // Test: Execute command with only whitespace
        let mut integration = TerminalIntegration::new();
        integration.current_command = "   ".to_string();

        let result = integration.execute_command();

        // Should not add to history (whitespace-only)
        assert!(result.is_ok());
        assert_eq!(integration.get_command_history().len(), 0);
        assert_eq!(integration.get_current_command(), "");
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
    fn test_navigate_history_bounds() {
        // Test: Navigate history beyond bounds
        let mut integration = TerminalIntegration::new();
        integration.command_history.push("ls".to_string());
        integration.command_history.push("cd".to_string());
        integration.history_index = 0;

        // Try to navigate up from beginning
        integration.navigate_history(-1);
        assert_eq!(integration.history_index, 0);
        assert_eq!(integration.get_current_command(), "");

        // Navigate to end
        integration.history_index = 2;
        integration.navigate_history(1);
        assert_eq!(integration.history_index, 2);
        assert_eq!(integration.get_current_command(), "");
    }

    #[test]
    fn test_navigate_history_multiple() {
        // Test: Navigate history multiple times
        let mut integration = TerminalIntegration::new();
        integration.command_history.push("first".to_string());
        integration.command_history.push("second".to_string());
        integration.command_history.push("third".to_string());
        integration.history_index = 3;

        // Navigate up multiple times
        integration.navigate_history(-1);
        assert_eq!(integration.get_current_command(), "third");
        assert_eq!(integration.history_index, 2);

        integration.navigate_history(-1);
        assert_eq!(integration.get_current_command(), "second");
        assert_eq!(integration.history_index, 1);

        integration.navigate_history(-1);
        assert_eq!(integration.get_current_command(), "first");
        assert_eq!(integration.history_index, 0);

        // Navigate down multiple times
        integration.navigate_history(1);
        assert_eq!(integration.get_current_command(), "second");
        assert_eq!(integration.history_index, 1);

        integration.navigate_history(1);
        assert_eq!(integration.get_current_command(), "third");
        assert_eq!(integration.history_index, 2);

        integration.navigate_history(1);
        assert_eq!(integration.get_current_command(), "");
        assert_eq!(integration.history_index, 3);
    }

    #[test]
    fn test_execute_shell_command_echo() {
        // Test: Execute shell command with echo
        let integration = TerminalIntegration::new();

        let result = integration.execute_shell_command("echo 'test'");

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("test"));
    }

    #[test]
    fn test_execute_shell_command_invalid() {
        // Test: Execute invalid shell command
        let integration = TerminalIntegration::new();

        let result = integration.execute_shell_command("invalid_command_12345");

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Error"));
    }

    #[test]
    fn test_execute_shell_command_empty() {
        // Test: Execute empty shell command
        let integration = TerminalIntegration::new();

        let result = integration.execute_shell_command("");

        assert!(result.is_ok());
        let output = result.unwrap();
        // Empty command should produce some output (usually empty or error)
        assert!(output.is_empty() || output.contains("Error"));
    }

    #[test]
    fn test_execute_shell_command_with_args() {
        // Test: Execute shell command with arguments
        let integration = TerminalIntegration::new();

        let result = integration.execute_shell_command("echo 'hello world'");

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("hello world"));
    }

    #[test]
    fn test_execute_shell_command_pipe() {
        // Test: Execute shell command with pipe
        let integration = TerminalIntegration::new();

        let result = integration.execute_shell_command("echo 'test' | wc -c");

        assert!(result.is_ok());
        let output = result.unwrap();
        // Should contain character count (varies by system)
        assert!(!output.is_empty());
    }

    #[test]
    fn test_process_command_results() {
        // Test: Process command execution results
        let mut integration = TerminalIntegration::new();

        // This test verifies the method exists and can be called
        // In a real scenario, this would involve async command execution
        let result = integration.process_command_results();

        assert!(result.is_ok());
    }

    #[test]
    fn test_command_output_buffer() {
        // Test: Command output buffer management
        let mut integration = TerminalIntegration::new();

        // Add some output to buffer
        integration.output_buffer.push(TerminalOutput {
            id: uuid::Uuid::new_v4(),
            content: "Command output".to_string(),
            output_type: "output".to_string(),
            timestamp: chrono::Utc::now(),
        });

        integration.output_buffer.push(TerminalOutput {
            id: uuid::Uuid::new_v4(),
            content: "Error output".to_string(),
            output_type: "error".to_string(),
            timestamp: chrono::Utc::now(),
        });

        assert_eq!(integration.output_buffer.len(), 2);
        assert_eq!(integration.output_buffer[0].content, "Command output");
        assert_eq!(integration.output_buffer[1].content, "Error output");
        assert_eq!(integration.output_buffer[0].output_type, "output");
        assert_eq!(integration.output_buffer[1].output_type, "error");
    }

    #[test]
    fn test_command_execution_with_output() {
        // Test: Command execution adds to output buffer
        let mut integration = TerminalIntegration::new();
        integration.current_command = "echo 'test'".to_string();

        // Execute command
        integration.execute_command().unwrap();

        // Verify command was added to history
        assert_eq!(integration.get_command_history().len(), 1);
        assert_eq!(integration.get_command_history()[0], "echo 'test'");

        // Verify output buffer contains command entry
        assert_eq!(integration.output_buffer.len(), 1);
        assert_eq!(integration.output_buffer[0].content, "$ echo 'test'");
        assert_eq!(integration.output_buffer[0].output_type, "command");
    }
}
