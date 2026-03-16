use crate::app::state::{
    ContentType, LoadState, PreviewDocument, PreviewFallbackReason, StyledPreviewLine,
    StyledPreviewSegment,
};
use crate::highlight::render::{render_with_highlight, HighlightRenderResult};
use crate::highlight::syntax::HighlightContext;
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageFormat};
#[cfg(not(feature = "turbojpeg-fastpath"))]
use jpeg_decoder::PixelFormat as JpegPixelFormat;
use ratatui::style::{Color, Style};
use std::fs;
use std::io::Cursor;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
#[cfg(feature = "turbojpeg-fastpath")]
use turbojpeg::{self, PixelFormat as TurboPixelFormat, ScalingFactor};

const BINARY_SAMPLE: usize = 1024;
const HIGHLIGHT_MAX_BYTES: usize = 256 * 1024;
const TEXT_PREVIEW_READ_LIMIT: usize = 2 * 1024 * 1024;
const IMAGE_PREVIEW_READ_LIMIT: usize = 50 * 1024 * 1024;
const ASCII_IMAGE_ASPECT_RATIO: f32 = 0.5;
const IMAGE_PREVIEW_MAX_WIDTH: u32 = 60;
const IMAGE_PREVIEW_MAX_HEIGHT: u32 = 30;
const ASCII_IMAGE_RAMP: [char; 4] = ['█', '▓', '▒', '░'];
const IMAGE_PREVIEW_MAX_PIXELS: u64 = 200_000_000;

const IMAGE_PREVIEW_TOO_LARGE_MESSAGE: &str = "Image preview skipped for very large image.";

#[derive(Debug, Clone)]
pub struct ImagePreviewUpdate {
    pub path: PathBuf,
    pub preview: PreviewDocument,
}

pub struct ImagePreviewWorker {
    request_tx: Option<Sender<(PathBuf, u16)>>,
    update_rx: Receiver<ImagePreviewUpdate>,
    join_handle: Option<JoinHandle<()>>,
}

impl ImagePreviewWorker {
    pub fn spawn() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<(PathBuf, u16)>();
        let (update_tx, update_rx) = mpsc::channel::<ImagePreviewUpdate>();

