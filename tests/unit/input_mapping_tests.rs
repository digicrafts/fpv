use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fpv::config::keymap::{action_from_name, default_keymap, Action};
use fpv::tui::input::map_key_to_action;

#[test]
fn key_event_maps_to_action() {
    let map = default_keymap();
    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    let action = map_key_to_action(key, &map);
    assert_eq!(action, Some(Action::MoveUp));
}

#[test]
fn default_toggle_hidden_mapping_exists() {
    let map = default_keymap();
    let key = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
    let action = map_key_to_action(key, &map);
    assert_eq!(action, Some(Action::ToggleHidden));
}

#[test]
fn default_toggle_help_mapping_exists() {
    let map = default_keymap();
    let key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
    let action = map_key_to_action(key, &map);
    assert_eq!(action, Some(Action::ToggleHelp));
}

#[test]
fn toggle_hidden_action_name_is_supported() {
    assert_eq!(
        action_from_name("toggle_hidden"),
        Some(Action::ToggleHidden)
    );
}

#[test]
fn resize_preview_action_names_are_supported() {
    assert_eq!(
        action_from_name("resize_preview_narrower"),
        Some(Action::ResizePreviewNarrower)
    );
    assert_eq!(
        action_from_name("resize_preview_wider"),
        Some(Action::ResizePreviewWider)
    );
}

#[test]
fn preview_scrolling_action_names_are_supported() {
    assert_eq!(
        action_from_name("preview_scroll_up"),
        Some(Action::PreviewScrollUp)
    );
    assert_eq!(
        action_from_name("preview_scroll_down"),
        Some(Action::PreviewScrollDown)
    );
}

#[test]
fn preview_line_numbers_and_fullscreen_action_names_are_supported() {
    assert_eq!(
        action_from_name("toggle_preview_line_numbers"),
        Some(Action::TogglePreviewLineNumbers)
    );
    assert_eq!(
        action_from_name("exit_fullscreen_preview"),
        Some(Action::ExitFullscreenPreview)
    );
    assert_eq!(
        action_from_name("toggle_preview_wrap"),
        Some(Action::TogglePreviewWrap)
    );
    assert_eq!(action_from_name("toggle_help"), Some(Action::ToggleHelp));
}

#[test]
fn default_resize_mappings_exist() {
    let map = default_keymap();
    let narrow = KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL);
    let wide = KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL);
    assert_eq!(
        map_key_to_action(narrow, &map),
        Some(Action::ResizePreviewNarrower)
    );
    assert_eq!(
        map_key_to_action(wide, &map),
        Some(Action::ResizePreviewWider)
    );
}

#[test]
fn default_preview_scroll_and_line_number_mappings_exist() {
    let map = default_keymap();
    let scroll_up = KeyEvent::new(KeyCode::Char('\''), KeyModifiers::NONE);
    let scroll_down = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
    let toggle_line_numbers = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
    let toggle_wrap = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(scroll_up, &map),
        Some(Action::PreviewScrollUp)
    );
    assert_eq!(
        map_key_to_action(scroll_down, &map),
        Some(Action::PreviewScrollDown)
    );
    assert_eq!(
        map_key_to_action(toggle_line_numbers, &map),
        Some(Action::TogglePreviewLineNumbers)
    );
    assert_eq!(
        map_key_to_action(toggle_wrap, &map),
        Some(Action::TogglePreviewWrap)
    );
}

#[test]
fn default_exit_fullscreen_mapping_exists() {
    let map = default_keymap();
    let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(esc, &map),
        Some(Action::ExitFullscreenPreview)
    );
}
