# Changelog

## [0.1.8] - 2026-03-09

### Changed

- Release version updated for packaging and workflow fixes.

## [0.1.7] - 2026-03-09

### Changed

- Improved responsiveness by moving slow Git status and image preview work off the UI render path and into background workers.
- Stabilized preview interaction under high-frequency input, including wheel and key reversal behavior and bounded scroll processing at pane edges.
- Hardened large file behavior with explicit limits for text/image previews, diff generation, and preview caching.

## [0.1.6] - 2026-03-08

### Added

- **Image preview mode** — common image formats such as PNG, JPEG, and GIF now render directly in the preview pane as colorized shaded-block terminal previews.
- **YAML syntax highlighting** — `.yaml` and `.yml` files now use syntax highlighting in the normal preview pipeline.

### Changed

- **Image preview sizing** — image previews now scale to the current preview pane while staying within a `60x30` character bound.
- **Image preview debounce** — image rendering is delayed by 1 second after selection to avoid expensive decode work while quickly moving through the tree.
- **Mouse divider resize UX** — dragging the tree/preview divider now shows a placeholder guide and applies the actual resize on mouse release.

## [0.1.5] - 2026-03-08

### Added

- **Inline git diff preview mode** — diff view now keeps the normal syntax-highlighted file preview as the base, inserts deleted lines inline, and marks changes with bright `+` / `-` gutter indicators.
- **Diff-aware scrollbar markers** — the preview scrollbar shows changed positions, and clicking the scrollbar jumps directly to the corresponding rendered line.

### Changed

- **Diff styling refresh** — added and deleted rows now use darker green/red full-line backgrounds, keep syntax-highlighted code colors, and show correct old/new line numbers in diff mode.
- **Diff mode indicator placement** — the `DIFF` badge now renders on the upper border of the preview pane instead of covering preview content.
- **Preview rendering performance** — large previews now use viewport-only rendering, cached rendered rows, and cached inline diff documents so repeated redraws and diff refreshes are cheaper.

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
