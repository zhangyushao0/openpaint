//! What lies over a compared copy on a practice sheet and what its card shows (S109): the shown
//! comparison's marks and card items (`openpaint_practice::Compared`) for what the user set: the
//! fit a drawn copy's original lies by (in place, or aligned once found), how strongly a picture
//! shows, the area the card compares, the Grey view, Construction and the point the eyedropper
//! compared; and the pictures its reading brought, which its marks lay.

use std::sync::Arc;

use openpaint_practice::{CardItem, Fit, Seen, SheetMark};

use super::PracticeSheet;
use super::compare::{Compared, Fitting};
use super::read::read_pictures;
use crate::pictures::PictureImage;

/// What a comparison shows: its marks, its card, and the pictures its reading brought.
type Shown = (
    Vec<SheetMark>,
    Vec<CardItem>,
    Option<Arc<Vec<PictureImage>>>,
);

impl PracticeSheet {
    /// The fit the overlay lies by now: none while the aligned one is being found.
    fn laid_by(&self, fitting: Fitting) -> Option<Fit> {
        if !self.aligned {
            return Some(Fit::SAME);
        }
        match fitting {
            Fitting::Asked(_) => None,
            Fitting::Found(fit) => Some(fit.unwrap_or(Fit::SAME)),
            Fitting::Unavailable => Some(Fit::SAME),
        }
    }

    /// Whether the aligned fit is chosen and still being found.
    pub(super) fn fitting(&self) -> bool {
        self.aligned && matches!(self.compared, Some(Compared::Drawn(Fitting::Asked(_), _)))
    }

    /// What lies over the shown copy while comparing, what the card shows of it, and the pictures
    /// its reading brought.
    pub(super) fn overlay(&self) -> Option<Shown> {
        let compared = self.compared.as_ref()?;
        let fit = match compared {
            Compared::Drawn(fitting, _) => self.laid_by(*fitting),
            _ => Some(Fit::SAME),
        };
        let (shown, chosen, outline, grey) = self.painted_overlay();
        let seen = Seen {
            construction: self.construction,
            shown,
            chosen,
            outline: &outline,
            grey,
            view: self.painting_state.view,
            fit,
            probe: self.probe_colours(),
        };
        let comparison = compared.shown();
        let pictures = match compared {
            Compared::Painted(_, state, _) => read_pictures(state),
            _ => None,
        };
        Some((comparison.marks(&seen), comparison.card(&seen), pictures))
    }
}
