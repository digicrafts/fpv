use fpv::app::navigation::{move_down, move_up};
use fpv::app::state::SessionState;
use std::path::PathBuf;

#[test]
fn selection_does_not_underflow_or_overflow() {
    let mut state = SessionState::new(PathBuf::from("."));
    move_up(&mut state);
    assert_eq!(state.selected_index, 0);
    move_down(&mut state, 2);
    move_down(&mut state, 2);
    move_down(&mut state, 2);
    assert_eq!(state.selected_index, 1);
}
