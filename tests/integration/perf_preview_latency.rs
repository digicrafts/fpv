use fpv::app::preview_controller::refresh_preview;
use fpv::app::state::{ContentType, NodeType, SessionState, TreeNode};
use fpv::fs::preview::load_preview;
use fpv::highlight::syntax::HighlightContext;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use tempfile::tempdir;

#[test]
fn preview_under_one_second_for_1mb_file() {
    let d = tempdir().expect("create tempdir");
    let p = d.path().join("large.md");
    let body = "a".repeat(1024 * 1024);
    fs::write(&p, body).expect("write file");
    let ctx = HighlightContext::new();

    let started = Instant::now();
    let doc = load_preview(&p, 1024 * 1024, &ctx);
    let elapsed_ms = started.elapsed().as_millis();
    assert!(elapsed_ms < 1000, "preview took {elapsed_ms}ms");
    assert!(matches!(
        doc.content_type,
        ContentType::Highlighted | ContentType::PlainText
    ));
}

#[test]
fn selection_driven_preview_refresh_stays_under_200ms() {
    let d = tempdir().expect("create tempdir");
    let mut nodes = Vec::new();
    for i in 0..200 {
        let path = d.path().join(format!("item-{i}.py"));
        fs::write(&path, format!("print('{}')", i)).expect("write file");
        nodes.push(TreeNode {
            path,
            name: format!("item-{i}.py"),
            node_type: NodeType::File,
            depth: 0,
            expanded: false,
            readable: true,
            children_loaded: false,
        });
    }

    let mut state = SessionState::new(PathBuf::from("."));
    let ctx = HighlightContext::new();
    let started = Instant::now();

    for idx in 0..nodes.len() {
        state.selected_index = idx;
        let doc = refresh_preview(&mut state, &nodes, &ctx, 1024 * 1024);
        assert!(matches!(doc.content_type, ContentType::Highlighted));
    }

    let avg_ms = started.elapsed().as_millis() as f64 / nodes.len() as f64;
    assert!(avg_ms < 200.0, "average preview refresh took {avg_ms:.2}ms");
}
