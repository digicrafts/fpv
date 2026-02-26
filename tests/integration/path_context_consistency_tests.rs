use fpv::app::navigation::{
    enter_selected_directory, format_status_with_path, go_to_parent_directory,
};
use fpv::app::state::SessionState;
use fpv::config::keymap::default_keymap;
use fpv::config::load::StatusDisplayMode;
use fpv::fs::current_dir::list_current_directory;
use fpv::tui::status_bar::{compose_bottom_status_line, compose_status_title_line};
use fpv::tui::tree_pane::display_path_with_home;
use std::fs;
use tempfile::tempdir;

#[test]
fn status_line_contains_current_path() {
    let d = tempdir().expect("tempdir");
    fs::create_dir_all(d.path().join("child")).expect("mkdir");

    let mut state = SessionState::new(d.path().to_path_buf());
    let mut nodes = list_current_directory(d.path(), 2000).expect("list");
    state.revalidate_selection(&nodes);
    let result = enter_selected_directory(&mut state, &mut nodes).expect("enter");
    let line = format_status_with_path(&result.message, &state.current_path);
    assert!(line.contains("Path:"));

    let _ = go_to_parent_directory(&mut state, &mut nodes).expect("parent");
    assert_eq!(state.current_path, d.path().to_path_buf());
}

#[test]
fn home_path_is_shortened_with_tilde_in_header_context() {
    std::env::set_var("HOME", "/tmp/fpv-home");
    let rendered = display_path_with_home(std::path::Path::new("/tmp/fpv-home/work/fpv"));
    assert_eq!(rendered, "~/work/fpv");
}

#[test]
fn status_line_is_single_row_in_bar_and_title_modes() {
    let mut state = SessionState::new(std::path::PathBuf::from("."));
    state.status_message = "Ready".to_string();
    let bindings = default_keymap();

    state.status_display_mode = StatusDisplayMode::Bar;
    let bar = compose_bottom_status_line(&state, &bindings, 60);
    assert_eq!(bar.lines().count(), 1);
    assert!(bar.contains("fpv "));

    state.status_display_mode = StatusDisplayMode::Title;
    let title = compose_status_title_line(&state, &bindings, 60);
    assert_eq!(title.lines().count(), 1);
    assert!(title.contains("fpv "));
}
