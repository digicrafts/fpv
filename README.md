# fpv — Fast Previewer TUI

[![GitHub stars](https://img.shields.io/github/stars/digicrafts/fpv?style=flat-square)](https://github.com/digicrafts/fpv/stargazers)
[![GitHub downloads](https://img.shields.io/github/downloads/digicrafts/fpv/total?style=flat-square)](https://github.com/digicrafts/fpv/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)

A minimal, keyboard-first TUI file previewer for browsing directories and viewing code with syntax highlighting in the terminal.

---

## Features

- **Split TUI** — Directory tree on the left, file preview on the right
- **Syntax highlighting** — Tree-sitter–powered highlighting for many languages (see [Supported file types](#supported-file-types))
- **Git-aware** — Tree indicators for repository status
- **Configurable** — Keybindings and theme via a TOML config file
- **Safe defaults** — Plain-text or fallback preview for binary or unreadable files

## Installation

### Homebrew

```bash
brew tap digicrafts/tap
brew install fpv
```

### From source

See [Build from source](#build-from-source) below.

## Usage

```bash
# Open current directory
fpv

# Open a specific path
fpv /path/to/project

# Use a custom config file
fpv /path/to/project --config ~/.config/fpv/config
```

**Quick tips: Press **?** in the app for shortcut help.

## Build from source

### Prerequisites

- [Rust](https://rustup.rs/) (Rust 1.70+)

### Build and run

```bash
git clone https://github.com/digicrafts/fpv.git
cd fpv
cargo build --release
```

The binary will be at `target/release/fpv`. Run it with:

```bash
./target/release/fpv
# or, if installed: fpv
```

To run without installing (e.g. for development):

```bash
cargo run -- /path/to/project
cargo run -- /path/to/project --config config/sample.user.toml
```

## Configuration

On first run, fpv creates a default config at:

- **Linux / macOS:** `~/.config/fpv/config`

You can override keybindings and theme there. Example `config`:

```toml
[mappings]
quit = "ctrl+q"
switch_focus = "ctrl+tab"

[theme]
directory_color = "yellow"
hidden_dim_enabled = true

status_display_mode = "bar"   # or "title"
```

Config keys under `[mappings]` include: `move_up`, `move_down`, `expand_node`, `collapse_node`, `open_node`, `exit_fullscreen_preview`, `switch_focus`, `page_up`, `page_down`, `preview_scroll_up`, `preview_scroll_down`, `toggle_preview_line_numbers`, `toggle_preview_wrap`, `toggle_help`, `toggle_hidden`, `resize_preview_narrower`, `resize_preview_wider`, `quit`. Use key names like `up`, `down`, `enter`, `tab`, `ctrl+q`, etc.

## Supported file types

Syntax highlighting is supported for:

| Category   | Extensions / names |
| ---------- | ------------------ |
| Shell      | `bash`, `sh`, `zsh`, `ksh` |
| C / C++    | `c`, `h`, `cpp`, `cxx`, `hpp`, `hxx` |
| Web        | `html`, `htm`, `css`, `xml` |
| Go         | `go` |
| Java       | `java` |
| JavaScript / TypeScript | `js`, `jsx`, `mjs`, `cjs`, `ts`, `tsx` |
| Data       | `json`, `toml`, `yaml`, `yml` |
| Markdown   | `md`, `markdown` |
| Python     | `py` |
| Rust       | `rs` |

Other files are shown as plain text or with a safe fallback.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
