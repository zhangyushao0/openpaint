//! `AppError`: why the session refused an intent. The platform layer turns it into a log line
//! or a user-facing message.

use std::fmt;

use openpaint_core::CoreError;

use crate::projection::{Availability, EditorAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Core(CoreError),
    /// The projection does not offer `action` now, for the reason in `availability`.
    Unavailable {
        action: EditorAction,
        availability: Availability,
    },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => write!(f, "{error}"),
            Self::Unavailable {
                action,
                availability,
            } => write!(f, "{action} is unavailable: {availability}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<CoreError> for AppError {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}
