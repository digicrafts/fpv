use crate::app::state::{
    ContentType, LoadState, PreviewDocument, PreviewFallbackReason, StyledPreviewLine,
    StyledPreviewSegment,
};
use crate::highlight::render::{render_with_highlight, HighlightRenderResult};
use crate::highlight::syntax::HighlightContext;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat};
use ratatui::style::{Color, Style};
use std::fs;
use std::path::{Path, PathBuf};

const BINARY_SAMPLE: usize = 1024;
const HIGHLIGHT_MAX_BYTES: usize = 256 * 1024;
const ASCII_IMAGE_ASPECT_RATIO: f32 = 0.5;
const IMAGE_PREVIEW_MAX_WIDTH: u32 = 60;
const IMAGE_PREVIEW_MAX_HEIGHT: u32 = 30;
const ASCII_IMAGE_RAMP: [char; 4] = ['█', '▓', '▒', '░'];

fn is_probably_text(bytes: &[u8]) -> bool {
    !bytes.iter().take(BINARY_SAMPLE).any(|b| *b == 0)
}

fn normalize_line_endings(text: &str) -> String {
    if !text.contains('\r') {
        return text.to_string();
    }
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn sanitize_terminal_control_chars(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\n' => out.push('\n'),
            '\t' => out.push_str("    "),
            _ if ch.is_control() => out.push(' '),
            _ => out.push(ch),
        }
    }
    out
}

fn is_supported_image_extension(path: &Path) -> bool {
    let Some(ext) = path.extension() else {
        return false;
    };
    matches!(
        ext.to_string_lossy().to_ascii_lowercase().as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "bmp"
            | "webp"
            | "tif"
            | "tiff"
            | "ico"
            | "pnm"
            | "ppm"
            | "pgm"
            | "pbm"
            | "pam"
            | "tga"
    )
}

pub fn is_supported_image_path(path: &Path) -> bool {
    is_supported_image_extension(path)
}

fn is_supported_image_format(format: ImageFormat) -> bool {
    matches!(
        format,
        ImageFormat::Png
            | ImageFormat::Jpeg
            | ImageFormat::Gif
            | ImageFormat::Bmp
            | ImageFormat::Ico
            | ImageFormat::Tiff
            | ImageFormat::WebP
            | ImageFormat::Pnm
            | ImageFormat::Tga
    )
}

fn try_render_image_ascii_preview(
    path: &Path,
    data: &[u8],
) -> Option<(String, Vec<StyledPreviewLine>)> {
    try_render_image_ascii_preview_with_width(path, data, None)
}

fn try_render_image_ascii_preview_with_width(
    path: &Path,
    data: &[u8],
    target_width: Option<u16>,
) -> Option<(String, Vec<StyledPreviewLine>)> {
    let guessed_format = image::guess_format(data).ok();
    let should_try_decode = is_supported_image_extension(path)
        || guessed_format
            .map(is_supported_image_format)
            .unwrap_or(false);
    if !should_try_decode {
        return None;
    }

    let image = match guessed_format.filter(|fmt| is_supported_image_format(*fmt)) {
        Some(format) => image::load_from_memory_with_format(data, format).ok()?,
        None => image::load_from_memory(data).ok()?,
    };
    Some(render_image_ascii_preview(&image, target_width))
}

fn render_image_ascii_preview(
    image: &DynamicImage,
    target_width: Option<u16>,
) -> (String, Vec<StyledPreviewLine>) {
    let source = image.to_rgba8();
    let width = source.width().max(1);
    let height = source.height().max(1);
    let mut target_width = target_width
        .map(u32::from)
        .unwrap_or(width.min(IMAGE_PREVIEW_MAX_WIDTH))
        .min(IMAGE_PREVIEW_MAX_WIDTH)
        .max(1);
    let mut scaled_height =
        (height as f32 / width as f32) * target_width as f32 * ASCII_IMAGE_ASPECT_RATIO;
    if scaled_height > IMAGE_PREVIEW_MAX_HEIGHT as f32 {
        let width_for_height = (IMAGE_PREVIEW_MAX_HEIGHT as f32 * width as f32)
            / (height as f32 * ASCII_IMAGE_ASPECT_RATIO);
        target_width = width_for_height
            .floor()
            .max(1.0)
            .min(IMAGE_PREVIEW_MAX_WIDTH as f32) as u32;
        scaled_height =
            (height as f32 / width as f32) * target_width as f32 * ASCII_IMAGE_ASPECT_RATIO;
    }
    let target_height = scaled_height
        .round()
        .max(1.0)
        .min(IMAGE_PREVIEW_MAX_HEIGHT as f32) as u32;
    let resized =
        image::imageops::resize(&source, target_width, target_height, FilterType::Triangle);

    let mut output = String::with_capacity(((target_width + 1) * target_height) as usize);
    let mut styled_lines = Vec::with_capacity(target_height as usize);
    let ramp_max = (ASCII_IMAGE_RAMP.len().saturating_sub(1)) as f32;
    for y in 0..target_height {
        let mut styled_line = Vec::with_capacity(target_width as usize);
        for x in 0..target_width {
            let pixel = resized.get_pixel(x, y).0;
            let alpha = pixel[3] as f32 / 255.0;
            let luma =
                0.2126 * pixel[0] as f32 + 0.7152 * pixel[1] as f32 + 0.0722 * pixel[2] as f32;
            let blended = luma * alpha + 255.0 * (1.0 - alpha);
            let ramp_index = ((blended / 255.0) * ramp_max).round() as usize;
            let ch = ASCII_IMAGE_RAMP[ramp_index];
            let (fg, bg) = enhanced_terminal_colors(pixel);
            output.push(ch);
            styled_line.push(StyledPreviewSegment {
                text: ch.to_string(),
                style: Style::default().fg(fg).bg(bg),
            });
        }
        styled_lines.push(styled_line);
        if y + 1 < target_height {
            output.push('\n');
        }
    }

    (output, styled_lines)
}

