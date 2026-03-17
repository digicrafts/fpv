use crate::app::state::{
    ContentType, LoadState, PreviewDocument, PreviewLineChange, PreviewRenderCache,
    PreviewRenderCacheKey, PreviewSelection, SessionState,
};
use crate::config::load::ThemeProfile;
use crate::tui::colors::{
    DIFF_ADDED_BG, DIFF_BADGE_BG, DIFF_BADGE_FG, DIFF_DELETED_BG, DIFF_MARKER_ADDED,
    DIFF_MARKER_DELETED, LINE_NUMBER_FG, OVERLAY_BG, OVERLAY_FG, SCROLLBAR_THUMB, SCROLLBAR_TRACK,
    SELECTION_BG, SELECTION_FG,
};
use crate::tui::status_bar::compose_preview_metadata_line;
use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use unicode_width::UnicodeWidthChar;

const RENDER_CACHE_MAX_LINES: usize = 8_000;
const RENDER_CACHE_MAX_TEXT_BYTES: usize = 512 * 1024;

pub fn preview_title_for_state(state: &SessionState) -> String {
    if state.selected_metadata.filename.trim().is_empty() || state.selected_metadata.filename == "-"
    {
        "Preview".to_string()
    } else {
        state.selected_metadata.filename.clone()
    }
}

pub fn preview_border_metadata_for_state(state: &SessionState, width: usize) -> String {
    compose_preview_metadata_line(&state.selected_metadata, width)
}

fn line_count(text: &str) -> usize {
    text.split('\n').count().max(1)
}

fn preview_scroll_position_text(state: &SessionState, total_lines: usize) -> String {
    let total = total_lines.max(1);
    let current = state.preview_scroll_row.saturating_add(1).min(total);
    format!("[{current}:{total}]")
}

fn preview_border_bottom_line(state: &SessionState, total_lines: usize, width: usize) -> String {
    let mut left = preview_scroll_position_text(state, total_lines);
    if state.preview_diff_mode {
        left.push_str(" DIFF");
    }
    if width <= left.len() {
        return left.chars().take(width).collect();
    }

    let right = preview_border_metadata_for_state(state, width.saturating_sub(left.len() + 1));
    let mut line = left;
    line.push(' ');
    let right_width = width.saturating_sub(line.len());

    if right_width == 0 {
        return line.chars().take(width).collect();
    }

    let right_with_pad = if right.is_empty() {
        String::new()
    } else {
        let gap = right_width.saturating_sub(right.len());
        format!("{}{}", " ".repeat(gap), right)
    };
    line.push_str(&right_with_pad);

    if line.len() > width {
        line.chars().take(width).collect()
    } else {
        line
    }
}

fn plain_text_for_doc(doc: &PreviewDocument) -> String {
    match doc.load_state {
        LoadState::Error | LoadState::Binary => doc
            .error_message
            .clone()
            .unwrap_or_else(|| "Unable to render preview".to_string()),
        _ => doc.content_excerpt.clone(),
    }
}

fn has_styled_preview_content(doc: &PreviewDocument) -> bool {
    !doc.styled_lines.is_empty()
}

fn line_number_width(total_lines: usize) -> usize {
    total_lines.max(1).to_string().len().max(2)
}

fn line_number_style() -> Style {
    Style::default().fg(LINE_NUMBER_FG)
}

fn displayed_line_number(doc: &PreviewDocument, index: usize) -> usize {
    doc.display_line_numbers
        .get(index)
        .copied()
        .flatten()
        .unwrap_or(index + 1)
}

