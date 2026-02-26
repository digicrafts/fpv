use crate::app::preview_controller::refresh_preview;
use crate::app::state::SessionState;
use crate::config::keymap::{default_keymap, UserKeymap};
use crate::config::load::{
    default_config_path, load_user_config, StatusDisplayMode, ThemeProfile, UserConfig,
};
use crate::config::merge::{merge_keymaps, merge_theme_profile};
use crate::config::validate::validate_bindings;
use crate::fs::current_dir::list_current_directory_with_visibility;
use crate::fs::git::git_repo_status_for_path;
use crate::highlight::syntax::HighlightContext;
use crate::tui::event_loop::process_once;
use crate::tui::preview_pane::{draw_preview, preview_total_lines};
use crate::tui::status_bar::{compose_shortcut_help_text, draw_status};
use crate::tui::tree_pane::{draw_current_directory_header, draw_tree};
use anyhow::Result;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Terminal;
use std::env;
use std::io;
use std::path::PathBuf;

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
    let path = config_path.unwrap_or_else(default_config_path);
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

pub fn run() -> Result<()> {
    let (root, cfg_path) = parse_args();
    let mut state = SessionState::new(root);
    let mut nodes =
        list_current_directory_with_visibility(&state.current_path, 2000, state.show_hidden)?;
    state.revalidate_selection(&nodes);
    state.update_selected_path(&nodes);

    let highlight = HighlightContext::new();
    let (bindings, theme, status_mode, warnings) = load_bindings_and_theme(cfg_path);
    state.status_display_mode = status_mode;
    state.git_status = git_repo_status_for_path(&state.current_path);
    let mut preview = refresh_preview(&mut state, &nodes, &highlight, 1024 * 1024);
    state.status_message = if warnings.is_empty() {
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

    loop {
        let frame_size = terminal.size()?;
        state.normalize_preview_width(frame_size.width);
        let preview_viewport_rows = frame_size.height.saturating_sub(4) as usize;
        let total_preview_lines = preview_total_lines(&preview);
        state.clamp_preview_scroll(total_preview_lines, preview_viewport_rows);

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(3),
                    Constraint::Length(1),
                ])
                .split(f.size());

            if state.layout_regions.top_directory_header && !state.preview_fullscreen {
                draw_current_directory_header(f, chunks[0], &state, &theme);
            }

            if state.preview_fullscreen {
                draw_preview(f, chunks[1], &preview, &state, &theme);
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
                draw_preview(f, main[1], &preview, &state, &theme);
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
        let (should_quit, should_refresh_preview) = process_once(
            &mut state,
            &mut nodes,
            &bindings,
            total_preview_lines,
            preview_viewport_rows,
        )?;
        if should_quit {
            break;
        }
        if state.current_path != previous_path {
            state.git_status = git_repo_status_for_path(&state.current_path);
        }
        if should_refresh_preview {
            preview = refresh_preview(&mut state, &nodes, &highlight, 1024 * 1024);
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