fn enhanced_terminal_colors(pixel: [u8; 4]) -> (Color, Color) {
    let alpha = pixel[3] as f32 / 255.0;
    let base = [
        (pixel[0] as f32 * alpha).round() as u8,
        (pixel[1] as f32 * alpha).round() as u8,
        (pixel[2] as f32 * alpha).round() as u8,
    ];
    let bg = saturate_and_scale(base, 1.08, 0.88);
    let fg = saturate_and_scale(base, 1.18, 1.12);
    (
        Color::Rgb(fg[0], fg[1], fg[2]),
        Color::Rgb(bg[0], bg[1], bg[2]),
    )
}

fn saturate_and_scale(rgb: [u8; 3], saturation_boost: f32, value_scale: f32) -> [u8; 3] {
    let [r, g, b] = rgb.map(|channel| channel as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let mut hue = 0.0;
    if delta > 0.0 {
        hue = if max == r {
            ((g - b) / delta).rem_euclid(6.0)
        } else if max == g {
            ((b - r) / delta) + 2.0
        } else {
            ((r - g) / delta) + 4.0
        };
        hue *= 60.0;
    }

    let saturation = if max == 0.0 { 0.0 } else { delta / max };
    let value = max;
    hsv_to_rgb(
        hue,
        (saturation * saturation_boost).clamp(0.0, 1.0),
        (value * value_scale).clamp(0.0, 1.0),
    )
}

fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> [u8; 3] {
    if saturation == 0.0 {
        let gray = (value * 255.0).round() as u8;
        return [gray, gray, gray];
    }

    let sector = (hue / 60.0).floor().rem_euclid(6.0);
    let fraction = (hue / 60.0) - sector;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - fraction * saturation);
    let t = value * (1.0 - (1.0 - fraction) * saturation);

    let (r, g, b) = match sector as i32 {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };

    [
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    ]
}

pub fn render_image_preview_for_width(
    path: &Path,
    target_width: u16,
) -> Option<(String, Vec<StyledPreviewLine>)> {
    let data = fs::read(path).ok()?;
    try_render_image_ascii_preview_with_width(path, &data, Some(target_width))
}

pub fn load_preview(path: &Path, max_bytes: usize, ctx: &HighlightContext) -> PreviewDocument {
    let mut doc = PreviewDocument {
        source_path: PathBuf::from(path),
        load_state: LoadState::Loading,
        content_type: ContentType::PlainText,
        image_preview: false,
        image_preview_pending: false,
        language_id: None,
        content_excerpt: String::new(),
        styled_lines: Vec::new(),
        display_line_numbers: Vec::new(),
        line_changes: Vec::new(),
        fallback_reason: None,
        truncated: false,
        error_message: None,
    };

    let Ok(data) = fs::read(path) else {
        doc.load_state = LoadState::Error;
        doc.error_message = Some("Cannot read file (permission denied or missing).".to_string());
        return doc;
    };

    if let Some((ascii_preview, styled_lines)) = try_render_image_ascii_preview(path, &data) {
        doc.load_state = LoadState::Ready;
        doc.content_type = ContentType::PlainText;
        doc.image_preview = true;
        doc.image_preview_pending = false;
        doc.content_excerpt = ascii_preview;
        doc.styled_lines = styled_lines;
        return doc;
    }

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
    let normalized_content = normalize_line_endings(&content);
    let safe_content = sanitize_terminal_control_chars(&normalized_content);
    let rendered = if decode_uncertain {
        HighlightRenderResult {
            rendered_text: safe_content.clone(),
            content_type: ContentType::PlainText,
            language_id: None,
            styled_lines: Vec::new(),
            fallback_reason: Some(PreviewFallbackReason::DecodeUncertain),
        }
    } else if clip.len() > HIGHLIGHT_MAX_BYTES {
        HighlightRenderResult {
            rendered_text: safe_content.clone(),
            content_type: ContentType::PlainText,
            language_id: None,
            styled_lines: Vec::new(),
            fallback_reason: Some(PreviewFallbackReason::TooLarge),
        }
    } else {
        render_with_highlight(ctx, path, &safe_content)
    };

    doc.load_state = LoadState::Ready;
    doc.content_type = rendered.content_type;
    doc.image_preview = false;
    doc.image_preview_pending = false;
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

#[cfg(test)]
mod tests {
    use super::{normalize_line_endings, sanitize_terminal_control_chars};

    #[test]
    fn normalize_line_endings_rewrites_crlf_and_cr() {
        let input = "a\r\nb\rc\n";
        let output = normalize_line_endings(input);
        assert_eq!(output, "a\nb\nc\n");
    }

    #[test]
    fn sanitize_terminal_control_chars_strips_ansi_controls() {
        let input = "ok\x1b[31mred\x1b[0m\tend\n";
        let output = sanitize_terminal_control_chars(input);
        assert_eq!(output, "ok [31mred [0m    end\n");
        assert!(!output.contains('\x1b'));
    }
}
