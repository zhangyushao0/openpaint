//! The storage port: `Project`, the one type that crosses into the store, and `ProjectStore`,
//! the trait the store implements. The app never sees keys, files or encodings.

use std::fmt;

use openpaint_core::{Artwork, CoreError};

/// Identifies a project in the library for its whole life; never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectId(u64);

impl ProjectId {
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A stored artwork. History is not stored; editor state joins once there is some (brushes, S11).
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub id: ProjectId,
    pub artwork: Artwork,
}

/// The local artwork library. Calls block and may be slow; the runtime makes them on a worker,
/// never on the UI thread.
pub trait ProjectStore: Send {
    /// The project opened or saved last; `None` when the library is empty.
    ///
    /// # Errors
    /// Any `PortError`; the stored data stays as it is.
    fn open_last(&mut self) -> Result<Option<Project>, PortError>;

    /// Stores `artwork` as a new project, which becomes the last one.
    ///
    /// # Errors
    /// Any `PortError`; nothing is stored then.
    fn create(&mut self, artwork: &Artwork) -> Result<ProjectId, PortError>;

    /// Makes `artwork` the stored content of project `id`, all or nothing.
    ///
    /// # Errors
    /// Any `PortError`; the project then keeps the content saved before.
    fn save(&mut self, id: ProjectId, artwork: &Artwork) -> Result<(), PortError>;
}

/// Why a port (storage now; pickers and assets later) could not serve a call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortError {
    /// The device refused: storage full, files locked or gone, an I/O error. `detail` is the
    /// system's message, for the log.
    Device { detail: String },
    /// Stored data cannot be decoded; `what` names the record.
    Corrupt { what: &'static str },
    /// Stored by a newer version of the app, which this one cannot read.
    NewerSchema { found: u32, newest: u32 },
    /// Stored data decodes but breaks an artwork rule.
    Invalid(CoreError),
    /// The worker serving the port stopped in the middle of the call (a bug, logged).
    Stopped,
}

impl fmt::Display for PortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Device { detail } => write!(f, "storage error: {detail}"),
            Self::Corrupt { what } => write!(f, "stored data is corrupt: {what}"),
            Self::NewerSchema { found, newest } => write!(
                f,
                "stored with schema {found}; this version reads up to {newest}"
            ),
            Self::Invalid(error) => write!(f, "stored artwork is invalid: {error}"),
            Self::Stopped => f.write_str("the storage worker stopped"),
        }
    }
}

impl std::error::Error for PortError {}

impl From<CoreError> for PortError {
    fn from(error: CoreError) -> Self {
        Self::Invalid(error)
    }
}