        let join_handle = thread::spawn(move || {
            while let Ok(mut request) = request_rx.recv() {
                while let Ok(next_request) = request_rx.try_recv() {
                    request = next_request;
                }

                let mut steps = Vec::new();
                let _ = send_loading_update(&update_tx, request.0.as_path(), &steps);

                let request_path = request.0;
                let request_width = request.1;
                let mut emit_step = |step: &str| {
                    steps.push(step.to_string());
                    let _ = send_loading_update(&update_tx, request_path.as_path(), &steps);
                };
                let preview = load_image_preview_for_width_with_steps(
                    &request_path,
                    request_width,
                    &mut emit_step,
                );

                if update_tx
                    .send(ImagePreviewUpdate {
                        path: request_path,
                        preview,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });

        Self {
            request_tx: Some(request_tx),
            update_rx,
            join_handle: Some(join_handle),
        }
    }

    pub fn request(&self, path: PathBuf, target_width: u16) -> bool {
        self.request_tx
            .as_ref()
            .is_some_and(|tx| tx.send((path, target_width)).is_ok())
    }

    pub fn latest_update(&self) -> Option<ImagePreviewUpdate> {
        let mut latest = None;
        loop {
            match self.update_rx.try_recv() {
                Ok(update) => latest = Some(update),
                Err(TryRecvError::Empty) => return latest,
                Err(TryRecvError::Disconnected) => return latest,
            }
        }
    }
}

impl Drop for ImagePreviewWorker {
    fn drop(&mut self) {
        self.request_tx.take();
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

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

// fn try_render_image_ascii_preview_with_width(
//     path: &Path,
//     data: &[u8],
//     target_width: Option<u16>,
// ) -> Option<(String, Vec<StyledPreviewLine>)> {
//     if is_supported_image_path(path) {
//         let (width, height) = image_dimensions_for_preview(data).ok()?;
//         let pixel_count = u64::from(width).saturating_mul(u64::from(height));
//         if pixel_count > IMAGE_PREVIEW_MAX_PIXELS {
//             return None;
//         }
//     }

//     let guessed_format = image::guess_format(data).ok();
//     let should_try_decode = is_supported_image_extension(path)
//         || guessed_format
//             .map(is_supported_image_format)
//             .unwrap_or(false);
//     if !should_try_decode {
//         return None;
//     }

//     let image = match guessed_format.filter(|fmt| is_supported_image_format(*fmt)) {
//         Some(format) => image::load_from_memory_with_format(data, format).ok()?,
//         None => image::load_from_memory(data).ok()?,
//     };
//     Some(render_image_ascii_preview(&image, target_width))
// }

fn image_preview_loading_step(path: &Path) -> PreviewDocument {
    PreviewDocument {
        source_path: PathBuf::from(path),
        load_state: LoadState::Ready,
        content_type: ContentType::PlainText,
        image_preview: false,
        image_preview_pending: true,
        content_excerpt: "Loading image preview".to_string(),
        ..PreviewDocument::default()
    }
}

fn format_image_preview_loading_steps(steps: &[String]) -> String {
    steps.join("\n")
}

fn send_loading_update(update_tx: &Sender<ImagePreviewUpdate>, path: &Path, step: &[String]) {
    let mut preview = image_preview_loading_step(path);
    preview.content_excerpt = format_image_preview_loading_steps(step);
    let _ = update_tx.send(ImagePreviewUpdate {
        path: path.to_path_buf(),
        preview,
    });
}

fn format_step_duration(duration: Duration) -> String {
    format!("{:.2}s", duration.as_secs_f64())
}

fn emit_timed_step<F>(name: &str, started_at: Instant, on_step: &mut F)
where
    F: FnMut(&str),
{
    on_step(&format!(
        "{name} ({})",
        format_step_duration(started_at.elapsed())
    ));
}

fn compute_ascii_dimensions(width: u32, height: u32, target_width: Option<u16>) -> (u32, u32) {
    let width = width.max(1);
    let height = height.max(1);
    let mut tw = target_width
        .map(u32::from)
        .unwrap_or(width.min(IMAGE_PREVIEW_MAX_WIDTH))
        .min(IMAGE_PREVIEW_MAX_WIDTH)
        .max(1);
    let mut scaled_height = (height as f32 / width as f32) * tw as f32 * ASCII_IMAGE_ASPECT_RATIO;
    if scaled_height > IMAGE_PREVIEW_MAX_HEIGHT as f32 {
        let width_for_height = (IMAGE_PREVIEW_MAX_HEIGHT as f32 * width as f32)
            / (height as f32 * ASCII_IMAGE_ASPECT_RATIO);
        tw = width_for_height
            .floor()
            .max(1.0)
            .min(IMAGE_PREVIEW_MAX_WIDTH as f32) as u32;
        scaled_height = (height as f32 / width as f32) * tw as f32 * ASCII_IMAGE_ASPECT_RATIO;
    }
    let th = scaled_height
        .round()
        .max(1.0)
        .min(IMAGE_PREVIEW_MAX_HEIGHT as f32) as u32;
    (tw, th)
}

/// Pre-computed char representations to avoid repeated heap allocations.
const ASCII_RAMP_STRS: [&str; 4] = ["\u{2588}", "\u{2593}", "\u{2592}", "\u{2591}"];

fn render_image_ascii_preview(
    image: &DynamicImage,
    target_width: Option<u16>,
) -> (String, Vec<StyledPreviewLine>) {
    let (src_width, src_height) = (image.width().max(1), image.height().max(1));
    let (target_width, target_height) =
        compute_ascii_dimensions(src_width, src_height, target_width);

    // Resize directly from the DynamicImage — avoids a full-resolution to_rgba8() copy.
    let resized = image
        .resize_exact(target_width, target_height, FilterType::Nearest)
        .to_rgba8();

    let mut output = String::with_capacity(((target_width + 1) * target_height) as usize);
    let mut styled_lines = Vec::with_capacity(target_height as usize);
    let ramp_max = (ASCII_IMAGE_RAMP.len() - 1) as f32;

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
                text: ASCII_RAMP_STRS[ramp_index].to_string(),
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

#[cfg(not(feature = "turbojpeg-fastpath"))]
fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(data));
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    Some((u32::from(info.width), u32::from(info.height)))
}

#[cfg(not(feature = "turbojpeg-fastpath"))]
fn decode_jpeg_for_preview(data: &[u8], target_width: Option<u16>) -> Option<DynamicImage> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(data));
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    let source_width = u32::from(info.width);
    let source_height = u32::from(info.height);
    let (requested_width, requested_height) =
        compute_ascii_dimensions(source_width, source_height, target_width);

    // Ask the JPEG decoder for a smaller output to avoid full-resolution decode cost.
    let _ = decoder.scale(
        requested_width.min(u16::MAX as u32) as u16,
        requested_height.min(u16::MAX as u32) as u16,
    );
    let pixels = decoder.decode().ok()?;
    let decoded_info = decoder.info()?;
    let width = u32::from(decoded_info.width);
    let height = u32::from(decoded_info.height);

    match decoded_info.pixel_format {
        JpegPixelFormat::L8 => {
            let image = image::GrayImage::from_raw(width, height, pixels)?;
            Some(DynamicImage::ImageLuma8(image))
        }
        JpegPixelFormat::RGB24 => {
            let image = image::RgbImage::from_raw(width, height, pixels)?;
            Some(DynamicImage::ImageRgb8(image))
        }
        // Keep uncommon formats on the generic path for compatibility.
        _ => None,
    }
}

fn read_file_with_limit(path: &Path, limit: usize) -> std::io::Result<Vec<u8>> {
    let file = fs::File::open(path)?;
    let mut data = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut data)?;
    Ok(data)
}

