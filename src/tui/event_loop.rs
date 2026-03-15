use crate::app::focus::switch_focus;
use crate::app::navigation::{
    enter_selected_directory, format_status_with_path, go_to_parent_directory, move_down, move_up,
    toggle_hidden_visibility,
};
use crate::app::state::{
    ContentPosition, FocusPane, NodeType, PreviewDocument, PreviewSelection, SessionState, TreeNode,
};
use crate::app::status::navigation_status_message;
use crate::config::keymap::Action;
use crate::fs::current_dir::is_filesystem_root;
use crate::tui::input::map_key_to_action;
use crate::tui::preview_pane::{extract_selected_text, preview_max_line_width};
use anyhow::Result;
use crossterm::event::{self, Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::collections::HashMap;
use std::time::Duration;

fn main_area_width() -> u16 {
    crossterm::terminal::size().map(|(w, _)| w).unwrap_or(80)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScrollIntent {
    Vertical,
    Horizontal,
}

fn apply_mouse_resize(state: &mut SessionState, mouse: MouseEvent) -> bool {
    let width = main_area_width();
    let divider = state.divider_column(width);
    let near_divider = mouse.column.abs_diff(divider) <= 1;

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) if near_divider => {
            state.divider_drag_active = true;
            state.divider_drag_column = Some(state.clamped_divider_column(mouse.column, width));
            false
        }
        MouseEventKind::Drag(MouseButton::Left) if state.divider_drag_active => {
            state.divider_drag_column = Some(state.clamped_divider_column(mouse.column, width));
            false
        }
        MouseEventKind::Moved if state.divider_drag_active => {
            state.divider_drag_column = Some(state.clamped_divider_column(mouse.column, width));
            false
        }
        MouseEventKind::Up(MouseButton::Left) if state.divider_drag_active => {
            let target = state
                .divider_drag_column
                .unwrap_or_else(|| state.clamped_divider_column(mouse.column, width));
            state.divider_drag_active = false;
            state.divider_drag_column = None;
            state.set_preview_width_from_divider(target, width);
            false
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

fn mouse_to_content_position(state: &SessionState, mouse: MouseEvent) -> ContentPosition {
    let (inner_x, inner_y, _, _) = state.preview_inner_rect;
    let ln_cols = state.preview_line_number_cols;

    let screen_row = mouse.row.saturating_sub(inner_y) as usize;
    let screen_col = mouse.column.saturating_sub(inner_x) as usize;

    let content_row = screen_row + state.preview_scroll_row;
    let content_col = if ln_cols > 0 {
        screen_col
            .saturating_sub(ln_cols)
            .saturating_add(state.preview_scroll_col)
    } else {
        screen_col.saturating_add(state.preview_scroll_col)
    };

    ContentPosition {
        row: content_row,
        col: content_col,
    }
}

fn rendered_preview_total_lines(state: &SessionState, preview_total_lines: usize) -> usize {
    state
        .preview_render_cache
        .as_ref()
        .map(|cache| cache.total_lines)
        .unwrap_or(preview_total_lines)
}

fn preview_scrollbar_target_row(
    state: &SessionState,
    mouse: MouseEvent,
    total_lines: usize,
) -> Option<usize> {
    let (inner_x, inner_y, inner_width, inner_height) = state.preview_inner_rect;
    if inner_width == 0 || inner_height == 0 || total_lines == 0 {
        return None;
    }

    let scrollbar_x = inner_x + inner_width.saturating_sub(1);
    if mouse.column != scrollbar_x
        || mouse.row < inner_y
        || mouse.row >= inner_y.saturating_add(inner_height)
    {
        return None;
    }

    let viewport_rows = inner_height as usize;
    if total_lines <= viewport_rows {
        return None;
    }

    let indicator_row = mouse.row.saturating_sub(inner_y) as usize;
    let max_indicator_index = viewport_rows.saturating_sub(1);
    let max_scroll = total_lines.saturating_sub(viewport_rows);
    let target_row = if max_indicator_index == 0 {
        0
    } else {
        indicator_row.saturating_mul(max_scroll) / max_indicator_index
    };
    Some(target_row.min(max_scroll))
}

fn handle_preview_scrollbar_drag(
    state: &mut SessionState,
    mouse: MouseEvent,
    total_lines: usize,
) -> bool {
    let Some(target_row) = preview_scrollbar_target_row(state, mouse, total_lines) else {
        return false;
    };
    if target_row == state.preview_scroll_row {
        return false;
    }
    state.preview_scroll_row = target_row;
    true
}

fn preview_viewport_cols(state: &SessionState) -> usize {
    let (_, _, inner_w, _) = state.preview_inner_rect;
    (inner_w as usize).saturating_sub(state.preview_line_number_cols)
}

fn preview_scroll_intent(
    event: &Event,
    state: &SessionState,
    bindings: &HashMap<Action, crossterm::event::KeyEvent>,
) -> Option<ScrollIntent> {
    match event {
        Event::Mouse(mouse) if in_preview_panel(state, *mouse) => match mouse.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                if mouse.modifiers.contains(KeyModifiers::SHIFT) && !state.preview_wrap_enabled {
                    Some(ScrollIntent::Horizontal)
                } else {
                    Some(ScrollIntent::Vertical)
                }
            }
            _ => None,
        },
        Event::Key(key) => {
            if state.help_overlay_visible {
                return None;
            }

            use crossterm::event::KeyCode;
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                return match key.code {
                    KeyCode::Up | KeyCode::Down => Some(ScrollIntent::Vertical),
                    KeyCode::Left | KeyCode::Right if !state.preview_wrap_enabled => {
                        Some(ScrollIntent::Horizontal)
                    }
                    _ => None,
                };
            }

            match map_key_to_action(*key, bindings) {
                Some(Action::PageUp)
                | Some(Action::PageDown)
                | Some(Action::PreviewScrollUp)
                | Some(Action::PreviewScrollDown) => {
                    if state.preview_fullscreen || state.focus_pane == FocusPane::Preview {
                        Some(ScrollIntent::Vertical)
                    } else {
                        None
                    }
                }
                Some(Action::PreviewScrollLeft) | Some(Action::PreviewScrollRight) => {
                    if (state.preview_fullscreen || state.focus_pane == FocusPane::Preview)
                        && !state.preview_wrap_enabled
                    {
                        Some(ScrollIntent::Horizontal)
                    } else {
                        None
                    }
                }
                Some(Action::MoveUp) | Some(Action::MoveDown) if state.preview_fullscreen => {
                    Some(ScrollIntent::Vertical)
                }
                Some(Action::Expand) | Some(Action::Collapse)
                    if state.preview_fullscreen && !state.preview_wrap_enabled =>
                {
                    Some(ScrollIntent::Horizontal)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn coalesce_preview_scroll_event(
    state: &mut SessionState,
    bindings: &HashMap<Action, crossterm::event::KeyEvent>,
    initial_event: Event,
) -> Result<Event> {
    let Some(initial_intent) = preview_scroll_intent(&initial_event, state, bindings) else {
        return Ok(initial_event);
    };

    let mut latest_event = initial_event;
    while event::poll(Duration::from_millis(0))? {
        let next_event = event::read()?;
        let next_intent = preview_scroll_intent(&next_event, state, bindings);
        if next_intent == Some(initial_intent) {
            latest_event = next_event;
            continue;
        }

        if next_intent.is_some() {
            latest_event = next_event;
            continue;
        }

        state.deferred_input_event = Some(next_event);
        break;
    }

    Ok(latest_event)
}

fn defer_or_discard_redundant_mouse_scrolls(
    state: &mut SessionState,
    kind: MouseEventKind,
    modifiers: KeyModifiers,
) -> Result<()> {
    while event::poll(Duration::from_millis(0))? {
        let next_event = event::read()?;
        let Event::Mouse(next_mouse) = next_event else {
            state.deferred_input_event = Some(next_event);
            return Ok(());
        };

        let same_scroll = next_mouse.kind == kind && next_mouse.modifiers == modifiers;
        if same_scroll && in_preview_panel(state, next_mouse) {
            continue;
        }

        state.deferred_input_event = Some(Event::Mouse(next_mouse));
        return Ok(());
    }

    Ok(())
}

fn build_osc52(data: &[u8]) -> Vec<u8> {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    let mut out = Vec::with_capacity(9 + encoded.len());
    out.extend_from_slice(b"\x1b]52;c;");
    out.extend_from_slice(encoded.as_bytes());
    out.extend_from_slice(b"\x1b\\");
    out
}

fn wrap_tmux_passthrough(inner: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity((inner.len() * 2) + 8);
    out.extend_from_slice(b"\x1bPtmux;");
    for byte in inner {
        if *byte == 0x1b {
            out.push(0x1b);
        }
        out.push(*byte);
    }
    out.extend_from_slice(b"\x1b\\");
    out
}

fn is_inside_tmux() -> bool {
    std::env::var_os("TMUX")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

fn tmux_enable_passthrough() -> Option<bool> {
    use std::process::Command;
    let output = Command::new("tmux")
        .args(["show-options", "-p", "-v", "allow-passthrough"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let current = String::from_utf8_lossy(&output.stdout);
    let was_on = current.trim() == "on";
    if !was_on {
        let _ = Command::new("tmux")
            .args(["set-option", "-p", "allow-passthrough", "on"])
            .output();
    }
    Some(was_on)
}

fn tmux_restore_passthrough(was_on: bool) {
    if !was_on {
        use std::process::Command;
        let _ = Command::new("tmux")
            .args(["set-option", "-p", "allow-passthrough", "off"])
            .output();
    }
}

fn copy_to_clipboard_osc52(text: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut payload = build_osc52(text.as_bytes());

    let mut restore: Option<bool> = None;
    if is_inside_tmux() {
        restore = tmux_enable_passthrough();
        payload = wrap_tmux_passthrough(&payload);
    }

    // Write to /dev/tty to bypass ratatui's stdout ownership
    let written = if let Ok(mut tty) = OpenOptions::new().write(true).open("/dev/tty") {
        tty.write_all(&payload).is_ok() && tty.flush().is_ok()
    } else {
        // Fallback to stdout if /dev/tty unavailable
        let mut out = std::io::stdout().lock();
        out.write_all(&payload).is_ok() && out.flush().is_ok()
    };

    if let Some(was_on) = restore {
        tmux_restore_passthrough(was_on);
    }

    let _ = written;
}

fn try_clipboard_tool(text: &str) -> bool {
    use std::process::{Command, Stdio};
    let tools: &[(&str, &[&str])] = &[
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
        ("wl-copy", &[]),
        ("pbcopy", &[]),
    ];
    for (cmd, args) in tools {
        if let Ok(mut child) = Command::new(cmd)
            .args(*args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
            }
            if let Ok(status) = child.wait() {
                if status.success() {
                    return true;
                }
            }
        }
    }
    false
}

fn copy_to_clipboard(text: &str) {
    // Always send OSC 52 (via /dev/tty) for remote terminal clipboard
    copy_to_clipboard_osc52(text);
    // Also try local clipboard tools as a bonus
    let _ = try_clipboard_tool(text);
}

fn finalize_selection(state: &mut SessionState, preview_doc: &PreviewDocument) {
    state.preview_selecting = false;
    state.preview_copying_indicator = false;
    if let Some(sel) = &state.preview_selection {
        if sel.anchor == sel.cursor {
            state.preview_selection = None;
            return;
        }
        let text = extract_selected_text(preview_doc, sel);
        if !text.is_empty() {
            copy_to_clipboard(&text);
            state.preview_copy_indicator = true;
        }
    }
    state.preview_selection = None;
}

/// Returns (should_quit, should_refresh_preview, should_refresh_tree)
pub fn process_once(
    state: &mut SessionState,
    nodes: &mut Vec<TreeNode>,
    bindings: &HashMap<Action, crossterm::event::KeyEvent>,
    preview_total_lines: usize,
    preview_viewport_rows: usize,
    preview_doc: &PreviewDocument,
) -> Result<(bool, bool, bool)> {
    let mut should_refresh_preview = false;
    let mut should_refresh_tree = false;
    let next_event = if let Some(event) = state.deferred_input_event.take() {
        event
    } else {
        if !event::poll(Duration::from_millis(50))? {
            return Ok((false, false, false));
        }
        event::read()?
    };
    let next_event = coalesce_preview_scroll_event(state, bindings, next_event)?;

    match next_event {
        Event::Key(key) => {
            state.preview_selection = None;
            state.preview_selecting = false;
            state.preview_copy_indicator = false;
            state.preview_copying_indicator = false;

            if !state.help_overlay_visible && key.modifiers.contains(KeyModifiers::SHIFT) {
                use crossterm::event::KeyCode;
                match key.code {
                    KeyCode::Up => {
                        let _ = state.scroll_preview_lines(
                            -3,
                            preview_total_lines,
                            preview_viewport_rows,
                        );
                        return Ok((false, false, false));
                    }
                    KeyCode::Down => {
                        let _ = state.scroll_preview_lines(
                            3,
                            preview_total_lines,
                            preview_viewport_rows,
                        );
                        return Ok((false, false, false));
                    }
                    KeyCode::Left if !state.preview_wrap_enabled => {
                        let vp = preview_viewport_cols(state);
                        let mw = preview_max_line_width(preview_doc);
                        let _ = state.scroll_preview_cols(-3, mw, vp);
                        return Ok((false, false, false));
                    }
                    KeyCode::Right if !state.preview_wrap_enabled => {
                        let vp = preview_viewport_cols(state);
                        let mw = preview_max_line_width(preview_doc);
                        let _ = state.scroll_preview_cols(3, mw, vp);
                        return Ok((false, false, false));
                    }
                    _ => {}
                }
            }

            if let Some(action) = map_key_to_action(key, bindings) {
                match action {
                    Action::ToggleHelp => {
                        state.help_overlay_visible = !state.help_overlay_visible;
                    }
                    Action::MoveUp => {
                        if state.help_overlay_visible {
                            return Ok((false, false, false));
                        }
                        if state.preview_fullscreen {
                            let _ = state.scroll_preview_lines(
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
                            return Ok((false, false, false));
                        }
                        if state.preview_fullscreen {
                            let _ = state.scroll_preview_lines(
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
                            return Ok((false, false, false));
                        }
                        if state.preview_fullscreen && !state.preview_wrap_enabled {
                            let vp = preview_viewport_cols(state);
                            let mw = preview_max_line_width(preview_doc);
                            let _ = state.scroll_preview_cols(1, mw, vp);
                        } else if !state.preview_fullscreen {
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
                            return Ok((false, false, false));
                        }
                        if state.preview_fullscreen && !state.preview_wrap_enabled {
                            let vp = preview_viewport_cols(state);
                            let mw = preview_max_line_width(preview_doc);
                            let _ = state.scroll_preview_cols(-1, mw, vp);
                        } else if !state.preview_fullscreen
                            && !is_filesystem_root(&state.current_path)
                        {
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
                            return Ok((false, false, false));
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
                            return Ok((false, false, false));
                        }
                        if !state.preview_fullscreen {
                            switch_focus(state);
                        }
                    }
                    Action::PageUp => {
                        if state.help_overlay_visible {
                            return Ok((false, false, false));
                        }
                        if state.preview_fullscreen || state.focus_pane == FocusPane::Preview {
                            let _ = state
                                .page_scroll_preview_up(preview_total_lines, preview_viewport_rows);
                        }
                    }
                    Action::PageDown => {
                        if state.help_overlay_visible {
                            return Ok((false, false, false));
                        }
                        if state.preview_fullscreen || state.focus_pane == FocusPane::Preview {
                            let _ = state.page_scroll_preview_down(
                                preview_total_lines,
                                preview_viewport_rows,
                            );
                        }
                    }
                    Action::PreviewScrollUp => {
                        if state.help_overlay_visible {
                            return Ok((false, false, false));
                        }
                        if state.preview_fullscreen || state.focus_pane == FocusPane::Preview {
                            let _ = state.scroll_preview_lines(
                                -3,
                                preview_total_lines,
                                preview_viewport_rows,
                            );
                        }
                    }
                    Action::PreviewScrollDown => {
                        if state.help_overlay_visible {
                            return Ok((false, false, false));
                        }
                        if state.preview_fullscreen || state.focus_pane == FocusPane::Preview {
                            let _ = state.scroll_preview_lines(
                                3,
                                preview_total_lines,
                                preview_viewport_rows,
                            );
                        }
                    }
                    Action::PreviewScrollLeft => {
                        if state.help_overlay_visible {
                            return Ok((false, false, false));
                        }
                        if (state.preview_fullscreen || state.focus_pane == FocusPane::Preview)
                            && !state.preview_wrap_enabled
                        {
                            let vp = preview_viewport_cols(state);
                            let mw = preview_max_line_width(preview_doc);
                            let _ = state.scroll_preview_cols(-3, mw, vp);
                        }
                    }
                    Action::PreviewScrollRight => {
                        if state.help_overlay_visible {
                            return Ok((false, false, false));
                        }
                        if (state.preview_fullscreen || state.focus_pane == FocusPane::Preview)
                            && !state.preview_wrap_enabled
                        {
                            let vp = preview_viewport_cols(state);
                            let mw = preview_max_line_width(preview_doc);
                            let _ = state.scroll_preview_cols(3, mw, vp);
                        }
                    }
                    Action::TogglePreviewLineNumbers => {
                        if state.help_overlay_visible {
                            return Ok((false, false, false));
                        }
                        state.preview_show_line_numbers = !state.preview_show_line_numbers;
                    }
                    Action::TogglePreviewWrap => {
                        if state.help_overlay_visible {
                            return Ok((false, false, false));
                        }
                        state.preview_wrap_enabled = !state.preview_wrap_enabled;
                        if state.preview_wrap_enabled {
                            state.preview_scroll_col = 0;
                        }
                        state.preview_selection = None;
                    }
                    Action::ToggleHidden => {
                        if state.help_overlay_visible {
                            return Ok((false, false, false));
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
                            return Ok((false, false, false));
                        }
                        if !state.preview_fullscreen {
                            let step = state.resize_step() as i16;
                            state.resize_preview_by(-step, main_area_width());
                            should_refresh_preview = true;
                        }
                    }
                    Action::ResizePreviewWider => {
                        if state.help_overlay_visible {
                            return Ok((false, false, false));
                        }
                        if !state.preview_fullscreen {
                            let step = state.resize_step() as i16;
                            state.resize_preview_by(step, main_area_width());
                            should_refresh_preview = true;
                        }
                    }
                    Action::Refresh => {
                        if state.help_overlay_visible {
                            return Ok((false, false, false));
                        }
                        should_refresh_tree = true;
                        should_refresh_preview = true;
                    }
                    Action::ToggleDiffPreview => {
                        if state.help_overlay_visible {
                            return Ok((false, false, false));
                        }
                        state.preview_diff_mode = !state.preview_diff_mode;
                        state.reset_preview_scroll();
                        should_refresh_preview = true;
                    }
                    Action::Quit => return Ok((true, false, false)),
                }
            }
        }
        Event::Mouse(mouse) => {
            if state.help_overlay_visible {
                return Ok((false, false, false));
            }
            if !state.preview_fullscreen {
                should_refresh_preview = apply_mouse_resize(state, mouse);
            }

            if !state.divider_drag_active {
                state.preview_copy_indicator = false;
                let rendered_total_lines = rendered_preview_total_lines(state, preview_total_lines);
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        // If we were selecting, finalize the previous selection
                        if state.preview_selecting {
                            finalize_selection(state, preview_doc);
                        }
                        state.preview_scrollbar_dragging = false;

                        let mut tree_clicked = false;
                        if let Some(index) = tree_index_for_click(state, mouse, nodes.len()) {
                            state.selected_index = index;
                            state.update_selected_path(nodes);
                            state.reset_preview_scroll();
                            should_refresh_preview = true;
                            tree_clicked = true;

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

                        if !tree_clicked {
                            if let Some(target_row) =
                                preview_scrollbar_target_row(state, mouse, rendered_total_lines)
                            {
                                state.preview_scroll_row = target_row;
                                state.preview_selection = None;
                                state.preview_selecting = false;
                                state.preview_copying_indicator = false;
                                state.preview_scrollbar_dragging = true;
                            } else if in_preview_panel(state, mouse) {
                                let pos = mouse_to_content_position(state, mouse);
                                state.preview_selection = Some(PreviewSelection {
                                    anchor: pos,
                                    cursor: pos,
                                });
                                state.preview_selecting = true;
                                state.preview_copying_indicator = true;
                            } else {
                                state.preview_selection = None;
                                state.preview_selecting = false;
                            }
                        }
                    }
                    MouseEventKind::Drag(MouseButton::Left) if state.preview_scrollbar_dragging => {
                        let _ = handle_preview_scrollbar_drag(state, mouse, rendered_total_lines);
                    }
                    MouseEventKind::Drag(MouseButton::Left) if state.preview_selecting => {
                        let pos = mouse_to_content_position(state, mouse);
                        if let Some(sel) = &mut state.preview_selection {
                            sel.cursor = pos;
                        }
                    }
                    // Some terminals send Moved instead of Drag during button hold
                    MouseEventKind::Moved if state.preview_scrollbar_dragging => {
                        let _ = handle_preview_scrollbar_drag(state, mouse, rendered_total_lines);
                    }
                    MouseEventKind::Moved if state.preview_selecting => {
                        let pos = mouse_to_content_position(state, mouse);
                        if let Some(sel) = &mut state.preview_selection {
                            sel.cursor = pos;
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) if state.preview_selecting => {
                        finalize_selection(state, preview_doc);
                    }
                    MouseEventKind::Up(MouseButton::Left) if state.preview_scrollbar_dragging => {
                        state.preview_scrollbar_dragging = false;
                    }
                    _ => {}
                }

                if in_preview_panel(state, mouse) {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            if mouse.modifiers.contains(KeyModifiers::SHIFT)
                                && !state.preview_wrap_enabled
                            {
                                let vp = preview_viewport_cols(state);
                                let mw = preview_max_line_width(preview_doc);
                                if !state.scroll_preview_cols(-3, mw, vp) {
                                    defer_or_discard_redundant_mouse_scrolls(
                                        state,
                                        mouse.kind,
                                        mouse.modifiers,
                                    )?;
                                }
                            } else {
                                if !state.scroll_preview_lines(
                                    -3,
                                    preview_total_lines,
                                    preview_viewport_rows,
                                ) {
                                    defer_or_discard_redundant_mouse_scrolls(
                                        state,
                                        mouse.kind,
                                        mouse.modifiers,
                                    )?;
                                }
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if mouse.modifiers.contains(KeyModifiers::SHIFT)
                                && !state.preview_wrap_enabled
                            {
                                let vp = preview_viewport_cols(state);
                                let mw = preview_max_line_width(preview_doc);
                                if !state.scroll_preview_cols(3, mw, vp) {
                                    defer_or_discard_redundant_mouse_scrolls(
                                        state,
                                        mouse.kind,
                                        mouse.modifiers,
                                    )?;
                                }
                            } else {
                                if !state.scroll_preview_lines(
                                    3,
                                    preview_total_lines,
                                    preview_viewport_rows,
                                ) {
                                    defer_or_discard_redundant_mouse_scrolls(
                                        state,
                                        mouse.kind,
                                        mouse.modifiers,
                                    )?;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Event::Resize(width, _) => {
            state.normalize_preview_width(width);
        }
        _ => {}
    }

    Ok((false, should_refresh_preview, should_refresh_tree))
}

#[cfg(test)]
mod tests {
    use super::{
        apply_mouse_resize, handle_preview_scrollbar_drag, preview_scrollbar_target_row,
        tree_index_for_click, tree_panel_area,
    };
    use crate::app::state::{PreviewRenderCache, PreviewRenderCacheKey, SessionState};
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use ratatui::text::Line;
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

    #[test]
    fn preview_scrollbar_click_maps_to_rendered_row_position() {
        let mut state = SessionState::new(PathBuf::from("."));
        state.preview_inner_rect = (40, 2, 30, 10);
        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 69,
            row: 7,
            modifiers: KeyModifiers::NONE,
        };

        let target = preview_scrollbar_target_row(&state, click, 100);

        assert_eq!(target, Some(50));
    }

    #[test]
    fn preview_scrollbar_click_is_ignored_when_no_scrollbar_is_visible() {
        let mut state = SessionState::new(PathBuf::from("."));
        state.preview_inner_rect = (10, 3, 20, 8);
        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 29,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };

        assert_eq!(preview_scrollbar_target_row(&state, click, 8), None);
    }

    #[test]
    fn preview_scrollbar_drag_moves_scroll_row() {
        let mut state = SessionState::new(PathBuf::from("."));
        state.preview_inner_rect = (40, 2, 30, 10);
        state.preview_scroll_row = 0;
        state.preview_scrollbar_dragging = true;

        let drag = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 69,
            row: 8,
            modifiers: KeyModifiers::NONE,
        };

        assert_eq!(handle_preview_scrollbar_drag(&mut state, drag, 100), true);
        assert_eq!(state.preview_scroll_row, 60);
    }

    #[test]
    fn rendered_preview_total_lines_prefers_render_cache() {
        let mut state = SessionState::new(PathBuf::from("."));
        state.preview_render_cache = Some(PreviewRenderCache {
            key: PreviewRenderCacheKey {
                epoch: 1,
                inner_width: 40,
                show_line_numbers: true,
                wrap_enabled: true,
                content_hash: 0,
                styled_lines_hash: 0,
                line_changes_hash: 0,
            },
            rendered_lines: vec![Line::default(); 25],
            rendered_row_changes: vec![None; 25],
            total_lines: 25,
            line_number_cols: 4,
        });

        assert_eq!(super::rendered_preview_total_lines(&state, 10), 25);
    }

    #[test]
    fn mouse_divider_drag_defers_resize_until_release() {
        let mut state = SessionState::new(PathBuf::from("."));
        state.preview_width_cols = 40;
        let width = super::main_area_width();
        let divider = state.divider_column(width);

        let down = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: divider,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        apply_mouse_resize(&mut state, down);
        assert!(state.divider_drag_active);
        assert_eq!(state.preview_width_cols, 40);
        assert_eq!(state.divider_drag_column, Some(divider));

        let drag = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: divider.saturating_sub(5),
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        apply_mouse_resize(&mut state, drag);
        assert_eq!(state.preview_width_cols, 40);
        assert_eq!(state.divider_drag_column, Some(divider.saturating_sub(5)));

        let up = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: divider.saturating_sub(5),
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        apply_mouse_resize(&mut state, up);
        assert!(!state.divider_drag_active);
        assert_eq!(state.divider_drag_column, None);
        assert_eq!(
            state.panel_widths(width),
            (
                divider.saturating_sub(5),
                width.saturating_sub(divider.saturating_sub(5))
            )
        );
    }
}