fn displayed_line_number_width(doc: &PreviewDocument) -> usize {
    let max_number = if doc.display_line_numbers.is_empty() {
        preview_total_lines(doc)
    } else {
        doc.display_line_numbers
            .iter()
            .flatten()
            .copied()
            .max()
            .unwrap_or_else(|| preview_total_lines(doc))
    };
    line_number_width(max_number)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffMarkerKind {
    Added,
    Deleted,
}

fn diff_marker_kind(change: Option<PreviewLineChange>) -> Option<DiffMarkerKind> {
    match change {
        Some(PreviewLineChange::Added) => Some(DiffMarkerKind::Added),
        Some(PreviewLineChange::Deleted) => Some(DiffMarkerKind::Deleted),
        None => None,
    }
}

fn cache_signature(
    content: &str,
    styled_lines: &[crate::app::state::StyledPreviewLine],
    line_changes: &[Option<PreviewLineChange>],
) -> (u64, u64, u64) {
    let mut content_hasher = DefaultHasher::new();
    content.hash(&mut content_hasher);

    let mut styled_lines_hasher = DefaultHasher::new();
    for line in styled_lines {
        line.len().hash(&mut styled_lines_hasher);
        for segment in line {
            segment.text.hash(&mut styled_lines_hasher);
            format!("{:?}", segment.style).hash(&mut styled_lines_hasher);
        }
    }

    let mut line_changes_hasher = DefaultHasher::new();
    for change in line_changes {
        change.hash(&mut line_changes_hasher);
    }

    (
        content_hasher.finish(),
        styled_lines_hasher.finish(),
        line_changes_hasher.finish(),
    )
}

fn diff_marker_span(kind: Option<DiffMarkerKind>) -> Span<'static> {
    match kind {
        Some(DiffMarkerKind::Added) => Span::styled(
            "+",
            Style::default()
                .fg(DIFF_MARKER_ADDED)
                .bg(DIFF_ADDED_BG)
                .add_modifier(Modifier::BOLD),
        ),
        Some(DiffMarkerKind::Deleted) => Span::styled(
            "-",
            Style::default()
                .fg(DIFF_MARKER_DELETED)
                .bg(DIFF_DELETED_BG)
                .add_modifier(Modifier::DIM),
        ),
        None => Span::raw(" "),
    }
}

fn diff_fill_span(kind: Option<DiffMarkerKind>, width: usize) -> Span<'static> {
    let text = " ".repeat(width);
    match kind {
        Some(DiffMarkerKind::Added) => Span::styled(text, Style::default().bg(DIFF_ADDED_BG)),
        Some(DiffMarkerKind::Deleted) => Span::styled(text, Style::default().bg(DIFF_DELETED_BG)),
        None => Span::raw(text),
    }
}

fn line_number_prefix(
    line_number: usize,
    line_number_width: usize,
    diff_marker: Option<DiffMarkerKind>,
) -> Vec<Span<'static>> {
    let number_style = match diff_marker {
        Some(DiffMarkerKind::Added) => Style::default()
            .fg(DIFF_MARKER_ADDED)
            .bg(DIFF_ADDED_BG)
            .add_modifier(Modifier::BOLD),
        Some(DiffMarkerKind::Deleted) => Style::default()
            .fg(DIFF_MARKER_DELETED)
            .bg(DIFF_DELETED_BG)
            .add_modifier(Modifier::DIM),
        None => line_number_style(),
    };
    vec![
        diff_marker_span(diff_marker),
        Span::styled(
            format!("{line_number:>width$}", width = line_number_width),
            number_style,
        ),
    ]
}

fn line_number_blank_prefix(line_number_width: usize) -> Vec<Span<'static>> {
    let width = line_number_width + 1;
    vec![Span::raw(format!("{:width$}", "", width = width))]
}

fn diff_only_prefix(diff_marker: Option<DiffMarkerKind>) -> Vec<Span<'static>> {
    vec![
        diff_marker_span(diff_marker),
        diff_fill_span(diff_marker, 1),
    ]
}

fn line_number_separator(diff_marker: Option<DiffMarkerKind>) -> Span<'static> {
    diff_fill_span(diff_marker, 2)
}

pub fn preview_total_lines(doc: &PreviewDocument) -> usize {
    if has_styled_preview_content(doc) {
        return doc.styled_lines.len();
    }
    line_count(&plain_text_for_doc(doc))
}

pub fn preview_max_line_width(doc: &PreviewDocument) -> usize {
    if has_styled_preview_content(doc) {
        doc.styled_lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|seg| seg.text.chars().count())
                    .sum::<usize>()
            })
            .max()
            .unwrap_or(0)
    } else {
        plain_text_for_doc(doc)
            .lines()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0)
    }
}

pub fn extract_selected_text(doc: &PreviewDocument, selection: &PreviewSelection) -> String {
    let (start, end) = selection.ordered();
    let lines: Vec<String> = if has_styled_preview_content(doc) {
        doc.styled_lines
            .iter()
            .map(|line| line.iter().map(|seg| seg.text.as_str()).collect::<String>())
            .collect()
    } else {
        plain_text_for_doc(doc)
            .lines()
            .map(|s| s.to_string())
            .collect()
    };

    if lines.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    for row in start.row..=end.row.min(lines.len().saturating_sub(1)) {
        let line = &lines[row];
        let chars: Vec<char> = line.chars().collect();
        let line_start = if row == start.row {
            start.col.min(chars.len())
        } else {
            0
        };
        let line_end = if row == end.row {
            (end.col + 1).min(chars.len())
        } else {
            chars.len()
        };
        if line_start <= line_end {
            result.extend(&chars[line_start..line_end]);
        }
        if row < end.row {
            result.push('\n');
        }
    }
    result
}

