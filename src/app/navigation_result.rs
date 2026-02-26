use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionOutcome {
    Changed,
    Blocked,
    NoChange,
}

#[derive(Debug, Clone)]
pub struct NavigationActionResult {
    pub action: &'static str,
    pub outcome: ActionOutcome,
    pub new_path: PathBuf,
    pub message: String,
}

impl NavigationActionResult {
    pub fn changed(action: &'static str, new_path: PathBuf, message: impl Into<String>) -> Self {
        Self {
            action,
            outcome: ActionOutcome::Changed,
            new_path,
            message: message.into(),
        }
    }

    pub fn blocked(action: &'static str, new_path: PathBuf, message: impl Into<String>) -> Self {
        Self {
            action,
            outcome: ActionOutcome::Blocked,
            new_path,
            message: message.into(),
        }
    }

    pub fn no_change(action: &'static str, new_path: PathBuf, message: impl Into<String>) -> Self {
        Self {
            action,
            outcome: ActionOutcome::NoChange,
            new_path,
            message: message.into(),
        }
    }
}
