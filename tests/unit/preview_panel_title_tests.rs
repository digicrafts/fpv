use fpv::app::state::{SelectedEntryMetadata, SessionState};
use fpv::tui::preview_pane::preview_title_for_state;
use std::path::PathBuf;

#[test]
fn preview_title_uses_selected_filename() {
    let mut state = SessionState::new(PathBuf::from("."));
    state.selected_metadata = SelectedEntryMetadata {
        filename: "config.toml".to_string(),
        ..SelectedEntryMetadata::default()
    };

    assert_eq!(preview_title_for_state(&state), "config.toml");
}

#[test]
fn preview_title_falls_back_when_selection_missing() {
    let mut state = SessionState::new(PathBuf::from("."));
    state.selected_metadata = SelectedEntryMetadata {
        filename: "-".to_string(),
        ..SelectedEntryMetadata::default()
    };

    assert_eq!(preview_title_for_state(&state), "Preview");
}
