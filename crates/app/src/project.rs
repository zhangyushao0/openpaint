//! The storage port: `Project`, the one artwork type that crosses into the store, the gallery's
//! `ProjectSummary`, and `ProjectStore`, the trait the store implements. The app never sees
//! keys, files or encodings.

use std::fmt;
use std::sync::Arc;

use openpaint_core::{Artwork, CoreError};

use crate::editor_state::EditorState;
use crate::title::Title;

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
    /// Its kept versions, the newest first.
    pub versions: Vec<KeptVersion>,
    /// Its records could not be upgraded to this build's schema and are kept as they were: it
    /// does not open, but can be saved as an OpenPaint file (S50).
    pub needs_repair: bool,
}

/// Most versions kept per artwork; keeping one more drops the oldest (DECISIONS D63).
pub const KEPT_VERSIONS: usize = 10;

/// Identifies a kept version within its artwork; never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionId(u64);

impl VersionId {
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// An artwork as it was stored when it was left after a change, kept to be restored as a copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeptVersion {
    pub id: VersionId,
    /// When it was kept, in seconds since 1970-01-01 UTC.
    pub kept_at: u64,
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

    /// Project `id` as it is stored, as the bytes of one OpenPaint file: the artwork and its
    /// title, without the toolbox or its versions.
    fn artwork_file(&mut self, id: ProjectId) -> Result<Vec<u8>, PortError>;

    /// A new project from the bytes of an OpenPaint file (older schemas upgraded), listed first;
    /// it never replaces one and does not become the open one. Returns its id.
    fn import_file(&mut self, bytes: &[u8]) -> Result<ProjectId, PortError>;

    /// Keeps project `id` as it is stored now as its newest version, kept at `kept_at` (seconds
    /// since 1970 UTC); the oldest beyond `KEPT_VERSIONS` goes.
    fn keep_version(&mut self, id: ProjectId, kept_at: u64) -> Result<(), PortError>;

    /// A new project titled `title` from version `version` of project `id`, listed first; the
    /// project and its versions stay as they are. Returns the new project's id.
    fn restore_version(
        &mut self,
        id: ProjectId,
        version: VersionId,
        title: &Title,
    ) -> Result<ProjectId, PortError>;
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
    /// Stored with an older schema whose upgrade failed on this artwork (a step met a record it
    /// could not read, or failed); its records are kept as they were (S50).
    NotUpgraded { schema: u32 },
    /// Stored data decodes but breaks an artwork rule.
    Invalid(CoreError),
    /// The project asked for is not in the library.
    Missing(ProjectId),
    /// A file opened as an OpenPaint file is something else.
    NotArtworkFile,
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
            Self::NotUpgraded { schema } => write!(
                f,
                "stored with schema {schema}; its records could not be upgraded"
            ),
            Self::Invalid(error) => write!(f, "stored artwork is invalid: {error}"),
            Self::Missing(id) => write!(f, "project {} is not in the library", id.get()),
            Self::NotArtworkFile => f.write_str("not an OpenPaint file"),
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
