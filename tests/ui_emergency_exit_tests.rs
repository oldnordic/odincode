//! UI Emergency Exit Tests (Phase 8)
//!
//! Regression tests for TUI emergency exit behavior.
//!
//! Phase 8: All /quit, /q, /exit commands exit immediately.
//! No modifiers needed - exit is always direct from any state.

use odincode::ui::state::AppState;
use odincode::ui::{parse_command, App, Command};

#[test]
fn test_q_command_exits() {
    // Phase 8: "/q" exits immediately

    let mut app = App::new(std::path::PathBuf::from("."));

    // Initial state
    assert_eq!(app.state(), AppState::Running);
    assert!(!app.should_quit);

    // Parse "/q" command
    let cmd = parse_command("/q");

    // Should be Command::Quit
    assert!(matches!(cmd, Command::Quit));

    // Execute the command
    odincode::ui::handlers::execute_command(&mut app, cmd);

    // App should now be in Quitting state
    assert!(app.should_quit);
    assert_eq!(app.state(), AppState::Quitting);
}

#[test]
fn test_quit_command_exits() {
    // Phase 8: "/quit" exits immediately

    let mut app = App::new(std::path::PathBuf::from("."));

    assert_eq!(app.state(), AppState::Running);
    assert!(!app.should_quit);

    let cmd = parse_command("/quit");
    assert!(matches!(cmd, Command::Quit));

    odincode::ui::handlers::execute_command(&mut app, cmd);

    assert!(app.should_quit);
    assert_eq!(app.state(), AppState::Quitting);
}

#[test]
fn test_exit_command_exits() {
    // Phase 8: "/exit" exits immediately

    let mut app = App::new(std::path::PathBuf::from("."));

    let cmd = parse_command("/exit");
    assert!(matches!(cmd, Command::Quit));

    odincode::ui::handlers::execute_command(&mut app, cmd);

    assert!(app.should_quit);
    assert_eq!(app.state(), AppState::Quitting);
}

#[test]
fn test_q_exits_from_planning_state() {
    // Emergency exit should work regardless of app state

    let mut app = App::new(std::path::PathBuf::from("."));

    // Simulate being in planning state
    app.set_planning_in_progress();
    assert_eq!(app.state(), AppState::PlanningInProgress);

    // "/q" should still exit
    let cmd = parse_command("/q");
    assert!(matches!(cmd, Command::Quit));

    odincode::ui::handlers::execute_command(&mut app, cmd);

    assert!(app.should_quit);
    assert_eq!(app.state(), AppState::Quitting);
}

#[test]
fn test_q_exits_from_plan_ready_state() {
    let mut app = App::new(std::path::PathBuf::from("."));

    // Set plan ready state
    use odincode::llm::{Intent, Plan};
    let dummy_plan = Plan {
        plan_id: "test".to_string(),
        intent: Intent::Read,
        steps: vec![],
        evidence_referenced: vec![],
    };
    app.set_plan_ready(dummy_plan);

    assert_eq!(app.state(), AppState::PlanReady);

    // "/q" should exit even with a pending plan
    let cmd = parse_command("/q");
    odincode::ui::handlers::execute_command(&mut app, cmd);

    assert!(app.should_quit);
    assert_eq!(app.state(), AppState::Quitting);
}

#[test]
fn test_q_exits_from_editing_plan_state() {
    let mut app = App::new(std::path::PathBuf::from("."));

    // Set a plan first, then enter edit mode
    use odincode::llm::{Intent, Plan};
    let dummy_plan = Plan {
        plan_id: "test".to_string(),
        intent: Intent::Read,
        steps: vec![],
        evidence_referenced: vec![],
    };
    app.set_plan_ready(dummy_plan);
    app.enter_edit_mode();

    assert_eq!(app.state(), AppState::EditingPlan);

    let cmd = parse_command("/q");
    odincode::ui::handlers::execute_command(&mut app, cmd);

    assert!(app.should_quit);
    assert_eq!(app.state(), AppState::Quitting);
}

#[test]
fn test_q_exits_from_plan_error_state() {
    let mut app = App::new(std::path::PathBuf::from("."));

    app.set_plan_error("Test error".to_string());
    assert_eq!(app.state(), AppState::PlanError);

    let cmd = parse_command("/q");
    odincode::ui::handlers::execute_command(&mut app, cmd);

    assert!(app.should_quit);
    assert_eq!(app.state(), AppState::Quitting);
}

#[test]
fn test_all_quit_variations_work() {
    // Test all quit command variations

    let quit_commands = vec!["/q", "/quit", "/exit"];

    for cmd_str in quit_commands {
        let mut app = App::new(std::path::PathBuf::from("."));
        let cmd = parse_command(cmd_str);
        assert!(
            matches!(cmd, Command::Quit),
            "'{}' should be Quit command",
            cmd_str
        );

        odincode::ui::handlers::execute_command(&mut app, cmd);
        assert!(app.should_quit, "'{}' should set should_quit", cmd_str);
        assert_eq!(app.state(), AppState::Quitting);
    }
}

#[test]
fn test_command_parsing_is_case_sensitive() {
    // Phase 8: Commands should be case-sensitive
    // "/Q" or "/QUIT" should NOT work (user must type exactly "/q" or "/quit")

    // "/q" works
    let cmd1 = parse_command("/q");
    assert!(matches!(cmd1, Command::Quit));

    // "/quit" works
    let cmd2 = parse_command("/quit");
    assert!(matches!(cmd2, Command::Quit));

    // "/Q" should NOT work (unknown command = None)
    let cmd3 = parse_command("/Q");
    assert!(!matches!(cmd3, Command::Quit));
    assert!(matches!(cmd3, Command::None));

    // "/QUIT" should NOT work (unknown command = None)
    let cmd4 = parse_command("/QUIT");
    assert!(!matches!(cmd4, Command::Quit));
    assert!(matches!(cmd4, Command::None));
}

#[test]
fn test_colon_q_is_chat_not_command() {
    // Phase 8: ":" is now plain text, NOT a command prefix
    // ":q" should be Chat, not Quit

    let cmd = parse_command(":q");
    assert!(!matches!(cmd, Command::Quit));
    assert!(matches!(cmd, Command::Chat(_)));

    let cmd2 = parse_command(":quit");
    assert!(!matches!(cmd2, Command::Quit));
    assert!(matches!(cmd2, Command::Chat(_)));

    let cmd3 = parse_command(":q!");
    assert!(!matches!(cmd3, Command::Quit));
    assert!(matches!(cmd3, Command::Chat(_)));
}

#[test]
fn test_only_explicit_slash_commands_quit() {
    // Verify that typing random commands doesn't exit
    // Only "/q", "/quit", "/exit" should exit

    let app = App::new(std::path::PathBuf::from("."));

    // These should NOT be quit commands
    let non_quit_commands = vec![
        "/help", "/open", "/read", "/lsp", "/Q",     // Wrong case = None (unknown command)
        "/QUIT",  // Wrong case = None (unknown command)
        "/exit!", // "!" modifier not valid = None (unknown command)
        "quit",   // No slash = Chat
        ":q",     // Colon = Chat
    ];

    for cmd_str in non_quit_commands {
        let cmd = parse_command(cmd_str);
        assert!(
            !matches!(cmd, Command::Quit),
            "'{}' should not be a quit command",
            cmd_str
        );
    }

    // App should still be running
    assert!(!app.should_quit);
    assert_eq!(app.state(), AppState::Running);
}
