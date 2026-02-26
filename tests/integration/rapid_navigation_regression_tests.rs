use fpv::app::navigation::{enter_selected_directory, go_to_parent_directory};
use fpv::app::state::{NodeType, SessionState, TreeNode};
use fpv::config::load::ThemeProfile;
use fpv::fs::current_dir::list_current_directory;
use fpv::tui::tree_pane::node_style;
use ratatui::style::Color;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn rapid_mixed_navigation_keeps_state_coherent() {
    let d = tempdir().expect("tempdir");
    fs::create_dir_all(d.path().join("a/b")).expect("mkdir");

    let mut state = SessionState::new(d.path().to_path_buf());
    let mut nodes = list_current_directory(d.path(), 2000).expect("list");
    state.revalidate_selection(&nodes);

    for _ in 0..20 {
        let _ = enter_selected_directory(&mut state, &mut nodes);
        let _ = go_to_parent_directory(&mut state, &mut nodes);
    }

    assert_eq!(state.current_path, d.path().to_path_buf());
}

#[test]
fn theme_file_type_mapping_resolves_expected_color() {
    let mut theme = ThemeProfile::default();
    theme
        .file_type_colors
        .insert("md".to_string(), "green".to_string());
    let node = TreeNode {
        path: PathBuf::from("notes.md"),
        name: "notes.md".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    };
    let style = node_style(&node, &theme, None);
    assert_eq!(style.fg, Some(Color::Green));
}
