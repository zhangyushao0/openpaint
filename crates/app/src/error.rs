//! `AppError`: why the session refused an intent. The platform layer turns it into a log line
//! or a user-facing message.

use std::fmt;

use openpaint_core::CoreError;

use crate::adjust::AdjustRefusal;
use crate::fill::FillRefusal;
use crate::projection::{Availability, EditorAction};
use crate::transform::TransformRefusal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Core(CoreError),
    /// The projection does not offer `action` now, for the reason in `availability`.
    Unavailable {
        action: EditorAction,
        availability: Availability,
    },
    /// A fill cannot go on the active layer.
    FillRefused(FillRefusal),
    /// The active layer cannot be transformed.
    TransformRefused(TransformRefusal),
    /// The active layer cannot be adjusted.
    AdjustRefused(AdjustRefusal),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => write!(f, "{error}"),
            Self::FillRefused(refusal) => write!(f, "no fill: {refusal}"),
            Self::TransformRefused(refusal) => write!(f, "no transform: {refusal}"),
            Self::AdjustRefused(refusal) => write!(f, "no adjustment: {refusal}"),
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
