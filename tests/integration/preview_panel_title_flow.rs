use fpv::app::preview_controller::refresh_preview;
use fpv::app::state::{NodeType, SessionState, TreeNode};
use fpv::highlight::syntax::HighlightContext;
use fpv::tui::preview_pane::{preview_border_metadata_for_state, preview_title_for_state};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn preview_title_tracks_selection_changes() {
    let d = tempdir().expect("tempdir");
    let a = d.path().join("a.txt");
    let b = d.path().join("b.txt");
    fs::write(&a, "a").expect("write");
    fs::write(&b, "b").expect("write");

    let nodes = vec![
        TreeNode {
            path: a,
            name: "a.txt".to_string(),
            node_type: NodeType::File,
            depth: 0,
            expanded: false,
            readable: true,
            children_loaded: false,
        },
        TreeNode {
            path: b,
            name: "b.txt".to_string(),
            node_type: NodeType::File,
            depth: 0,
            expanded: false,
            readable: true,
            children_loaded: false,
        },
    ];

    let mut state = SessionState::new(PathBuf::from("."));
    let ctx = HighlightContext::new();

    state.selected_index = 0;
    let _ = refresh_preview(&mut state, &nodes, &ctx, 1024);
    assert_eq!(preview_title_for_state(&state), "a.txt");

    state.selected_index = 1;
    let _ = refresh_preview(&mut state, &nodes, &ctx, 1024);
    assert_eq!(preview_title_for_state(&state), "b.txt");
}

#[test]
fn border_metadata_uses_compact_contract_without_filename_prefix() {
    let d = tempdir().expect("tempdir");
    let p = d.path().join("note.py");
    fs::write(&p, "print('x')").expect("write");

    let nodes = vec![TreeNode {
        path: p,
        name: "note.py".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    }];

    let mut state = SessionState::new(PathBuf::from("."));
    let ctx = HighlightContext::new();
    let _ = refresh_preview(&mut state, &nodes, &ctx, 1024);

    let line = preview_border_metadata_for_state(&state, 120);
    assert!(line.starts_with("Python(.py) | "));
    assert!(!line.contains("name:"));
}
