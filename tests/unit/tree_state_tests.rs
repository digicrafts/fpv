use fpv::app::navigation::{collapse_selected, expand_selected, move_down, move_up};
use fpv::app::state::{LayoutRegions, NodeType, SessionState, TreeNode};
use fpv::config::load::ThemeProfile;
use fpv::fs::git::{GitFileStatus, GitRepoStatus};
use fpv::tui::tree_pane::{
    color_from_name, current_directory_header_line, directory_contains_uncommitted_changes,
    display_path_with_home, entry_prefix, node_style,
};
use ratatui::style::{Color, Modifier};
use std::path::PathBuf;

fn sample_nodes() -> Vec<TreeNode> {
    vec![
        TreeNode {
            path: PathBuf::from("a"),
            name: "a".to_string(),
            node_type: NodeType::Directory,
            depth: 0,
            expanded: false,
            readable: true,
            children_loaded: true,
        },
        TreeNode {
            path: PathBuf::from("b"),
            name: "b".to_string(),
            node_type: NodeType::File,
            depth: 1,
            expanded: false,
            readable: true,
            children_loaded: true,
        },
    ]
}

#[test]
fn move_selection_stays_in_range() {
    let mut state = SessionState::new(PathBuf::from("."));
    move_down(&mut state, 2);
    assert_eq!(state.selected_index, 1);
    move_down(&mut state, 2);
    assert_eq!(state.selected_index, 1);
    move_up(&mut state);
    assert_eq!(state.selected_index, 0);
}

#[test]
fn expand_and_collapse_directory() {
    let mut nodes = sample_nodes();
    expand_selected(&mut nodes, 0);
    assert!(nodes[0].expanded);
    collapse_selected(&mut nodes, 0);
    assert!(!nodes[0].expanded);
}

#[test]
fn prefix_mapping_uses_requested_ascii_markers() {
    assert_eq!(entry_prefix(&NodeType::Directory), "/");
    assert_eq!(entry_prefix(&NodeType::File), "");
    assert_eq!(entry_prefix(&NodeType::Symlink), "@");
}

#[test]
fn layout_regions_default_to_enabled() {
    let regions = LayoutRegions::default();
    assert!(regions.top_directory_header);
    assert!(regions.left_navigation_panel);
    assert!(regions.right_preview_panel);
    assert!(regions.preview_top_status_bar);
    assert!(regions.bottom_global_status_bar);
}

#[test]
fn current_directory_header_truncates_predictably() {
    let mut state = SessionState::new(PathBuf::from("/tmp/very/long/path/for/testing/header"));
    state.current_path = PathBuf::from("/tmp/very/long/path/for/testing/header");
    let line = current_directory_header_line(&state, 20);
    assert_eq!(line.chars().count(), 20);
    assert!(line.ends_with('…'));
}

#[test]
fn current_directory_header_normalizes_dot_root() {
    let state = SessionState::new(PathBuf::from("."));
    let line = current_directory_header_line(&state, 512);
    let cwd = std::env::current_dir().expect("cwd");
    assert_eq!(line, cwd.display().to_string());
}

#[test]
fn current_directory_header_appends_git_branch() {
    let mut state = SessionState::new(PathBuf::from("/tmp/work/fpv"));
    state.current_path = PathBuf::from("/tmp/work/fpv");
    state.git_status = Some(GitRepoStatus {
        branch: "feature/ui".to_string(),
        repo_root: PathBuf::from("/tmp/work/fpv"),
        file_statuses: Default::default(),
    });

    let line = current_directory_header_line(&state, 200);
    assert!(line.contains("/tmp/work/fpv"));
    assert!(line.contains("git:(feature/ui)"));
    assert!(!line.contains("changes"));
}

#[test]
fn current_directory_header_includes_change_count_when_dirty() {
    let mut state = SessionState::new(PathBuf::from("/tmp/work/fpv"));
    state.current_path = PathBuf::from("/tmp/work/fpv");
    let mut file_statuses = std::collections::HashMap::new();
    file_statuses.insert(PathBuf::from("a.txt"), GitFileStatus::Modified);
    file_statuses.insert(PathBuf::from("b.txt"), GitFileStatus::Deleted);
    state.git_status = Some(GitRepoStatus {
        branch: "main".to_string(),
        repo_root: PathBuf::from("/tmp/work/fpv"),
        file_statuses,
    });

    let line = current_directory_header_line(&state, 200);
    assert!(line.contains("git:(main)"));
    assert!(line.contains("[2 changes]"));
}

#[test]
fn home_path_is_rendered_with_tilde_prefix() {
    std::env::set_var("HOME", "/tmp/fpv-home");
    let rendered = display_path_with_home(&PathBuf::from("/tmp/fpv-home/projects/fpv"));
    assert_eq!(rendered, "~/projects/fpv");
}

#[test]
fn non_home_path_stays_absolute() {
    std::env::set_var("HOME", "/tmp/fpv-home");
    let rendered = display_path_with_home(&PathBuf::from("/opt/work/fpv"));
    assert_eq!(rendered, "/opt/work/fpv");
}

