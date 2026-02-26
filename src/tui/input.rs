use crate::config::keymap::Action;
use crossterm::event::KeyEvent;
use std::collections::HashMap;

pub fn map_key_to_action(key: KeyEvent, bindings: &HashMap<Action, KeyEvent>) -> Option<Action> {
    bindings
        .iter()
        .find_map(|(action, mapped)| if *mapped == key { Some(*action) } else { None })
}
