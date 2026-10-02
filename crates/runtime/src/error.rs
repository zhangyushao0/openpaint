//! `RuntimeError`: why the frame loop could not do what the host asked. The host turns it into
//! a log line or a user-facing message.

use std::fmt;

use openpaint_render::RenderError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    Render(RenderError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Render(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<RenderError> for RuntimeError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}
