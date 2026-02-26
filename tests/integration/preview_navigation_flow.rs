use fpv::app::preview_controller::refresh_preview;
use fpv::app::state::{ContentType, NodeType, SessionState, TreeNode};
use fpv::highlight::syntax::HighlightContext;
use fpv::tui::status_bar::compose_preview_metadata_line;
use fpv::tui::tree_pane::current_directory_header_line;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn selection_change_updates_preview_source() {
    let d = tempdir().expect("create tempdir");
    let p1 = d.path().join("a.md");
    let p2 = d.path().join("b.py");
    fs::write(&p1, "# a\n").expect("write file");
    fs::write(&p2, "print('b')\n").expect("write file");

    let nodes = vec![
        TreeNode {
            path: p1.clone(),
            name: "a.md".to_string(),
            node_type: NodeType::File,
            depth: 0,
            expanded: false,
            readable: true,
            children_loaded: false,
        },
        TreeNode {
            path: p2.clone(),
            name: "b.py".to_string(),
            node_type: NodeType::File,
            depth: 0,
            expanded: false,
            readable: true,
            children_loaded: false,
        },
    ];

    let mut state = SessionState::new(PathBuf::from("."));
    let ctx = HighlightContext::new();

    state.selected_index = 1;
    let doc = refresh_preview(&mut state, &nodes, &ctx, 1024);
    assert_eq!(doc.source_path, p2);
    assert_eq!(state.selected_path, p2);
    assert_eq!(doc.content_type, ContentType::Highlighted);
    assert!(!doc.styled_lines.is_empty());
}

#[test]
fn highlighted_preview_remains_consistent_across_file_switches() {
    let d = tempdir().expect("create tempdir");
    let a = d.path().join("a.rs");
    let b = d.path().join("b.py");
    fs::write(&a, "fn main() { let v = 1; }\n").expect("write a");
    fs::write(&b, "print('v')\n").expect("write b");

    let nodes = vec![
        TreeNode {
            path: a,
            name: "a.rs".to_string(),
            node_type: NodeType::File,
            depth: 0,
            expanded: false,
            readable: true,
            children_loaded: false,
        },
        TreeNode {
            path: b,
            name: "b.py".to_string(),
            node_type: NodeType::File,
            depth: 0,
            expanded: false,
            readable: true,
            children_loaded: false,
        },
    ];

    let mut state = SessionState::new(PathBuf::from("."));
    let ctx = HighlightContext::new();
    for idx in [0, 1, 0, 1] {
        state.selected_index = idx;
        let doc = refresh_preview(&mut state, &nodes, &ctx, 1024);
        assert_eq!(doc.content_type, ContentType::Highlighted);
        assert!(doc.language_id.is_some());
        assert!(!doc.styled_lines.is_empty());
    }
}

#[test]
fn directory_selection_shows_neutral_preview_while_file_errors_remain() {
    let d = tempdir().expect("create tempdir");
    let folder = d.path().join("folder");
    fs::create_dir_all(&folder).expect("mkdir");
    let missing = d.path().join("missing.py");

    let nodes = vec![
        TreeNode {
            path: folder.clone(),
            name: "folder".to_string(),
            node_type: NodeType::Directory,
            depth: 0,
            expanded: false,
            readable: true,
            children_loaded: false,
        },
        TreeNode {
            path: missing.clone(),
            name: "missing.py".to_string(),
            node_type: NodeType::File,
            depth: 0,
            expanded: false,
            readable: false,
            children_loaded: false,
        },
    ];

    let mut state = SessionState::new(PathBuf::from("."));
    let ctx = HighlightContext::new();

    state.selected_index = 0;
    let dir_doc = refresh_preview(&mut state, &nodes, &ctx, 1024);
    assert!(dir_doc.error_message.is_none());
    assert_eq!(dir_doc.content_excerpt, "(directory selected)");
    let meta_line = compose_preview_metadata_line(&state.selected_metadata, 120);
    assert!(meta_line.starts_with("Unknown(none) | "));

    state.selected_index = 1;
    let file_doc = refresh_preview(&mut state, &nodes, &ctx, 1024);
    assert!(file_doc.error_message.is_some());
}

#[test]
fn header_and_preview_metadata_strings_follow_layout_contract() {
    let d = tempdir().expect("create tempdir");
    let file = d.path().join("sample.md");
    fs::write(&file, "hello").expect("write");

    let nodes = vec![TreeNode {
        path: file,
        name: "sample.md".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    }];
    let mut state = SessionState::new(d.path().to_path_buf());
    let ctx = HighlightContext::new();
    let _ = refresh_preview(&mut state, &nodes, &ctx, 1024);

    let header = current_directory_header_line(&state, 120);
    assert!(!header.starts_with("Current: "));
    assert!(header.contains(&d.path().display().to_string()));

    let meta_line = compose_preview_metadata_line(&state.selected_metadata, 120);
    assert!(meta_line.starts_with("Markdown(.md) | "));
    assert!(meta_line.contains(" | "));
}
