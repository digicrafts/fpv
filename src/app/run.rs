use crate::app::preview_controller::refresh_preview;
use crate::app::state::{PreviewDocument, SessionState};
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
use std::path::{Path, PathBuf};
use std::time::Instant;

fn image_paths_match(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    let left_canon = left.canonicalize().ok();
    let right_canon = right.canonicalize().ok();
    match (left_canon, right_canon) {
        (Some(left_canonical), Some(right_canonical)) => left_canonical == right_canonical,
        _ => left.to_string_lossy() == right.to_string_lossy(),
    }
}

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
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_symbol("│");
            cell.set_style(Style::default().fg(Color::Gray));
        }
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

const IMAGE_PREVIEW_PROGRESS_STEPS: [&str; 5] = ["Load image", "Decode Image 1", "Decode Image 2", "Decode Image 3", "Draw Image"];

fn image_preview_step_index(step_name: &str) -> Option<usize> {
    IMAGE_PREVIEW_PROGRESS_STEPS
        .iter()
        .position(|name| *name == step_name)
}

fn parse_image_preview_step_timings(content: &str) -> [Option<f64>; 5] {
    let mut step_timings = [None, None, None, None, None];
    for line in content.lines() {
        let line = line.trim();
        let Some((step_name, value_part)) = line.rsplit_once(" (") else {
            continue;
        };
        let Some(value_part) = value_part.strip_suffix(")") else {
            continue;
        };
        let Some(value_part) = value_part.strip_suffix("s") else {
            continue;
        };
        let Ok(duration) = value_part.parse::<f64>() else {
            continue;
        };
        if let Some(step_index) = image_preview_step_index(step_name) {
            step_timings[step_index] = Some(duration.max(0.0));
        }
    }
    step_timings
}

fn parse_image_preview_step_count(content: &str) -> usize {
    parse_image_preview_step_timings(content)
        .iter()
        .filter(|value| value.is_some())
        .count()
}

fn format_step_duration(duration_secs: f64) -> String {
    format!("{duration_secs:.2}s")
}

fn refresh_image_preview_loading_text(
    _state: &SessionState,
    preview: &mut PreviewDocument,
    step_count: usize,
    active_step_index: &mut Option<usize>,
    active_step_started_at: &mut Option<Instant>,
) {
    if !preview.image_preview_pending {
        *active_step_index = None;
        *active_step_started_at = None;
        return;
    }

    let step_timings = parse_image_preview_step_timings(&preview.content_excerpt);
    let active_step = match step_count {
        0 => 0,
        1 => 1,
        _ => 2,
    };

    if *active_step_index != Some(active_step) {
        *active_step_index = Some(active_step);
        *active_step_started_at = Some(Instant::now());
    }
    let Some(active_started_at) = active_step_started_at.as_ref() else {
        return;
    };

    let active_elapsed = active_started_at.elapsed().as_secs_f64();
    let mut lines = Vec::with_capacity(active_step + 1);
    for index in 0..=active_step {
        let duration = if index == active_step {
            active_elapsed
        } else {
            step_timings[index].unwrap_or(0.0)
        };
        lines.push(format!(
            "{} ({})",
            IMAGE_PREVIEW_PROGRESS_STEPS[index],
            format_step_duration(duration)
        ));
    }
    preview.content_excerpt = lines.join("\n");
}

/// Check the git status worker for updates and refresh the preview if the
/// current selection is a directory whose listing depends on git annotations.
fn poll_git_status(
    git_worker: &GitStatusWorker,
    state: &mut SessionState,
    nodes: &[TreeNode],
    last_requested_git_path: &mut Option<PathBuf>,
    preview: &mut PreviewDocument,
    highlight: &HighlightContext,
) {
    if let Some(update) = git_worker.latest_update() {
        if apply_git_status_update(update, state, last_requested_git_path)
            && nodes
                .get(state.selected_index)
                .is_some_and(|node| node.node_type == crate::app::state::NodeType::Directory)
        {
            *preview = refresh_preview(state, nodes, highlight, 1024 * 1024);
        }
    }
}

/// Accept a completed image preview from the background worker when it matches
/// the currently selected file.
fn poll_image_worker(
    image_worker: &ImagePreviewWorker,
    state: &SessionState,
    preview: &mut PreviewDocument,
    pending_image_path: &mut Option<PathBuf>,
    pending_image_step_count: &mut usize,
    active_image_step_index: &mut Option<usize>,
    active_image_step_started_at: &mut Option<Instant>,
) {
    if let Some(update) = image_worker.latest_update() {
        let matches_selected = image_paths_match(&state.selected_path, &update.path);
        let matches_pending = pending_image_path
            .as_ref()
            .is_some_and(|pending| image_paths_match(pending, &update.path));
        if matches_selected {
            if update.preview.image_preview_pending {
                let update_step_count = parse_image_preview_step_count(&update.preview.content_excerpt);
                let should_apply = update_step_count >= *pending_image_step_count;
                if should_apply {
                    *pending_image_step_count = update_step_count;
                    *preview = update.preview;
                    *active_image_step_index = None;
                    *active_image_step_started_at = Some(Instant::now());
                }
            } else {
                *preview = update.preview;
                *pending_image_step_count = 0;
                *pending_image_path = None;
                *active_image_step_index = None;
                *active_image_step_started_at = None;
            }
        } else if matches_pending && !update.preview.image_preview_pending {
            *pending_image_path = None;
            *active_image_step_index = None;
            *active_image_step_started_at = None;
        }
    }
}

