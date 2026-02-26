use fpv::app::state::SessionState;
use std::path::PathBuf;

#[test]
fn divider_drag_updates_preview_width_with_clamp() {
    let mut state = SessionState::new(PathBuf::from("."));
    state.tree_min_width_cols = 24;
    state.preview_min_width_cols = 20;

    state.set_preview_width_from_divider(50, 100);
    assert_eq!(state.panel_widths(100), (50, 50));

    state.set_preview_width_from_divider(95, 100);
    assert_eq!(state.panel_widths(100), (80, 20));

    state.set_preview_width_from_divider(5, 100);
    assert_eq!(state.panel_widths(100), (24, 76));
}
