//! Shared color and style constants used across TUI rendering.

use ratatui::style::Color;

// ── Diff backgrounds ──────────────────────────────────────────────────────
/// Background for added/inserted lines in diff view.
pub const DIFF_ADDED_BG: Color = Color::Rgb(0, 70, 0);
/// Background for deleted/removed lines in diff view.
pub const DIFF_DELETED_BG: Color = Color::Rgb(90, 0, 0);

// ── Syntax highlighting theme ─────────────────────────────────────────────
pub const SYNTAX_COMMENT: Color = Color::Rgb(120, 150, 120);
pub const SYNTAX_KEYWORD: Color = Color::Rgb(220, 150, 80);
pub const SYNTAX_STRING: Color = Color::Rgb(140, 200, 130);
pub const SYNTAX_NUMBER: Color = Color::Rgb(120, 190, 210);
pub const SYNTAX_TYPE: Color = Color::Rgb(120, 170, 230);
pub const SYNTAX_FUNCTION: Color = Color::Rgb(220, 200, 120);
pub const SYNTAX_PARAMETER: Color = Color::Rgb(210, 170, 230);
pub const SYNTAX_TITLE: Color = Color::Rgb(110, 170, 240);
pub const SYNTAX_REFERENCE: Color = Color::Rgb(130, 180, 230);

// ── Git status indicators ────────────────────────────────────────────────
pub const GIT_DELETED: Color = Color::Red;
pub const GIT_MODIFIED: Color = Color::Yellow;
pub const GIT_ADDED: Color = Color::Green;
pub const GIT_RENAMED: Color = Color::Cyan;
pub const GIT_CONFLICTED: Color = Color::Magenta;
pub const GIT_IGNORED: Color = Color::DarkGray;

// ── Diff markers ─────────────────────────────────────────────────────────
pub const DIFF_MARKER_ADDED: Color = Color::LightGreen;
pub const DIFF_MARKER_DELETED: Color = Color::LightRed;

// ── Selection ────────────────────────────────────────────────────────────
pub const SELECTION_BG: Color = Color::LightBlue;
pub const SELECTION_FG: Color = Color::Black;

// ── Scrollbar ────────────────────────────────────────────────────────────
pub const SCROLLBAR_TRACK: Color = Color::DarkGray;
pub const SCROLLBAR_THUMB: Color = Color::Gray;

// ── Line numbers ─────────────────────────────────────────────────────────
pub const LINE_NUMBER_FG: Color = Color::Gray;

// ── Status bar ───────────────────────────────────────────────────────────
pub const STATUS_BAR_FG: Color = Color::White;
pub const STATUS_BAR_BG: Color = Color::DarkGray;

// ── Overlays ─────────────────────────────────────────────────────────────
pub const OVERLAY_FG: Color = Color::Black;
pub const OVERLAY_BG: Color = Color::White;
pub const DIFF_BADGE_FG: Color = Color::Black;
pub const DIFF_BADGE_BG: Color = Color::Yellow;
