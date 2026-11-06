//! Compare on a practice sheet (D99): the original laid over the copy as the copy is now, as often
//! as the hand asks, with nothing judged but colours. A line's original, or a group of combined
//! lines, lies in place (where it lies in its frame, so placement shows) or aligned (moved and
//! evenly sized onto the copy's best fit, the group's lines fitted together as one shape,
//! `openpaint_practice::align`, so the shape alone shows). A single shape's picture lies faint
//! over the copy with its outline in red, in place or aligned (the outline fitted onto the copy's
//! lines, `align_shape`, the picture moved with it); a picture without an outline lies in place
//! only. The fit is found off the frame loop (`FitJob`, the runtime's worker) and shows when ready;
//! a copy no fit can be found for is shown in place in both. A colour's swatch and the colours
//! laid swap frames. A painted copy (a colour combination, a value scale, a colour copy) has its
//! original over it in place as strong as the user sets, and is read from its pixels
//! (`practice_read.rs`). A box's, an ellipse's, an estimate's or a point in space's copy is
//! checked as geometry against its exact answer, at once on the frame loop
//! (`practice_checks.rs`). A division (S95) has its picture laid under its lines, which are
//! counted and measured per level, and a whole copy its picture laid over it, its division
//! hidden: both in place and as strongly as the user sets. What lies over the copy is
//! `practice_overlay.rs`.
//!
//! The first Compare stores the exercise's result: when, the sheet's next stroke id (every stroke
//! in the frame from it on is a correction) and a colour's numbers (a painted copy's when its
//! pixels arrive) or an estimate's or a point in space's error, which judge the eye before the
//! answer was seen; every later one counts. A drawn exercise's pen turns blue. Anything laid in
//! the frame afterwards takes the comparison away: corrections are made by eye.

use std::sync::Arc;

use openpaint_core::{Artwork, RegionMask};
use openpaint_practice::{
    DivisionCheck, ExerciseResult, Family, Fit, Frame, Metrics, Original, Rgb8, score_colour,
};

use super::read::Reading;
use super::{PracticeSheet, unavailable};
use crate::error::AppError;
use crate::layer_rules::LayerRefusal;
use crate::practice::{Overlay, Pen, PracticeNotice};
use crate::practice_copy::{ALL_STROKES, colour_copy, holds_copy, line_pieces};
use crate::practice_pictures::FitJob;
use crate::projection::Availability;

/// The check over the shown copy, as Compare found the copy.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Compared {
    /// A drawn copy (lines, a group or a shape) and the original's fit onto it.
    Drawn(Fitting),
    Colour {
        original: Rgb8,
        first: Rgb8,
        last: Rgb8,
    },
    /// A painted copy and what its pixels read as.
    Painted(Reading),
    /// A box's or an ellipse's copy, its geometry checked (`Overlay::Box`, `FreeBoxes`,
    /// `Ellipse`).
    Checked(Overlay),
    /// A division's lines read (S95): its picture lies under them as strongly as the user sets.
    /// Lines drawn after it are the division's own: no correcting pen.
    Divided(Arc<DivisionCheck>),
}

