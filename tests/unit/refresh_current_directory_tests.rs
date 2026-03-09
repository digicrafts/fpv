use fpv::app::navigation::refresh_current_directory;
use fpv::app::navigation_result::ActionOutcome;
use fpv::app::state::SessionState;
use std::fs;
use tempfile::tempdir;

#[test]
fn refreshing_unreachable_current_directory_sets_error_state() {
    let d = tempdir().expect("tempdir");
    let missing = d.path().join("missing");
    fs::create_dir_all(&missing).expect("mkdir");

    let mut state = SessionState::new(missing.clone());
    let mut nodes = Vec::new();

    fs::remove_dir_all(&missing).expect("remove dir");

    let result = refresh_current_directory(&mut state, &mut nodes).expect("refresh");
    assert_eq!(result.outcome, ActionOutcome::Blocked);
    assert!(nodes.is_empty());
    assert_eq!(state.current_path, missing);
    assert_eq!(state.selected_path, state.current_path);
    assert_eq!(state.current_dir_error.as_deref(), Some("Directory unreachable."));
}
