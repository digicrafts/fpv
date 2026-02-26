use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fpv::config::keymap::Action;
use fpv::config::load::{ThemeProfile, UserThemeConfig};
use fpv::config::merge::merge_theme_profile;
use fpv::config::validate::validate_bindings;
use std::collections::HashMap;

#[test]
fn duplicate_bindings_are_reported() {
    let mut map = HashMap::new();
    let k = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
    map.insert(Action::MoveUp, k);
    map.insert(Action::MoveDown, k);
    let warnings = validate_bindings(&map);
    assert!(!warnings.is_empty());
}

#[test]
fn toggle_hidden_conflict_is_reported() {
    let mut map = HashMap::new();
    let k = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
    map.insert(Action::ToggleHidden, k);
    map.insert(Action::Quit, k);
    let warnings = validate_bindings(&map);
    assert!(!warnings.is_empty());
}

#[test]
fn theme_merge_overrides_selected_keys_only() {
    let defaults = ThemeProfile::default();
    let overrides = UserThemeConfig {
        directory_color: Some("blue".to_string()),
        fallback_file_color: None,
        hidden_dim_enabled: Some(false),
        file_type_colors: HashMap::from([("md".to_string(), "magenta".to_string())]),
    };
    let merged = merge_theme_profile(defaults, &overrides);
    assert_eq!(merged.directory_color, "blue");
    assert!(!merged.hidden_dim_enabled);
    assert_eq!(
        merged.file_type_colors.get("md"),
        Some(&"magenta".to_string())
    );
}
