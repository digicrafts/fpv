use fpv::app::navigation::{enter_selected_directory, go_to_parent_directory};
use fpv::app::preview_controller::refresh_preview;
use fpv::app::state::{NodeType, SessionState, TreeNode};
use fpv::fs::current_dir::list_current_directory;
use fpv::highlight::syntax::HighlightContext;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use tempfile::tempdir;

#[test]
fn directory_transitions_are_fast() {
    let d = tempdir().expect("tempdir");
    fs::create_dir_all(d.path().join("a")).expect("mkdir");

    let mut state = SessionState::new(d.path().to_path_buf());
    let mut nodes = list_current_directory(d.path(), 2000).expect("list");
    state.revalidate_selection(&nodes);

    let started = Instant::now();
    let _ = enter_selected_directory(&mut state, &mut nodes).expect("enter");
    let _ = go_to_parent_directory(&mut state, &mut nodes).expect("parent");
    assert!(started.elapsed().as_millis() < 200);
}

#[test]
fn metadata_refresh_stays_within_budget() {
    let d = tempdir().expect("tempdir");
    let file = d.path().join("perf.txt");
    fs::write(&file, "x").expect("write");
    let nodes = vec![TreeNode {
        path: file,
        name: "perf.txt".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    }];
    let mut state = SessionState::new(PathBuf::from("."));
    let ctx = HighlightContext::new();

    let started = Instant::now();
    for _ in 0..10 {
        let _ = refresh_preview(&mut state, &nodes, &ctx, 1024);
    }
    assert!(started.elapsed().as_millis() < 200);
}
