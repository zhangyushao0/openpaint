//! The gallery as one UI frame sees it (`gallery.rs`): the artworks listed, whether they are
//! listed yet, whether an artwork is being opened, the notice, and the Today card the app's
//! practice log adds.

use std::sync::Arc;

use crate::gallery::GalleryNotice;
use crate::practice::TodayProjection;
use crate::project::ProjectSummary;
use crate::projection::Availability;

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
