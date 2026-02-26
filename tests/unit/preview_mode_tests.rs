use fpv::app::preview_controller::refresh_preview;
use fpv::app::state::{
    ContentType, LoadState, NodeType, PreviewFallbackReason, SelectedEntryMetadata, SessionState,
    TreeNode,
};
use fpv::fs::preview::load_preview;
use fpv::highlight::syntax::HighlightContext;
use fpv::tui::preview_pane::preview_total_lines;
use fpv::tui::status_bar::compose_preview_metadata_line;
use ratatui::style::Style;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn has_non_default_style(doc: &fpv::app::state::PreviewDocument) -> bool {
    doc.styled_lines
        .iter()
        .flat_map(|line| line.iter())
        .any(|segment| segment.style != Style::default())
}

#[test]
fn preview_selects_highlight_for_known_extensions() {
    let d = tempdir().expect("create tempdir");
    let p = d.path().join("main.py");
    fs::write(&p, "print('ok')\n").expect("write file");
    let ctx = HighlightContext::new();
    let doc = load_preview(&p, 1024, &ctx);
    assert_eq!(doc.load_state, LoadState::Ready);
    assert_eq!(doc.content_type, ContentType::Highlighted);
    assert!(has_non_default_style(&doc));
}

#[test]
fn preview_selects_highlight_for_tsx_and_jsx() {
    let d = tempdir().expect("create tempdir");
    let tsx = d.path().join("app.tsx");
    let jsx = d.path().join("app.jsx");
    fs::write(&tsx, "export const App = () => <div />;\n").expect("write tsx");
    fs::write(&jsx, "export const App = () => <div />;\n").expect("write jsx");
    let ctx = HighlightContext::new();

    let tsx_doc = load_preview(&tsx, 1024, &ctx);
    assert_eq!(tsx_doc.load_state, LoadState::Ready);
    assert_eq!(tsx_doc.content_type, ContentType::Highlighted);
    assert!(has_non_default_style(&tsx_doc));

    let jsx_doc = load_preview(&jsx, 1024, &ctx);
    assert_eq!(jsx_doc.load_state, LoadState::Ready);
    assert_eq!(jsx_doc.content_type, ContentType::Highlighted);
    assert!(has_non_default_style(&jsx_doc));
}

#[test]
fn preview_selects_highlight_for_requested_common_file_types() {
    let d = tempdir().expect("create tempdir");
    let cases = [
        ("readme.md", "# heading\n\ntext\n"),
        (
            "index.html",
            "<!doctype html><html><body>hi</body></html>\n",
        ),
        ("data.json", "{\"a\": 1, \"b\": true}\n"),
        ("main.go", "package main\nfunc main() {}\n"),
        ("main.c", "int main(void) { return 0; }\n"),
    ];
    let ctx = HighlightContext::new();

    for (name, content) in cases {
        let path = d.path().join(name);
        fs::write(&path, content).expect("write sample");
        let doc = load_preview(&path, 1024, &ctx);
        assert_eq!(
            doc.load_state,
            LoadState::Ready,
            "unexpected state for {name}"
        );
        assert_eq!(
            doc.content_type,
            ContentType::Highlighted,
            "expected highlighting for {name}"
        );
    }
}

#[test]
fn markdown_preview_contains_non_default_styled_segments() {
    let d = tempdir().expect("create tempdir");
    let path = d.path().join("readme.md");
    fs::write(&path, "# Title\n\n* item\n").expect("write md");
    let ctx = HighlightContext::new();

    let doc = load_preview(&path, 1024, &ctx);
    assert_eq!(doc.load_state, LoadState::Ready);
    assert_eq!(doc.content_type, ContentType::Highlighted);
    assert!(has_non_default_style(&doc));
}

#[test]
fn markdown_fenced_rust_block_uses_rust_keyword_style() {
    let d = tempdir().expect("create tempdir");
    let path = d.path().join("readme.md");
    fs::write(
        &path,
        "# Title\n\n```rust\nfn main() {\n    let x = 1;\n}\n```\n",
    )
    .expect("write md");
    let ctx = HighlightContext::new();

    let doc = load_preview(&path, 1024, &ctx);
    assert_eq!(doc.load_state, LoadState::Ready);
    assert_eq!(doc.content_type, ContentType::Highlighted);

    let code_text = "fn main() {\n    let x = 1;\n}";
    let code_styles: std::collections::HashSet<String> = doc
        .styled_lines
        .iter()
        .flat_map(|line| line.iter())
        .filter(|segment| {
            let t = segment.text.trim();
            !t.is_empty() && code_text.contains(t)
        })
        .map(|segment| format!("{:?}", segment.style))
        .collect();
    assert!(code_styles.len() >= 2);
}

