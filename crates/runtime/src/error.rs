//! `RuntimeError`: why the frame loop could not do what the host asked. The host turns it into
//! a log line or a user-facing message.

use std::fmt;

use openpaint_app::AppError;
use openpaint_core::CoreError;
use openpaint_render::RenderError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    Render(RenderError),
    App(AppError),
    Core(CoreError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Render(error) => write!(f, "{error}"),
            Self::App(error) => write!(f, "{error}"),
            Self::Core(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<RenderError> for RuntimeError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}

impl From<AppError> for RuntimeError {
    fn from(error: AppError) -> Self {
        Self::App(error)
    }
}

impl From<CoreError> for RuntimeError {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}
