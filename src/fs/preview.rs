use crate::app::state::{ContentType, LoadState, PreviewDocument, PreviewFallbackReason};
use crate::highlight::render::{render_with_highlight, HighlightRenderResult};
use crate::highlight::syntax::HighlightContext;
use std::fs;
use std::path::{Path, PathBuf};

const BINARY_SAMPLE: usize = 1024;
const HIGHLIGHT_MAX_BYTES: usize = 256 * 1024;

fn is_probably_text(bytes: &[u8]) -> bool {
    !bytes.iter().take(BINARY_SAMPLE).any(|b| *b == 0)
}

pub fn load_preview(path: &Path, max_bytes: usize, ctx: &HighlightContext) -> PreviewDocument {
    let mut doc = PreviewDocument {
        source_path: PathBuf::from(path),
        load_state: LoadState::Loading,
        content_type: ContentType::PlainText,
        language_id: None,
        content_excerpt: String::new(),
        styled_lines: Vec::new(),
        fallback_reason: None,
        truncated: false,
        error_message: None,
    };

    let Ok(data) = fs::read(path) else {
        doc.load_state = LoadState::Error;
        doc.error_message = Some("Cannot read file (permission denied or missing).".to_string());
        return doc;
    };

    if !is_probably_text(&data) {
        doc.load_state = LoadState::Binary;
        doc.content_type = ContentType::Unsupported;
        doc.error_message = Some("Binary file preview is not supported.".to_string());
        return doc;
    }

    let truncated = data.len() > max_bytes;
    let clip = if truncated { &data[..max_bytes] } else { &data };
    let (content, decode_uncertain) = match std::str::from_utf8(clip) {
        Ok(s) => (s.to_string(), false),
        Err(_) => (String::from_utf8_lossy(clip).into_owned(), true),
    };
    let rendered = if decode_uncertain {
        HighlightRenderResult {
            rendered_text: content.clone(),
            content_type: ContentType::PlainText,
            language_id: None,
            styled_lines: Vec::new(),
            fallback_reason: Some(PreviewFallbackReason::DecodeUncertain),
        }
    } else if clip.len() > HIGHLIGHT_MAX_BYTES {
        HighlightRenderResult {
            rendered_text: content.clone(),
            content_type: ContentType::PlainText,
            language_id: None,
            styled_lines: Vec::new(),
            fallback_reason: Some(PreviewFallbackReason::TooLarge),
        }
    } else {
        render_with_highlight(ctx, path, &content)
    };

    doc.load_state = LoadState::Ready;
    doc.content_type = rendered.content_type;
    doc.language_id = rendered.language_id;
    doc.styled_lines = rendered.styled_lines;
    doc.fallback_reason = rendered.fallback_reason;
    doc.content_excerpt = if truncated {
        format!("{}\n\n[truncated]", rendered.rendered_text)
    } else {
        rendered.rendered_text
    };
    doc.truncated = truncated;
    doc
}
