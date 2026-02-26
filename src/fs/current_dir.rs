use crate::app::current_dir_state::METADATA_FALLBACK;
use crate::app::state::{NodeType, SelectedEntryMetadata, TreeNode};
use anyhow::Result;
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use time::{Month, OffsetDateTime};

pub fn list_current_directory(path: &Path, max_entries: usize) -> Result<Vec<TreeNode>> {
    list_current_directory_with_visibility(path, max_entries, true)
}

pub fn selected_entry_metadata(node: &TreeNode) -> SelectedEntryMetadata {
    let metadata = fs::symlink_metadata(&node.path).ok();
    let filename = node.name.clone();
    let size_text = metadata
        .as_ref()
        .map(|m| format_size(m.len()))
        .unwrap_or_else(|| METADATA_FALLBACK.to_string());
    let permission_text = metadata
        .as_ref()
        .map(permission_summary)
        .unwrap_or_else(|| METADATA_FALLBACK.to_string());
    let modified_text = metadata
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| format_modified_timestamp(d.as_secs() as i64))
        .unwrap_or_else(|| METADATA_FALLBACK.to_string());
    let hidden_text = if is_hidden_name(&node.name) {
        "on"
    } else {
        "off"
    }
    .to_string();

    SelectedEntryMetadata {
        filename,
        size_text,
        permission_text,
        modified_text,
        hidden_text,
    }
}

pub fn list_current_directory_with_visibility(
    path: &Path,
    max_entries: usize,
    show_hidden: bool,
) -> Result<Vec<TreeNode>> {
    let mut nodes = Vec::new();
    let entries = fs::read_dir(path)?;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !show_hidden && is_hidden_name(&name) {
            continue;
        }

        if nodes.len() >= max_entries {
            break;
        }

        let path = entry.path();
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

        let readable = match node_type {
            NodeType::Directory => fs::read_dir(&path).is_ok(),
            _ => fs::File::open(&path).is_ok(),
        };

        nodes.push(TreeNode {
            name,
            path,
            node_type,
            depth: 0,
            expanded: false,
            readable,
            children_loaded: false,
        });
    }

    nodes.sort_by(directory_first_cmp);
    Ok(nodes)
}

fn is_hidden_name(name: &str) -> bool {
    name.starts_with('.')
}

fn format_size(size: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    let n = size as f64;
    if n < KIB {
        format!("{size} B")
    } else if n < MIB {
        format!("{:.1} KiB", n / KIB)
    } else {
        format!("{:.1} MiB", n / MIB)
    }
}

#[cfg(unix)]
fn permission_summary(meta: &fs::Metadata) -> String {
    use std::os::unix::fs::PermissionsExt;
    let mode = meta.permissions().mode() & 0o777;
    symbolic_permissions(mode)
}

#[cfg(not(unix))]
fn permission_summary(meta: &fs::Metadata) -> String {
    if meta.permissions().readonly() {
        "r--r--r--".to_string()
    } else {
        "rw-rw-rw-".to_string()
    }
}

#[cfg(unix)]
fn symbolic_permissions(mode: u32) -> String {
    let mut chars = ['-'; 9];
    let flags = [
        0o400, 0o200, 0o100, // owner
        0o040, 0o020, 0o010, // group
        0o004, 0o002, 0o001, // others
    ];
    for (idx, bit) in flags.iter().enumerate() {
        if mode & bit != 0 {
            chars[idx] = match idx % 3 {
                0 => 'r',
                1 => 'w',
                _ => 'x',
            };
        }
    }
    chars.iter().collect()
}

fn format_modified_timestamp(epoch_seconds: i64) -> String {
    let Ok(dt) = OffsetDateTime::from_unix_timestamp(epoch_seconds) else {
        return epoch_seconds.to_string();
    };
    let month = match dt.month() {
        Month::January => "Jan",
        Month::February => "Feb",
        Month::March => "Mar",
        Month::April => "Apr",
        Month::May => "May",
        Month::June => "Jun",
        Month::July => "Jul",
        Month::August => "Aug",
        Month::September => "Sep",
        Month::October => "Oct",
        Month::November => "Nov",
        Month::December => "Dec",
    };
    format!(
        "{:02}:{:02} {:02}-{}-{:04}",
        dt.hour(),
        dt.minute(),
        dt.day(),
        month,
        dt.year()
    )
}

fn directory_first_cmp(a: &TreeNode, b: &TreeNode) -> Ordering {
    let group = |n: &TreeNode| match n.node_type {
        NodeType::Directory => 0_u8,
        _ => 1_u8,
    };

    group(a)
        .cmp(&group(b))
        .then_with(|| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        })
        .then_with(|| a.name.cmp(&b.name))
}

pub fn parent_path(path: &Path) -> Option<PathBuf> {
    path.parent().map(|p| p.to_path_buf())
}

pub fn is_filesystem_root(path: &Path) -> bool {
    parent_path(path).is_none()
}
