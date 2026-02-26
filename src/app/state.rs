use crate::config::load::StatusDisplayMode;
use crate::fs::git::GitRepoStatus;
use ratatui::style::Style;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    File,
    Directory,
    Symlink,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub path: PathBuf,
    pub name: String,
    pub node_type: NodeType,
    pub depth: usize,
    pub expanded: bool,
    pub readable: bool,
    pub children_loaded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadState {
    Idle,
    Loading,
    Ready,
    Error,
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentType {
    Highlighted,
    PlainText,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewFallbackReason {
    UnsupportedExtension,
    EngineFailure,
    TooLarge,
    DecodeUncertain,
}

#[derive(Debug, Clone)]
pub struct StyledPreviewSegment {
    pub text: String,
    pub style: Style,
}

pub type StyledPreviewLine = Vec<StyledPreviewSegment>;

#[derive(Debug, Clone)]
pub struct PreviewDocument {
    pub source_path: PathBuf,
    pub load_state: LoadState,
    pub content_type: ContentType,
    pub language_id: Option<String>,
    pub content_excerpt: String,
    pub styled_lines: Vec<StyledPreviewLine>,
    pub fallback_reason: Option<PreviewFallbackReason>,
    pub truncated: bool,
    pub error_message: Option<String>,
}

impl Default for PreviewDocument {
    fn default() -> Self {
        Self {
            source_path: PathBuf::new(),
            load_state: LoadState::Idle,
            content_type: ContentType::PlainText,
            language_id: None,
            content_excerpt: String::new(),
            styled_lines: Vec::new(),
            fallback_reason: None,
            truncated: false,
            error_message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusPane {
    Tree,
    Preview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedEntryMetadata {
    pub filename: String,
    pub size_text: String,
    pub permission_text: String,
    pub modified_text: String,
    pub hidden_text: String,
}

impl Default for SelectedEntryMetadata {
    fn default() -> Self {
        Self {
            filename: "-".to_string(),
            size_text: "-".to_string(),
            permission_text: "-".to_string(),
            modified_text: "-".to_string(),
            hidden_text: "off".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutRegions {
    pub top_directory_header: bool,
    pub left_navigation_panel: bool,
    pub right_preview_panel: bool,
    pub preview_top_status_bar: bool,
    pub bottom_global_status_bar: bool,
}

impl Default for LayoutRegions {
    fn default() -> Self {
        Self {
            top_directory_header: true,
            left_navigation_panel: true,
            right_preview_panel: true,
            preview_top_status_bar: true,
            bottom_global_status_bar: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub root_path: PathBuf,
    pub current_path: PathBuf,
    pub selected_index: usize,
    pub selected_path: PathBuf,
    pub focus_pane: FocusPane,
    pub status_message: String,
    pub last_preview_latency_ms: u128,
    pub last_child_path: Option<PathBuf>,
    pub show_hidden: bool,
    pub selected_metadata: SelectedEntryMetadata,
    pub layout_regions: LayoutRegions,
    pub preview_width_cols: u16,
    pub tree_min_width_cols: u16,
    pub preview_min_width_cols: u16,
    pub preview_resize_step_cols: u16,
    pub preview_scroll_row: usize,
    pub preview_show_line_numbers: bool,
    pub preview_wrap_enabled: bool,
    pub preview_fullscreen: bool,
    pub divider_drag_active: bool,
    pub help_overlay_visible: bool,
    pub status_display_mode: StatusDisplayMode,
    pub git_status: Option<GitRepoStatus>,
}

impl SessionState {
    const DEFAULT_TREE_WIDTH_DENOMINATOR: u16 = 6;
    const DEFAULT_PANEL_MIN_WIDTH: u16 = 20;
    const DEFAULT_RESIZE_STEP: u16 = 2;

    pub fn new(root_path: PathBuf) -> Self {
        Self {
            root_path: root_path.clone(),
            current_path: root_path.clone(),
            selected_index: 0,
            selected_path: root_path,
            focus_pane: FocusPane::Tree,
            status_message: String::new(),
            last_preview_latency_ms: 0,
            last_child_path: None,
            show_hidden: false,
            selected_metadata: SelectedEntryMetadata::default(),
            layout_regions: LayoutRegions::default(),
            preview_width_cols: 0,
            tree_min_width_cols: Self::DEFAULT_PANEL_MIN_WIDTH,
            preview_min_width_cols: Self::DEFAULT_PANEL_MIN_WIDTH,
            preview_resize_step_cols: Self::DEFAULT_RESIZE_STEP,
            preview_scroll_row: 0,
            preview_show_line_numbers: true,
            preview_wrap_enabled: false,
            preview_fullscreen: false,
            divider_drag_active: false,
            help_overlay_visible: false,
            status_display_mode: StatusDisplayMode::Bar,
            git_status: None,
        }
    }

    pub fn normalize_preview_width(&mut self, main_width: u16) {
        self.preview_width_cols = self.effective_preview_width(main_width);
    }

    pub fn panel_widths(&self, main_width: u16) -> (u16, u16) {
        let preview = self.effective_preview_width(main_width);
        let tree = main_width.saturating_sub(preview);
        (tree, preview)
    }

    pub fn resize_step(&self) -> u16 {
        self.preview_resize_step_cols.max(1)
    }

    pub fn reset_preview_scroll(&mut self) {
        self.preview_scroll_row = 0;
    }

    pub fn clamp_preview_scroll(&mut self, total_lines: usize, viewport_rows: usize) {
        let max_scroll = max_scroll_row(total_lines, viewport_rows);
        if self.preview_scroll_row > max_scroll {
            self.preview_scroll_row = max_scroll;
        }
    }

    pub fn scroll_preview_lines(&mut self, delta: isize, total_lines: usize, viewport_rows: usize) {
        let max_scroll = max_scroll_row(total_lines, viewport_rows);
        if delta < 0 {
            self.preview_scroll_row = self.preview_scroll_row.saturating_sub((-delta) as usize);
        } else if delta > 0 {
            self.preview_scroll_row = self
                .preview_scroll_row
                .saturating_add(delta as usize)
                .min(max_scroll);
        }
    }

    pub fn page_scroll_preview_down(&mut self, total_lines: usize, viewport_rows: usize) {
        let page = viewport_rows.max(1) as isize;
        self.scroll_preview_lines(page, total_lines, viewport_rows);
    }

    pub fn page_scroll_preview_up(&mut self, total_lines: usize, viewport_rows: usize) {
        let page = viewport_rows.max(1) as isize;
        self.scroll_preview_lines(-page, total_lines, viewport_rows);
    }

    pub fn resize_preview_by(&mut self, delta_cols: i16, main_width: u16) {
        let base = i32::from(self.effective_preview_width(main_width));
        let desired = (base + i32::from(delta_cols)).max(0) as u16;
        self.preview_width_cols = self.clamped_preview_width(main_width, desired);
    }

    pub fn set_preview_width_from_divider(&mut self, divider_col: u16, main_width: u16) {
        let tree = divider_col.min(main_width);
        let desired_preview = main_width.saturating_sub(tree);
        self.preview_width_cols = self.clamped_preview_width(main_width, desired_preview);
    }

    pub fn divider_column(&self, main_width: u16) -> u16 {
        let (tree, _) = self.panel_widths(main_width);
        tree
    }

    fn effective_preview_width(&self, main_width: u16) -> u16 {
        let desired = if self.preview_width_cols == 0 {
            let default_tree_width = main_width / Self::DEFAULT_TREE_WIDTH_DENOMINATOR;
            main_width.saturating_sub(default_tree_width)
        } else {
            self.preview_width_cols
        };
        self.clamped_preview_width(main_width, desired)
    }

    fn clamped_preview_width(&self, main_width: u16, desired: u16) -> u16 {
        if main_width == 0 {
            return 0;
        }

        let preview_min = self.preview_min_width_cols.min(main_width);
        let tree_min = self
            .tree_min_width_cols
            .min(main_width.saturating_sub(preview_min));
        let preview_max = main_width.saturating_sub(tree_min).max(preview_min);
        desired.clamp(preview_min, preview_max)
    }

    pub fn revalidate_selection(&mut self, nodes: &[TreeNode]) {
        if nodes.is_empty() {
            self.selected_index = 0;
            return;
        }
        if self.selected_index >= nodes.len() {
            self.selected_index = nodes.len().saturating_sub(1);
        }
    }

    pub fn restore_or_default_selection(
        &mut self,
        nodes: &[TreeNode],
        preferred: Option<&PathBuf>,
    ) {
        if nodes.is_empty() {
            self.selected_index = 0;
            return;
        }

        if let Some(path) = preferred {
            if let Some(idx) = nodes.iter().position(|n| &n.path == path) {
                self.selected_index = idx;
                return;
            }
        }

        self.selected_index = 0;
    }

    pub fn update_selected_path(&mut self, nodes: &[TreeNode]) {
        self.selected_path = nodes
            .get(self.selected_index)
            .map(|n| n.path.clone())
            .unwrap_or_else(|| self.current_path.clone());
    }
}

fn max_scroll_row(total_lines: usize, viewport_rows: usize) -> usize {
    total_lines.saturating_sub(viewport_rows.max(1))
}
