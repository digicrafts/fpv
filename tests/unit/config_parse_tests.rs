use crossterm::event::KeyCode;
use fpv::config::keymap::{parse_key_combo, Action, UserKeymap};
use fpv::config::load::{ensure_default_config_exists, load_user_config, load_user_keymap};
use std::collections::HashMap;
use std::fs;
use tempfile::tempdir;

#[test]
fn parse_key_combo_accepts_ctrl_char() {
    let key = parse_key_combo("ctrl+k").expect("parse combo");
    assert_eq!(format!("{:?}", key.modifiers), "KeyModifiers(CONTROL)");
}

#[test]
fn parse_key_combo_accepts_esc_and_symbol_keys() {
    let esc = parse_key_combo("esc").expect("parse esc");
    assert_eq!(esc.code, KeyCode::Esc);

    let quote = parse_key_combo("'").expect("parse quote");
    assert_eq!(quote.code, KeyCode::Char('\''));

    let slash = parse_key_combo("/").expect("parse slash");
    assert_eq!(slash.code, KeyCode::Char('/'));
}

#[test]
fn parse_user_toml_file() {
    let d = tempdir().expect("create tempdir");
    let p = d.path().join("config.toml");
    fs::write(&p, "[mappings]\nquit = 'ctrl+q'\n").expect("write file");
    let keymap = load_user_keymap(&p).expect("load keymap");
    assert_eq!(keymap.mappings.get("quit"), Some(&"ctrl+q".to_string()));
}

#[test]
fn action_name_mapping_is_known() {
    let sample = UserKeymap {
        mappings: HashMap::from([("quit".to_string(), "q".to_string())]),
    };
    assert!(sample.mappings.contains_key("quit"));
    let _ = Action::Quit;
}

#[test]
fn parse_theme_values_from_config_file() {
    let d = tempdir().expect("create tempdir");
    let p = d.path().join("config.toml");
    fs::write(
        &p,
        "[mappings]\nquit = 'ctrl+q'\n\n[theme]\ndirectory_color='yellow'\nfallback_file_color='white'\nhidden_dim_enabled=true\n\n[theme.file_type_colors]\nrs='cyan'\n",
    )
    .expect("write file");
    let cfg = load_user_config(&p).expect("load config");
    assert_eq!(cfg.theme.directory_color.as_deref(), Some("yellow"));
    assert_eq!(
        cfg.theme.file_type_colors.get("rs"),
        Some(&"cyan".to_string())
    );
}

#[test]
fn parse_status_display_mode_from_config_file() {
    let d = tempdir().expect("create tempdir");
    let p = d.path().join("config.toml");
    fs::write(
        &p,
        "status_display_mode='title'\n\n[mappings]\nquit = 'ctrl+q'\n",
    )
    .expect("write file");
    let cfg = load_user_config(&p).expect("load config");
    assert_eq!(
        cfg.status_display_mode.map(|m| format!("{m:?}")),
        Some("Title".to_string())
    );
}

#[test]
fn ensure_default_config_creates_file_on_first_run() {
    let d = tempdir().expect("create tempdir");
    let config_path = d.path().join(".config/fpv/config");
    assert!(!config_path.exists());

    ensure_default_config_exists(&config_path).expect("create default config");
    assert!(config_path.exists());

    let cfg = load_user_config(&config_path).expect("load generated config");
    assert_eq!(cfg.mappings.get("toggle_help"), Some(&"?".to_string()));
    assert_eq!(cfg.mappings.get("toggle_hidden"), Some(&"h".to_string()));
}