fn read_text_preview_bytes(path: &Path, max_bytes: usize) -> std::io::Result<(Vec<u8>, bool)> {
    let limit = max_bytes.min(TEXT_PREVIEW_READ_LIMIT);
    let mut data = read_file_with_limit(path, limit)?;
    let truncated = data.len() > limit;
    if truncated {
        data.truncate(limit);
    }
    Ok((data, truncated))
}

fn read_image_preview_bytes(path: &Path) -> std::io::Result<Vec<u8>> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > IMAGE_PREVIEW_READ_LIMIT as u64 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Image preview skipped for large file.",
        ));
    }

    let data = read_file_with_limit(path, IMAGE_PREVIEW_READ_LIMIT)?;
    if data.len() > IMAGE_PREVIEW_READ_LIMIT {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Image preview skipped for large file.",
        ));
    }
    Ok(data)
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

// pub fn render_image_preview_for_width(
//     path: &Path,
//     target_width: u16,
// ) -> Option<(String, Vec<StyledPreviewLine>)> {
//     let data = read_image_preview_bytes(path).ok()?;
//     try_render_image_ascii_preview_with_width(path, &data, Some(target_width))
// }

pub fn load_image_preview_for_width(path: &Path, target_width: u16) -> PreviewDocument {
    load_image_preview_for_width_with_steps(path, target_width, |_| {})
}

