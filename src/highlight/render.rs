use crate::app::state::{
    ContentType, PreviewFallbackReason, StyledPreviewLine, StyledPreviewSegment,
};
use crate::highlight::syntax::{HighlightContext, HIGHLIGHT_NAMES};
use ratatui::style::{Color, Modifier, Style};
use std::path::Path;
use tree_sitter_highlight::{Highlight, HighlightEvent, Highlighter};

#[derive(Debug, Clone)]
pub struct HighlightRenderResult {
    pub rendered_text: String,
    pub content_type: ContentType,
    pub language_id: Option<String>,
    pub styled_lines: Vec<StyledPreviewLine>,
    pub fallback_reason: Option<PreviewFallbackReason>,
}

fn style_for_capture(name: &str) -> Style {
    if name.starts_with("comment") {
        return Style::default()
            .fg(Color::Rgb(120, 150, 120))
            .add_modifier(Modifier::ITALIC);
    }
    if name.starts_with("keyword") {
        return Style::default()
            .fg(Color::Rgb(220, 150, 80))
            .add_modifier(Modifier::BOLD);
    }
    if name.starts_with("string") || name.starts_with("escape") {
        return Style::default().fg(Color::Rgb(140, 200, 130));
    }
    if name.starts_with("number") || name.starts_with("constant") {
        return Style::default().fg(Color::Rgb(120, 190, 210));
    }
    if name.starts_with("type") || name.starts_with("tag") || name.starts_with("attribute") {
        return Style::default().fg(Color::Rgb(120, 170, 230));
    }
    if name.starts_with("function") || name.starts_with("constructor") {
        return Style::default().fg(Color::Rgb(220, 200, 120));
    }
    if name.starts_with("variable.parameter") || name.starts_with("property") {
        return Style::default().fg(Color::Rgb(210, 170, 230));
    }
    if name.starts_with("text.title") {
        return Style::default()
            .fg(Color::Rgb(110, 170, 240))
            .add_modifier(Modifier::BOLD);
    }
    if name.starts_with("text.literal") {
        return Style::default().fg(Color::Rgb(140, 200, 130));
    }
    if name.starts_with("text.emphasis") {
        return Style::default().add_modifier(Modifier::ITALIC);
    }
    if name.starts_with("text.strong") {
        return Style::default().add_modifier(Modifier::BOLD);
    }
    if name.starts_with("text.reference") || name.starts_with("text.uri") {
        return Style::default()
            .fg(Color::Rgb(130, 180, 230))
            .add_modifier(Modifier::UNDERLINED);
    }
    Style::default()
}

fn current_style(active: &[usize]) -> Style {
    if let Some(idx) = active.last().copied() {
        if let Some(name) = HIGHLIGHT_NAMES.get(idx) {
            return style_for_capture(name);
        }
    }
    Style::default()
}

fn push_text(styled_lines: &mut Vec<StyledPreviewLine>, text: &str, style: Style) {
    if styled_lines.is_empty() {
        styled_lines.push(Vec::new());
    }

    for chunk in text.split_inclusive('\n') {
        let (segment, ends_newline) = if let Some(trimmed) = chunk.strip_suffix('\n') {
            (trimmed, true)
        } else {
            (chunk, false)
        };

        if !segment.is_empty() {
            styled_lines
                .last_mut()
                .expect("line exists")
                .push(StyledPreviewSegment {
                    text: segment.to_string(),
                    style,
                });
        }

        if ends_newline {
            styled_lines.push(Vec::new());
        }
    }
}

pub fn render_with_highlight(
    ctx: &HighlightContext,
    path: &Path,
    content: &str,
) -> HighlightRenderResult {
    let Some(target) = ctx.target_for_path(path) else {
        return HighlightRenderResult {
            rendered_text: content.to_string(),
            content_type: ContentType::PlainText,
            language_id: None,
            styled_lines: Vec::new(),
            fallback_reason: Some(PreviewFallbackReason::UnsupportedExtension),
        };
    };

    let mut highlighter = Highlighter::new();
    let Ok(events) = highlighter.highlight(target.config, content.as_bytes(), None, |injection| {
        ctx.injection_config(injection)
    }) else {
        return HighlightRenderResult {
            rendered_text: content.to_string(),
            content_type: ContentType::PlainText,
            language_id: None,
            styled_lines: Vec::new(),
            fallback_reason: Some(PreviewFallbackReason::EngineFailure),
        };
    };

    let mut styled_lines = vec![Vec::new()];
    let mut active = Vec::<usize>::new();

    for event in events {
        let Ok(event) = event else {
            return HighlightRenderResult {
                rendered_text: content.to_string(),
                content_type: ContentType::PlainText,
                language_id: None,
                styled_lines: Vec::new(),
                fallback_reason: Some(PreviewFallbackReason::EngineFailure),
            };
        };

        match event {
            HighlightEvent::HighlightStart(Highlight(index)) => active.push(index),
            HighlightEvent::HighlightEnd => {
                active.pop();
            }
            HighlightEvent::Source { start, end } => {
                if end > start {
                    let segment = String::from_utf8_lossy(&content.as_bytes()[start..end]);
                    push_text(&mut styled_lines, &segment, current_style(&active));
                }
            }
        }
    }

    HighlightRenderResult {
        rendered_text: content.to_string(),
        content_type: ContentType::Highlighted,
        language_id: Some(target.language_id.to_string()),
        styled_lines,
        fallback_reason: None,
    }
}
