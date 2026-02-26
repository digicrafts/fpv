use crate::app::current_dir_state::truncate_for_status;
use crate::app::state::{NodeType, SessionState, TreeNode};
use crate::config::load::ThemeProfile;
use crate::fs::git::GitFileStatus;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use std::path::{Component, Path, PathBuf};
use unicode_width::UnicodeWidthStr;

pub fn entry_prefix(node_type: &NodeType) -> &'static str {
    match node_type {
        NodeType::Directory => "/",
        NodeType::File => "",
        NodeType::Symlink => "@",
        NodeType::Unknown => "?",
    }
}

pub fn current_directory_header_line(state: &SessionState, width: usize) -> String {
    truncate_for_status(&raw_current_directory_header_line(state), width)
}

fn raw_current_directory_header_line(state: &SessionState) -> String {
    let path = full_display_path(&state.current_path);
    let Some(git) = &state.git_status else {
        return path;
    };
    let changes = git.change_count();
    if changes == 0 {
        format!("{path} git:({})", git.branch)
    } else {
        format!("{path} git:({}) [{changes} changes]", git.branch)
    }
}

fn full_display_path(path: &Path) -> String {
    if path.is_absolute() {
        return normalize_display_path(path).display().to_string();
    }
    std::env::current_dir()
        .map(|cwd| {
            normalize_display_path(&cwd.join(path))
                .display()
                .to_string()
        })
        .unwrap_or_else(|_| path.display().to_string())
}

fn absolute_display_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize_display_path(path)
    } else {
        std::env::current_dir()
            .map(|cwd| normalize_display_path(&cwd.join(path)))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

fn normalize_display_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        if !matches!(component, Component::CurDir) {
            normalized.push(component.as_os_str());
        }
    }
    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

pub fn display_path_with_home(path: &Path) -> String {
    let rendered = path.display().to_string();
    let Some(home) = std::env::var_os("HOME").map(|s| s.to_string_lossy().to_string()) else {
        return rendered;
    };
    if rendered == home {
        "~".to_string()
    } else if rendered.starts_with(&(home.clone() + "/")) {
        format!("~{}", &rendered[home.len()..])
    } else {
        rendered
    }
}

pub fn color_from_name(name: &str) -> Color {
    match name {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "white" => Color::White,
        _ => Color::White,
    }
}

pub fn node_style(
    node: &TreeNode,
    theme: &ThemeProfile,
    git_status: Option<GitFileStatus>,
) -> Style {
    let mut style = match node.node_type {
        NodeType::Directory => Style::default().fg(color_from_name(&theme.directory_color)),
        NodeType::File => {
            let ext = node
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let color_name = theme
                .file_type_colors
                .get(&ext)
                .map(String::as_str)
                .unwrap_or(&theme.fallback_file_color);
            Style::default().fg(color_from_name(color_name))
        }
        _ => Style::default().fg(color_from_name(&theme.fallback_file_color)),
    };
    if theme.hidden_dim_enabled && node.name.starts_with('.') {
        style = style.add_modifier(Modifier::DIM);
    }
    if git_status == Some(GitFileStatus::Ignored) {
        style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM);
    }
    style
}

pub fn draw_current_directory_header(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    state: &SessionState,
    theme: &ThemeProfile,
) {
    let path = full_display_path(&state.current_path);
    let mut spans = vec![Span::raw(path.clone())];
    let mut raw_line = path;

    if let Some(git) = &state.git_status {
        let changes = git.change_count();
        let git_text = if changes == 0 {
            format!("git:({})", git.branch)
        } else {
            format!("git:({}) [{changes} changes]", git.branch)
        };
        raw_line.push(' ');
        raw_line.push_str(&git_text);
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            git_text,
            Style::default().fg(color_from_name(&theme.directory_color)),
        ));
    }

    let truncated = truncate_for_status(&raw_line, area.width as usize);
    let display = if truncated == raw_line {
        Line::from(spans)
    } else {
        Line::from(truncated)
    };
    frame.render_widget(Paragraph::new(display), area);
}

pub fn draw_tree(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    nodes: &[TreeNode],
    state: &SessionState,
    theme: &ThemeProfile,
) {
    if nodes.is_empty() {
        let empty =
            Paragraph::new("(empty directory)").block(Block::default().borders(Borders::ALL));
        frame.render_widget(empty, area);
        return;
    }

    let content_width = area.width.saturating_sub(2) as usize;
    let items: Vec<ListItem<'_>> = nodes
        .iter()
        .map(|n| {
            let icon = entry_prefix(&n.node_type);
            let status = git_status_label_for_node(state, n);
            let left = if icon.is_empty() {
                n.name.clone()
            } else {
                format!("{icon} {}", n.name)
            };
            let (left_text, padding, right_label) =
                compose_tree_entry_segments(&left, status, content_width);

            let mut spans = Vec::with_capacity(3);
            spans.push(Span::styled(left_text, node_style(n, theme, status)));
            if padding > 0 {
                spans.push(Span::raw(" ".repeat(padding)));
            }
            if let Some(label) = right_label {
                spans.push(Span::styled(label, status_label_style(status)));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_index));
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn git_status_label_for_node(state: &SessionState, node: &TreeNode) -> Option<GitFileStatus> {
    let repo = state.git_status.as_ref()?;
    let abs = absolute_display_path(&node.path);
    let rel = abs.strip_prefix(&repo.repo_root).ok()?;
    repo.file_statuses.get(rel).copied()
}

fn compose_tree_entry_segments(
    left: &str,
    status: Option<GitFileStatus>,
    content_width: usize,
) -> (String, usize, Option<&'static str>) {
    if content_width == 0 {
        return (String::new(), 0, None);
    }

    let Some(status) = status else {
        return (truncate_for_status(left, content_width), 0, None);
    };

    let Some(label) = right_label_for_status(status) else {
        return (truncate_for_status(left, content_width), 0, None);
    };
    let reserved = UnicodeWidthStr::width(label).saturating_add(1);

    if content_width <= reserved {
        return (truncate_for_status(left, content_width), 0, None);
    }

    let left_capacity = content_width - reserved;
    let left_text = truncate_for_status(left, left_capacity);
    let left_width = UnicodeWidthStr::width(left_text.as_str());
    let padding = content_width
        .saturating_sub(left_width)
        .saturating_sub(UnicodeWidthStr::width(label));

    (left_text, padding, Some(label))
}

fn right_label_for_status(status: GitFileStatus) -> Option<&'static str> {
    match status {
        GitFileStatus::Ignored => None,
        _ => Some(status.label()),
    }
}

fn status_label_style(status: Option<GitFileStatus>) -> Style {
    match status {
        Some(GitFileStatus::Deleted) => {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        }
        Some(GitFileStatus::Modified) => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        Some(GitFileStatus::Added) | Some(GitFileStatus::Untracked) => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        Some(GitFileStatus::Renamed) | Some(GitFileStatus::Copied) => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        Some(GitFileStatus::Conflicted) => Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
        Some(GitFileStatus::Ignored) => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM),
        None => Style::default(),
    }
}
