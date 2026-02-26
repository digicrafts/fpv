use crate::app::focus::switch_focus;
use crate::app::navigation::{
    enter_selected_directory, format_status_with_path, go_to_parent_directory, move_down, move_up,
    toggle_hidden_visibility,
};
use crate::app::state::{FocusPane, NodeType, SessionState, TreeNode};
use crate::app::status::navigation_status_message;
use crate::config::keymap::Action;
use crate::fs::current_dir::is_filesystem_root;
use crate::tui::input::map_key_to_action;
use anyhow::Result;
use crossterm::event::{self, Event, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::collections::HashMap;
use std::time::Duration;

fn main_area_width() -> u16 {
    crossterm::terminal::size().map(|(w, _)| w).unwrap_or(80)
}

fn apply_mouse_resize(state: &mut SessionState, mouse: MouseEvent) -> bool {
    let width = main_area_width();
    let divider = state.divider_column(width);
    let near_divider = mouse.column.abs_diff(divider) <= 1;
    let before = state.preview_width_cols;

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) if near_divider => {
            state.divider_drag_active = true;
            state.set_preview_width_from_divider(mouse.column, width);
            state.preview_width_cols != before
        }
        MouseEventKind::Drag(MouseButton::Left) if state.divider_drag_active => {
            state.set_preview_width_from_divider(mouse.column, width);
            state.preview_width_cols != before
        }
        MouseEventKind::Moved if state.divider_drag_active => {
            state.set_preview_width_from_divider(mouse.column, width);
            state.preview_width_cols != before
        }
        MouseEventKind::Up(MouseButton::Left) if state.divider_drag_active => {
            state.divider_drag_active = false;
            state.set_preview_width_from_divider(mouse.column, width);
            state.preview_width_cols != before
        }
        _ => false,
    }
}

fn in_preview_panel(state: &SessionState, mouse: MouseEvent) -> bool {
    let divider = state.divider_column(main_area_width());
    state.preview_fullscreen || mouse.column >= divider
}

fn tree_panel_area(state: &SessionState, terminal_width: u16, terminal_height: u16) -> Rect {
    let main_height = terminal_height.saturating_sub(2);
    let (tree_width, _) = state.panel_widths(terminal_width);
    Rect::new(0, 1, tree_width, main_height)
}

fn tree_index_for_click(
    state: &SessionState,
    mouse: MouseEvent,
    nodes_len: usize,
) -> Option<usize> {
    if state.preview_fullscreen {
        return None;
    }
    let (terminal_width, terminal_height) = crossterm::terminal::size().ok()?;
    let area = tree_panel_area(state, terminal_width, terminal_height);

    if mouse.column < area.x
        || mouse.column >= area.x.saturating_add(area.width)
        || mouse.row <= area.y
        || mouse.row >= area.y.saturating_add(area.height).saturating_sub(1)
    {
        return None;
    }

    let index = mouse.row.saturating_sub(area.y + 1) as usize;
    if index < nodes_len {
        Some(index)
    } else {
        None
    }
}

fn can_enter_fullscreen_preview(state: &SessionState, nodes: &[TreeNode]) -> bool {
    nodes
        .get(state.selected_index)
        .map(|node| node.node_type == NodeType::File)
        .unwrap_or(false)
}