fn apply_selection_highlight(
    frame: &mut Frame<'_>,
    inner: ratatui::layout::Rect,
    selection: &PreviewSelection,
    scroll_row: usize,
    scroll_col: usize,
    line_number_cols: usize,
) {
    let (start, end) = selection.ordered();
    let buf = frame.buffer_mut();

    for screen_y in inner.y..inner.y.saturating_add(inner.height) {
        let content_row = (screen_y.saturating_sub(inner.y)) as usize + scroll_row;

        for screen_x in inner.x..inner.x.saturating_add(inner.width) {
            let screen_col_offset = (screen_x.saturating_sub(inner.x)) as usize;

            let content_col = if line_number_cols > 0 {
                if screen_col_offset < line_number_cols {
                    continue;
                }
                screen_col_offset - line_number_cols + scroll_col
            } else {
                screen_col_offset + scroll_col
            };

            let in_selection = if content_row > start.row && content_row < end.row {
                true
            } else if content_row == start.row && content_row == end.row {
                content_col >= start.col && content_col <= end.col
            } else if content_row == start.row {
                content_col >= start.col
            } else if content_row == end.row {
                content_col <= end.col
            } else {
                false
            };

            if in_selection {
                if let Some(cell) = buf.cell_mut((screen_x, screen_y)) {
                    let s = cell
                        .style()
                        .bg(SELECTION_BG)
                        .fg(SELECTION_FG)
                        .remove_modifier(Modifier::DIM);
                    cell.set_style(s);
                }
            }
        }
    }
}

fn apply_search_highlights(
    frame: &mut Frame<'_>,
    inner: ratatui::layout::Rect,
    search: &crate::app::state::PreviewSearch,
    scroll_row: usize,
    scroll_col: usize,
    line_number_cols: usize,
    content_lines: &[&str],
) {
    use crate::tui::colors::{SEARCH_CURRENT_MATCH_BG, SEARCH_MATCH_BG, SEARCH_MATCH_FG};

    let buf = frame.buffer_mut();
    if search.match_positions.is_empty() {
        return;
    };

    for (match_idx, &(match_line, match_start, match_end)) in search.match_positions.iter().enumerate() {
        let is_current = match_idx == search.current_match_index;
        let bg = if is_current {
            SEARCH_CURRENT_MATCH_BG
        } else {
            SEARCH_MATCH_BG
        };

        if match_line < scroll_row || match_line >= scroll_row + inner.height as usize {
            continue;
        }

        let screen_y = inner.y + (match_line - scroll_row) as u16;
        let line_text = content_lines.get(match_line).copied().unwrap_or("");
        if line_text.is_empty() {
            continue;
        }

        let start_col = byte_to_screen_col(line_text, match_start);
        let end_col = byte_to_screen_col(line_text, match_end);

        for col in start_col..end_col {
            let screen_col_offset = col.saturating_sub(scroll_col);
            if screen_col_offset >= inner.width as usize {
                continue;
            }

            let line_number_width = line_number_cols.min(inner.width as usize) as u16;
            let screen_x = inner.x + line_number_width + screen_col_offset as u16;

            if screen_x >= inner.x && screen_x < inner.x + inner.width && screen_y >= inner.y && screen_y < inner.y + inner.height {
                if let Some(cell) = buf.cell_mut((screen_x, screen_y)) {
                    let style = cell.style().bg(bg).fg(SEARCH_MATCH_FG);
                    cell.set_style(style);
                }
            }
        }
    }
}

fn byte_to_screen_col(line: &str, byte_idx: usize) -> usize {
    let target = byte_idx.min(line.len());
    let mut col = 0usize;
    for (idx, ch) in line.char_indices() {
        if idx >= target {
            break;
        }
        col += UnicodeWidthChar::width(ch).unwrap_or(0);
    }
    col
}

fn render_overlay_label(
    frame: &mut Frame<'_>,
    inner: ratatui::layout::Rect,
    label: &str,
    style: Style,
) {
    let label_width = label.len() as u16;
    if inner.width < label_width || inner.height == 0 {
        return;
    }
    let x = inner.x + inner.width.saturating_sub(label_width);
    let y = inner.y;
    let area = ratatui::layout::Rect::new(x, y, label_width, 1);
    let widget = Paragraph::new(Span::styled(label, style));
    frame.render_widget(widget, area);
}

