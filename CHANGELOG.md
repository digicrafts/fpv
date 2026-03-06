# Changelog

## [0.1.4] - 2026-03-06

### Added

- **Mouse selection & copy in preview pane** — click and drag to select text, copied to clipboard on release via OSC 52 with tmux passthrough support. Writes to `/dev/tty` for reliable SSH clipboard forwarding.
- **Horizontal scrolling** — scroll preview content horizontally when word wrap is disabled:
  - `[` / `]` keys to scroll left/right
  - Shift+Left / Shift+Right arrow keys
  - Left/Right arrows in fullscreen preview mode
  - Shift+MouseWheel for horizontal mouse scroll
- **Shift+Up/Down** scrolls preview vertically from any pane (not just when preview is focused)
- **Git status in directory preview** — when a directory is selected, the preview pane shows git status indicators (`[M]`, `[A]`, `[D]`, `[R]`, `[?]`, etc.) next to each file entry
- **"Copy Completed" overlay** — a bold inverted-color indicator appears at the top-right of the preview pane after a successful copy, disappears on next action
- `PreviewScrollLeft` / `PreviewScrollRight` actions available for custom keybinding
- **Auto-refresh** — tree list and preview automatically update when files/directories change on disk (polling every 2 seconds)
- **Manual refresh** — press `F5` to force refresh the tree and preview immediately

### Changed

- Selection highlight uses light blue background (instead of REVERSED modifier) for better visibility across terminals
- Clipboard copy now always sends OSC 52 via `/dev/tty` first, then also tries local tools (`xclip`, `xsel`, `wl-copy`, `pbcopy`) as a bonus
- OSC 52 uses proper `ESC \` string terminator instead of BEL for wider terminal compatibility
- tmux `allow-passthrough` is automatically enabled per-pane during copy and restored afterward

## [0.1.3] - 2025-01-01

- Initial public release
