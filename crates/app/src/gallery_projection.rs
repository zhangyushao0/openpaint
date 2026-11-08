//! The gallery as one UI frame sees it (`gallery.rs`): the artworks listed, whether they are
//! listed yet, whether an artwork is being opened and the notice.

use std::sync::Arc;

use crate::project::ProjectSummary;
use crate::projection::Availability;

/// Why a chosen file did not open as an artwork.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileProblem {
    /// Not an OpenPaint file.
    NotOpenPaint,
    /// An OpenPaint file that breaks its format or check sum.
    Damaged,
    /// An OpenPaint file of another format, from another version of OpenPaint.
    OtherFormat,
    /// The file could not be read.
    Unreadable,
}

/// What the gallery tells the user, with an OK to close it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GalleryNotice {
    #[default]
    None,
    /// The library could not be listed.
    ListFailed,
    /// The library is of another format, from another version of OpenPaint; it is left as it is
    /// and nothing is written to it.
    OtherLibrary,
    /// The artwork could not be opened; it stays in the library as it was.
    OpenFailed,
    CreateFailed,
    /// A rename, duplicate or delete failed (an artwork's, or a mode's change); the gallery shows
    /// the library as it is.
    ChangeFailed,
    /// The artwork was saved as an OpenPaint file.
    FileSaved,
    FileSaveFailed,
    /// The chosen file was not added; nothing changed.
    FileRefused(FileProblem),
    /// The version could not be restored; nothing changed.
    RestoreFailed,
}

/// A snapshot of the gallery for one UI frame.
#[derive(Debug, Clone, PartialEq)]
pub struct GalleryProjection {
    pub(crate) items: Arc<[ProjectSummary]>,
    pub(crate) listed: bool,
    pub(crate) opening: bool,
    pub(crate) notice: GalleryNotice,
}

/// Every gallery action, unless an artwork is being `opening`.
pub(crate) const fn offered_unless(opening: bool) -> Availability {
    if opening {
        Availability::Running
    } else {
        Availability::Available
    }
}

impl GalleryProjection {
    /// The artworks, the one changed last first.
    #[must_use]
    pub fn items(&self) -> &[ProjectSummary] {
        &self.items
    }

    /// False until the library has been listed; an empty list before that is not "no artworks".
    #[must_use]
    pub const fn listed(&self) -> bool {
        self.listed
    }

    /// Whether the artworks can be opened, created, renamed, duplicated and deleted now.
    #[must_use]
    pub const fn availability(&self) -> Availability {
        offered_unless(self.opening)
    }

    #[must_use]
    pub const fn notice(&self) -> GalleryNotice {
        self.notice
    }
}
