use crate::config::load::StatusDisplayMode;
use crate::fs::git::GitRepoStatus;
use crossterm::event::Event;
use ratatui::style::Style;
use ratatui::text::Line;
use std::path::PathBuf;
use std::time::Instant;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentPosition {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSelection {
    pub anchor: ContentPosition,
    pub cursor: ContentPosition,
}

impl PreviewSelection {
    pub fn ordered(&self) -> (ContentPosition, ContentPosition) {
        if self.anchor.row < self.cursor.row
            || (self.anchor.row == self.cursor.row && self.anchor.col <= self.cursor.col)
        {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }
}

#[derive(Debug, Clone)]
pub struct StyledPreviewSegment {
    pub text: String,
    pub style: Style,
}

pub type StyledPreviewLine = Vec<StyledPreviewSegment>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreviewLineChange {
    Added,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSearch {
    pub query: String,
    pub case_sensitive: bool,
    pub current_match_index: usize,
    pub match_positions: Vec<(usize, usize, usize)>, // (line, col_start, col_end)
}

impl Default for PreviewSearch {
    fn default() -> Self {
        Self {
            query: String::new(),
            case_sensitive: false,
            current_match_index: 0,
            match_positions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewRenderCacheKey {
    pub epoch: u64,
    pub inner_width: u16,
    pub show_line_numbers: bool,
    pub wrap_enabled: bool,
    pub content_hash: u64,
    pub styled_lines_hash: u64,
    pub line_changes_hash: u64,
}

#[derive(Debug, Clone)]
pub struct PreviewRenderCache {
    pub key: PreviewRenderCacheKey,
    pub rendered_lines: Vec<Line<'static>>,
    pub rendered_row_changes: Vec<Option<PreviewLineChange>>,
    pub total_lines: usize,
    pub line_number_cols: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewDiffCacheKey {
    pub path: PathBuf,
    pub current_content_hash: u64,
    pub content_type: ContentType,
    pub language_id: Option<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub struct PreviewDiffCache {
    pub key: PreviewDiffCacheKey,
    pub merged_doc: PreviewDocument,
}

#[derive(Debug, Clone)]
pub struct PreviewDocument {
    pub source_path: PathBuf,
    pub load_state: LoadState,
    pub content_type: ContentType,
    pub image_preview: bool,
    pub image_preview_pending: bool,
    pub language_id: Option<String>,
    pub content_excerpt: String,
    pub styled_lines: Vec<StyledPreviewLine>,
    pub display_line_numbers: Vec<Option<usize>>,
    pub line_changes: Vec<Option<PreviewLineChange>>,
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
    pub selected_changed_at: Instant,
    pub focus_pane: FocusPane,
    pub status_message: String,
    pub current_dir_error: Option<String>,
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
    pub divider_drag_column: Option<u16>,
    pub preview_scroll_col: usize,
    pub preview_selection: Option<PreviewSelection>,
    pub preview_selecting: bool,
    pub preview_scrollbar_dragging: bool,
    pub preview_inner_rect: (u16, u16, u16, u16),
    pub preview_line_number_cols: usize,
    pub preview_render_epoch: u64,
    pub preview_render_cache: Option<PreviewRenderCache>,
    pub preview_diff_cache: Option<PreviewDiffCache>,
    pub preview_copy_indicator: bool,
    pub preview_copying_indicator: bool,
    pub preview_diff_mode: bool,
    pub preview_search: Option<PreviewSearch>,
    pub preview_search_input_active: bool,
    pub help_overlay_visible: bool,
    pub status_display_mode: StatusDisplayMode,
    pub git_status: Option<GitRepoStatus>,
    pub deferred_input_event: Option<Event>,
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
            selected_changed_at: Instant::now(),
            focus_pane: FocusPane::Tree,
            status_message: String::new(),
            current_dir_error: None,
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
            divider_drag_column: None,
            preview_scroll_col: 0,
            preview_selection: None,
            preview_selecting: false,
            preview_scrollbar_dragging: false,
            preview_inner_rect: (0, 0, 0, 0),
            preview_line_number_cols: 0,
            preview_render_epoch: 0,
            preview_render_cache: None,
            preview_diff_cache: None,
            preview_copy_indicator: false,
            preview_copying_indicator: false,
            preview_diff_mode: false,
            preview_search: None,
            preview_search_input_active: false,
            help_overlay_visible: false,
            status_display_mode: StatusDisplayMode::Bar,
            git_status: None,
            deferred_input_event: None,
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
        self.preview_scroll_col = 0;
        self.preview_selection = None;
        self.preview_selecting = false;
        self.preview_scrollbar_dragging = false;
    }

    pub fn clamp_preview_scroll(&mut self, total_lines: usize, viewport_rows: usize) {
        let max_scroll = max_scroll_row(total_lines, viewport_rows);
        if self.preview_scroll_row > max_scroll {
            self.preview_scroll_row = max_scroll;
        }
    }

    pub fn scroll_preview_lines(
        &mut self,
        delta: isize,
        total_lines: usize,
        viewport_rows: usize,
    ) -> bool {
        let before = self.preview_scroll_row;
        let max_scroll = max_scroll_row(total_lines, viewport_rows);
        if delta < 0 {
            self.preview_scroll_row = self.preview_scroll_row.saturating_sub((-delta) as usize);
        } else if delta > 0 {
            self.preview_scroll_row = self
                .preview_scroll_row
                .saturating_add(delta as usize)
                .min(max_scroll);
        }
        self.preview_scroll_row != before
    }

    pub fn scroll_preview_cols(
        &mut self,
        delta: isize,
        max_width: usize,
        viewport_cols: usize,
    ) -> bool {
        let before = self.preview_scroll_col;
        let max_scroll = max_width.saturating_sub(viewport_cols);
        if delta < 0 {
            self.preview_scroll_col = self.preview_scroll_col.saturating_sub((-delta) as usize);
        } else {
            self.preview_scroll_col = self
                .preview_scroll_col
                .saturating_add(delta as usize)
                .min(max_scroll);
        }
        self.preview_scroll_col != before
    }

    pub fn page_scroll_preview_down(&mut self, total_lines: usize, viewport_rows: usize) -> bool {
        let page = viewport_rows.max(1) as isize;
        self.scroll_preview_lines(page, total_lines, viewport_rows)
    }

    pub fn page_scroll_preview_up(&mut self, total_lines: usize, viewport_rows: usize) -> bool {
        let page = viewport_rows.max(1) as isize;
        self.scroll_preview_lines(-page, total_lines, viewport_rows)
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

    pub fn clamped_divider_column(&self, divider_col: u16, main_width: u16) -> u16 {
        let tree = divider_col.min(main_width);
        let desired_preview = main_width.saturating_sub(tree);
        let preview = self.clamped_preview_width(main_width, desired_preview);
        main_width.saturating_sub(preview)
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
        let next = nodes
            .get(self.selected_index)
            .map(|n| n.path.clone())
            .unwrap_or_else(|| self.current_path.clone());
        self.set_selected_path(next);
    }

    pub fn set_selected_path(&mut self, path: PathBuf) {
        if self.selected_path != path {
            self.selected_path = path;
            self.selected_changed_at = Instant::now();
        }
    }

    pub fn set_current_dir_error(&mut self, message: impl Into<String>) {
        self.current_dir_error = Some(message.into());
    }

    pub fn clear_current_dir_error(&mut self) {
        self.current_dir_error = None;
    }
}

fn max_scroll_row(total_lines: usize, viewport_rows: usize) -> usize {
    total_lines.saturating_sub(viewport_rows.max(1))
}
