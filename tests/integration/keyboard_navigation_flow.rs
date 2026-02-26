use fpv::app::focus::switch_focus;
use fpv::app::navigation::{move_down, move_up, toggle_hidden_visibility};
use fpv::app::state::{FocusPane, SessionState};
use fpv::config::keymap::default_keymap;
use fpv::fs::current_dir::list_current_directory;
use fpv::tui::status_bar::compose_bottom_status_line;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn keyboard_navigation_state_flow() {
    let mut state = SessionState::new(PathBuf::from("."));
    move_down(&mut state, 3);
    move_down(&mut state, 3);
    assert_eq!(state.selected_index, 2);
    move_up(&mut state);
    assert_eq!(state.selected_index, 1);

    assert_eq!(state.focus_pane, FocusPane::Tree);
    switch_focus(&mut state);
    assert_eq!(state.focus_pane, FocusPane::Preview);
}

#[test]
fn hidden_toggle_updates_list_and_keeps_valid_selection() {
    let d = tempdir().expect("tempdir");
    fs::create_dir_all(d.path().join("dir_visible")).expect("mkdir");
    fs::create_dir_all(d.path().join(".dir_hidden")).expect("mkdir");
    fs::write(d.path().join("visible.txt"), "v").expect("write");
    fs::write(d.path().join(".hidden.txt"), "h").expect("write");

    let mut state = SessionState::new(d.path().to_path_buf());
    let mut nodes = list_current_directory(d.path(), 2000).expect("list");
    state.selected_index = nodes.len().saturating_sub(1);
    let previous_path = nodes[state.selected_index].path.clone();

    let _ = toggle_hidden_visibility(&mut state, &mut nodes).expect("toggle on");
    assert!(state.show_hidden);
    assert_eq!(state.current_path, d.path().to_path_buf());
    assert!(nodes.iter().any(|n| n.name.starts_with('.')));
    assert!(state.selected_index < nodes.len() || nodes.is_empty());
    let bindings = default_keymap();
    let on_status = compose_bottom_status_line(&state, &bindings, 120);
    assert!(on_status.contains("hidden=on"));
    assert!(on_status.contains("Help (?)"));

    let _ = toggle_hidden_visibility(&mut state, &mut nodes).expect("toggle off");
    assert!(!state.show_hidden);
    assert_eq!(state.current_path, d.path().to_path_buf());
    assert!(nodes.iter().all(|n| !n.name.starts_with('.')));
    assert!(state.selected_index < nodes.len() || nodes.is_empty());
    let off_status = compose_bottom_status_line(&state, &bindings, 120);
    assert!(off_status.contains("hidden=off"));
    assert!(off_status.contains("Help (?)"));
    if !previous_path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name.starts_with('.'))
    {
        assert!(nodes.iter().any(|n| n.path == previous_path));
    }
}

#[test]
fn keyboard_resize_updates_preview_panel_width() {
    let mut state = SessionState::new(PathBuf::from("."));
    state.preview_width_cols = 40;
    state.preview_resize_step_cols = 2;

    state.resize_preview_by(state.resize_step() as i16, 100);
    assert_eq!(state.panel_widths(100), (58, 42));

    state.resize_preview_by(-(state.resize_step() as i16), 100);
    assert_eq!(state.panel_widths(100), (60, 40));
}