fn render_search_bar(
    frame: &mut Frame<'_>,
    top_area: ratatui::layout::Rect,
    search: &crate::app::state::PreviewSearch,
) {
    let (current, total) = crate::app::preview_search::match_count(search);
    let case_indicator = if search.case_sensitive { "[Aa]" } else { "[aa]" };
    let match_text = if total == 0 {
        "no matches".to_string()
    } else {
        format!("{}/{}", current, total)
    };

    let style = Style::default().fg(OVERLAY_FG).bg(OVERLAY_BG);

    let query_text = format!("Search: {}", search.query);
    let status_text = format!("{} {}", match_text, case_indicator);
    let status_width = status_text.chars().count() as u16;

    if top_area.height == 0 || top_area.width == 0 {
        return;
    }

    let usable_status_width = status_width.min(top_area.width);
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Fill(1),
            ratatui::layout::Constraint::Length(usable_status_width),
        ])
        .split(top_area);

    let query_widget = Paragraph::new(query_text).style(style);
    let status_widget = Paragraph::new(status_text).style(style);
    frame.render_widget(query_widget, chunks[0]);
    frame.render_widget(status_widget, chunks[1]);
}

fn render_bottom_diff_badge(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    state: &SessionState,
    total_lines: usize,
) {
    if !state.preview_diff_mode || area.width <= 2 || area.height == 0 {
        return;
    }

    let line_count_text = preview_scroll_position_text(state, total_lines);
    let badge_text = "DIFF";
    let badge_x = area.x + 1 + line_count_text.chars().count() as u16 + 1;
    let content_right_x = area.x + area.width.saturating_sub(1);
    if badge_x.saturating_add(badge_text.len() as u16) > content_right_x {
        return;
    }

    let badge_style = Style::default()
        .fg(DIFF_BADGE_FG)
        .bg(DIFF_BADGE_BG)
        .add_modifier(Modifier::BOLD);
    let badge_area = ratatui::layout::Rect::new(
        badge_x,
        area.y + area.height.saturating_sub(1),
        badge_text.len() as u16,
        1,
    );
    frame.render_widget(Paragraph::new(Span::styled(badge_text, badge_style)), badge_area);
}

fn is_unsupported_preview(doc: &PreviewDocument) -> bool {
    matches!(doc.content_type, ContentType::Unsupported)
        || matches!(doc.load_state, LoadState::Binary)
        || matches!(doc.load_state, LoadState::Error)
}

fn line_numbers_enabled(state: &SessionState, doc: &PreviewDocument) -> bool {
    state.preview_show_line_numbers
        && doc.source_path.is_file()
        && !doc.image_preview
        && !doc.image_preview_pending
        && !is_unsupported_preview(doc)
}

