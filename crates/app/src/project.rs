//! The storage port: `Project`, the one artwork type that crosses into the store, the gallery's
//! `ProjectSummary`, and `ProjectStore`, the trait the store implements. The app never sees
//! keys, files or encodings.

use std::fmt;
use std::sync::Arc;

use openpaint_core::{Artwork, CoreError};

use crate::title::Title;
use crate::toolbox::EditorState;

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

/// A stored artwork and its title. History is not stored; the editor state (`EditorState`) is
/// stored apart.
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub id: ProjectId,
    pub title: Title,
    pub artwork: Artwork,
}

/// A small picture of an artwork for the gallery: 8-bit RGBA, rows top first, tightly packed.
/// Derived from the artwork when it is left; cloning shares the pixels.
#[derive(Debug, Clone)]
pub struct Thumbnail {
    size: [u32; 2],
    pixels: Arc<[u8]>,
}

impl Thumbnail {
    /// `None` unless `pixels` holds exactly `width * height * 4` bytes and no side is zero.
    #[must_use]
    pub fn new(size: [u32; 2], pixels: Vec<u8>) -> Option<Self> {
        let expected = u64::from(size[0]) * u64::from(size[1]) * 4;
        (!size.contains(&0) && pixels.len() as u64 == expected).then(|| Self {
            size,
            pixels: pixels.into(),
        })
    }

    /// `[width, height]` in pixels.
    #[must_use]
    pub const fn size(&self) -> [u32; 2] {
        self.size
    }

    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// True when both share one pixel buffer: the same thumbnail, without comparing pixels.
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.pixels, &other.pixels)
    }
}

impl PartialEq for Thumbnail {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && (self.same(other) || self.pixels == other.pixels)
    }
}

/// One artwork as the gallery lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectSummary {
    pub id: ProjectId,
    pub title: Title,
    /// `None` until the artwork has been left once since it was created.
    pub thumbnail: Option<Thumbnail>,
}

/// The local artwork library. Calls block and may be slow; the runtime makes them on a worker,
/// never on the UI thread. "Open" is the artwork the user works on: the next start reopens it.
/// Every call that fails leaves the stored data as it was.
///
/// # Errors
/// Every call returns a `PortError` when it cannot be served.
pub trait ProjectStore: Send {
    /// The artwork open when the app was left; `None` when none was (an empty library, or the
    /// user was in the gallery).
    fn open_last(&mut self) -> Result<Option<Project>, PortError>;

    /// Every artwork, the one changed last first.
    fn list(&mut self) -> Result<Vec<ProjectSummary>, PortError>;

    /// Stores `artwork` as a new project titled `title`, which becomes the open one.
    fn create(&mut self, artwork: &Artwork, title: &Title) -> Result<ProjectId, PortError>;

    /// Project `id`, which becomes the open one.
    fn load(&mut self, id: ProjectId) -> Result<Project, PortError>;

    /// Makes `artwork` the stored content of project `id`, all or nothing.
    fn save(&mut self, id: ProjectId, artwork: &Artwork) -> Result<(), PortError>;

    /// Replaces the gallery picture of project `id`.
    fn save_thumbnail(&mut self, id: ProjectId, thumbnail: &Thumbnail) -> Result<(), PortError>;

    /// No artwork is open any more: the next start shows the gallery.
    fn close(&mut self) -> Result<(), PortError>;

    fn rename(&mut self, id: ProjectId, title: &Title) -> Result<(), PortError>;

    /// A copy of project `id` with the same title, listed first; returns the copy's id.
    fn duplicate(&mut self, id: ProjectId) -> Result<ProjectId, PortError>;

    /// Removes project `id` and everything stored for it, for good. Deleting a project that is
    /// not there succeeds.
    fn delete(&mut self, id: ProjectId) -> Result<(), PortError>;

    /// The toolbox as it was last stored; `None` before it ever was. Kept apart from every
    /// artwork: an error here never keeps an artwork from opening.
    fn editor(&mut self) -> Result<Option<EditorState>, PortError>;

    /// Replaces the stored toolbox.
    fn save_editor(&mut self, state: &EditorState) -> Result<(), PortError>;
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
    /// The project asked for is not in the library.
    Missing(ProjectId),
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
            Self::Missing(id) => write!(f, "project {} is not in the library", id.get()),
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
