use fpv::app::navigation::enter_selected_directory;
use fpv::app::state::SessionState;
use fpv::fs::current_dir::list_current_directory;
use std::fs;
use tempfile::tempdir;

#[test]
fn entering_empty_directory_results_in_empty_entries() {
    let d = tempdir().expect("tempdir");
    fs::create_dir_all(d.path().join("empty")).expect("mkdir");

    let mut state = SessionState::new(d.path().to_path_buf());
    let mut nodes = list_current_directory(d.path(), 2000).expect("list");
    state.revalidate_selection(&nodes);
    let _ = enter_selected_directory(&mut state, &mut nodes).expect("enter");
    assert!(nodes.is_empty());
}
