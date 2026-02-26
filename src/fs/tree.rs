use crate::app::state::{NodeType, TreeNode};
use anyhow::Result;
use ignore::WalkBuilder;
use std::fs;
use std::path::Path;

pub fn build_tree(root: &Path, max_entries: usize) -> Result<Vec<TreeNode>> {
    let mut nodes = Vec::new();

    for entry in WalkBuilder::new(root).hidden(false).build().flatten() {
        if nodes.len() >= max_entries {
            break;
        }

        let path = entry.path().to_path_buf();
        let depth = entry.depth();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        let metadata = fs::symlink_metadata(&path).ok();
        let node_type = if let Some(meta) = &metadata {
            let ft = meta.file_type();
            if ft.is_dir() {
                NodeType::Directory
            } else if ft.is_file() {
                NodeType::File
            } else if ft.is_symlink() {
                NodeType::Symlink
            } else {
                NodeType::Unknown
            }
        } else {
            NodeType::Unknown
        };

        let readable = fs::metadata(&path).is_ok();
        nodes.push(TreeNode {
            path,
            name,
            node_type,
            depth,
            expanded: depth <= 1,
            readable,
            children_loaded: depth <= 1,
        });
    }

    Ok(nodes)
}