/// Dispatch the image preview request to the background worker while preserving
/// immediate responsiveness in the preview pane.
fn dispatch_pending_image(
    image_worker: &ImagePreviewWorker,
    state: &SessionState,
    preview: &PreviewDocument,
    pending_image_path: &mut Option<PathBuf>,
    pending_image_step_count: &mut usize,
    active_image_step_index: &mut Option<usize>,
    active_image_step_started_at: &mut Option<Instant>,
) {
    if preview.image_preview_pending {
        let target_path = state.selected_path.clone();
        if pending_image_path.as_ref() != Some(&target_path)
            && image_worker.request(target_path.clone(), current_image_target_width(state))
        {
            *pending_image_step_count = 0;
            *active_image_step_index = None;
            *active_image_step_started_at = None;
            *pending_image_path = Some(target_path);
        }
    }
}

/// Re-read the directory listing when the filesystem modification time has
/// changed, keeping the selection stable when possible.  Returns `true` if a
/// refresh was performed.
fn auto_refresh_directory(
    state: &mut SessionState,
    nodes: &mut Vec<TreeNode>,
    git_worker: &GitStatusWorker,
    last_requested_git_path: &mut Option<PathBuf>,
    preview: &mut PreviewDocument,
    pending_image_path: &mut Option<PathBuf>,
    pending_image_step_count: &mut usize,
    active_image_step_index: &mut Option<usize>,
    active_image_step_started_at: &mut Option<Instant>,
    highlight: &HighlightContext,
    last_auto_refresh: &mut Instant,
    auto_refresh_interval: std::time::Duration,
    last_dir_mtime: &mut Option<std::time::SystemTime>,
) -> bool {
    if last_auto_refresh.elapsed() < auto_refresh_interval {
        return false;
    }

    *last_auto_refresh = Instant::now();
    let current_mtime = dir_mtime(&state.current_path);
    if current_mtime == *last_dir_mtime {
        return false;
    }

    *last_dir_mtime = current_mtime;
    let prev_path = state.selected_path.clone();
    match list_current_directory_with_visibility(&state.current_path, 2000, state.show_hidden) {
        Ok(entries) => apply_directory_entries(state, nodes, entries, Some(&prev_path)),
        Err(error) => {
            let message = apply_directory_error(state, nodes, &error);
            state.status_message = format_status_with_path(&message, &state.current_path);
        }
    }
    request_git_status_refresh(git_worker, state, last_requested_git_path, true);
    *preview = refresh_preview(state, nodes, highlight, 1024 * 1024);
    if preview.image_preview_pending {
        *pending_image_step_count = 0;
        *active_image_step_index = None;
        *active_image_step_started_at = None;
    }
    *pending_image_path = None;
    true
}

/// After `process_once` reports what changed, reload the tree listing and/or
/// preview as needed.
fn apply_input_results(
    should_refresh_tree: bool,
    should_refresh_preview: bool,
    previous_path: &PathBuf,
    state: &mut SessionState,
    nodes: &mut Vec<TreeNode>,
    git_worker: &GitStatusWorker,
    last_requested_git_path: &mut Option<PathBuf>,
    last_dir_mtime: &mut Option<std::time::SystemTime>,
    preview: &mut PreviewDocument,
    pending_image_path: &mut Option<PathBuf>,
    pending_image_step_count: &mut usize,
    active_image_step_index: &mut Option<usize>,
    active_image_step_started_at: &mut Option<Instant>,
    highlight: &HighlightContext,
) {
    if should_refresh_tree {
        let prev_selected = state.selected_path.clone();
        match list_current_directory_with_visibility(&state.current_path, 2000, state.show_hidden) {
            Ok(entries) => {
                apply_directory_entries(state, nodes, entries, Some(&prev_selected));
            }
            Err(error) => {
                let message = apply_directory_error(state, nodes, &error);
                state.status_message = format_status_with_path(&message, &state.current_path);
            }
        }
        request_git_status_refresh(git_worker, state, last_requested_git_path, true);
        *last_dir_mtime = dir_mtime(&state.current_path);
        *preview = refresh_preview(state, nodes, highlight, 1024 * 1024);
        if preview.image_preview_pending {
            *pending_image_step_count = 0;
            *active_image_step_index = None;
            *active_image_step_started_at = None;
        }
        *pending_image_path = None;
    } else {
        if state.current_path != *previous_path {
            request_git_status_refresh(git_worker, state, last_requested_git_path, true);
            *last_dir_mtime = dir_mtime(&state.current_path);
        }
        if should_refresh_preview {
            *preview = refresh_preview(state, nodes, highlight, 1024 * 1024);
            if preview.image_preview_pending {
                *pending_image_step_count = 0;
                *active_image_step_index = None;
                *active_image_step_started_at = None;
            }
            *pending_image_path = None;
        }
    }
}

