//! `PackError`: why a `practice-pack` command failed. Printed once by `main`.

use std::{fmt, io};

use openpaint_app::PortError;

#[derive(Debug)]
pub enum PackError {
    Io(io::Error),
    Png(png::EncodingError),
    /// Something the command needs was not produced, or its arguments are not the usage.
    Missing(&'static str),
    /// The store refused a pack or a picture.
    Port(PortError),
    /// A download failed (its URL and the client's or server's reason).
    Fetch {
        url: String,
        detail: String,
    },
    /// The source list cannot be read.
    List(String),
    /// An item cannot go into the pack (its id in the list, and why: a licence that disagrees
    /// with the list's, a picture that does not decode, a download that failed).
    Pack {
        item: String,
        problem: String,
    },
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "file error: {error}"),
            Self::Png(error) => write!(f, "PNG encoding error: {error}"),
            Self::Missing(what) => write!(f, "{what}"),
            Self::Port(error) => write!(f, "library: {error}"),
            Self::Fetch { url, detail } => write!(f, "{url}: {detail}"),
            Self::List(detail) => write!(f, "source list: {detail}"),
            Self::Pack { item, problem } => write!(f, "item {item}: {problem}"),
        }
    }
}

impl From<io::Error> for PackError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<png::EncodingError> for PackError {
    fn from(error: png::EncodingError) -> Self {
        Self::Png(error)
    }
}