#[test]
fn style_resolution_uses_theme_and_hidden_dimming() {
    let mut theme = ThemeProfile {
        directory_color: "yellow".to_string(),
        fallback_file_color: "white".to_string(),
        hidden_dim_enabled: true,
        ..ThemeProfile::default()
    };
    theme
        .file_type_colors
        .insert("rs".to_string(), "cyan".to_string());

    let node = TreeNode {
        path: PathBuf::from(".main.rs"),
        name: ".main.rs".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    };
    let style = node_style(&node, &theme, None);
    assert_eq!(style.fg, Some(Color::Cyan));
    assert!(style.add_modifier.contains(Modifier::DIM));
    assert_eq!(color_from_name("yellow"), Color::Yellow);
}

#[test]
fn gitignored_entries_use_dim_gray_style() {
    let theme = ThemeProfile::default();
    let node = TreeNode {
        path: PathBuf::from("target"),
        name: "target".to_string(),
        node_type: NodeType::Directory,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    };
    let style = node_style(&node, &theme, Some(GitFileStatus::Ignored));
    assert_eq!(style.fg, Some(Color::DarkGray));
    assert!(style.add_modifier.contains(Modifier::DIM));
}

#[test]
fn preview_resize_clamps_to_minimum_widths() {
    let mut state = SessionState::new(PathBuf::from("."));
    state.tree_min_width_cols = 30;
    state.preview_min_width_cols = 20;
    state.preview_width_cols = 10;

    state.normalize_preview_width(80);
    let (tree, preview) = state.panel_widths(80);
    assert_eq!(preview, 20);
    assert_eq!(tree, 60);

    state.preview_width_cols = 70;
    state.normalize_preview_width(80);
    let (tree, preview) = state.panel_widths(80);
    assert_eq!(preview, 50);
    assert_eq!(tree, 30);
}

#[test]
fn preview_resize_step_is_applied_for_keyboard_updates() {
    let mut state = SessionState::new(PathBuf::from("."));
    state.preview_width_cols = 40;
    state.preview_resize_step_cols = 4;

    state.resize_preview_by(state.resize_step() as i16, 100);
    assert_eq!(state.panel_widths(100).1, 44);

    state.resize_preview_by(-(state.resize_step() as i16), 100);
    assert_eq!(state.panel_widths(100).1, 40);
}

#[test]
fn preview_scroll_clamps_and_pages_within_bounds() {
    let mut state = SessionState::new(PathBuf::from("."));
    state.scroll_preview_lines(50, 100, 20);
    assert_eq!(state.preview_scroll_row, 50);

    state.page_scroll_preview_down(100, 20);
    assert_eq!(state.preview_scroll_row, 70);

    state.page_scroll_preview_down(100, 20);
    assert_eq!(state.preview_scroll_row, 80);

    state.page_scroll_preview_up(100, 20);
    assert_eq!(state.preview_scroll_row, 60);

    state.scroll_preview_lines(-999, 100, 20);
    assert_eq!(state.preview_scroll_row, 0);
}

#[test]
fn preview_scroll_is_reclamped_when_content_shrinks() {
    let mut state = SessionState::new(PathBuf::from("."));
    state.scroll_preview_lines(80, 120, 20);
    assert_eq!(state.preview_scroll_row, 80);

    state.clamp_preview_scroll(30, 20);
    assert_eq!(state.preview_scroll_row, 10);
}

#[test]
fn preview_defaults_enable_line_numbers_and_disable_wrap() {
    let state = SessionState::new(PathBuf::from("."));
    assert!(state.preview_show_line_numbers);
    assert!(!state.preview_wrap_enabled);
}

#[test]
fn directory_change_marker_detects_nested_uncommitted_changes() {
    let node = TreeNode {
        path: PathBuf::from("/repo/src"),
        name: "src".to_string(),
        node_type: NodeType::Directory,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: true,
    };
    let mut file_statuses = std::collections::HashMap::new();
    file_statuses.insert(PathBuf::from("src/lib/main.rs"), GitFileStatus::Modified);

    let mut state = SessionState::new(PathBuf::from("/repo"));
    state.git_status = Some(GitRepoStatus {
        branch: "main".to_string(),
        repo_root: PathBuf::from("/repo"),
        file_statuses,
    });

    assert!(directory_contains_uncommitted_changes(&state, &node));
}

#[test]
fn directory_change_marker_ignores_gitignored_entries() {
    let node = TreeNode {
        path: PathBuf::from("/repo/target"),
        name: "target".to_string(),
        node_type: NodeType::Directory,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: true,
    };
    let mut file_statuses = std::collections::HashMap::new();
    file_statuses.insert(PathBuf::from("target/"), GitFileStatus::Ignored);

    let mut state = SessionState::new(PathBuf::from("/repo"));
    state.git_status = Some(GitRepoStatus {
        branch: "main".to_string(),
        repo_root: PathBuf::from("/repo"),
        file_statuses,
    });

    assert!(!directory_contains_uncommitted_changes(&state, &node));
}

#[test]
fn directory_change_marker_applies_to_directories_only() {
    let node = TreeNode {
        path: PathBuf::from("/repo/src/main.rs"),
        name: "main.rs".to_string(),
        node_type: NodeType::File,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: true,
    };
    let mut file_statuses = std::collections::HashMap::new();
    file_statuses.insert(PathBuf::from("src/main.rs"), GitFileStatus::Modified);

    let mut state = SessionState::new(PathBuf::from("/repo"));
    state.git_status = Some(GitRepoStatus {
        branch: "main".to_string(),
        repo_root: PathBuf::from("/repo"),
        file_statuses,
    });

    assert!(!directory_contains_uncommitted_changes(&state, &node));
}
