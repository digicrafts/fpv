use fpv::app::preview_controller::refresh_preview;
use fpv::app::state::{ContentType, NodeType, PreviewFallbackReason, SessionState, TreeNode};
use fpv::config::load::ThemeProfile;
use fpv::fs::preview::load_preview;
use fpv::highlight::syntax::HighlightContext;
use fpv::tui::preview_pane::preview_title_for_state;
use fpv::tui::status_bar::compose_preview_metadata_line;
use fpv::tui::tree_pane::node_style;
use ratatui::style::Modifier;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn binary_preview_never_panics() {
    let d = tempdir().expect("create tempdir");
    let p = d.path().join("blob.bin");
    fs::write(&p, [0_u8, 1, 2, 3, 4]).expect("write file");
    let ctx = HighlightContext::new();
    let doc = load_preview(&p, 1024, &ctx);
    assert!(doc.error_message.is_some());
}

#[test]
fn missing_file_returns_error_state_message() {
    let ctx = HighlightContext::new();
    let p = std::path::Path::new("/tmp/fpv-not-found-file");
    let doc = load_preview(p, 1024, &ctx);
    assert!(doc.error_message.is_some());
}

#[test]
fn directory_preview_does_not_emit_error_message() {
    let d = tempdir().expect("create tempdir");
    let dir = d.path().join("dir");
    fs::create_dir_all(&dir).expect("mkdir");

    let mut state = SessionState::new(PathBuf::from("."));
    let nodes = vec![TreeNode {
        path: dir,
        name: "dir".to_string(),
        node_type: NodeType::Directory,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    }];

    let ctx = HighlightContext::new();
    let doc = refresh_preview(&mut state, &nodes, &ctx, 1024);
    assert!(doc.error_message.is_none());
}

#[test]
fn hidden_dimming_applies_without_removing_base_color() {
    let theme = ThemeProfile {
        hidden_dim_enabled: true,
        ..ThemeProfile::default()
    };
    let hidden = TreeNode {
        path: PathBuf::from(".secret.txt"),
        name: ".secret.txt".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    };
    let style = node_style(&hidden, &theme, None);
    assert!(style.fg.is_some());
    assert!(style.add_modifier.contains(Modifier::DIM));
}

#[test]
fn preview_title_and_footer_remain_in_sync_after_multiple_selection_changes() {
    let d = tempdir().expect("create tempdir");
    let p1 = d.path().join("alpha.txt");
    let p2 = d.path().join("beta.txt");
    fs::write(&p1, "alpha").expect("write file");
    fs::write(&p2, "beta").expect("write file");

    let nodes = vec![
        TreeNode {
            path: p1,
            name: "alpha.txt".to_string(),
            node_type: NodeType::File,
            depth: 0,
            expanded: false,
            readable: true,
            children_loaded: false,
        },
        TreeNode {
            path: p2,
            name: "beta.txt".to_string(),
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
        let _ = refresh_preview(&mut state, &nodes, &ctx, 1024);
        let title = preview_title_for_state(&state);
        assert_eq!(title, state.selected_metadata.filename);

        let footer = compose_preview_metadata_line(&state.selected_metadata, 200);
        assert!(footer.starts_with("Unknown(.txt) | "));
        assert!(footer.contains(" | "));
    }
}

#[test]
fn mixed_supported_and_unsupported_navigation_keeps_preview_readable() {
    let d = tempdir().expect("create tempdir");
    let supported = d.path().join("main.rs");
    let unsupported = d.path().join("notes.xyz");
    fs::write(&supported, "fn main() { println!(\"ok\"); }\n").expect("write rs");
    fs::write(&unsupported, "plain fallback body\n").expect("write xyz");

    let nodes = vec![
        TreeNode {
            path: supported,
            name: "main.rs".to_string(),
            node_type: NodeType::File,
            depth: 0,
            expanded: false,
            readable: true,
            children_loaded: false,
        },
        TreeNode {
            path: unsupported,
            name: "notes.xyz".to_string(),
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
    let highlighted = refresh_preview(&mut state, &nodes, &ctx, 1024);
    assert_eq!(highlighted.content_type, ContentType::Highlighted);

    state.selected_index = 1;
    let fallback = refresh_preview(&mut state, &nodes, &ctx, 1024);
    assert_eq!(fallback.content_type, ContentType::PlainText);
    assert_eq!(
        fallback.fallback_reason,
        Some(PreviewFallbackReason::UnsupportedExtension)
    );
    assert!(fallback.error_message.is_none());
}
