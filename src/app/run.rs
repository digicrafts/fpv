use crate::app::preview_controller::{refresh_preview, IMAGE_PREVIEW_DELAY};
use crate::app::state::SessionState;
use crate::config::keymap::{default_keymap, UserKeymap};
use crate::config::load::{
    default_config_path, ensure_default_config_exists, load_user_config, StatusDisplayMode,
    ThemeProfile, UserConfig,
};
use crate::config::merge::{merge_keymaps, merge_theme_profile};
use crate::config::validate::validate_bindings;
use crate::fs::current_dir::{
    directory_access_error_message, list_current_directory_with_visibility,
};
use crate::fs::git::{GitStatusUpdate, GitStatusWorker};
use crate::fs::preview::ImagePreviewWorker;
use crate::highlight::syntax::HighlightContext;
use crate::tui::event_loop::process_once;
use crate::tui::preview_pane::{draw_preview, preview_total_lines};
use crate::tui::status_bar::{compose_shortcut_help_text, draw_status};
use crate::tui::tree_pane::{draw_current_directory_header, draw_tree};
use crate::{app::navigation::format_status_with_path, app::state::TreeNode};
use anyhow::Result;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Terminal;
use std::env;
use std::io;
use std::path::PathBuf;
use std::time::Instant;

fn dir_mtime(path: &PathBuf) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn parse_args() -> (PathBuf, Option<PathBuf>) {
    let mut root = PathBuf::from(".");
    let mut cfg = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        if arg == "--config" {
            if let Some(v) = args.next() {
                cfg = Some(PathBuf::from(v));
            }
        } else if !arg.starts_with("--") {
            root = PathBuf::from(arg);
        }
    }

    (root, cfg)
}

