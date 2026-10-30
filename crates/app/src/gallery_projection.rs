//! The gallery as one UI frame sees it (`gallery.rs`): the artworks listed, whether they are
//! listed yet, whether an artwork is being opened, the notice, and the Today card the app's
//! practice log adds.

use std::sync::Arc;

use crate::practice_today::TodayProjection;
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
    /// A rename, duplicate or delete failed (an artwork's, a colour copy's or a picture's); the
    /// gallery shows the library as it is.
    ChangeFailed,
    /// The artwork was saved as an OpenPaint file.
    FileSaved,
    FileSaveFailed,
    /// The chosen file was not added; nothing changed.
    FileRefused(FileProblem),
    /// The version could not be restored; nothing changed.
    RestoreFailed,
    /// The chosen practice pack is kept: its pictures come up in single shapes, or (a pack of
    /// compositions) in divisions and whole copies.
    PackImported {
        compositions: bool,
    },
    /// The chosen file was not kept as a practice pack; nothing changed.
    PackRefused(FileProblem),
    /// The chosen picture is among the user's own, with an outline found or none.
    PictureAdded {
        outline: bool,
    },
    /// The chosen picture was not added (not a PNG or JPEG one, or unreadable); nothing changed.
    PictureRefused,
}

/// A snapshot of the gallery for one UI frame.
#[derive(Debug, Clone, PartialEq)]
pub struct GalleryProjection {
    pub(crate) items: Arc<[ProjectSummary]>,
    pub(crate) listed: bool,
    pub(crate) opening: bool,
    pub(crate) notice: GalleryNotice,
    pub(crate) today: Option<TodayProjection>,
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

    /// The Today card, once the app's practice log is added (`with_today`).
    #[must_use]
    pub const fn today(&self) -> Option<&TodayProjection> {
        self.today.as_ref()
    }

    /// This projection with the Today card, which the app's practice log gives.
    #[must_use]
    pub fn with_today(self, today: TodayProjection) -> Self {
        Self {
            today: Some(today),
            ..self
        }
    }
}
