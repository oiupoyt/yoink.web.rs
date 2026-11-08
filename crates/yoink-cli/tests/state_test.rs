use yoink_cli::state::{AppState, Phase};

#[test]
fn test_app_state_initial() {
    let state = AppState::new(None);
    match state.phase {
        Phase::Input { warning } => assert!(warning.is_none()),
        _ => panic!("Expected Input phase"),
    }
}

#[test]
fn test_app_state_url_validation() {
    let mut state = AppState::new(None);
    state.set_url_input("not-a-url".to_string());
    state.submit_input();
    match state.phase {
        Phase::Input { warning } => assert!(warning.is_some()),
        _ => panic!("Expected Input phase with warning"),
    }
}
