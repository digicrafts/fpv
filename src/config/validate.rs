use crate::config::keymap::Action;
use crossterm::event::KeyEvent;
use std::collections::{HashMap, HashSet};

pub fn validate_bindings(bindings: &HashMap<Action, KeyEvent>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut warnings = Vec::new();

    for (action, key) in bindings {
        let key_repr = format!("{:?}:{:?}", key.code, key.modifiers);
        if !seen.insert(key_repr.clone()) {
            warnings.push(format!(
                "conflict detected for action {:?}: {key_repr}",
                action
            ));
        }
    }

    warnings
}
