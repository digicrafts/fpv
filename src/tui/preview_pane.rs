use crate::app::state::{
    ContentType, LoadState, PreviewDocument, PreviewFallbackReason, PreviewLineChange,
    PreviewRenderCache, PreviewRenderCacheKey, PreviewSelection, SessionState,
};
use crate::config::load::ThemeProfile;
use crate::tui::status_bar::compose_preview_metadata_line;
use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthChar;

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

fn plain_text_for_doc(doc: &PreviewDocument) -> String {
    match doc.load_state {
        LoadState::Error | LoadState::Binary => doc
            .error_message
            .clone()
            .unwrap_or_else(|| "Unable to render preview".to_string()),
        _ => {
            let mut content = doc.content_excerpt.clone();
            if matches!(doc.content_type, ContentType::PlainText) {
                if let Some(reason) = &doc.fallback_reason {
                    let reason_text = match reason {
                        PreviewFallbackReason::UnsupportedExtension => "unsupported-extension",
                        PreviewFallbackReason::EngineFailure => "highlight-failed",
                        PreviewFallbackReason::TooLarge => "large-file-guard",
                        PreviewFallbackReason::DecodeUncertain => "decode-uncertain",
                    };
                    content = format!("[plain-text fallback: {reason_text}]\n{content}");
                }
            }
            content
        }
    }
}

fn line_number_width(total_lines: usize) -> usize {
    total_lines.max(1).to_string().len().max(2)
}

fn line_number_style() -> Style {
    Style::default().fg(Color::Gray)
}

fn added_diff_background() -> Color {
    Color::Rgb(0, 70, 0)
}

fn deleted_diff_background() -> Color {
    Color::Rgb(90, 0, 0)
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

fn diff_marker_span(kind: Option<DiffMarkerKind>) -> Span<'static> {
    match kind {
        Some(DiffMarkerKind::Added) => Span::styled(
            "+",
            Style::default()
                .fg(Color::LightGreen)
                .bg(added_diff_background())
                .add_modifier(Modifier::BOLD),
        ),
        Some(DiffMarkerKind::Deleted) => Span::styled(
            "-",
            Style::default()
                .fg(Color::LightRed)
                .bg(deleted_diff_background())
                .add_modifier(Modifier::DIM),
        ),
        None => Span::raw(" "),
    }
}

fn diff_fill_span(kind: Option<DiffMarkerKind>, width: usize) -> Span<'static> {
    let text = " ".repeat(width);
    match kind {
        Some(DiffMarkerKind::Added) => {
            Span::styled(text, Style::default().bg(added_diff_background()))
        }
        Some(DiffMarkerKind::Deleted) => {
            Span::styled(text, Style::default().bg(deleted_diff_background()))
        }
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
            .fg(Color::LightGreen)
            .bg(added_diff_background())
            .add_modifier(Modifier::BOLD),
        Some(DiffMarkerKind::Deleted) => Style::default()
            .fg(Color::LightRed)
            .bg(deleted_diff_background())
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
    if matches!(doc.content_type, ContentType::Highlighted) && !doc.styled_lines.is_empty() {
        return doc.styled_lines.len();
    }
    line_count(&plain_text_for_doc(doc))
}

pub fn preview_max_line_width(doc: &PreviewDocument) -> usize {
    if matches!(doc.content_type, ContentType::Highlighted) && !doc.styled_lines.is_empty() {
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
    let lines: Vec<String> =
        if matches!(doc.content_type, ContentType::Highlighted) && !doc.styled_lines.is_empty() {
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
                let cell = buf.get_mut(screen_x, screen_y);
                let s = cell
                    .style()
                    .bg(Color::LightBlue)
                    .fg(Color::Black)
                    .remove_modifier(Modifier::DIM);
                cell.set_style(s);
            }
        }
    }
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

fn render_border_label_top_left(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    label: &str,
    style: Style,
) {
    let label_width = label.len() as u16;
    if area.width <= 2 || area.height == 0 || label_width > area.width.saturating_sub(2) {
        return;
    }

    let widget_area = ratatui::layout::Rect::new(area.x + 1, area.y, label_width, 1);
    let widget = Paragraph::new(Span::styled(label, style));
    frame.render_widget(widget, widget_area);
}

