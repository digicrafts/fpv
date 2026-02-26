# fpv

Terminal file tree preview tool written in Rust.

## Features

- Single-layer directory navigation: list current directory only
- `Right` key enters selected directory, `Left` key returns to parent directory
- Keyboard-first navigation in a split directory/preview TUI
- Syntax highlighting for common text/code files (HTML, Markdown, Python, Go, JSON, Rust, etc.)
- Configurable keyboard shortcuts via TOML
- Safe fallback for binary/unreadable files

## Run

```bash
cargo run -- /path/to/project
```

Custom keymap:

```bash
cargo run -- /path/to/project --config config/sample.user.toml
```

## Quality gates

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Release and publishing workflow

This repo includes a GitHub Actions release workflow at:

`/.github/workflows/release.yml`

On tag push `v*` (for example `v0.2.0`), it will:
- Run tests
- Build release archives for Linux and macOS
- Build a Debian package (`.deb`)
- Create a GitHub Release with artifacts + `checksums.txt`

Optional publish steps are enabled only when corresponding secrets/variables are set:
- `crates.io`: set secret `CARGO_REGISTRY_TOKEN`
- `Homebrew tap`: set secret `HOMEBREW_TAP_TOKEN` and variable `HOMEBREW_TAP_REPO` (format: `owner/homebrew-tap`)
- `APT (Cloudsmith)`: set secret `CLOUDSMITH_API_KEY` and variable `CLOUDSMITH_REPO` (format: `owner/repo`)

### Typical release flow

```bash
git tag v0.2.0
git push origin v0.2.0
```
