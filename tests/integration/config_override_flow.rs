use fpv::config::keymap::{default_keymap, Action, UserKeymap};
use fpv::config::merge::merge_keymaps;
use std::collections::HashMap;

#[test]
fn valid_override_is_applied_and_invalid_warns() {
    let defaults = default_keymap();
    let user = UserKeymap {
        mappings: HashMap::from([
            ("quit".to_string(), "ctrl+q".to_string()),
            ("bad".to_string(), "ctrl+*".to_string()),
        ]),
    };
    let (merged, warnings) = merge_keymaps(defaults, &user);
    assert!(merged.contains_key(&Action::Quit));
    assert!(!warnings.is_empty());
}
