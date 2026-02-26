use crate::app::state::TreeNode;
use std::path::PathBuf;

pub const METADATA_FALLBACK: &str = "-";

#[derive(Debug, Clone)]
pub struct CurrentDirectoryState {
    pub current_path: PathBuf,
    pub parent_path: Option<PathBuf>,
    pub entries: Vec<TreeNode>,
    pub selected_index: usize,
    pub last_child_path: Option<PathBuf>,
    pub status_message: String,
}

impl CurrentDirectoryState {
    pub fn with_entries(
        current_path: PathBuf,
        parent_path: Option<PathBuf>,
        entries: Vec<TreeNode>,
    ) -> Self {
        Self {
            current_path,
            parent_path,
            entries,
            selected_index: 0,
            last_child_path: None,
            status_message: String::new(),
        }
    }

    pub fn selected_entry(&self) -> Option<&TreeNode> {
        self.entries.get(self.selected_index)
    }

    pub fn revalidate_selection(&mut self) {
        if self.entries.is_empty() {
            self.selected_index = 0;
            return;
        }

        if self.selected_index >= self.entries.len() {
            self.selected_index = self.entries.len().saturating_sub(1);
        }
    }

    pub fn restore_or_default_selection(&mut self, preferred: Option<&PathBuf>) {
        if self.entries.is_empty() {
            self.selected_index = 0;
            return;
        }

        if let Some(target) = preferred {
            if let Some(idx) = self.entries.iter().position(|e| &e.path == target) {
                self.selected_index = idx;
                return;
            }
        }

        self.selected_index = 0;
    }
}

pub fn truncate_for_status(input: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let chars: Vec<char> = input.chars().collect();
    if chars.len() <= max_chars {
        return input.to_string();
    }
    if max_chars == 1 {
        return "…".to_string();
    }
    let keep = max_chars - 1;
    let head: String = chars.into_iter().take(keep).collect();
    format!("{head}…")
}
