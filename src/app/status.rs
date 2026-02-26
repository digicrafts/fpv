use crate::app::navigation_result::{ActionOutcome, NavigationActionResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub level: StatusLevel,
    pub text: String,
}

impl StatusMessage {
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Info,
            text: text.into(),
        }
    }

    pub fn warning(text: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Warning,
            text: text.into(),
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Error,
            text: text.into(),
        }
    }
}

pub fn navigation_status_message(result: &NavigationActionResult) -> String {
    match result.outcome {
        ActionOutcome::Changed => result.message.to_string(),
        ActionOutcome::Blocked | ActionOutcome::NoChange => result.message.to_string(),
    }
}