/// The aligned overlay's fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Fitting {
    /// Asked of the worker by the Compare numbered so.
    Asked(u64),
    /// Found; none when the copy had no length: in place.
    Found(Option<Fit>),
    /// None to be had: a picture without an outline.
    Unavailable,
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
            (self.compared, self.fit_job) = (None, None);
            return Ok(());
        }
        let frame = self.sheet.pairs[index].copy;
        let nothing = || unavailable(Availability::NothingLeft);
        if !self.compare_offered(Some(index), artwork).is_available() {
            return Err(nothing());
        }
        self.forget_probe();
        let compared = self.compared_now(index, artwork, &frame)?;
        match self
            .record
            .results
            .iter_mut()
            .find(|r| r.index as usize == index)
        {
            Some(result) => result.compares = result.compares.saturating_add(1),
            None => {
                let metrics = self.numbers(artwork, index, &compared);
                let index = u32::try_from(index).unwrap_or(u32::MAX);
                let next = artwork.next_stroke_id();
                (self.record.results).push(ExerciseResult::first(index, now, next, metrics));
            }
        }
        self.changed = true;
        if matches!(compared, Compared::Drawn(_) | Compared::Checked(_)) {
            self.pen = Pen::Correct;
        }
        self.compared = Some(compared);
        self.construction = false;
        Ok(())
    }

    /// Whether the shown comparison has a construction to lay over it: whatever is checked as
    /// geometry (S97: a box's, free boxes', an ellipse's; S98: an estimate's, a trick's, a solid's,
    /// an object's).
    pub(super) fn construction_offered(&self) -> bool {
        matches!(self.compared, Some(Compared::Checked(_)))
    }

    /// Exercise `index`'s copy in `frame` of `artwork` as Compare finds it now: a colour's
    /// colours, a painted copy asked of the runtime, a box's or an ellipse's geometry checked, a
    /// drawn copy's fit asked of the worker; nothing laid, nothing to compare.
    fn compared_now(
        &mut self,
        index: usize,
        artwork: &Artwork,
        frame: &Frame,
    ) -> Result<Compared, AppError> {
        let nothing = || unavailable(Availability::NothingLeft);
        Ok(match &self.sheet.originals[index] {
            Original::Colour(original) => {
                let picks = colour_copy(artwork, frame).picks;
                let (Some(&first), Some(&last)) = (picks.first(), picks.last()) else {
                    return Err(nothing());
                };
                Compared::Colour {
                    original: *original,
                    first,
                    last,
                }
            }
            _ if !holds_copy(artwork, frame) => return Err(nothing()),
            // A colour combination, a crop's (S101) or a generated one's: read against its areas.
            _ if self.sheet.areas(index).is_some() => Compared::Painted(self.ask_read(index)),
            // A division: its lines counted, the picture under them (S95); a whole copy: the
            // picture over it, its division hidden; both as strongly as the slider sets.
            Original::Picture(_) if self.family(index) == Some(Family::Division) => {
                let check = self.division(index, artwork).ok_or_else(nothing)?;
                Compared::Divided(Arc::new(check))
            }
            Original::Picture(_) if self.family(index) == Some(Family::WholeCopy) => {
                Compared::Painted(Reading::Shown)
            }
            // Value masses (S96): the copy read against the picture's answer.
            Original::Picture(_) if self.family(index) == Some(Family::ValueMasses) => {
                Compared::Painted(self.ask_read(index))
            }
            Original::Scale(_) => Compared::Painted(self.ask_read(index)),
            Original::Picture(_) if !self.drawn(index) => Compared::Painted(Reading::Shown),
            Original::Lines(_) | Original::Picture(_) => {
                Compared::Drawn(self.ask_fit(index, artwork, frame))
            }
            geometry => {
                let checked = self.checked(index, geometry, (artwork, frame));
                Compared::Checked(checked.ok_or_else(nothing)?)
            }
        })
    }

    /// Whether Compare is offered on exercise `current` of `artwork`: once it holds a copy or was
    /// compared, unless its picture was removed (nothing is left to compare it with).
    pub(super) fn compare_offered(
        &self,
        current: Option<usize>,
        artwork: &Artwork,
    ) -> Availability {
        let Some(index) = current else {
            return Availability::NothingLeft;
        };
        let removed = matches!(self.sheet.originals[index], Original::Picture(_))
            && self.sheet.pictures[index].is_none();
        let held = self.record.result(index).is_some()
            || (self.pair(index)).is_some_and(|pair| holds_copy(artwork, &pair.copy));
        if held && !removed {
            Availability::Available
        } else {
            Availability::NothingLeft
        }
    }

    /// The fit for exercise `index`'s copy in `frame`, handed to the worker: the original's
    /// lines, or its picture's outline, onto the copy's drawn lines (the sketch pen's left out).
    fn ask_fit(&mut self, index: usize, artwork: &Artwork, frame: &Frame) -> Fitting {
        let original = match &self.sheet.originals[index] {
            Original::Lines(lines) => Some((lines.clone(), false)),
            Original::Picture(_) => (self.sheet.pictures[index].as_ref())
                .and_then(|picture| picture.outline.clone())
                .map(|outline| (outline, true)),
            _ => None,
        };
        let Some((original, shape)) = original else {
            self.fit_job = None;
            return Fitting::Unavailable;
        };
        self.fits += 1;
        self.fit_job = Some(FitJob {
            sequence: self.fits,
            original,
            copy: line_pieces(artwork, frame, ALL_STROKES),
            shape,
        });
        Fitting::Asked(self.fits)
    }

    /// The fit for the Compare numbered `sequence` was found (`None`: the copy had no length);
    /// a later Compare's or none waiting drops it.
    pub(crate) fn fitted(&mut self, sequence: u64, fit: Option<Fit>) {
        if let Some(Compared::Drawn(fitting)) = &mut self.compared
            && *fitting == Fitting::Asked(sequence)
        {
            *fitting = Fitting::Found(fit);
        }
    }

    /// The fit to find, once: the runtime's worker takes it.
    pub(crate) fn fit_request(&mut self) -> Option<FitJob> {
        self.fit_job.take()
    }

    /// A colour's numbers now, or an estimate's or a point in space's error as `compared` found
    /// it; other drawn exercises have none, a painted one's come with its pixels.
    fn numbers(&self, artwork: &Artwork, index: usize, compared: &Compared) -> Metrics {
        let frame = self.sheet.pairs[index].copy;
        match (compared, &self.sheet.originals[index]) {
            (_, Original::Colour(rgb)) => (score_colour(*rgb, &colour_copy(artwork, &frame)))
                .map(|score| score.metrics())
                .unwrap_or_default(),
            (Compared::Checked(Overlay::Flat(check)), _) => check.metrics.clone(),
            (Compared::Checked(Overlay::Space(check)), _) => check.metrics.clone(),
            (Compared::Checked(Overlay::Solid(check) | Overlay::Object(check)), _) => {
                check.metrics.clone()
            }
            (Compared::Divided(check), _) => check.metrics.clone(),
            // A whole copy keeps its own division's numbers.
            _ if self.family(index) == Some(Family::WholeCopy) => (self.division(index, artwork))
                .map(|check| check.metrics)
                .unwrap_or_default(),
            _ => Metrics::new(),
        }
    }

    /// The region a fill of the shown colour's, colour combination's or value scale's cell's
    /// frame covers, of value masses' (S96) or, while a painting pen is chosen, of a whole copy's
    /// (its colour blocks, S96): the frame, the picture's shape (S99) (with `selection`); or why
    /// none: not one of those shown, or the active layer of `artwork` hidden or locked (the
    /// notice says so). Filling takes the comparison away; a value scale's next cell comes in
    /// hand.
    pub(crate) fn fill_region(
        &mut self,
        artwork: &Artwork,
        selection: Option<&RegionMask>,
    ) -> Result<RegionMask, AppError> {
        let family = self.current().and_then(|index| self.family(index));
        let fills = matches!(
            family,
            Some(
                Family::Colour
                    | Family::Areas
                    | Family::Scale
                    | Family::ValueMasses
                    | Family::KeyShift
            )
        );
        let blocks = family == Some(Family::WholeCopy) && self.pen.paints();
        let Some(clip) = self.clip.clone().filter(|_| fills || blocks) else {
            return Err(unavailable(Availability::NothingLeft));
        };
        if let Some(refusal) = LayerRefusal::of(artwork) {
            self.notice = Some(PracticeNotice::CannotFill(refusal));
            return Err(unavailable(Availability::NothingLeft));
        }
        let region = match selection {
            Some(selection) => clip.intersect(selection),
            None => Some(clip),
        };
        self.laid();
        self.filled_cell();
        region.ok_or(unavailable(Availability::NothingLeft))
    }

    /// Something was laid in the frame (a fill): the comparison goes.
    pub(crate) fn laid(&mut self) {
        (self.compared, self.fit_job) = (None, None);
    }
}
