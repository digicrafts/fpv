use crate::app::state::{
    ContentType, LoadState, NodeType, PreviewDocument, PreviewLineChange, SessionState,
    StyledPreviewLine, StyledPreviewSegment, TreeNode,
};
use crate::fs::current_dir::{list_current_directory_with_visibility, selected_entry_metadata};
use crate::fs::git::{
    git_head_content_for_file, git_repo_status_for_path, GitFileStatus, GitRepoStatus,
};
use crate::fs::preview::load_preview;
use crate::highlight::render::render_with_highlight;
use crate::highlight::syntax::HighlightContext;
use ratatui::style::{Color, Modifier};
use std::path::Path;
use std::time::Instant;

const DIRECTORY_PREVIEW_MAX_ENTRIES: usize = 2000;

fn directory_entry_label(node: &TreeNode) -> String {
    match node.node_type {
        NodeType::Directory => format!("{}/", node.name),
        NodeType::Symlink => format!("@{}", node.name),
        NodeType::Unknown => format!("?{}", node.name),
        NodeType::File => node.name.clone(),
    }
}

fn git_status_for_entry(
    node: &TreeNode,
    dir_path: &Path,
    git: &GitRepoStatus,
) -> Option<GitFileStatus> {
    let rel = node.path.strip_prefix(&git.repo_root).ok()?;

    // Direct match for files
    if let Some(status) = git.file_statuses.get(rel) {
        return Some(*status);
    }

    // For directories, check if any child has a status
    if node.node_type == NodeType::Directory {
        for (path, status) in &git.file_statuses {
            if path.starts_with(rel) && *status != GitFileStatus::Ignored {
                return Some(GitFileStatus::Modified);
            }
        }
        return None;
    }

    // Try relative to the directory being previewed
    let dir_rel = dir_path
        .strip_prefix(&git.repo_root)
        .ok()
        .map(|d| d.join(&node.name));
    if let Some(dr) = dir_rel {
        if let Some(status) = git.file_statuses.get(&dr) {
            return Some(*status);
        }
    }

    None
}

fn format_git_label(status: GitFileStatus) -> &'static str {
    match status {
        GitFileStatus::Added => "[A]",
        GitFileStatus::Modified => "[M]",
        GitFileStatus::Deleted => "[D]",
        GitFileStatus::Renamed => "[R]",
        GitFileStatus::Copied => "[C]",
        GitFileStatus::Untracked => "[?]",
        GitFileStatus::Conflicted => "[U]",
        GitFileStatus::Ignored => "[!]",
    }
}

fn directory_preview(path: &Path, show_hidden: bool) -> PreviewDocument {
    match list_current_directory_with_visibility(path, DIRECTORY_PREVIEW_MAX_ENTRIES, show_hidden) {
        Ok(entries) => {
            let git = git_repo_status_for_path(path);
            let mut lines = Vec::with_capacity(entries.len().saturating_add(1));
            if entries.is_empty() {
                lines.push("(empty directory)".to_string());
            } else {
                for node in &entries {
                    let name = directory_entry_label(node);
                    if let Some(git_status) = git
                        .as_ref()
                        .and_then(|g| git_status_for_entry(node, path, g))
                    {
                        lines.push(format!("{} {}", format_git_label(git_status), name));
                    } else {
                        lines.push(format!("    {}", name));
                    }
                }
            }
            PreviewDocument {
                source_path: path.to_path_buf(),
                load_state: LoadState::Ready,
                content_excerpt: lines.join("\n"),
                ..PreviewDocument::default()
            }
        }
        Err(_) => PreviewDocument {
            source_path: path.to_path_buf(),
            load_state: LoadState::Error,
            error_message: Some("Cannot read directory.".to_string()),
            ..PreviewDocument::default()
        },
    }
}

fn styled_lines_to_text_lines(lines: &[StyledPreviewLine]) -> Vec<String> {
    lines
        .iter()
        .map(|line| line.iter().map(|segment| segment.text.as_str()).collect())
        .collect()
}

fn plain_text_to_styled_lines(text: &str) -> Vec<StyledPreviewLine> {
    text.split('\n')
        .map(|line| {
            if line.is_empty() {
                Vec::new()
            } else {
                vec![StyledPreviewSegment {
                    text: line.to_string(),
                    style: Default::default(),
                }]
            }
        })
        .collect()
}

fn display_styled_lines(doc: &PreviewDocument) -> Vec<StyledPreviewLine> {
    if matches!(doc.content_type, ContentType::Highlighted) && !doc.styled_lines.is_empty() {
        doc.styled_lines.clone()
    } else {
        plain_text_to_styled_lines(&doc.content_excerpt)
    }
}

