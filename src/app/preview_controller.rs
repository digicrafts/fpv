use crate::app::state::{LoadState, NodeType, PreviewDocument, SessionState, TreeNode};
use crate::fs::current_dir::selected_entry_metadata;
use crate::fs::preview::load_preview;
use crate::highlight::syntax::HighlightContext;
use std::time::Instant;

pub fn refresh_preview(
    state: &mut SessionState,
    nodes: &[TreeNode],
    ctx: &HighlightContext,
    max_bytes: usize,
) -> PreviewDocument {
    let started = Instant::now();
    let preview = if let Some(node) = nodes.get(state.selected_index) {
        state.selected_path = node.path.clone();
        state.selected_metadata = selected_entry_metadata(node);
        if node.node_type == NodeType::Directory {
            PreviewDocument {
                source_path: node.path.clone(),
                load_state: LoadState::Ready,
                content_excerpt: String::from("(directory selected)"),
                ..PreviewDocument::default()
            }
        } else {
            load_preview(&node.path, max_bytes, ctx)
        }
    } else {
        state.selected_metadata = Default::default();
        PreviewDocument {
            load_state: LoadState::Error,
            error_message: Some("No selection".to_string()),
            ..PreviewDocument::default()
        }
    };
    state.last_preview_latency_ms = started.elapsed().as_millis();
    preview
}
