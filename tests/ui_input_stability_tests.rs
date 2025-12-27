//! UI Input Stability Tests (Phase 7.1)
//!
//! Regression tests for TUI input stability bug.
//! Bug: Natural language input caused immediate app exit.
//! Root cause: Event loop only continued while AppState::Running,
//!           but planning states (PlanningInProgress, PlanReady, etc.)
//!           caused loop to exit.

use odincode::ui::state::{App, AppState};

#[test]
fn test_natural_language_input_does_not_exit() {
    // Simulate: user types "read src/lib.rs" and presses Enter
    // Expected: app remains in a valid state (not Quitting)

    let mut app = App::new(std::path::PathBuf::from("."));

    // Initial state should be Running
    assert_eq!(app.state(), AppState::Running);

    // Simulate entering natural language input (this happens in handle_nlp_intent)
    // The app sets planning state to InProgress
    app.set_planning_in_progress();

    // The app state should be PlanningInProgress (a valid, alive state)
    assert_eq!(app.state(), AppState::PlanningInProgress);

    // Critical: The app should NOT be in Quitting state
    assert_ne!(app.state(), AppState::Quitting);

    // should_quit flag must remain false
    assert!(!app.should_quit);

    // After planning completes (either success or error), app should return to Running
    app.clear_planning_state();
    assert_eq!(app.state(), AppState::Running);
    assert!(!app.should_quit);
}

#[test]
fn test_only_explicit_quit_sets_quitting_state() {
    let mut app = App::new(std::path::PathBuf::from("."));

    // Initial state
    assert_eq!(app.state(), AppState::Running);
    assert!(!app.should_quit);

    // Only explicit quit should set should_quit
    app.quit();
    assert!(app.should_quit);
    assert_eq!(app.state(), AppState::Quitting);
}

#[test]
fn test_planning_states_are_valid_alive_states() {
    let mut app = App::new(std::path::PathBuf::from("."));

    // All planning states should be "alive" states (not Quitting)

    // PlanningInProgress
    app.set_planning_in_progress();
    assert_ne!(app.state(), AppState::Quitting);
    assert!(!app.should_quit);

    // Clear and test PlanReady
    app.clear_planning_state();
    use odincode::llm::{Intent, Plan};
    let dummy_plan = Plan {
        plan_id: "test".to_string(),
        intent: Intent::Read,
        steps: vec![],
        evidence_referenced: vec![],
    };
    app.set_plan_ready(dummy_plan);
    assert_ne!(app.state(), AppState::Quitting);
    assert!(!app.should_quit);

    // Clear and test PlanError
    app.clear_planning_state();
    app.set_plan_error("test error".to_string());
    assert_ne!(app.state(), AppState::Quitting);
    assert!(!app.should_quit);
}

#[test]
fn test_empty_input_does_not_exit() {
    let mut app = App::new(std::path::PathBuf::from("."));

    // Simulate: user presses Enter with empty input_buffer
    // (Command::None from parse_command)

    // Empty input should be handled gracefully
    // The handle_enter method just clears the buffer
    app.handle_enter();
    assert_eq!(app.state(), AppState::Running);
    assert!(!app.should_quit);
    assert_eq!(app.input_buffer, "");
}
