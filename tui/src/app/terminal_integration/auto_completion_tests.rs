//! Tests for Terminal Integration Auto-completion
//!
//! This module contains tests for auto-completion functionality
//! in the TerminalIntegration struct.

use super::*;

#[cfg(test)]
mod auto_completion_tests {
    use super::*;

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
    fn test_handle_auto_completion_full_match() {
        // Test: Auto-complete with full command already typed
        let mut integration = TerminalIntegration::new();
        integration.current_command = "git".to_string();

        let result = integration.handle_auto_completion();

        assert!(result.is_ok());
        assert_eq!(integration.get_current_command(), "git");
    }

    #[test]
    fn test_handle_auto_completion_multiple_commands() {
        // Test: Auto-complete multiple commands in sequence
        let mut integration = TerminalIntegration::new();

        // Complete git
        integration.current_command = "gi".to_string();
        integration.handle_auto_completion().unwrap();
        assert_eq!(integration.get_current_command(), "git");

        // Complete cargo
        integration.current_command = "car".to_string();
        integration.handle_auto_completion().unwrap();
        assert_eq!(integration.get_current_command(), "cargo");

        // Complete npm
        integration.current_command = "np".to_string();
        integration.handle_auto_completion().unwrap();
        assert_eq!(integration.get_current_command(), "npm");
    }

    #[test]
    fn test_handle_auto_completion_case_sensitive() {
        // Test: Auto-completion case sensitivity
        let mut integration = TerminalIntegration::new();

        // Test lowercase
        integration.current_command = "gi".to_string();
        integration.handle_auto_completion().unwrap();
        assert_eq!(integration.get_current_command(), "git");

        // Test uppercase (should not complete)
        integration.current_command = "GI".to_string();
        integration.handle_auto_completion().unwrap();
        assert_eq!(integration.get_current_command(), "GI");
    }

    #[test]
    fn test_handle_auto_completion_partial_word() {
        // Test: Auto-complete partial word in command
        let mut integration = TerminalIntegration::new();
        integration.current_command = "echo gi".to_string();

        let result = integration.handle_auto_completion();

        assert!(result.is_ok());
        // Should complete the last word
        assert_eq!(integration.get_current_command(), "echo git");
    }

    #[test]
    fn test_handle_auto_completion_with_spaces() {
        // Test: Auto-complete with leading spaces
        let mut integration = TerminalIntegration::new();
        integration.current_command = "  gi".to_string();

        let result = integration.handle_auto_completion();

        assert!(result.is_ok());
        assert_eq!(integration.get_current_command(), "  git");
    }

    #[test]
    fn test_handle_auto_completion_all_commands() {
        // Test: Auto-complete all common commands
        let test_cases = vec![
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

        for (input, expected) in test_cases {
            let mut integration = TerminalIntegration::new();
            integration.current_command = input.to_string();

            let result = integration.handle_auto_completion();

            assert!(result.is_ok(), "Failed for input: {}", input);
            assert_eq!(
                integration.get_current_command(),
                expected,
                "Failed for input: {}",
                input
            );
        }
    }

    #[test]
    fn test_handle_auto_completion_no_duplicates() {
        // Test: Auto-complete doesn't create duplicates
        let mut integration = TerminalIntegration::new();
        integration.current_command = "git".to_string();

        // Try to complete already complete command
        integration.handle_auto_completion().unwrap();

        // Should remain the same
        assert_eq!(integration.get_current_command(), "git");
    }

    #[test]
    fn test_handle_auto_completion_long_command() {
        // Test: Auto-complete with long command prefix
        let mut integration = TerminalIntegration::new();
        integration.current_command = "verylongprefixthatdoesnotmatch".to_string();

        let result = integration.handle_auto_completion();

        assert!(result.is_ok());
        // Should remain unchanged
        assert_eq!(
            integration.get_current_command(),
            "verylongprefixthatdoesnotmatch"
        );
    }

    #[test]
    fn test_handle_auto_completion_special_characters() {
        // Test: Auto-complete with special characters
        let mut integration = TerminalIntegration::new();
        integration.current_command = "gi!".to_string();

        let result = integration.handle_auto_completion();

        assert!(result.is_ok());
        // Should remain unchanged (no match)
        assert_eq!(integration.get_current_command(), "gi!");
    }

    #[test]
    fn test_handle_auto_completion_numbers() {
        // Test: Auto-complete with numbers
        let mut integration = TerminalIntegration::new();
        integration.current_command = "git123".to_string();

        let result = integration.handle_auto_completion();

        assert!(result.is_ok());
        // Should remain unchanged (no match)
        assert_eq!(integration.get_current_command(), "git123");
    }

    #[test]
    fn test_handle_auto_completion_unicode() {
        // Test: Auto-complete with unicode characters
        let mut integration = TerminalIntegration::new();
        integration.current_command = "gí".to_string(); // i with accent

        let result = integration.handle_auto_completion();

        assert!(result.is_ok());
        // Should remain unchanged (no match)
        assert_eq!(integration.get_current_command(), "gí");
    }

    #[test]
    fn test_handle_auto_completion_whitespace_only() {
        // Test: Auto-complete with whitespace only
        let mut integration = TerminalIntegration::new();
        integration.current_command = "   ".to_string();

        let result = integration.handle_auto_completion();

        assert!(result.is_ok());
        // Should remain unchanged
        assert_eq!(integration.get_current_command(), "   ");
    }

    #[test]
    fn test_handle_auto_completion_mixed_case() {
        // Test: Auto-complete with mixed case
        let mut integration = TerminalIntegration::new();
        integration.current_command = "GiT".to_string();

        let result = integration.handle_auto_completion();

        assert!(result.is_ok());
        // Should remain unchanged (case sensitive)
        assert_eq!(integration.get_current_command(), "GiT");
    }
}
