//! Compare on a practice sheet (D99): the original laid over the copy as the copy is now, as often
//! as the hand asks, with nothing judged but colours. A line's original lies in place (where it
//! lies in its frame, so placement shows) or aligned (moved and evenly sized onto the copy's best
//! fit, `openpaint_practice::align`, so the shape alone shows); a copy no fit can be found for is
//! shown in place in both. A colour's swatch and the colours laid swap frames.
//!
//! The first Compare stores the exercise's result: when, the sheet's next stroke id (every stroke
//! in the frame from it on is a correction) and a colour's numbers, which judge the eye before
//! the answer was seen; every later one counts. A line's pen turns blue. Anything laid in the
//! frame afterwards takes the comparison away: corrections are made by eye.

use openpaint_core::{Artwork, RegionMask};
use openpaint_practice::{
    ExerciseResult, Fit, Metrics, Original, Point, Rgb8, align, score_colour,
};

use super::{PracticeSheet, unavailable};
use crate::error::AppError;
use crate::layer_rules::LayerRefusal;
use crate::practice::{Overlay, Pen, PracticeNotice};
use crate::practice_copy::{colour_copy, holds_copy, line_pieces};
use crate::projection::Availability;

/// The check over the shown copy, as Compare found the copy.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Compared {
    /// The original's fit onto the copy, for the aligned overlay.
    Line(Fit),
    Colour {
        original: Rgb8,
        first: Rgb8,
        last: Rgb8,
    },
}

impl PracticeSheet {
    /// The comparison shown (`on`) or taken away.
    pub(super) fn compare(
        &mut self,
        on: bool,
        artwork: &Artwork,
        now: u64,
    ) -> Result<(), AppError> {
        let Some(index) = self.current() else {
            return Err(unavailable(Availability::NothingLeft));
        };
        self.instructions = None;
        if !on {
            self.compared = None;
            return Ok(());
        }
        let frame = self.sheet.pairs[index].copy;
        let nothing = || unavailable(Availability::NothingLeft);
        let compared = match &self.sheet.originals[index] {
            Original::Line(line) => {
                if !holds_copy(artwork, &frame) {
                    return Err(nothing());
                }
                let pieces = line_pieces(artwork, &frame, &self.pens);
                let pieces: Vec<&[Point]> = pieces.iter().map(Vec::as_slice).collect();
                Compared::Line(align(&line.points, &pieces).unwrap_or(Fit::SAME))
            }
            Original::Colour(original) => {
                let picks = colour_copy(artwork, &frame).picks;
                let (Some(&first), Some(&last)) = (picks.first(), picks.last()) else {
                    return Err(nothing());
                };
                Compared::Colour {
                    original: *original,
                    first,
                    last,
                }
            }
        };
        match self
            .record
            .results
            .iter_mut()
            .find(|r| r.index as usize == index)
        {
            Some(result) => result.compares = result.compares.saturating_add(1),
            None => {
                let metrics = self.numbers(artwork, index);
                let index = u32::try_from(index).unwrap_or(u32::MAX);
                let next = artwork.next_stroke_id();
                (self.record.results).push(ExerciseResult::first(index, now, next, metrics));
            }
        }
        self.changed = true;
        if matches!(compared, Compared::Line(_)) {
            self.pen = Pen::Correct;
        }
        self.compared = Some(compared);
        Ok(())
    }

    /// A colour's numbers now; a line has none.
    fn numbers(&self, artwork: &Artwork, index: usize) -> Metrics {
        let frame = self.sheet.pairs[index].copy;
        match &self.sheet.originals[index] {
            Original::Colour(rgb) => (score_colour(*rgb, &colour_copy(artwork, &frame)))
                .map(|score| score.metrics())
                .unwrap_or_default(),
            Original::Line(_) => Metrics::new(),
        }
    }

    /// The region a fill of the shown colour's frame covers (with `selection`), or why none: not
    /// a colour shown, or the active layer of `artwork` hidden or locked (the notice says so).
    /// Filling takes the comparison away.
    pub(crate) fn fill_region(
        &mut self,
        artwork: &Artwork,
        selection: Option<&RegionMask>,
    ) -> Result<RegionMask, AppError> {
        let colour = (self.current()).is_some_and(|index| !self.is_line(index));
        let clip = self.clip.as_ref().filter(|_| colour);
        let Some(clip) = clip else {
            return Err(unavailable(Availability::NothingLeft));
        };
        if let Some(refusal) = LayerRefusal::of(artwork) {
            self.notice = Some(PracticeNotice::CannotFill(refusal));
            return Err(unavailable(Availability::NothingLeft));
        }
        let region = match selection {
            Some(selection) => clip.intersect(selection),
            None => Some(clip.clone()),
        };
        self.laid();
        region.ok_or(unavailable(Availability::NothingLeft))
    }

    /// Something was laid in the frame (a fill): the comparison goes.
    pub(crate) fn laid(&mut self) {
        self.compared = None;
    }

    /// What lies over exercise `index`'s copy while comparing.
    pub(super) fn overlay(&self, index: usize) -> Option<Overlay> {
        let frame = self.pair(index)?.copy;
        match (self.compared.as_ref()?, &self.sheet.originals[index]) {
            (Compared::Line(fit), Original::Line(line)) => {
                let fit = if self.aligned { *fit } else { Fit::SAME };
                let canvas = |p: &Point| frame.to_canvas(fit.apply(*p)).map(|v| v as f32);
                Some(Overlay::Line(line.points.iter().map(canvas).collect()))
            }
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
}
