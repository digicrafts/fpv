use fpv::app::navigation::go_to_parent_directory;
use fpv::app::state::SessionState;
use std::path::PathBuf;

#[test]
fn left_at_root_keeps_path_unchanged() {
    let root = if cfg!(windows) {
        PathBuf::from("C:\\")
    } else {
        PathBuf::from("/")
    };
    let mut state = SessionState::new(root.clone());
    state.current_path = root.clone();
    let mut nodes = Vec::new();

    let result = go_to_parent_directory(&mut state, &mut nodes).expect("go parent");
    assert_eq!(result.new_path, root);
}

#[test]
fn left_from_dot_start_path_moves_to_parent_directory() {
    let cwd = std::env::current_dir().expect("cwd");
    let Some(expected_parent) = cwd.parent().map(|p| p.to_path_buf()) else {
        return;
    };

    let mut state = SessionState::new(PathBuf::from("."));
    state.current_path = PathBuf::from(".");
    let mut nodes = Vec::new();

    let result = go_to_parent_directory(&mut state, &mut nodes).expect("go parent from dot");
    assert_eq!(result.new_path, expected_parent);
    assert_eq!(state.current_path, expected_parent);
}
