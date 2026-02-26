use fpv::app::navigation::{enter_selected_directory, go_to_parent_directory};
use fpv::app::state::{NodeType, SessionState};
use fpv::fs::current_dir::list_current_directory;
use fpv::tui::tree_pane::{display_path_with_home, entry_prefix};
use std::fs;
use tempfile::tempdir;

#[test]
fn right_then_left_returns_to_parent() {
    let d = tempdir().expect("tempdir");
    fs::create_dir_all(d.path().join("dir_a")).expect("mkdir");
    fs::write(d.path().join("a.txt"), "a").expect("write");
    fs::write(d.path().join("z.txt"), "z").expect("write");

    let mut state = SessionState::new(d.path().to_path_buf());
    let mut nodes = list_current_directory(d.path(), 2000).expect("list");
    state.revalidate_selection(&nodes);

    assert_eq!(nodes[0].node_type, NodeType::Directory);
    assert_eq!(entry_prefix(&nodes[0].node_type), "/");
    assert_eq!(entry_prefix(&nodes[1].node_type), "");

    let _ = enter_selected_directory(&mut state, &mut nodes).expect("enter");
    let after_enter = state.current_path.clone();
    assert!(after_enter.ends_with("dir_a"));

    let _ = go_to_parent_directory(&mut state, &mut nodes).expect("parent");
    assert_eq!(state.current_path, d.path().to_path_buf());
}

#[test]
fn non_home_paths_remain_absolute_in_display_format() {
    std::env::set_var("HOME", "/tmp/fpv-home");
    let rendered = display_path_with_home(std::path::Path::new("/var/tmp/fpv"));
    assert_eq!(rendered, "/var/tmp/fpv");
}