fn load_bindings_and_theme(
    config_path: Option<PathBuf>,
) -> (
    std::collections::HashMap<crate::config::keymap::Action, crossterm::event::KeyEvent>,
    ThemeProfile,
    StatusDisplayMode,
    Vec<String>,
) {
    let defaults = default_keymap();
    let using_default_path = config_path.is_none();
    let path = config_path.unwrap_or_else(default_config_path);
    if using_default_path {
        let _ = ensure_default_config_exists(&path);
    }
    let user_config = load_user_config(&path).unwrap_or(UserConfig {
        mappings: Default::default(),
        theme: Default::default(),
        status_display_mode: None,
    });
    let user_keymap = UserKeymap {
        mappings: user_config.mappings,
    };
    let status_mode = user_config.status_display_mode.unwrap_or_default();
    let (merged, mut warnings) = merge_keymaps(defaults, &user_keymap);
    let theme = merge_theme_profile(ThemeProfile::default(), &user_config.theme);
    warnings.extend(validate_bindings(&merged));
    (merged, theme, status_mode, warnings)
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn draw_preview_resize_placeholder(frame: &mut ratatui::Frame<'_>, area: Rect, divider_col: u16) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let x = divider_col.clamp(area.x, area.x.saturating_add(area.width).saturating_sub(1));
    let buf = frame.buffer_mut();
    for y in area.y..area.y.saturating_add(area.height) {
        let cell = buf.get_mut(x, y);
        cell.set_symbol("│");
        cell.set_style(Style::default().fg(Color::Gray));
    }
}

fn apply_directory_entries(
    state: &mut SessionState,
    nodes: &mut Vec<TreeNode>,
    entries: Vec<TreeNode>,
    preferred: Option<&PathBuf>,
) {
    state.clear_current_dir_error();
    *nodes = entries;
    state.restore_or_default_selection(nodes, preferred);
    state.update_selected_path(nodes);
}

fn apply_directory_error(
    state: &mut SessionState,
    nodes: &mut Vec<TreeNode>,
    error: &anyhow::Error,
) -> String {
    let message = directory_access_error_message(error);
    state.set_current_dir_error(message.clone());
    nodes.clear();
    state.selected_index = 0;
    state.set_selected_path(state.current_path.clone());
    message
}

fn request_git_status_refresh(
    worker: &GitStatusWorker,
    state: &mut SessionState,
    last_requested_path: &mut Option<PathBuf>,
    force: bool,
) {
    if !force && last_requested_path.as_ref() == Some(&state.current_path) {
        return;
    }

    state.git_status = None;
    if worker.request(state.current_path.clone()) {
        *last_requested_path = Some(state.current_path.clone());
    } else {
        *last_requested_path = None;
    }
}

fn apply_git_status_update(
    update: GitStatusUpdate,
    state: &mut SessionState,
    last_requested_path: &mut Option<PathBuf>,
) -> bool {
    if update.requested_path != state.current_path {
        return false;
    }

    state.git_status = update.status;
    *last_requested_path = Some(update.requested_path);
    true
}

fn current_image_target_width(state: &SessionState) -> u16 {
    state.preview_width_cols.saturating_sub(4).max(1)
}

pub fn run() -> Result<()> {
    let (root, cfg_path) = parse_args();
    let mut state = SessionState::new(root);
    let mut nodes = Vec::new();
    match list_current_directory_with_visibility(&state.current_path, 2000, state.show_hidden) {
        Ok(entries) => apply_directory_entries(&mut state, &mut nodes, entries, None),
        Err(error) => {
            let message = apply_directory_error(&mut state, &mut nodes, &error);
            state.status_message = format_status_with_path(&message, &state.current_path);
        }
    }

    let highlight = HighlightContext::new();
    let git_worker = GitStatusWorker::spawn();
    let image_worker = ImagePreviewWorker::spawn();
    let mut last_requested_git_path = None;
    let mut pending_image_path: Option<PathBuf> = None;
    let (bindings, theme, status_mode, warnings) = load_bindings_and_theme(cfg_path);
    state.status_display_mode = status_mode;
    request_git_status_refresh(
        &git_worker,
        &mut state,
        &mut last_requested_git_path,
        true,
    );
    let mut preview = refresh_preview(&mut state, &nodes, &highlight, 1024 * 1024);
    state.status_message = if let Some(message) = state.current_dir_error.clone() {
        format_status_with_path(&message, &state.current_path)
    } else if warnings.is_empty() {
        format!("Ready. Path: {}", state.current_path.display())
    } else {
        format!(
            "{} Path: {}",
            crate::tui::config_warnings::render_warning_text(&warnings),
            state.current_path.display()
        )
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut last_auto_refresh = Instant::now();
    let auto_refresh_interval = std::time::Duration::from_secs(2);
    let mut last_dir_mtime = dir_mtime(&state.current_path);

    loop {
        if let Some(update) = git_worker.latest_update() {
            if apply_git_status_update(update, &mut state, &mut last_requested_git_path)
                && nodes
                    .get(state.selected_index)
                    .is_some_and(|node| node.node_type == crate::app::state::NodeType::Directory)
            {
                preview = refresh_preview(&mut state, &nodes, &highlight, 1024 * 1024);
            }
        }

        if let Some(update) = image_worker.latest_update() {
            if pending_image_path.as_ref() == Some(&update.path)
                && state.selected_path == update.path
                && preview.image_preview_pending
            {
                preview = update.preview;
            }
            if pending_image_path.as_ref() == Some(&update.path) {
                pending_image_path = None;
            }
        }

        if preview.image_preview_pending
            && state.selected_changed_at.elapsed() >= IMAGE_PREVIEW_DELAY
        {
            let target_path = state.selected_path.clone();
            if pending_image_path.as_ref() != Some(&target_path)
                && image_worker.request(target_path.clone(), current_image_target_width(&state))
            {
                pending_image_path = Some(target_path);
            }
        }

        // Auto-refresh: check if directory mtime changed
        if last_auto_refresh.elapsed() >= auto_refresh_interval {
            last_auto_refresh = Instant::now();
            let current_mtime = dir_mtime(&state.current_path);
            if current_mtime != last_dir_mtime {
                last_dir_mtime = current_mtime;
                let prev_path = state.selected_path.clone();
                match list_current_directory_with_visibility(
                    &state.current_path,
                    2000,
                    state.show_hidden,
                ) {
                    Ok(entries) => apply_directory_entries(
                        &mut state,
                        &mut nodes,
                        entries,
                        Some(&prev_path),
                    ),
                    Err(error) => {
                        let message = apply_directory_error(&mut state, &mut nodes, &error);
                        state.status_message =
                            format_status_with_path(&message, &state.current_path);
                    }
                }
                request_git_status_refresh(
                    &git_worker,
                    &mut state,
                    &mut last_requested_git_path,
                    true,
                );
                preview = refresh_preview(&mut state, &nodes, &highlight, 1024 * 1024);
                pending_image_path = None;
            }
        }

        let frame_size = terminal.size()?;
        state.normalize_preview_width(frame_size.width);
        let preview_viewport_rows = frame_size.height.saturating_sub(4) as usize;
        let total_preview_lines = preview_total_lines(&preview);
        state.clamp_preview_scroll(total_preview_lines, preview_viewport_rows);

        terminal.draw(|f| {
            f.render_widget(Clear, f.size());
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(3),
                    Constraint::Length(1),
                ])
                .split(f.size());
            f.render_widget(Clear, chunks[1]);

            if state.layout_regions.top_directory_header && !state.preview_fullscreen {
                draw_current_directory_header(f, chunks[0], &state, &theme);
            }

            if state.preview_fullscreen {
                draw_preview(f, chunks[1], &preview, &mut state, &theme);
            } else {
                let main = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints({
                        let (tree_width, preview_width) = state.panel_widths(chunks[1].width);
                        [
                            Constraint::Length(tree_width),
                            Constraint::Length(preview_width),
                        ]
                    })
                    .split(chunks[1]);
                draw_tree(f, main[0], &nodes, &state, &theme);
                draw_preview(f, main[1], &preview, &mut state, &theme);
                if state.divider_drag_active {
                    if let Some(divider_col) = state.divider_drag_column {
                        draw_preview_resize_placeholder(f, chunks[1], divider_col);
                    }
                }
            }
            draw_status(f, chunks[2], &state, &bindings);

            if state.help_overlay_visible {
                let modal = centered_rect(72, 78, f.size());
                let help_body = compose_shortcut_help_text(&bindings);
                let help = Paragraph::new(help_body)
                    .block(
                        Block::default()
                            .title(" Shortcut Help ")
                            .title_alignment(Alignment::Left)
                            .borders(Borders::ALL),
                    )
                    .wrap(Wrap { trim: false });
                f.render_widget(Clear, modal);
                f.render_widget(help, modal);
            }
        })?;

        let previous_path = state.current_path.clone();
        let (should_quit, should_refresh_preview, should_refresh_tree) = process_once(
            &mut state,
            &mut nodes,
            &bindings,
            total_preview_lines,
            preview_viewport_rows,
            &preview,
        )?;
        if should_quit {
            break;
        }
        if should_refresh_tree {
            let prev_selected = state.selected_path.clone();
            match list_current_directory_with_visibility(
                &state.current_path,
                2000,
                state.show_hidden,
            ) {
                Ok(entries) => {
                    apply_directory_entries(&mut state, &mut nodes, entries, Some(&prev_selected));
                }
                Err(error) => {
                    let message = apply_directory_error(&mut state, &mut nodes, &error);
                    state.status_message = format_status_with_path(&message, &state.current_path);
                }
            }
            request_git_status_refresh(
                &git_worker,
                &mut state,
                &mut last_requested_git_path,
                true,
            );
            last_dir_mtime = dir_mtime(&state.current_path);
            preview = refresh_preview(&mut state, &nodes, &highlight, 1024 * 1024);
            pending_image_path = None;
        } else {
            if state.current_path != previous_path {
                request_git_status_refresh(
                    &git_worker,
                    &mut state,
                    &mut last_requested_git_path,
                    true,
                );
                last_dir_mtime = dir_mtime(&state.current_path);
            }
            if should_refresh_preview {
                preview = refresh_preview(&mut state, &nodes, &highlight, 1024 * 1024);
                pending_image_path = None;
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