fn render_scroll_indicator(
    frame: &mut Frame<'_>,
    inner: ratatui::layout::Rect,
    total_lines: usize,
    scroll_row: usize,
    rendered_row_changes: &[Option<PreviewLineChange>],
    search: Option<&crate::app::state::PreviewSearch>,
) {
    use crate::tui::colors::{SEARCH_CURRENT_MATCH_BG, SEARCH_MATCH_BG};

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    #[derive(Clone, Copy, Default)]
    struct ScrollMarker {
        diff: Option<PreviewLineChange>,
        has_match: bool,
        current_match: bool,
    }

    let viewport_rows = inner.height as usize;
    if total_lines <= viewport_rows {
        return;
    }

    let indicator_height = viewport_rows;
    let thumb_height = ((viewport_rows * viewport_rows) / total_lines)
        .max(1)
        .min(indicator_height);
    let max_scroll = total_lines.saturating_sub(viewport_rows);
    let max_thumb_top = indicator_height.saturating_sub(thumb_height);
    let thumb_top = if max_scroll == 0 {
        0
    } else {
        scroll_row.saturating_mul(max_thumb_top) / max_scroll
    };

    let track_style = Style::default().fg(SCROLLBAR_TRACK);
    let thumb_style = Style::default().fg(SCROLLBAR_THUMB);
    let mut scroll_markers = vec![ScrollMarker::default(); indicator_height];
    if total_lines > 0 {
        let max_row_index = total_lines.saturating_sub(1).max(1);
        let max_indicator_index = indicator_height.saturating_sub(1);
        for (row_index, change) in rendered_row_changes.iter().copied().enumerate() {
            let Some(change) = change else {
                continue;
            };
            let indicator_row = row_index.saturating_mul(max_indicator_index) / max_row_index;
            let slot = &mut scroll_markers[indicator_row];
            slot.diff = Some(match slot.diff {
                Some(PreviewLineChange::Deleted) => PreviewLineChange::Deleted,
                Some(existing) => existing,
                None => change,
            });
        }
    }

    // Add search match markers
    if let Some(search) = search {
        for (index, &(match_line, _, _)) in search.match_positions.iter().enumerate() {
            if match_line < total_lines {
                let max_row_index = total_lines.saturating_sub(1).max(1);
                let max_indicator_index = indicator_height.saturating_sub(1);
                let indicator_row = match_line.saturating_mul(max_indicator_index) / max_row_index;
                if indicator_row < scroll_markers.len() {
                    if index == search.current_match_index {
                        scroll_markers[indicator_row].current_match = true;
                    } else {
                        scroll_markers[indicator_row].has_match = true;
                    }
                }
            }
        }
    }

    let mut scrollbar_lines = Vec::with_capacity(indicator_height);
    let mut border_lines = Vec::with_capacity(indicator_height);
    for row in 0..indicator_height {
        let is_thumb = row >= thumb_top && row < thumb_top + thumb_height;
        let marker = scroll_markers[row];
        let (scroll_char, scroll_style) = if is_thumb {
            ("█", thumb_style)
        } else {
            ("│", track_style)
        };
        scrollbar_lines.push(Line::from(Span::styled(scroll_char, scroll_style)));

        let (indicator_char, indicator_style) = if marker.current_match {
            ("◉", Style::default().fg(SEARCH_CURRENT_MATCH_BG))
        } else if marker.has_match {
            ("◌", Style::default().fg(SEARCH_MATCH_BG))
        } else {
            match marker.diff {
                Some(PreviewLineChange::Added) => ("•", Style::default().fg(DIFF_MARKER_ADDED)),
                Some(PreviewLineChange::Deleted) => ("•", Style::default().fg(DIFF_MARKER_DELETED)),
                None => ("│", track_style),
            }
        };
        border_lines.push(Line::from(Span::styled(indicator_char, indicator_style)));
    }

    let scrollbar_area = ratatui::layout::Rect {
        x: inner.x + inner.width.saturating_sub(1),
        y: inner.y,
        width: 1,
        height: inner.height,
    };
    let border_indicator_area = ratatui::layout::Rect {
        x: inner.x + inner.width,
        y: inner.y,
        width: 1,
        height: inner.height,
    };

    frame.render_widget(Paragraph::new(Text::from(scrollbar_lines)), scrollbar_area);
    frame.render_widget(Paragraph::new(Text::from(border_lines)), border_indicator_area);
}

fn diff_background_for_change(change: Option<PreviewLineChange>) -> Option<Color> {
    match change {
        Some(PreviewLineChange::Added) => Some(DIFF_ADDED_BG),
        Some(PreviewLineChange::Deleted) => Some(DIFF_DELETED_BG),
        None => None,
    }
}

fn paint_full_row_diff_background(
    frame: &mut Frame<'_>,
    inner: ratatui::layout::Rect,
    rendered_row_changes: &[Option<PreviewLineChange>],
    scroll_row: usize,
    line_number_cols: u16,
) {
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let paint_start = inner.x.saturating_add(line_number_cols.min(inner.width));
    let paint_end = inner.x.saturating_add(inner.width);

    let buf = frame.buffer_mut();
    for screen_y in 0..inner.height {
        let rendered_row = scroll_row + screen_y as usize;
        let Some(bg) = rendered_row_changes
            .get(rendered_row)
            .copied()
            .flatten()
            .and_then(|change| diff_background_for_change(Some(change)))
        else {
            continue;
        };

        for screen_x in paint_start..paint_end {
            if let Some(cell) = buf.cell_mut((screen_x, inner.y + screen_y)) {
                cell.set_style(cell.style().bg(bg));
            }
        }
    }
}

