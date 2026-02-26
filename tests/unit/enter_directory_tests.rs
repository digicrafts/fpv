use fpv::app::navigation::enter_selected_directory;
use fpv::app::state::{NodeType, SessionState, TreeNode};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn enter_directory_updates_current_path() {
    let d = tempdir().expect("tempdir");
    let child = d.path().join("child");
    fs::create_dir_all(&child).expect("mkdir");

    let mut state = SessionState::new(d.path().to_path_buf());
    let mut nodes = vec![TreeNode {
        path: child.clone(),
        name: "child".to_string(),
        node_type: NodeType::Directory,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    }];

    let result = enter_selected_directory(&mut state, &mut nodes).expect("enter");
    assert_eq!(result.new_path, child);
    assert_eq!(state.current_path, result.new_path);
}

#[test]
fn entering_file_is_no_change() {
    let mut state = SessionState::new(PathBuf::from("."));
    let mut nodes = vec![TreeNode {
        path: PathBuf::from("README.md"),
        name: "README.md".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    }];
    let result = enter_selected_directory(&mut state, &mut nodes).expect("result");
    assert_eq!(result.new_path, state.current_path);
}
