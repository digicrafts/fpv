use fpv::app::preview_controller::refresh_preview;
use fpv::app::state::{NodeType, SessionState, TreeNode};
use fpv::highlight::syntax::HighlightContext;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn permission_text_remains_stable_across_selection_changes() {
    let d = tempdir().expect("tempdir");
    let a = d.path().join("a.txt");
    let b = d.path().join("b.txt");
    fs::write(&a, "a").expect("write");
    fs::write(&b, "b").expect("write");

    #[cfg(unix)]
    {
        let mut perms_a = fs::metadata(&a).expect("metadata").permissions();
        perms_a.set_mode(0o640);
        fs::set_permissions(&a, perms_a).expect("chmod a");

        let mut perms_b = fs::metadata(&b).expect("metadata").permissions();
        perms_b.set_mode(0o755);
        fs::set_permissions(&b, perms_b).expect("chmod b");
    }

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
    let first = state.selected_metadata.permission_text.clone();

    state.selected_index = 1;
    let _ = refresh_preview(&mut state, &nodes, &ctx, 1024);
    let second = state.selected_metadata.permission_text.clone();

    assert_eq!(first.chars().count(), 9);
    assert_eq!(second.chars().count(), 9);
    assert_ne!(first, second);
}