fn wrap_styled_spans(spans: Vec<Span<'_>>, width: usize) -> Vec<Vec<Span<'static>>> {
    let wrap_width = width.max(1);
    let mut wrapped: Vec<Vec<Span<'static>>> = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut current_width = 0usize;

    for span in spans {
        let style = span.style;
        let text = span.content.into_owned();
        if text.is_empty() {
            continue;
        }

        let mut chunk = String::new();
        let mut chunk_width = 0usize;

        for ch in text.chars() {
            let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            let pending_width = current_width + chunk_width + char_width;

            if pending_width > wrap_width && (current_width > 0 || chunk_width > 0) {
                if !chunk.is_empty() {
                    current.push(Span::styled(std::mem::take(&mut chunk), style));
                    chunk_width = 0;
                }
                wrapped.push(std::mem::take(&mut current));
                current_width = 0;
            }

            chunk.push(ch);
            chunk_width += char_width;

            if current_width + chunk_width >= wrap_width {
                current.push(Span::styled(std::mem::take(&mut chunk), style));
                chunk_width = 0;
                wrapped.push(std::mem::take(&mut current));
                current_width = 0;
            }
        }

        if !chunk.is_empty() {
            current.push(Span::styled(chunk, style));
            current_width += chunk_width;
        }
    }

    if !current.is_empty() {
        wrapped.push(current);
    }
    if wrapped.is_empty() {
        wrapped.push(Vec::new());
    }
    wrapped
}

fn numbered_lines_with_wrapped_content(
    line_number: usize,
    line_number_width: usize,
    diff_marker: Option<DiffMarkerKind>,
    content_spans: Vec<Span<'_>>,
    content_width: usize,
) -> Vec<Line<'static>> {
    let wrapped = wrap_styled_spans(content_spans, content_width);
    let mut lines = Vec::with_capacity(wrapped.len());

    for (index, content_line) in wrapped.into_iter().enumerate() {
        let prefix_spans = if index == 0 {
            line_number_prefix(line_number, line_number_width, diff_marker)
        } else {
            line_number_blank_prefix(line_number_width)
        };
        let mut spans = Vec::with_capacity(content_line.len() + prefix_spans.len() + 1);
        spans.extend(prefix_spans);
        spans.push(line_number_separator(diff_marker));
        spans.extend(content_line);
        lines.push(Line::from(spans));
    }
    lines
}

