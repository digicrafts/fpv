use crate::app::current_dir_state::truncate_for_status;
use crate::app::state::SelectedEntryMetadata;
use crate::app::state::SessionState;
use crate::config::keymap::Action;
use crate::config::load::StatusDisplayMode;
use crossterm::event::KeyEvent;
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::collections::HashMap;
use std::path::Path;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn key_display(bindings: &HashMap<Action, KeyEvent>, action: Action, default: &str) -> String {
    bindings
        .get(&action)
        .map(|k| match k.code {
            crossterm::event::KeyCode::Up => "Up".to_string(),
            crossterm::event::KeyCode::Down => "Down".to_string(),
            crossterm::event::KeyCode::Left => "Left".to_string(),
            crossterm::event::KeyCode::Right => "Right".to_string(),
            crossterm::event::KeyCode::Enter => "Enter".to_string(),
            crossterm::event::KeyCode::Tab => "Tab".to_string(),
            crossterm::event::KeyCode::PageUp => "PageUp".to_string(),
            crossterm::event::KeyCode::PageDown => "PageDown".to_string(),
            crossterm::event::KeyCode::Esc => "Esc".to_string(),
            crossterm::event::KeyCode::Char(c) => c.to_string(),
            _ => format!("{:?}", k.code),
        })
        .unwrap_or_else(|| default.to_string())
}

pub fn help_line(bindings: &HashMap<Action, KeyEvent>) -> String {
    format!("Help ({})", key_display(bindings, Action::ToggleHelp, "?"))
}

pub fn compose_shortcut_help_text(bindings: &HashMap<Action, KeyEvent>) -> String {
    let up = key_display(bindings, Action::MoveUp, "Up");
    let down = key_display(bindings, Action::MoveDown, "Down");
    let expand = key_display(bindings, Action::Expand, "Right");
    let collapse = key_display(bindings, Action::Collapse, "Left");
    let open = key_display(bindings, Action::Open, "Enter");
    let quit = key_display(bindings, Action::Quit, "q");
    let focus = key_display(bindings, Action::SwitchFocus, "Tab");
    let hidden = key_display(bindings, Action::ToggleHidden, "h");
    let narrower = key_display(bindings, Action::ResizePreviewNarrower, "Ctrl+Left");
    let wider = key_display(bindings, Action::ResizePreviewWider, "Ctrl+Right");
    let page_up = key_display(bindings, Action::PageUp, "PageUp");
    let page_down = key_display(bindings, Action::PageDown, "PageDown");
    let scroll_up = key_display(bindings, Action::PreviewScrollUp, "'");
    let scroll_down = key_display(bindings, Action::PreviewScrollDown, "/");
    let toggle_lines = key_display(bindings, Action::TogglePreviewLineNumbers, "l");
    let toggle_wrap = key_display(bindings, Action::TogglePreviewWrap, "w");
    let esc = key_display(bindings, Action::ExitFullscreenPreview, "Esc");
    let help = key_display(bindings, Action::ToggleHelp, "?");

    format!(
        "Shortcuts\n\nNavigation\n  {up}/{down}: move selection\n  {expand}: enter directory\n  {collapse}: parent directory\n  {open}: open (directory/fullscreen)\n\nPanels\n  {focus}: switch tree/preview focus\n  {narrower}/{wider}: resize preview panel\n\nPreview\n  {scroll_up}/{scroll_down}: scroll 3 lines\n  {page_up}/{page_down}: page up/down\n  {toggle_lines}: toggle line numbers\n  {toggle_wrap}: toggle wrap\n  {esc}: exit fullscreen\n\nOther\n  {hidden}: show/hide hidden files\n  {help}: close help\n  {quit}: quit fpv"
    )
}

fn type_name_for_extension(ext: &str) -> &'static str {
    match ext {
        "bash" | "sh" | "zsh" | "ksh" => "Shell",
        "c" | "h" => "C",
        "cc" | "cp" | "cpp" | "cxx" | "c++" | "hpp" | "hh" | "hxx" => "C++",
        "css" => "CSS",
        "go" => "Go",
        "html" | "htm" => "HTML",
        "java" => "Java",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "json" => "JSON",
        "md" | "markdown" => "Markdown",
        "py" => "Python",
        "rs" => "Rust",
        "toml" => "TOML",
        "tsx" | "ts" => "TypeScript",
        "xml" => "XML",
        "yaml" | "yml" => "YAML",
        _ => "Unknown",
    }
}

fn file_type_label(filename: &str) -> String {
    let trimmed = filename.trim();
    if trimmed.is_empty() || trimmed == "-" {
        return "Unknown(none)".to_string();
    }

    let name = Path::new(trimmed)
        .file_name()
        .and_then(|part| part.to_str())
        .unwrap_or(trimmed);
    if let Some((_, ext)) = name.rsplit_once('.') {
        if !ext.is_empty() {
            let normalized = ext.to_ascii_lowercase();
            return format!("{}(.{})", type_name_for_extension(&normalized), normalized);
        }
    }

    "Unknown(none)".to_string()
}

pub fn compose_preview_metadata_line(metadata: &SelectedEntryMetadata, width: usize) -> String {
    let raw = format!(
        "{} | {} | {} | {}",
        file_type_label(&metadata.filename),
        metadata.size_text,
        metadata.permission_text,
        metadata.modified_text
    );
    truncate_for_status(&raw, width)
}

pub fn compose_bottom_status_line(
    state: &SessionState,
    bindings: &HashMap<Action, KeyEvent>,
    width: usize,
) -> String {
    let raw = format!(
        "fpv {} | hidden={} preview={}ms | {}",
        APP_VERSION,
        if state.show_hidden { "on" } else { "off" },
        state.last_preview_latency_ms,
        help_line(bindings)
    );
    let padded = format!(" {raw} ");
    truncate_for_status(&padded, width)
}

pub fn compose_status_title_line(
    state: &SessionState,
    bindings: &HashMap<Action, KeyEvent>,
    width: usize,
) -> String {
    compose_bottom_status_line(state, bindings, width)
}

fn status_bar_style() -> Style {
    Style::default().fg(Color::White).bg(Color::DarkGray)
}

pub fn draw_status(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    state: &SessionState,
    bindings: &HashMap<Action, KeyEvent>,
) {
    let content = match state.status_display_mode {
        StatusDisplayMode::Bar => compose_bottom_status_line(state, bindings, area.width as usize),
        StatusDisplayMode::Title => compose_status_title_line(state, bindings, area.width as usize),
    };
    frame.render_widget(Paragraph::new(content).style(status_bar_style()), area);
}
