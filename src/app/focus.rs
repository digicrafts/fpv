use crate::app::state::{FocusPane, SessionState};

pub fn switch_focus(state: &mut SessionState) {
    state.focus_pane = match state.focus_pane {
        FocusPane::Tree => FocusPane::Preview,
        FocusPane::Preview => FocusPane::Tree,
    };
}