fn build_preview_render_cache(
    doc: &PreviewDocument,
    epoch: u64,
    inner_width: u16,
    show_line_numbers: bool,
    use_wrap: bool,
    diff_mode: bool,
) -> PreviewRenderCache {
    let text = plain_text_for_doc(doc);
    let show_diff_markers = diff_mode && doc.line_changes.iter().any(|c| c.is_some());
    let render_styled_lines: Cow<'_, [crate::app::state::StyledPreviewLine]> =
        Cow::Borrowed(&doc.styled_lines);

    let (rendered_lines, rendered_row_changes, line_number_cols) = if show_line_numbers {
        if has_styled_preview_content(doc) {
            let line_number_cols = displayed_line_number_width(doc);
            let content_width = inner_width
                .saturating_sub((line_number_cols + 3) as u16)
                .max(1) as usize;
            let mut lines = Vec::new();
            let mut row_changes = Vec::new();

            for (index, styled_line) in render_styled_lines.iter().enumerate() {
                let line_number = displayed_line_number(doc, index);
                let content_spans = styled_line
                    .iter()
                    .map(|segment| Span::styled(segment.text.clone(), segment.style))
                    .collect::<Vec<_>>();
                let diff_marker = if diff_mode {
                    diff_marker_kind(doc.line_changes.get(index).copied().flatten())
                } else {
                    None
                };
                if use_wrap {
                    let wrapped = numbered_lines_with_wrapped_content(
                        line_number,
                        line_number_cols,
                        diff_marker,
                        content_spans,
                        content_width,
                    );
                    row_changes.extend(std::iter::repeat_n(
                        doc.line_changes.get(index).copied().flatten(),
                        wrapped.len(),
                    ));
                    lines.extend(wrapped);
                } else {
                    let mut spans = line_number_prefix(line_number, line_number_cols, diff_marker);
                    spans.push(line_number_separator(diff_marker));
                    spans.extend(content_spans);
                    lines.push(Line::from(spans));
                    row_changes.push(doc.line_changes.get(index).copied().flatten());
                }
            }
            (lines, row_changes, line_number_cols + 3)
        } else {
            let rows = text.split('\n').collect::<Vec<_>>();
            let line_number_cols = displayed_line_number_width(doc);
            let content_width = inner_width
                .saturating_sub((line_number_cols + 3) as u16)
                .max(1) as usize;
            let mut lines = Vec::new();
            let mut row_changes = Vec::new();

            for (index, row) in rows.iter().enumerate() {
                let line_number = displayed_line_number(doc, index);
                let content_spans = vec![Span::raw((*row).to_string())];
                if use_wrap {
                    let wrapped = numbered_lines_with_wrapped_content(
                        line_number,
                        line_number_cols,
                        None,
                        content_spans,
                        content_width,
                    );
                    row_changes.extend(std::iter::repeat_n(None, wrapped.len()));
                    lines.extend(wrapped);
                } else {
                    let mut spans = line_number_prefix(line_number, line_number_cols, None);
                    spans.push(line_number_separator(None));
                    spans.push(Span::raw((*row).to_string()));
                    lines.push(Line::from(spans));
                    row_changes.push(None);
                }
            }
            (lines, row_changes, line_number_cols + 3)
        }
    } else if has_styled_preview_content(doc) {
        let line_number_cols = if show_diff_markers { 2 } else { 0 };
        let content_width = inner_width.max(1) as usize;
        let mut lines = Vec::new();
        let mut row_changes = Vec::new();

        for (index, styled_line) in render_styled_lines.iter().enumerate() {
            let content_spans: Vec<Span<'_>> = styled_line
                .iter()
                .map(|segment| Span::styled(segment.text.clone(), segment.style))
                .collect();
            let diff_marker = if show_diff_markers {
                diff_marker_kind(doc.line_changes.get(index).copied().flatten())
            } else {
                None
            };
            if use_wrap {
                let wrapped = wrap_styled_spans(content_spans, content_width);
                row_changes.extend(std::iter::repeat_n(
                    doc.line_changes.get(index).copied().flatten(),
                    wrapped.len(),
                ));
                for (wrapped_index, wrapped_line) in wrapped.into_iter().enumerate() {
                    let mut spans = if wrapped_index == 0 {
                        diff_only_prefix(diff_marker)
                    } else if show_diff_markers {
                        diff_only_prefix(None)
                    } else {
                        Vec::new()
                    };
                    spans.extend(wrapped_line);
                    lines.push(Line::from(spans));
                }
            } else {
                let mut spans = if show_diff_markers {
                    diff_only_prefix(diff_marker)
                } else {
                    Vec::new()
                };
                spans.extend(content_spans);
                lines.push(Line::from(spans));
                row_changes.push(doc.line_changes.get(index).copied().flatten());
            }
        }
        (lines, row_changes, line_number_cols)
    } else if use_wrap {
        let line_number_cols = if show_diff_markers { 2 } else { 0 };
        let rows = text.split('\n').collect::<Vec<_>>();
        let content_width = inner_width.saturating_sub(line_number_cols as u16).max(1) as usize;
        let mut lines = Vec::new();
        let mut row_changes = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            let diff_marker = if show_diff_markers {
                diff_marker_kind(doc.line_changes.get(index).copied().flatten())
            } else {
                None
            };
            let wrapped = wrap_styled_spans(vec![Span::raw((*row).to_string())], content_width);
            row_changes.extend(std::iter::repeat_n(
                doc.line_changes.get(index).copied().flatten(),
                wrapped.len(),
            ));
            for (wrapped_index, wrapped_line) in wrapped.into_iter().enumerate() {
                let mut spans = if wrapped_index == 0 {
                    diff_only_prefix(diff_marker)
                } else if show_diff_markers {
                    diff_only_prefix(None)
                } else {
                    Vec::new()
                };
                spans.extend(wrapped_line);
                lines.push(Line::from(spans));
            }
        }
        (lines, row_changes, line_number_cols)
    } else {
        let line_number_cols = if show_diff_markers { 2 } else { 0 };
        let rows = text.split('\n').collect::<Vec<_>>();
        let mut lines = Vec::new();
        let mut row_changes = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            if show_diff_markers {
                let mut spans = diff_only_prefix(diff_marker_kind(
                    doc.line_changes.get(index).copied().flatten(),
                ));
                spans.push(Span::raw((*row).to_string()));
                lines.push(Line::from(spans));
                row_changes.push(doc.line_changes.get(index).copied().flatten());
            } else {
                lines.push(Line::from(Span::raw((*row).to_string())));
            }
        }
        (lines, row_changes, line_number_cols)
    };

    let (content_hash, styled_lines_hash, line_changes_hash) =
        cache_signature(&doc.content_excerpt, &doc.styled_lines, &doc.line_changes);

    PreviewRenderCache {
        key: PreviewRenderCacheKey {
            epoch,
            inner_width,
            show_line_numbers,
            wrap_enabled: use_wrap,
            content_hash,
            styled_lines_hash,
            line_changes_hash,
        },
        total_lines: rendered_lines.len().max(1),
        rendered_lines,
        rendered_row_changes,
        line_number_cols,
    }
}

fn should_cache_render(doc: &PreviewDocument, cache: &PreviewRenderCache) -> bool {
    cache.total_lines <= RENDER_CACHE_MAX_LINES
        && doc.content_excerpt.len() <= RENDER_CACHE_MAX_TEXT_BYTES
}

