use crate::app::state::{
    ContentType, LoadState, PreviewDocument, PreviewFallbackReason, SessionState,
};
use crate::config::load::ThemeProfile;
use crate::tui::status_bar::compose_preview_metadata_line;
use ratatui::layout::Alignment;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
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
    Style::default().fg(Color::White).bg(Color::DarkGray)
}

fn line_number_prefix(line_number: usize, total_lines: usize) -> Span<'static> {
    let width = line_number_width(total_lines);
    Span::styled(
        format!("{line_number:>width$}", width = width),
        line_number_style(),
    )
}

fn line_number_blank_prefix(total_lines: usize) -> Span<'static> {
    let width = line_number_width(total_lines);
    Span::raw(format!("{:width$}", "", width = width))
}

pub fn preview_total_lines(doc: &PreviewDocument) -> usize {
    if matches!(doc.content_type, ContentType::Highlighted) && !doc.styled_lines.is_empty() {
        return doc.styled_lines.len();
    }
    line_count(&plain_text_for_doc(doc))
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
    let mut lines = Vec::with_capacity(indicator_height);
    for row in 0..indicator_height {
        let is_thumb = row >= thumb_top && row < thumb_top + thumb_height;
        let ch = if is_thumb { "█" } else { "│" };
        lines.push(Line::from(Span::styled(
            ch,
            if is_thumb { thumb_style } else { track_style },
        )));
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
    total_lines: usize,
    content_spans: Vec<Span<'_>>,
    content_width: usize,
) -> Vec<Line<'static>> {
    let wrapped = wrap_styled_spans(content_spans, content_width);
    let mut lines = Vec::with_capacity(wrapped.len());

    for (index, content_line) in wrapped.into_iter().enumerate() {
        let prefix = if index == 0 {
            line_number_prefix(line_number, total_lines)
        } else {
            line_number_blank_prefix(total_lines)
        };
        let mut spans = Vec::with_capacity(content_line.len() + 1);
        spans.push(prefix);
        spans.push(Span::raw(" "));
        spans.extend(content_line);
        lines.push(Line::from(spans));
    }
    lines
}

pub fn draw_preview(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    doc: &PreviewDocument,
    state: &SessionState,
    theme: &ThemeProfile,
) {
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

    let text = plain_text_for_doc(doc);
    let scroll_row_usize = state.preview_scroll_row;
    let scroll_row = scroll_row_usize.min(u16::MAX as usize) as u16;
    let show_line_numbers = line_numbers_enabled(state, doc);
    let use_wrap = state.preview_wrap_enabled;

    let (content_widget, rendered_total_lines) = if show_line_numbers {
        if matches!(doc.content_type, ContentType::Highlighted) && !doc.styled_lines.is_empty() {
            let total_lines = doc.styled_lines.len();
            let line_number_cols = line_number_width(total_lines) + 1;
            let content_width = inner.width.saturating_sub(line_number_cols as u16).max(1) as usize;
            let mut lines = Vec::new();

            for (index, styled_line) in doc.styled_lines.iter().enumerate() {
                let line_number = index + 1;
                let content_spans = styled_line
                    .iter()
                    .map(|segment| Span::styled(segment.text.clone(), segment.style))
                    .collect::<Vec<_>>();
                if use_wrap {
                    lines.extend(numbered_lines_with_wrapped_content(
                        line_number,
                        total_lines,
                        content_spans,
                        content_width,
                    ));
                } else {
                    lines.push(Line::from(
                        std::iter::once(line_number_prefix(line_number, total_lines))
                            .chain(std::iter::once(Span::raw(" ")))
                            .chain(content_spans)
                            .collect::<Vec<_>>(),
                    ));
                }
            }
            let rendered_total = lines.len();
            (
                Paragraph::new(Text::from(lines)).scroll((scroll_row, 0)),
                rendered_total,
            )
        } else {
            let rows = text.split('\n').collect::<Vec<_>>();
            let total_lines = rows.len().max(1);
            let line_number_cols = line_number_width(total_lines) + 1;
            let content_width = inner.width.saturating_sub(line_number_cols as u16).max(1) as usize;
            let mut lines = Vec::new();

            for (index, row) in rows.iter().enumerate() {
                let line_number = index + 1;
                let content_spans = vec![Span::raw((*row).to_string())];
                if use_wrap {
                    lines.extend(numbered_lines_with_wrapped_content(
                        line_number,
                        total_lines,
                        content_spans,
                        content_width,
                    ));
                } else {
                    lines.push(Line::from(vec![
                        line_number_prefix(line_number, total_lines),
                        Span::raw(" "),
                        Span::raw((*row).to_string()),
                    ]));
                }
            }
            let rendered_total = lines.len();
            (
                Paragraph::new(Text::from(lines)).scroll((scroll_row, 0)),
                rendered_total,
            )
        }
    } else if use_wrap {
        let mut rendered_total = 0usize;
        for row in text.split('\n') {
            rendered_total = rendered_total.saturating_add(
                wrap_styled_spans(
                    vec![Span::raw(row.to_string())],
                    inner.width.max(1) as usize,
                )
                .len(),
            );
        }
        (
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .scroll((scroll_row, 0)),
            rendered_total.max(1),
        )
    } else {
        (
            Paragraph::new(text).scroll((scroll_row, 0)),
            preview_total_lines(doc),
        )
    };
    let _ = theme;
    frame.render_widget(content_widget, inner);
    render_scroll_indicator(frame, inner, rendered_total_lines, scroll_row_usize);
}
