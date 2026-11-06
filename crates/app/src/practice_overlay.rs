//! What lies over a compared copy on a practice sheet (`practice_compare.rs`): a line's original
//! or a group laid by the fit (in place or aligned), a single shape's picture and outline, a
//! painted copy's original as strong as the slider sets with what its pixels read as, a geometry
//! check, a division's picture under its lines (S95), value masses' answer over the copy with
//! their hatching (S96), a colour's swatches.

use std::sync::Arc;

use openpaint_practice::{Family, Fit, Original, Point};

use super::PracticeSheet;
use super::compare::{Compared, Fitting};
use super::read::{CopyRead, Reading};
use crate::practice::{Overlay, SHAPE_SHOWN};

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
        self.aligned && matches!(self.compared, Some(Compared::Drawn(Fitting::Asked(_))))
    }

    /// What lies over exercise `index`'s copy while comparing.
    pub(super) fn overlay(&self, index: usize) -> Option<Overlay> {
        let frame = self.pair(index)?.copy;
        let laid = |fit: Fit, p: &Point| frame.to_canvas(fit.apply(*p)).map(|v| v as f32);
        match (self.compared.as_ref()?, &self.sheet.originals[index]) {
            (Compared::Drawn(fitting), Original::Lines(lines)) => {
                let fit = self.laid_by(*fitting)?;
                let lines = lines
                    .iter()
                    .map(|line| line.iter().map(|p| laid(fit, p)).collect());
                Some(Overlay::Lines(lines.collect()))
            }
            (Compared::Drawn(fitting), Original::Picture(_)) => {
                let fit = self.laid_by(*fitting)?;
                self.picture_overlay(index, |p| laid(fit, p), (SHAPE_SHOWN, true))
            }
            (Compared::Painted(reading), _) => {
                self.painted_over(index, reading, |p| laid(Fit::SAME, p))
            }
            (Compared::Checked(checked), _) => Some(checked.clone()),
            (Compared::Divided(check), Original::Picture(_)) => Some(Overlay::Division {
                corners: self.picture_corners(index, |p| laid(Fit::SAME, p))?,
                shown: self.painted_overlay().0,
                check: Arc::clone(check),
            }),
            (
                Compared::Colour {
                    original,
                    first,
                    last,
                },
                Original::Colour(_),
            ) => Some(Overlay::Colour {
                original: *original,
                first: *first,
                last: *last,
            }),
            _ => None,
        }
    }

    /// What lies over exercise `index`'s painted copy, read as `reading`, its points laid in
    /// place by `laid`.
    fn painted_over(
        &self,
        index: usize,
        reading: &Reading,
        laid: impl Fn(&Point) -> [f32; 2],
    ) -> Option<Overlay> {
        let (shown, chosen, outline) = self.painted_overlay();
        let read = match reading {
            Reading::Read(read) => Some(read),
            _ => None,
        };
        match &self.sheet.originals[index] {
            // A key shift (S107): no answer, only what is off.
            _ if self.key_shift(index) => Some(self.key_overlay(reading)),
            // A colour combination, a crop's (S101) or a generated one's.
            _ if self.sheet.areas(index).is_some() => Some(Overlay::Areas {
                shown,
                read: match read {
                    Some(CopyRead::Areas(read)) => Some(Arc::clone(read)),
                    _ => None,
                },
                chosen,
                outline,
            }),
            Original::Picture(_) if self.family(index) == Some(Family::ValueMasses) => {
                Some(Overlay::Masses {
                    corners: self.picture_corners(index, laid)?,
                    shown,
                    read: match read {
                        Some(CopyRead::Masses(check)) => Some(Arc::clone(check)),
                        _ => None,
                    },
                })
            }
            // A colour copy compares colours: the picture alone, no outline.
            Original::Picture(_) => self.picture_overlay(index, laid, (shown, false)),
            Original::Scale(_) => Some(Overlay::Scale(match read {
                Some(CopyRead::Scale(check)) => Some(Arc::clone(check)),
                _ => None,
            })),
            _ => None,
        }
    }

    /// Exercise `index`'s picture over its copy, its points laid by `laid`, `shown` strong, with
    /// its outline where it has one if `outlined`.
    fn picture_overlay(
        &self,
        index: usize,
        laid: impl Fn(&Point) -> [f32; 2],
        (shown, outlined): (f32, bool),
    ) -> Option<Overlay> {
        let picture = self.sheet.pictures[index].as_ref()?;
        let outline = picture.outline.iter().filter(|_| outlined).flatten();
        Some(Overlay::Picture {
            corners: self.picture_corners(index, &laid)?,
            outline: outline
                .map(|line| line.iter().map(&laid).collect())
                .collect(),
            shown,
        })
    }

    /// Exercise `index`'s picture's corners (top left, top right, bottom right, bottom left), its
    /// frame's (S99), laid by `laid`.
    fn picture_corners(
        &self,
        index: usize,
        laid: impl Fn(&Point) -> [f32; 2],
    ) -> Option<[[f32; 2]; 4]> {
        let picture = self.sheet.pictures[index].as_ref()?;
        Some(picture.fit.corners().map(|p| laid(&p)))
    }
}