pub fn draw_preview(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    doc: &PreviewDocument,
    state: &mut SessionState,
    theme: &ThemeProfile,
) {
    frame.render_widget(Clear, area);
    let has_search = state.preview_search.is_some();
    let (block_area, search_bar_area) = if has_search {
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(1),
                ratatui::layout::Constraint::Min(0),
            ])
            .split(area);
        (chunks[1], chunks[0])
    } else {
        (area, ratatui::layout::Rect::default())
    };

    let title = preview_title_for_state(state);
    let total_lines = preview_total_lines(doc);
    let metadata_line = preview_border_bottom_line(
        state,
        total_lines,
        block_area.width.saturating_sub(2) as usize,
    );
    let block = Block::default()
        .title(
            Line::from(vec![Span::raw(" "), Span::raw(title), Span::raw(" ")])
                .alignment(Alignment::Right),
        )
        .title_bottom(Line::from(metadata_line).alignment(Alignment::Left))
        .borders(Borders::ALL);
    let inner = block.inner(block_area);
    frame.render_widget(block, block_area);
    render_bottom_diff_badge(frame, block_area, state, total_lines);
    if let Some(search) = &state.preview_search {
        render_search_bar(frame, search_bar_area, search);
    }

    state.preview_inner_rect = (inner.x, inner.y, inner.width, inner.height);

    let scroll_row_usize = state.preview_scroll_row;
    let scroll_col = if state.preview_wrap_enabled {
        0u16
    } else {
        state.preview_scroll_col.min(u16::MAX as usize) as u16
    };
    let show_line_numbers = line_numbers_enabled(state, doc);
    let use_wrap = state.preview_wrap_enabled;
    let (content_hash, styled_lines_hash, line_changes_hash) =
        cache_signature(&doc.content_excerpt, &doc.styled_lines, &doc.line_changes);
    let cache_key = PreviewRenderCacheKey {
        epoch: state.preview_render_epoch,
        inner_width: inner.width,
        show_line_numbers,
        wrap_enabled: use_wrap,
        content_hash,
        styled_lines_hash,
        line_changes_hash,
    };
    let cache_matches = state
        .preview_render_cache
        .as_ref()
        .is_some_and(|cache| cache.key == cache_key);
    if !cache_matches {
        let built_cache = build_preview_render_cache(
            doc,
            state.preview_render_epoch,
            inner.width,
            show_line_numbers,
            use_wrap,
            state.preview_diff_mode,
        );
        state.preview_render_cache = should_cache_render(doc, &built_cache).then_some(built_cache);
    }
    let fallback_cache;
    let cache = if let Some(cache) = state.preview_render_cache.as_ref() {
        cache
    } else {
        fallback_cache = build_preview_render_cache(
            doc,
            state.preview_render_epoch,
            inner.width,
            show_line_numbers,
            use_wrap,
            state.preview_diff_mode,
        );
        &fallback_cache
    };

    state.preview_line_number_cols = cache.line_number_cols;
    let viewport_rows = inner.height as usize;
    let visible_start = scroll_row_usize.min(cache.rendered_lines.len());
    let visible_end = visible_start
        .saturating_add(viewport_rows)
        .min(cache.rendered_lines.len());
    let visible_lines = cache.rendered_lines[visible_start..visible_end].to_vec();
    let content_widget = Paragraph::new(Text::from(visible_lines)).scroll((0, scroll_col));
    let rendered_total_lines = cache.total_lines;
    let _ = theme;
    frame.render_widget(Clear, inner);
    frame.render_widget(content_widget, inner);
    paint_full_row_diff_background(
        frame,
        inner,
        &cache.rendered_row_changes,
        scroll_row_usize,
        cache.line_number_cols as u16,
    );
    render_scroll_indicator(
        frame,
        inner,
        rendered_total_lines,
        scroll_row_usize,
        &cache.rendered_row_changes,
        state.preview_search.as_ref(),
    );

    if let Some(sel) = &state.preview_selection {
        let sel_clone = sel.clone();
        apply_selection_highlight(
            frame,
            inner,
            &sel_clone,
            scroll_row_usize,
            state.preview_scroll_col,
            state.preview_line_number_cols,
        );
    }

    if let Some(search) = &state.preview_search {
        apply_search_highlights(
            frame,
            inner,
            search,
            scroll_row_usize,
            state.preview_scroll_col,
            state.preview_line_number_cols,
            &doc.content_excerpt.lines().collect::<Vec<&str>>(),
        );
    }

    let bold_inverted = Style::default()
        .fg(OVERLAY_FG)
        .bg(OVERLAY_BG)
        .add_modifier(Modifier::BOLD);

    if state.preview_copy_indicator {
        render_overlay_label(frame, inner, " Copy Completed ", bold_inverted);
    } else if state.preview_copying_indicator {
        render_overlay_label(frame, inner, " Copying... ", bold_inverted);
    } else if state.preview_diff_mode {
        if doc.line_changes.iter().all(|change| change.is_none()) {
            render_overlay_label(frame, inner, " No changes ", bold_inverted);
        }
    }
}