fn is_unsupported_preview(doc: &PreviewDocument) -> bool {
    matches!(doc.content_type, ContentType::Unsupported)
        || matches!(doc.load_state, LoadState::Binary)
        || doc.fallback_reason.is_some()
        || (matches!(doc.content_type, ContentType::PlainText) && doc.language_id.is_none())
}

fn line_numbers_enabled(state: &SessionState, doc: &PreviewDocument) -> bool {
    state.preview_show_line_numbers && !is_unsupported_preview(doc)
}

fn render_scroll_indicator(
    frame: &mut Frame<'_>,
    inner: ratatui::layout::Rect,
    total_lines: usize,
    scroll_row: usize,
    rendered_row_changes: &[Option<PreviewLineChange>],
) {
    if inner.width == 0 || inner.height == 0 {
        return;
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

    let track_style = Style::default().fg(Color::DarkGray);
    let thumb_style = Style::default().fg(Color::Gray);
    let mut change_markers = vec![None; indicator_height];
    if total_lines > 0 {
        let max_row_index = total_lines.saturating_sub(1).max(1);
        let max_indicator_index = indicator_height.saturating_sub(1);
        for (row_index, change) in rendered_row_changes.iter().copied().enumerate() {
            let Some(change) = change else {
                continue;
            };
            let indicator_row = row_index.saturating_mul(max_indicator_index) / max_row_index;
            let slot = &mut change_markers[indicator_row];
            *slot = match (*slot, change) {
                (None, change) => Some(change),
                (Some(existing), PreviewLineChange::Added)
                    if existing != PreviewLineChange::Added =>
                {
                    Some(existing)
                }
                (Some(_), PreviewLineChange::Deleted) => Some(PreviewLineChange::Deleted),
                (Some(existing), _) => Some(existing),
            };
        }
    }
    let mut lines = Vec::with_capacity(indicator_height);
    for row in 0..indicator_height {
        let is_thumb = row >= thumb_top && row < thumb_top + thumb_height;
        let marker_change = change_markers[row];
        let (ch, style) = if is_thumb {
            let style = match marker_change {
                Some(PreviewLineChange::Added) => thumb_style.bg(added_diff_background()),
                Some(PreviewLineChange::Deleted) => thumb_style.bg(deleted_diff_background()),
                None => thumb_style,
            };
            ("█", style)
        } else {
            match marker_change {
                Some(PreviewLineChange::Added) => ("•", Style::default().fg(Color::LightGreen)),
                Some(PreviewLineChange::Deleted) => ("•", Style::default().fg(Color::LightRed)),
                None => ("│", track_style),
            }
        };
        lines.push(Line::from(Span::styled(ch, style)));
    }

    let indicator_x = inner.x + inner.width.saturating_sub(1);
    let indicator_area = ratatui::layout::Rect {
        x: indicator_x,
        y: inner.y,
        width: 1,
        height: inner.height,
    };
    frame.render_widget(Paragraph::new(Text::from(lines)), indicator_area);
}

fn diff_background_for_change(change: Option<PreviewLineChange>) -> Option<Color> {
    match change {
        Some(PreviewLineChange::Added) => Some(added_diff_background()),
        Some(PreviewLineChange::Deleted) => Some(deleted_diff_background()),
        None => None,
    }
}

fn paint_full_row_diff_background(
    frame: &mut Frame<'_>,
    inner: ratatui::layout::Rect,
    rendered_row_changes: &[Option<PreviewLineChange>],
    scroll_row: usize,
) {
    if inner.width == 0 || inner.height == 0 {
        return;
    }

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

        for screen_x in inner.x..inner.x.saturating_add(inner.width) {
            let cell = buf.get_mut(screen_x, inner.y + screen_y);
            cell.set_style(cell.style().bg(bg));
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

    let (rendered_lines, rendered_row_changes, line_number_cols) = if show_line_numbers {
        if matches!(doc.content_type, ContentType::Highlighted) && !doc.styled_lines.is_empty() {
            let line_number_cols = displayed_line_number_width(doc);
            let content_width = inner_width
                .saturating_sub((line_number_cols + 3) as u16)
                .max(1) as usize;
            let mut lines = Vec::new();
            let mut row_changes = Vec::new();

            for (index, styled_line) in doc.styled_lines.iter().enumerate() {
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
    } else if matches!(doc.content_type, ContentType::Highlighted) && !doc.styled_lines.is_empty() {
        let line_number_cols = if show_diff_markers { 2 } else { 0 };
        let content_width = inner_width.max(1) as usize;
        let mut lines = Vec::new();
        let mut row_changes = Vec::new();

        for (index, styled_line) in doc.styled_lines.iter().enumerate() {
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

    PreviewRenderCache {
        key: PreviewRenderCacheKey {
            epoch,
            inner_width,
            show_line_numbers,
            wrap_enabled: use_wrap,
            content_ptr: doc.content_excerpt.as_ptr() as usize,
            styled_lines_ptr: doc.styled_lines.as_ptr() as usize,
            line_changes_ptr: doc.line_changes.as_ptr() as usize,
        },
        total_lines: rendered_lines.len().max(1),
        rendered_lines,
        rendered_row_changes,
        line_number_cols,
    }
}

pub fn draw_preview(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    doc: &PreviewDocument,
    state: &mut SessionState,
    theme: &ThemeProfile,
) {
    frame.render_widget(Clear, area);
    let title = preview_title_for_state(state);
    let metadata_line =
        preview_border_metadata_for_state(state, area.width.saturating_sub(2) as usize);
    let block = Block::default()
        .title(
            Line::from(vec![Span::raw(" "), Span::raw(title), Span::raw(" ")])
                .alignment(Alignment::Right),
        )
        .title_bottom(Line::from(metadata_line).alignment(Alignment::Right))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    state.preview_inner_rect = (inner.x, inner.y, inner.width, inner.height);

    let scroll_row_usize = state.preview_scroll_row;
    let scroll_col = if state.preview_wrap_enabled {
        0u16
    } else {
        state.preview_scroll_col.min(u16::MAX as usize) as u16
    };
    let show_line_numbers = line_numbers_enabled(state, doc);
    let use_wrap = state.preview_wrap_enabled;
    let cache_key = PreviewRenderCacheKey {
        epoch: state.preview_render_epoch,
        inner_width: inner.width,
        show_line_numbers,
        wrap_enabled: use_wrap,
        content_ptr: doc.content_excerpt.as_ptr() as usize,
        styled_lines_ptr: doc.styled_lines.as_ptr() as usize,
        line_changes_ptr: doc.line_changes.as_ptr() as usize,
    };
    let cache_matches = state
        .preview_render_cache
        .as_ref()
        .is_some_and(|cache| cache.key == cache_key);
    if !cache_matches {
        state.preview_render_cache = Some(build_preview_render_cache(
            doc,
            state.preview_render_epoch,
            inner.width,
            show_line_numbers,
            use_wrap,
            state.preview_diff_mode,
        ));
    }
    let cache = state
        .preview_render_cache
        .as_ref()
        .expect("preview render cache initialized");

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
    paint_full_row_diff_background(frame, inner, &cache.rendered_row_changes, scroll_row_usize);
    render_scroll_indicator(
        frame,
        inner,
        rendered_total_lines,
        scroll_row_usize,
        &cache.rendered_row_changes,
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

    let bold_inverted = Style::default()
        .fg(Color::Black)
        .bg(Color::White)
        .add_modifier(Modifier::BOLD);

    if state.preview_copy_indicator {
        render_overlay_label(frame, inner, " Copy Completed ", bold_inverted);
    } else if state.preview_copying_indicator {
        render_overlay_label(frame, inner, " Copying... ", bold_inverted);
    } else if state.preview_diff_mode {
        let diff_style = Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        render_border_label_top_left(frame, area, " DIFF ", diff_style);
    }
}
