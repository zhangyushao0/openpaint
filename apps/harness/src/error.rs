//! `HarnessError`: why a harness command failed. Printed once by `main`.

use std::{fmt, io};

use openpaint_render::RenderError;

#[derive(Debug)]
pub enum HarnessError {
    Render(RenderError),
    Io(io::Error),
    Png(png::EncodingError),
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Render(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "file error: {error}"),
            Self::Png(error) => write!(f, "PNG encoding error: {error}"),
        }
    }
}

impl From<RenderError> for HarnessError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}

impl From<io::Error> for HarnessError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<png::EncodingError> for HarnessError {
    fn from(error: png::EncodingError) -> Self {
        Self::Png(error)
    }
}