pub fn process_once(
    state: &mut SessionState,
    nodes: &mut Vec<TreeNode>,
    bindings: &HashMap<Action, crossterm::event::KeyEvent>,
    preview_total_lines: usize,
    preview_viewport_rows: usize,
) -> Result<(bool, bool)> {
    if !event::poll(Duration::from_millis(50))? {
        return Ok((false, false));
    }

    let mut should_refresh_preview = false;
    match event::read()? {
        Event::Key(key) => {
            if let Some(action) = map_key_to_action(key, bindings) {
                match action {
                    Action::ToggleHelp => {
                        state.help_overlay_visible = !state.help_overlay_visible;
                    }
                    Action::MoveUp => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        if state.preview_fullscreen {
                            state.scroll_preview_lines(
                                -1,
                                preview_total_lines,
                                preview_viewport_rows,
                            );
                        } else {
                            move_up(state);
                            state.update_selected_path(nodes);
                            state.reset_preview_scroll();
                            should_refresh_preview = true;
                        }
                    }
                    Action::MoveDown => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        if state.preview_fullscreen {
                            state.scroll_preview_lines(
                                1,
                                preview_total_lines,
                                preview_viewport_rows,
                            );
                        } else {
                            move_down(state, nodes.len());
                            state.update_selected_path(nodes);
                            state.reset_preview_scroll();
                            should_refresh_preview = true;
                        }
                    }
                    Action::Expand => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        if !state.preview_fullscreen {
                            let result = enter_selected_directory(state, nodes)?;
                            state.status_message = format_status_with_path(
                                &navigation_status_message(&result),
                                &state.current_path,
                            );
                            state.reset_preview_scroll();
                            should_refresh_preview = true;
                        }
                    }
                    Action::Collapse => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        if !state.preview_fullscreen && !is_filesystem_root(&state.current_path) {
                            let result = go_to_parent_directory(state, nodes)?;
                            state.status_message = format_status_with_path(
                                &navigation_status_message(&result),
                                &state.current_path,
                            );
                            state.reset_preview_scroll();
                            should_refresh_preview = true;
                        }
                    }
                    Action::Open => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        if !state.preview_fullscreen {
                            if let Some(node) = nodes.get(state.selected_index) {
                                if state.focus_pane == FocusPane::Tree
                                    && node.node_type == NodeType::Directory
                                {
                                    let result = enter_selected_directory(state, nodes)?;
                                    state.status_message = format_status_with_path(
                                        &navigation_status_message(&result),
                                        &state.current_path,
                                    );
                                    state.reset_preview_scroll();
                                    should_refresh_preview = true;
                                } else if node.node_type == NodeType::File
                                    && can_enter_fullscreen_preview(state, nodes)
                                {
                                    state.preview_fullscreen = true;
                                    state.focus_pane = FocusPane::Preview;
                                }
                            }
                        }
                    }
                    Action::ExitFullscreenPreview => {
                        if state.help_overlay_visible {
                            state.help_overlay_visible = false;
                        } else if state.preview_fullscreen {
                            state.preview_fullscreen = false;
                        }
                    }
                    Action::SwitchFocus => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        if !state.preview_fullscreen {
                            switch_focus(state);
                        }
                    }
                    Action::PageUp => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        if state.preview_fullscreen || state.focus_pane == FocusPane::Preview {
                            state
                                .page_scroll_preview_up(preview_total_lines, preview_viewport_rows);
                        }
                    }
                    Action::PageDown => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        if state.preview_fullscreen || state.focus_pane == FocusPane::Preview {
                            state.page_scroll_preview_down(
                                preview_total_lines,
                                preview_viewport_rows,
                            );
                        }
                    }
                    Action::PreviewScrollUp => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        if state.preview_fullscreen || state.focus_pane == FocusPane::Preview {
                            state.scroll_preview_lines(
                                -3,
                                preview_total_lines,
                                preview_viewport_rows,
                            );
                        }
                    }
                    Action::PreviewScrollDown => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        if state.preview_fullscreen || state.focus_pane == FocusPane::Preview {
                            state.scroll_preview_lines(
                                3,
                                preview_total_lines,
                                preview_viewport_rows,
                            );
                        }
                    }
                    Action::TogglePreviewLineNumbers => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        state.preview_show_line_numbers = !state.preview_show_line_numbers;
                    }
                    Action::TogglePreviewWrap => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        state.preview_wrap_enabled = !state.preview_wrap_enabled;
                    }
                    Action::ToggleHidden => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        if !state.preview_fullscreen {
                            let result = toggle_hidden_visibility(state, nodes)?;
                            state.status_message = format_status_with_path(
                                &navigation_status_message(&result),
                                &state.current_path,
                            );
                            state.reset_preview_scroll();
                            should_refresh_preview = true;
                        }
                    }
                    Action::ResizePreviewNarrower => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        if !state.preview_fullscreen {
                            let step = state.resize_step() as i16;
                            state.resize_preview_by(-step, main_area_width());
                            should_refresh_preview = true;
                        }
                    }
                    Action::ResizePreviewWider => {
                        if state.help_overlay_visible {
                            return Ok((false, false));
                        }
                        if !state.preview_fullscreen {
                            let step = state.resize_step() as i16;
                            state.resize_preview_by(step, main_area_width());
                            should_refresh_preview = true;
                        }
                    }
                    Action::Quit => return Ok((true, false)),
                }
            }
        }
        Event::Mouse(mouse) => {
            if state.help_overlay_visible {
                return Ok((false, false));
            }
            if !state.preview_fullscreen {
                should_refresh_preview = apply_mouse_resize(state, mouse);
            }

            if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                if let Some(index) = tree_index_for_click(state, mouse, nodes.len()) {
                    state.selected_index = index;
                    state.update_selected_path(nodes);
                    state.reset_preview_scroll();
                    should_refresh_preview = true;

                    if nodes
                        .get(index)
                        .is_some_and(|node| node.node_type == NodeType::Directory)
                    {
                        let result = enter_selected_directory(state, nodes)?;
                        state.status_message = format_status_with_path(
                            &navigation_status_message(&result),
                            &state.current_path,
                        );
                        state.reset_preview_scroll();
                        should_refresh_preview = true;
                    }
                }
            }

            if in_preview_panel(state, mouse) {
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        state.scroll_preview_lines(-3, preview_total_lines, preview_viewport_rows);
                    }
                    MouseEventKind::ScrollDown => {
                        state.scroll_preview_lines(3, preview_total_lines, preview_viewport_rows);
                    }
                    _ => {}
                }
            }
        }
        Event::Resize(width, _) => {
            state.normalize_preview_width(width);
        }
        _ => {}
    }

    Ok((false, should_refresh_preview))
}

#[cfg(test)]
mod tests {
    use super::{tree_index_for_click, tree_panel_area};
    use crate::app::state::SessionState;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use std::path::PathBuf;

    #[test]
    fn tree_panel_area_matches_expected_rows() {
        let state = SessionState::new(PathBuf::from("."));
        let area = tree_panel_area(&state, 120, 40);
        assert_eq!(area.x, 0);
        assert_eq!(area.y, 1);
        assert_eq!(area.height, 38);
    }

    #[test]
    fn click_outside_or_on_border_has_no_index() {
        let state = SessionState::new(PathBuf::from("."));
        let click_on_top_border = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        assert!(tree_index_for_click(&state, click_on_top_border, 10).is_none());
    }
}
