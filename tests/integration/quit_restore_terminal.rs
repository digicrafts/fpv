use fpv::config::keymap::{default_keymap, Action};

#[test]
fn default_quit_binding_exists() {
    let map = default_keymap();
    assert!(map.contains_key(&Action::Quit));
}