fn load_image_preview_for_width_with_steps<F>(
    path: &Path,
    target_width: u16,
    mut on_step: F,
) -> PreviewDocument
where
    F: FnMut(&str),
{
    let step_started = Instant::now();
    // emit_timed_step("Load image", step_started, &mut on_step);
    let mut doc = PreviewDocument {
        source_path: PathBuf::from(path),
        load_state: LoadState::Loading,
        content_type: ContentType::PlainText,
        ..PreviewDocument::default()
    };

    let Ok(data) = read_image_preview_bytes(path) else {
        emit_timed_step("Load image", step_started, &mut on_step);
        doc.load_state = LoadState::Error;
        doc.error_message = Some("Cannot read image preview.".to_string());
        return doc;
    };
    emit_timed_step("Load image", step_started, &mut on_step);

    let step_started = Instant::now();

    // Guess the format once and reuse for both dimension check and decode.
    let guessed_format = image::guess_format(&data).ok();
    let should_try_decode = is_supported_image_extension(path)
        || guessed_format
            .map(is_supported_image_format)
            .unwrap_or(false);
    if !should_try_decode {
        emit_timed_step("Decode Image", step_started, &mut on_step);
        doc.load_state = LoadState::Error;
        doc.error_message = Some("Cannot decode image preview.".to_string());
        return doc;
    }

    let selected_format = guessed_format.filter(|fmt| is_supported_image_format(*fmt));
    if selected_format == Some(ImageFormat::Jpeg) {
        if let Some((w, h)) = jpeg_dimensions(&data) {
            if u64::from(w).saturating_mul(u64::from(h)) > IMAGE_PREVIEW_MAX_PIXELS {
                emit_timed_step("Decode Image", step_started, &mut on_step);
                doc.load_state = LoadState::Error;
                doc.error_message = Some(IMAGE_PREVIEW_TOO_LARGE_MESSAGE.to_string());
                return doc;
            }
        }
    }    

    // JPEG fast-path: format-specific decoder + scaled decode near preview size.
    let image = if selected_format == Some(ImageFormat::Jpeg) {
        decode_jpeg_for_preview(&data, Some(target_width)).or_else(|| {
            let decoder = image::ImageReader::with_format(Cursor::new(&data), ImageFormat::Jpeg)
                .into_decoder()
                .ok()?;
            if is_supported_image_path(path) {
                let (w, h) = decoder.dimensions();
                if u64::from(w).saturating_mul(u64::from(h)) > IMAGE_PREVIEW_MAX_PIXELS {
                    return None;
                }
            }
            DynamicImage::from_decoder(decoder).ok()
        })
    } else {
        // Generic path: build one decoder and reuse for dimensions + decode.
        let decoder = match selected_format {
            Some(format) => image::ImageReader::with_format(Cursor::new(&data), format)
                .into_decoder()
                .ok(),
            None => image::ImageReader::new(Cursor::new(&data))
                .with_guessed_format()
                .ok()
                .and_then(|reader| reader.into_decoder().ok()),
        };
        decoder.and_then(|decoder| {
            if is_supported_image_path(path) {
                let (w, h) = decoder.dimensions();
                if u64::from(w).saturating_mul(u64::from(h)) > IMAGE_PREVIEW_MAX_PIXELS {
                    return None;
                }
            }
            DynamicImage::from_decoder(decoder).ok()
        })
    };

    if image.is_none() && is_supported_image_path(path) && selected_format != Some(ImageFormat::Jpeg) {
        // Re-checking dimensions directly keeps the user-facing "too large" error accurate.
        let dim_result = match selected_format {
            Some(format) => image::ImageReader::with_format(Cursor::new(&data), format)
                .into_dimensions()
                .ok(),
            None => image::ImageReader::new(Cursor::new(&data))
                .with_guessed_format()
                .ok()
                .and_then(|reader| reader.into_dimensions().ok()),
        };
        if let Some((w, h)) = dim_result {
            if u64::from(w).saturating_mul(u64::from(h)) > IMAGE_PREVIEW_MAX_PIXELS {
                emit_timed_step("Decode Image", step_started, &mut on_step);
                doc.load_state = LoadState::Error;
                doc.error_message = Some(IMAGE_PREVIEW_TOO_LARGE_MESSAGE.to_string());
                return doc;
            }
        }
    }

    let image = match image {
        Some(image) => {
            // Drop the raw bytes immediately after decode to free memory before rendering.
            drop(data);
            image
        }
        None => {
            emit_timed_step("Decode Image", step_started, &mut on_step);
            doc.load_state = LoadState::Error;
            doc.error_message = Some("Cannot decode image preview.".to_string());
            return doc;
        }
    };
    emit_timed_step("Decode Image", step_started, &mut on_step);
    let step_started = Instant::now();
    let (ascii_preview, styled_lines) = render_image_ascii_preview(&image, Some(target_width));
    emit_timed_step("Draw Image", step_started, &mut on_step);

    doc.load_state = LoadState::Ready;
    doc.content_type = ContentType::PlainText;
    doc.image_preview = true;
    doc.image_preview_pending = false;
    doc.content_excerpt = ascii_preview;
    doc.styled_lines = styled_lines;
    doc
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

    if is_supported_image_path(path) {
        return load_image_preview_for_width(path, IMAGE_PREVIEW_MAX_WIDTH as u16);
    }

    let Ok((data, truncated)) = read_text_preview_bytes(path, max_bytes) else {
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

    let (content, decode_uncertain) = match std::str::from_utf8(&data) {
        Ok(s) => (s.to_string(), false),
        Err(_) => (String::from_utf8_lossy(&data).into_owned(), true),
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
    } else if data.len() > HIGHLIGHT_MAX_BYTES {
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
