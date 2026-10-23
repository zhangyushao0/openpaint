//! Why a transfer did not happen (`transfer.rs`): an export that did not reach its file, a
//! picture that could not be used as the reference, inserted or added to the user's own.

use std::fmt;

use crate::project::PortError;

/// Why an export did not reach its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// The system's save dialog could not be shown, or the file it gave could not be opened.
    Picker(PortError),
    /// The artwork could not be rendered (the GPU refused; the runtime logs why).
    Render,
    /// The pixels could not be encoded.
    Encode,
    /// The chosen file could not be written.
    Write(PortError),
}

/// Why an image could not be used as the reference or inserted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    /// The system's picker could not be shown, or the file it gave could not be opened.
    Picker(PortError),
    /// The file could not be read.
    Read(PortError),
    /// Not a PNG or JPEG file.
    Unsupported,
    /// A PNG or JPEG file that breaks its format.
    Damaged,
    /// More pixels or bytes than the app reads (`MAX_IMAGE_PIXELS`, `MAX_IMAGE_BYTES`).
    TooLarge,
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Picker(error) => write!(f, "the save dialog failed: {error}"),
            Self::Render => f.write_str("the artwork could not be rendered"),
            Self::Encode => f.write_str("the image could not be encoded"),
            Self::Write(error) => write!(f, "the file could not be written: {error}"),
        }
    }
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Picker(error) => write!(f, "the picker failed: {error}"),
            Self::Read(error) => write!(f, "the file could not be read: {error}"),
            Self::Unsupported => f.write_str("not a PNG or JPEG file"),
            Self::Damaged => f.write_str("a damaged image file"),
            Self::TooLarge => f.write_str("the image is too large"),
        }
    }
}

impl std::error::Error for ExportError {}
impl std::error::Error for ImageError {}
