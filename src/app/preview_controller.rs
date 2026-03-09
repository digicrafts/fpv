use crate::app::state::{
    ContentType, LoadState, NodeType, PreviewDiffCache, PreviewDiffCacheKey, PreviewDocument,
    PreviewLineChange, SessionState, StyledPreviewLine, StyledPreviewSegment, TreeNode,
};
use crate::fs::current_dir::{list_current_directory_with_visibility, selected_entry_metadata};
use crate::fs::git::{git_head_content_for_file, GitFileStatus, GitRepoStatus};
use crate::fs::preview::{is_supported_image_path, load_preview};
use crate::highlight::render::render_with_highlight;
use crate::highlight::syntax::HighlightContext;
use ratatui::style::{Color, Modifier};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::{Duration, Instant};

const DIRECTORY_PREVIEW_MAX_ENTRIES: usize = 2000;
const DIFF_MAX_TOTAL_LINES: usize = 4_000;
const DIFF_MAX_EDIT_DISTANCE: usize = 1_024;
const DIFF_CACHE_MAX_LINES: usize = 8_000;
const DIFF_CACHE_MAX_BYTES: usize = 512 * 1024;
pub const IMAGE_PREVIEW_DELAY: Duration = Duration::from_secs(1);

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

fn directory_preview(
    path: &Path,
    show_hidden: bool,
    git_status: Option<&GitRepoStatus>,
) -> PreviewDocument {
    match list_current_directory_with_visibility(path, DIRECTORY_PREVIEW_MAX_ENTRIES, show_hidden) {
        Ok(entries) => {
            let mut lines = Vec::with_capacity(entries.len().saturating_add(1));
            if entries.is_empty() {
                lines.push("(empty directory)".to_string());
            } else {
                for node in &entries {
                    let name = directory_entry_label(node);
                    if let Some(git_status) = git_status
                        .and_then(|repo_status| git_status_for_entry(node, path, repo_status))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffOp {
    Equal(usize, usize),
    Delete(usize),
    Insert(usize),
}

fn myers_diff(left: &[String], right: &[String]) -> Option<Vec<DiffOp>> {
    let n = left.len();
    let m = right.len();
    if n + m > DIFF_MAX_TOTAL_LINES {
        return None;
    }

    let max = n + m;
    let offset = max as isize;
    let mut v = vec![0isize; 2 * max + 1];
    let mut trace = Vec::new();

    for d in 0..=max {
        if d > DIFF_MAX_EDIT_DISTANCE {
            return None;
        }
        for k in (-(d as isize)..=d as isize).step_by(2) {
            let index = (k + offset) as usize;
            let x = if k == -(d as isize)
                || (k != d as isize && v[index.saturating_sub(1)] < v[index + 1])
            {
                v[index + 1]
            } else {
                v[index.saturating_sub(1)] + 1
            };
            let mut x = x;
            let mut y = x - k;

            while x < n as isize && y < m as isize && left[x as usize] == right[y as usize] {
                x += 1;
                y += 1;
            }
            v[index] = x;

            if x >= n as isize && y >= m as isize {
                trace.push(v.clone());
                return Some(backtrack_myers(&trace, left, right, offset));
            }
        }
        trace.push(v.clone());
    }

    None
}

fn backtrack_myers(
    trace: &[Vec<isize>],
    left: &[String],
    right: &[String],
    offset: isize,
) -> Vec<DiffOp> {
    let mut x = left.len() as isize;
    let mut y = right.len() as isize;
    let mut ops = Vec::new();

    for d in (1..trace.len()).rev() {
        let v = &trace[d - 1];
        let k = x - y;
        let d = d as isize;
        let prev_k = if k == -d
            || (k != d
                && v[(k - 1 + offset) as usize] < v[(k + 1 + offset) as usize])
        {
            k + 1
        } else {
            k - 1
        };

        let prev_x = v[(prev_k + offset) as usize];
        let prev_y = prev_x - prev_k;

        while x > prev_x && y > prev_y {
            x -= 1;
            y -= 1;
            ops.push(DiffOp::Equal(x as usize, y as usize));
        }

        if x == prev_x {
            y -= 1;
            ops.push(DiffOp::Insert(y as usize));
        } else {
            x -= 1;
            ops.push(DiffOp::Delete(x as usize));
        }
    }

    while x > 0 && y > 0 {
        x -= 1;
        y -= 1;
        ops.push(DiffOp::Equal(x as usize, y as usize));
    }

    ops.reverse();
    ops
}

fn preview_diff_cache_key(path: &Path, current_doc: &PreviewDocument) -> PreviewDiffCacheKey {
    let mut hasher = DefaultHasher::new();
    current_doc.content_excerpt.hash(&mut hasher);

    PreviewDiffCacheKey {
        path: path.to_path_buf(),
        current_content_hash: hasher.finish(),
        content_type: current_doc.content_type.clone(),
        language_id: current_doc.language_id.clone(),
        truncated: current_doc.truncated,
    }
}

fn should_cache_diff(doc: &PreviewDocument) -> bool {
    doc.styled_lines.len() <= DIFF_CACHE_MAX_LINES && doc.content_excerpt.len() <= DIFF_CACHE_MAX_BYTES
}

fn diff_preview(
    path: &Path,
    ctx: &HighlightContext,
    max_bytes: usize,
    state: &mut SessionState,
) -> PreviewDocument {
    let current_doc = load_preview(path, max_bytes, ctx);
    if current_doc.load_state != LoadState::Ready {
        return current_doc;
    }

    let cache_key = preview_diff_cache_key(path, &current_doc);
    if let Some(cache) = state.preview_diff_cache.as_ref() {
        if cache.key == cache_key {
            return cache.merged_doc.clone();
        }
    }

    let Some(base_content) = git_head_content_for_file(path) else {
        state.preview_diff_cache = should_cache_diff(&current_doc).then(|| PreviewDiffCache {
            key: cache_key,
            merged_doc: current_doc.clone(),
        });
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
        state.preview_diff_cache = should_cache_diff(&current_doc).then(|| PreviewDiffCache {
            key: cache_key,
            merged_doc: current_doc.clone(),
        });
        return current_doc;
    }

    let Some(ops) = myers_diff(&base_text_lines, &current_text_lines) else {
        state.preview_diff_cache = should_cache_diff(&current_doc).then(|| PreviewDiffCache {
            key: cache_key,
            merged_doc: current_doc.clone(),
        });
        return current_doc;
    };

    let mut merged_lines = Vec::new();
    let mut display_line_numbers = Vec::new();
    let mut line_changes = Vec::new();

    for op in ops {
        match op {
            DiffOp::Equal(_, current_index) => {
                merged_lines.push(current_lines[current_index].clone());
                display_line_numbers.push(Some(current_index + 1));
                line_changes.push(None);
            }
            DiffOp::Insert(current_index) => {
                merged_lines.push(apply_modifier_to_line(
                    &current_lines[current_index],
                    PreviewLineChange::Added,
                ));
                display_line_numbers.push(Some(current_index + 1));
                line_changes.push(Some(PreviewLineChange::Added));
            }
            DiffOp::Delete(base_index) => {
                merged_lines.push(apply_modifier_to_line(
                    &base_lines[base_index],
                    PreviewLineChange::Deleted,
                ));
                display_line_numbers.push(Some(base_index + 1));
                line_changes.push(Some(PreviewLineChange::Deleted));
            }
        }
    }

    let merged_doc = PreviewDocument {
        source_path: path.to_path_buf(),
        load_state: LoadState::Ready,
        content_type: ContentType::Highlighted,
        image_preview: false,
        image_preview_pending: false,
        language_id: current_doc.language_id.clone(),
        content_excerpt: styled_lines_to_text_lines(&merged_lines).join("\n"),
        styled_lines: merged_lines,
        display_line_numbers,
        line_changes,
        fallback_reason: None,
        truncated: current_doc.truncated,
        error_message: None,
    };
    state.preview_diff_cache = should_cache_diff(&merged_doc).then(|| PreviewDiffCache {
        key: cache_key,
        merged_doc: merged_doc.clone(),
    });
    merged_doc
}

fn pending_image_preview(path: &Path) -> PreviewDocument {
    PreviewDocument {
        source_path: path.to_path_buf(),
        load_state: LoadState::Ready,
        content_type: ContentType::PlainText,
        image_preview: false,
        image_preview_pending: true,
        content_excerpt: "Loading image preview...".to_string(),
        ..PreviewDocument::default()
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
        state.set_selected_path(node.path.clone());
        state.selected_metadata = selected_entry_metadata(node);
        if node.node_type == NodeType::Directory {
            state.preview_diff_cache = None;
            directory_preview(&node.path, state.show_hidden, state.git_status.as_ref())
        } else if is_supported_image_path(&node.path)
            && state.selected_changed_at.elapsed() < IMAGE_PREVIEW_DELAY
        {
            state.preview_diff_cache = None;
            pending_image_preview(&node.path)
        } else if state.preview_diff_mode {
            diff_preview(&node.path, ctx, max_bytes, state)
        } else {
            state.preview_diff_cache = None;
            load_preview(&node.path, max_bytes, ctx)
        }
    } else {
        state.preview_diff_cache = None;
        state.selected_metadata = Default::default();
        PreviewDocument {
            load_state: LoadState::Error,
            error_message: Some(
                state
                    .current_dir_error
                    .clone()
                    .unwrap_or_else(|| "No selection".to_string()),
            ),
            ..PreviewDocument::default()
        }
    };
    state.preview_render_epoch = state.preview_render_epoch.saturating_add(1);
    state.preview_render_cache = None;
    state.last_preview_latency_ms = started.elapsed().as_millis();
    preview
}
