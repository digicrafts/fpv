use anyhow::{anyhow, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    MoveUp,
    MoveDown,
    Expand,
    Collapse,
    Open,
    ExitFullscreenPreview,
    SwitchFocus,
    PageUp,
    PageDown,
    PreviewScrollUp,
    PreviewScrollDown,
    TogglePreviewLineNumbers,
    TogglePreviewWrap,
    ToggleHelp,
    ToggleHidden,
    ResizePreviewNarrower,
    ResizePreviewWider,
    Quit,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserKeymap {
    pub mappings: HashMap<String, String>,
}

pub const SINGLE_LAYER_NAVIGATION_DEFAULT: bool = true;

pub fn default_keymap() -> HashMap<Action, KeyEvent> {
    HashMap::from([
        (
            Action::MoveUp,
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
        ),
        (
            Action::MoveDown,
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        ),
        (
            Action::Expand,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        ),
        (
            Action::Collapse,
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        ),
        (
            Action::Open,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        ),
        (
            Action::ExitFullscreenPreview,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        ),
        (
            Action::SwitchFocus,
            KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
        ),
        (
            Action::PageUp,
            KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE),
        ),
        (
            Action::PageDown,
            KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
        ),
        (
            Action::PreviewScrollUp,
            KeyEvent::new(KeyCode::Char('\''), KeyModifiers::NONE),
        ),
        (
            Action::PreviewScrollDown,
            KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
        ),
        (
            Action::TogglePreviewLineNumbers,
            KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
        ),
        (
            Action::TogglePreviewWrap,
            KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
        ),
        (
            Action::ToggleHelp,
            KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
        ),
        (
            Action::ToggleHidden,
            KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
        ),
        (
            Action::ResizePreviewNarrower,
            KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL),
        ),
        (
            Action::ResizePreviewWider,
            KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL),
        ),
        (
            Action::Quit,
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        ),
    ])
}

pub fn action_from_name(name: &str) -> Option<Action> {
    match name {
        "move_up" => Some(Action::MoveUp),
        "move_down" => Some(Action::MoveDown),
        "expand_node" => Some(Action::Expand),
        "collapse_node" => Some(Action::Collapse),
        "open_node" => Some(Action::Open),
        "exit_fullscreen_preview" => Some(Action::ExitFullscreenPreview),
        "switch_focus" => Some(Action::SwitchFocus),
        "page_up" => Some(Action::PageUp),
        "page_down" => Some(Action::PageDown),
        "preview_scroll_up" => Some(Action::PreviewScrollUp),
        "preview_scroll_down" => Some(Action::PreviewScrollDown),
        "toggle_preview_line_numbers" => Some(Action::TogglePreviewLineNumbers),
        "toggle_preview_wrap" => Some(Action::TogglePreviewWrap),
        "toggle_help" => Some(Action::ToggleHelp),
        "toggle_hidden" => Some(Action::ToggleHidden),
        "resize_preview_narrower" => Some(Action::ResizePreviewNarrower),
        "resize_preview_wider" => Some(Action::ResizePreviewWider),
        "quit" => Some(Action::Quit),
        _ => None,
    }
}

pub fn parse_key_combo(value: &str) -> Result<KeyEvent> {
    let lower = value.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split('+').collect();
    let mut mods = KeyModifiers::NONE;
    let mut code = None;

    for p in parts {
        match p.trim() {
            "ctrl" => mods |= KeyModifiers::CONTROL,
            "alt" => mods |= KeyModifiers::ALT,
            "shift" => mods |= KeyModifiers::SHIFT,
            "up" => code = Some(KeyCode::Up),
            "down" => code = Some(KeyCode::Down),
            "left" => code = Some(KeyCode::Left),
            "right" => code = Some(KeyCode::Right),
            "enter" => code = Some(KeyCode::Enter),
            "tab" => code = Some(KeyCode::Tab),
            "pageup" => code = Some(KeyCode::PageUp),
            "pagedown" => code = Some(KeyCode::PageDown),
            "esc" => code = Some(KeyCode::Esc),
            single if single.len() == 1 => {
                code = Some(KeyCode::Char(single.chars().next().unwrap_or(' ')));
            }
            _ => return Err(anyhow!("invalid key combo: {value}")),
        }
    }

    Ok(KeyEvent::new(
        code.ok_or_else(|| anyhow!("missing key code in combo: {value}"))?,
        mods,
    ))
}