fn apply_modifier_to_line(
    line: &StyledPreviewLine,
    change: PreviewLineChange,
) -> StyledPreviewLine {
    line.iter()
        .map(|segment| {
            let style = match change {
                PreviewLineChange::Added => segment
                    .style
                    .bg(Color::Rgb(0, 70, 0))
                    .add_modifier(Modifier::BOLD),
                PreviewLineChange::Deleted => segment
                    .style
                    .bg(Color::Rgb(90, 0, 0))
                    .add_modifier(Modifier::DIM),
            };
            StyledPreviewSegment {
                text: segment.text.clone(),
                style,
            }
        })
        .collect()
}

fn lcs_table(left: &[String], right: &[String]) -> Vec<Vec<usize>> {
    let mut table = vec![vec![0; right.len() + 1]; left.len() + 1];
    for i in (0..left.len()).rev() {
        for j in (0..right.len()).rev() {
            table[i][j] = if left[i] == right[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }
    table
}

fn diff_preview(path: &Path, ctx: &HighlightContext, max_bytes: usize) -> PreviewDocument {
    let current_doc = load_preview(path, max_bytes, ctx);
    if current_doc.load_state != LoadState::Ready {
        return current_doc;
    }

    let Some(base_content) = git_head_content_for_file(path) else {
        return current_doc;
    };

    let current_lines = display_styled_lines(&current_doc);
    let current_text_lines = styled_lines_to_text_lines(&current_lines);
    let base_rendered = render_with_highlight(ctx, path, &base_content);
    let base_lines = if matches!(base_rendered.content_type, ContentType::Highlighted)
        && !base_rendered.styled_lines.is_empty()
    {
        base_rendered.styled_lines
    } else {
        plain_text_to_styled_lines(&base_content)
    };
    let base_text_lines = styled_lines_to_text_lines(&base_lines);

    if base_text_lines == current_text_lines {
        return current_doc;
    }

    let table = lcs_table(&base_text_lines, &current_text_lines);
    let mut i = 0usize;
    let mut j = 0usize;
    let mut merged_lines = Vec::new();
    let mut display_line_numbers = Vec::new();
    let mut line_changes = Vec::new();

    while i < base_lines.len() || j < current_lines.len() {
        if i < base_lines.len()
            && j < current_lines.len()
            && base_text_lines[i] == current_text_lines[j]
        {
            merged_lines.push(current_lines[j].clone());
            display_line_numbers.push(Some(j + 1));
            line_changes.push(None);
            i += 1;
            j += 1;
        } else if j < current_lines.len()
            && (i == base_lines.len() || table[i][j + 1] > table[i + 1][j])
        {
            merged_lines.push(apply_modifier_to_line(
                &current_lines[j],
                PreviewLineChange::Added,
            ));
            display_line_numbers.push(Some(j + 1));
            line_changes.push(Some(PreviewLineChange::Added));
            j += 1;
        } else if i < base_lines.len() {
            merged_lines.push(apply_modifier_to_line(
                &base_lines[i],
                PreviewLineChange::Deleted,
            ));
            display_line_numbers.push(Some(i + 1));
            line_changes.push(Some(PreviewLineChange::Deleted));
            i += 1;
        }
    }

    PreviewDocument {
        source_path: path.to_path_buf(),
        load_state: LoadState::Ready,
        content_type: ContentType::Highlighted,
        language_id: current_doc.language_id.clone(),
        content_excerpt: styled_lines_to_text_lines(&merged_lines).join("\n"),
        styled_lines: merged_lines,
        display_line_numbers,
        line_changes,
        fallback_reason: None,
        truncated: current_doc.truncated,
        error_message: None,
    }
}

pub fn refresh_preview(
    state: &mut SessionState,
    nodes: &[TreeNode],
    ctx: &HighlightContext,
    max_bytes: usize,
) -> PreviewDocument {
    let started = Instant::now();
    let preview = if let Some(node) = nodes.get(state.selected_index) {
        state.selected_path = node.path.clone();
        state.selected_metadata = selected_entry_metadata(node);
        if node.node_type == NodeType::Directory {
            directory_preview(&node.path, state.show_hidden)
        } else if state.preview_diff_mode {
            diff_preview(&node.path, ctx, max_bytes)
        } else {
            load_preview(&node.path, max_bytes, ctx)
        }
    } else {
        state.selected_metadata = Default::default();
        PreviewDocument {
            load_state: LoadState::Error,
            error_message: Some("No selection".to_string()),
            ..PreviewDocument::default()
        }
    };
    state.last_preview_latency_ms = started.elapsed().as_millis();
    preview
}
