use crate::config::keymap::{action_from_name, parse_key_combo, Action, UserKeymap};
use crate::config::load::{ThemeProfile, UserThemeConfig};
use crossterm::event::KeyEvent;
use std::collections::HashMap;

pub fn merge_keymaps(
    mut defaults: HashMap<Action, KeyEvent>,
    user: &UserKeymap,
) -> (HashMap<Action, KeyEvent>, Vec<String>) {
    let mut warnings = Vec::new();

    for (name, combo) in &user.mappings {
        let Some(action) = action_from_name(name) else {
            warnings.push(format!("unknown action '{name}' ignored"));
            continue;
        };

        match parse_key_combo(combo) {
            Ok(key) => {
                defaults.insert(action, key);
            }
            Err(err) => warnings.push(err.to_string()),
        }
    }

    (defaults, warnings)
}

pub fn merge_theme_profile(defaults: ThemeProfile, user: &UserThemeConfig) -> ThemeProfile {
    let mut merged = defaults;
    if let Some(color) = &user.directory_color {
        merged.directory_color = color.to_ascii_lowercase();
    }
    if let Some(color) = &user.fallback_file_color {
        merged.fallback_file_color = color.to_ascii_lowercase();
    }
    if let Some(dim) = user.hidden_dim_enabled {
        merged.hidden_dim_enabled = dim;
    }
    for (ext, color) in &user.file_type_colors {
        merged
            .file_type_colors
            .insert(ext.to_ascii_lowercase(), color.to_ascii_lowercase());
    }
    merged
}
