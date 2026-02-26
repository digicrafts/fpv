use crate::app::state::{LoadState, NodeType, PreviewDocument, SessionState, TreeNode};
use crate::fs::current_dir::{list_current_directory_with_visibility, selected_entry_metadata};
use crate::fs::preview::load_preview;
use crate::highlight::syntax::HighlightContext;
use std::path::Path;
use std::time::Instant;

const DIRECTORY_PREVIEW_MAX_ENTRIES: usize = 2000;

fn directory_entry_label(node: &TreeNode) -> String {
    match node.node_type {
        NodeType::Directory => format!("{}/", node.name),
        NodeType::Symlink => format!("@{}", node.name),
        NodeType::Unknown => format!("?{}", node.name),
        NodeType::File => node.name.clone(),
    }
}

fn directory_preview(path: &Path, show_hidden: bool) -> PreviewDocument {
    match list_current_directory_with_visibility(path, DIRECTORY_PREVIEW_MAX_ENTRIES, show_hidden) {
        Ok(entries) => {
            let mut lines = Vec::with_capacity(entries.len().saturating_add(1));
            if entries.is_empty() {
                lines.push("(empty directory)".to_string());
            } else {
                lines.extend(entries.iter().map(directory_entry_label));
            }
            PreviewDocument {
                source_path: path.to_path_buf(),
                load_state: LoadState::Ready,
                content_excerpt: lines.join("\n"),
                ..PreviewDocument::default()
            }
        }
        Err(_) => PreviewDocument {
            source_path: path.to_path_buf(),
            load_state: LoadState::Error,
            error_message: Some("Cannot read directory.".to_string()),
            ..PreviewDocument::default()
        },
    }
}

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
            directory_preview(&node.path, state.show_hidden)
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