#[test]
fn preview_selects_highlight_for_additional_common_languages() {
    let d = tempdir().expect("create tempdir");
    let cases = [
        ("main.cpp", "int main() { return 0; }\n"),
        (
            "Main.java",
            "class Main { public static void main(String[] a) {} }\n",
        ),
        ("script.sh", "echo hi\n"),
        ("style.css", "body { color: red; }\n"),
        ("config.toml", "name = \"fpv\"\n"),
        ("mod.ts", "export const x: number = 1;\n"),
    ];
    let ctx = HighlightContext::new();

    for (name, content) in cases {
        let path = d.path().join(name);
        fs::write(&path, content).expect("write sample");
        let doc = load_preview(&path, 1024, &ctx);
        assert_eq!(
            doc.load_state,
            LoadState::Ready,
            "unexpected state for {name}"
        );
        assert_eq!(
            doc.content_type,
            ContentType::Highlighted,
            "expected highlighting for {name}"
        );
    }
}

#[test]
fn preview_falls_back_plain_text_for_unknown_extension() {
    let d = tempdir().expect("create tempdir");
    let p = d.path().join("notes.xyz");
    fs::write(&p, "hello\n").expect("write file");
    let ctx = HighlightContext::new();
    let doc = load_preview(&p, 1024, &ctx);
    assert_eq!(doc.load_state, LoadState::Ready);
    assert_eq!(doc.content_type, ContentType::PlainText);
    assert_eq!(
        doc.fallback_reason,
        Some(PreviewFallbackReason::UnsupportedExtension)
    );
}

#[test]
fn preview_marks_binary_files() {
    let d = tempdir().expect("create tempdir");
    let p = d.path().join("blob.bin");
    fs::write(&p, [0_u8, 1, 2, 3]).expect("write file");
    let ctx = HighlightContext::new();
    let doc = load_preview(&p, 1024, &ctx);
    assert_eq!(doc.load_state, LoadState::Binary);
}

#[test]
fn large_supported_file_uses_plain_text_guard() {
    let d = tempdir().expect("create tempdir");
    let p = d.path().join("large.rs");
    fs::write(&p, "fn x() {}\n".repeat(50_000)).expect("write file");
    let ctx = HighlightContext::new();
    let doc = load_preview(&p, 1024 * 1024, &ctx);
    assert_eq!(doc.load_state, LoadState::Ready);
    assert_eq!(doc.content_type, ContentType::PlainText);
    assert_eq!(doc.fallback_reason, Some(PreviewFallbackReason::TooLarge));
}

#[test]
fn decode_uncertain_bytes_fall_back_to_plain_text() {
    let d = tempdir().expect("create tempdir");
    let p = d.path().join("broken.py");
    fs::write(&p, [0x66_u8, 0x6f, 0x80, 0x6f, 0x0a]).expect("write file");
    let ctx = HighlightContext::new();
    let doc = load_preview(&p, 1024, &ctx);
    assert_eq!(doc.load_state, LoadState::Ready);
    assert_eq!(doc.content_type, ContentType::PlainText);
    assert_eq!(
        doc.fallback_reason,
        Some(PreviewFallbackReason::DecodeUncertain)
    );
}

#[test]
fn directory_selection_uses_neutral_preview() {
    let d = tempdir().expect("create tempdir");
    let dir_path = d.path().join("folder");
    fs::create_dir_all(&dir_path).expect("mkdir");

    let nodes = vec![TreeNode {
        path: dir_path.clone(),
        name: "folder".to_string(),
        node_type: NodeType::Directory,
        depth: 0,
        expanded: false,
        readable: true,
        children_loaded: false,
    }];

    let mut state = SessionState::new(PathBuf::from("."));
    let ctx = HighlightContext::new();
    let doc = refresh_preview(&mut state, &nodes, &ctx, 1024);

    assert_eq!(doc.load_state, LoadState::Ready);
    assert!(doc.error_message.is_none());
    assert_eq!(doc.content_excerpt, "(directory selected)");
    assert_eq!(state.selected_metadata.filename, "folder");
    assert_eq!(state.selected_metadata.hidden_text, "off");
}

#[test]
fn metadata_status_line_truncates_when_width_is_small() {
    let metadata = SelectedEntryMetadata {
        filename: "very-long-file-name.rs".to_string(),
        size_text: "1024".to_string(),
        permission_text: "644".to_string(),
        modified_text: "123456789".to_string(),
        hidden_text: "off".to_string(),
    };
    let line = compose_preview_metadata_line(&metadata, 24);
    assert_eq!(line.chars().count(), 24);
    assert!(line.ends_with('…'));
}

#[test]
fn preview_total_lines_counts_fallback_header_and_content() {
    let mut doc = fpv::app::state::PreviewDocument {
        load_state: LoadState::Ready,
        content_type: ContentType::PlainText,
        content_excerpt: "a\nb".to_string(),
        fallback_reason: Some(PreviewFallbackReason::UnsupportedExtension),
        ..fpv::app::state::PreviewDocument::default()
    };
    assert_eq!(preview_total_lines(&doc), 3);

    doc.fallback_reason = None;
    assert_eq!(preview_total_lines(&doc), 2);
}