/// Render a single frame: directory header, tree pane, preview pane, status
/// bar, and the optional help overlay.
fn draw_frame(
    frame: &mut ratatui::Frame<'_>,
    state: &mut SessionState,
    nodes: &[TreeNode],
    preview: &PreviewDocument,
    theme: &ThemeProfile,
    bindings: &std::collections::HashMap<crate::config::keymap::Action, crossterm::event::KeyEvent>,
) {
    frame.render_widget(Clear, frame.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(frame.area());
    frame.render_widget(Clear, chunks[1]);

    if state.layout_regions.top_directory_header && !state.preview_fullscreen {
        draw_current_directory_header(frame, chunks[0], state, theme);
    }

    if state.preview_fullscreen {
        draw_preview(frame, chunks[1], preview, state, theme);
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
        draw_tree(frame, main[0], nodes, state, theme);
        draw_preview(frame, main[1], preview, state, theme);
        if state.divider_drag_active {
            if let Some(divider_col) = state.divider_drag_column {
                draw_preview_resize_placeholder(frame, chunks[1], divider_col);
            }
        }
    }
    draw_status(frame, chunks[2], state, bindings);

    if state.help_overlay_visible {
        let modal = centered_rect(72, 78, frame.area());
        let help_body = compose_shortcut_help_text(bindings);
        let help = Paragraph::new(help_body)
            .block(
                Block::default()
                    .title(" Shortcut Help ")
                    .title_alignment(Alignment::Left)
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(Clear, modal);
        frame.render_widget(help, modal);
    }
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
    let mut pending_image_active_step_index: Option<usize> = None;
    let mut pending_image_active_step_started_at: Option<Instant> = None;
    let (bindings, theme, status_mode, warnings) = load_bindings_and_theme(cfg_path);
    state.status_display_mode = status_mode;
    request_git_status_refresh(&git_worker, &mut state, &mut last_requested_git_path, true);
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
    let mut pending_image_step_count = 0usize;

    loop {
        let frame_size = terminal.size()?;
        state.normalize_preview_width(frame_size.width);

        poll_git_status(
            &git_worker,
            &mut state,
            &nodes,
            &mut last_requested_git_path,
            &mut preview,
            &highlight,
        );
        poll_image_worker(
            &image_worker,
            &state,
            &mut preview,
            &mut pending_image_path,
            &mut pending_image_step_count,
            &mut pending_image_active_step_index,
            &mut pending_image_active_step_started_at,
        );
        dispatch_pending_image(
            &image_worker,
            &state,
            &preview,
            &mut pending_image_path,
            &mut pending_image_step_count,
            &mut pending_image_active_step_index,
            &mut pending_image_active_step_started_at,
        );
        auto_refresh_directory(
            &mut state,
            &mut nodes,
            &git_worker,
            &mut last_requested_git_path,
            &mut preview,
            &mut pending_image_path,
            &mut pending_image_step_count,
            &mut pending_image_active_step_index,
            &mut pending_image_active_step_started_at,
            &highlight,
            &mut last_auto_refresh,
            auto_refresh_interval,
            &mut last_dir_mtime,
        );

        let previous_path = state.current_path.clone();
        let preview_viewport_rows = frame_size.height.saturating_sub(4) as usize;
        let total_preview_lines = preview_total_lines(&preview);
        state.clamp_preview_scroll(total_preview_lines, preview_viewport_rows);
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

        apply_input_results(
            should_refresh_tree,
            should_refresh_preview,
            &previous_path,
            &mut state,
            &mut nodes,
            &git_worker,
            &mut last_requested_git_path,
            &mut last_dir_mtime,
            &mut preview,
            &mut pending_image_path,
            &mut pending_image_step_count,
            &mut pending_image_active_step_index,
            &mut pending_image_active_step_started_at,
            &highlight,
        );

        refresh_image_preview_loading_text(
            &state,
            &mut preview,
            pending_image_step_count,
            &mut pending_image_active_step_index,
            &mut pending_image_active_step_started_at,
        );

        let frame_size = terminal.size()?;
        let preview_viewport_rows = frame_size.height.saturating_sub(4) as usize;
        let total_preview_lines = preview_total_lines(&preview);
        state.clamp_preview_scroll(total_preview_lines, preview_viewport_rows);

        terminal.draw(|f| {
            draw_frame(f, &mut state, &nodes, &preview, &theme, &bindings);
        })?;
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
