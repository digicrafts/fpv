use crate::app::navigation_result::NavigationActionResult;
use crate::app::state::{NodeType, SessionState, TreeNode};
use crate::fs::current_dir::{
    is_filesystem_root, list_current_directory_with_visibility, parent_path,
};
use anyhow::Result;
use std::path::{Path, PathBuf};

const MAX_DIR_ENTRIES: usize = 2000;

fn absolute_current_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

pub fn move_up(state: &mut SessionState) {
    if state.selected_index > 0 {
        state.selected_index -= 1;
    }
}

pub fn move_down(state: &mut SessionState, nodes_len: usize) {
    if state.selected_index + 1 < nodes_len {
        state.selected_index += 1;
    }
}

pub fn expand_selected(nodes: &mut [TreeNode], selected_index: usize) {
    if let Some(node) = nodes.get_mut(selected_index) {
        if node.node_type == NodeType::Directory {
            node.expanded = true;
        }
    }
}

pub fn collapse_selected(nodes: &mut [TreeNode], selected_index: usize) {
    if let Some(node) = nodes.get_mut(selected_index) {
        if node.node_type == NodeType::Directory {
            node.expanded = false;
        }
    }
}

pub fn enter_selected_directory(
    state: &mut SessionState,
    nodes: &mut Vec<TreeNode>,
) -> Result<NavigationActionResult> {
    let Some(node) = nodes.get(state.selected_index) else {
        return Ok(NavigationActionResult::blocked(
            "enter_directory",
            state.current_path.clone(),
            "No directory selected.",
        ));
    };

    if node.node_type != NodeType::Directory {
        return Ok(NavigationActionResult::no_change(
            "enter_directory",
            state.current_path.clone(),
            "Selected item is not a directory.",
        ));
    }

    if !node.readable {
        return Ok(NavigationActionResult::blocked(
            "enter_directory",
            state.current_path.clone(),
            "Permission denied for selected directory.",
        ));
    }

    let target = node.path.clone();
    let entries =
        list_current_directory_with_visibility(&target, MAX_DIR_ENTRIES, state.show_hidden)?;
    state.last_child_path = Some(target.clone());
    state.current_path = target.clone();
    *nodes = entries;
    state.selected_index = 0;
    state.revalidate_selection(nodes);
    state.update_selected_path(nodes);

    Ok(NavigationActionResult::changed(
        "enter_directory",
        target,
        "Entered directory.",
    ))
}

pub fn go_to_parent_directory(
    state: &mut SessionState,
    nodes: &mut Vec<TreeNode>,
) -> Result<NavigationActionResult> {
    let current_path = absolute_current_path(&state.current_path);

    if is_filesystem_root(&current_path) {
        return Ok(NavigationActionResult::no_change(
            "go_parent",
            current_path,
            "Already at filesystem root.",
        ));
    }

    let Some(parent) = parent_path(&current_path) else {
        return Ok(NavigationActionResult::no_change(
            "go_parent",
            current_path,
            "No parent directory available.",
        ));
    };

    let previous_child = current_path;
    let entries =
        list_current_directory_with_visibility(&parent, MAX_DIR_ENTRIES, state.show_hidden)?;
    state.current_path = parent.clone();
    *nodes = entries;
    state.restore_or_default_selection(nodes, Some(&previous_child));
    state.update_selected_path(nodes);

    Ok(NavigationActionResult::changed(
        "go_parent",
        parent,
        "Moved to parent directory.",
    ))
}

pub fn refresh_current_directory(
    state: &mut SessionState,
    nodes: &mut Vec<TreeNode>,
) -> Result<NavigationActionResult> {
    let previous_selected_path = nodes.get(state.selected_index).map(|n| n.path.clone());
    let entries = list_current_directory_with_visibility(
        &state.current_path,
        MAX_DIR_ENTRIES,
        state.show_hidden,
    )?;
    *nodes = entries;
    state.restore_or_default_selection(nodes, previous_selected_path.as_ref());
    state.update_selected_path(nodes);

    Ok(NavigationActionResult::changed(
        "refresh_current_dir",
        state.current_path.clone(),
        "Refreshed directory listing.",
    ))
}

pub fn toggle_hidden_visibility(
    state: &mut SessionState,
    nodes: &mut Vec<TreeNode>,
) -> Result<NavigationActionResult> {
    state.show_hidden = !state.show_hidden;
    let mut result = refresh_current_directory(state, nodes)?;
    result.action = "toggle_hidden";
    result.message = if state.show_hidden {
        "Hidden files shown.".to_string()
    } else {
        "Hidden files hidden.".to_string()
    };
    Ok(result)
}

pub fn format_status_with_path(message: &str, current_path: &Path) -> String {
    format!("{message} Path: {}", current_path.display())
}
