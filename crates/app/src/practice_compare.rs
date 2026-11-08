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
//! hidden: both in place and as strongly as the user sets. Whatever was found is a comparison
//! (`openpaint_practice::Comparison`, S109) whose marks and card the screen draws
//! (`practice_overlay.rs`).
//!
//! The first Compare stores the exercise's result: when, the sheet's next stroke id (every stroke
//! in the frame from it on is a correction) and a colour's numbers (a painted copy's when its
//! pixels arrive) or an estimate's or a point in space's error, which judge the eye before the
//! answer was seen; every later one counts. A drawn exercise's pen turns blue. Anything laid in
//! the frame afterwards takes the comparison away: corrections are made by eye.

use openpaint_core::{Artwork, RegionMask};
use openpaint_practice::{
    ColourCompared, Comparison, DivisionCompared, ExerciseResult, Family, Fit, Frame,
    LinesCompared, Metrics, Original, PictureCompared, ShapeCompared, score_colour,
};

use super::read::Reading;
use super::{PracticeSheet, unavailable};
use crate::error::AppError;
use crate::layer_rules::LayerRefusal;
use crate::practice::{Pen, PracticeNotice};
use crate::practice_copy::{ALL_STROKES, colour_copy, holds_copy, line_pieces};
use crate::practice_pictures::FitJob;
use crate::projection::Availability;

/// The comparison over the shown copy, as Compare found the copy.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Compared {
    /// A drawn copy (lines, a group or a shape) and the original's fit onto it.
    Drawn(Fitting, Comparison),
    /// A colour's colours, or a division's lines read (S95: lines drawn after it are the
    /// division's own, no correcting pen).
    Made(Comparison),
    /// A painted copy, what its pixels read as, and what shows until they are read.
    Painted(Reading, Comparison),
    /// A copy checked as geometry (S97, S98): a Construction to lay over it.
    Checked(Comparison),
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

impl Compared {
    /// What it shows now.
    pub(super) fn shown(&self) -> &Comparison {
        match self {
            Self::Painted(Reading::Read(read), _) => &read.comparison,
            Self::Drawn(_, shown)
            | Self::Made(shown)
            | Self::Painted(_, shown)
            | Self::Checked(shown) => shown,
        }
    }
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
        if matches!(compared, Compared::Drawn(..) | Compared::Checked(_)) {
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
        let corners = |sheet: &Self| {
            (sheet.sheet.pictures.get(index).and_then(Option::as_ref)).map(|p| p.fit.corners())
        };
        let sheet = std::sync::Arc::clone(&self.sheet);
        Ok(match &sheet.originals[index] {
            Original::Colour(original) => {
                let copy = colour_copy(artwork, frame);
                let (Some(&first), Some(&last)) = (copy.picks.first(), copy.picks.last()) else {
                    return Err(nothing());
                };
                let metrics = (score_colour(*original, &copy))
                    .map(|score| score.metrics())
                    .unwrap_or_default();
                Compared::Made(Comparison::of(ColourCompared {
                    original: *original,
                    first,
                    last,
                    metrics,
                }))
            }
            _ if !holds_copy(artwork, frame) => return Err(nothing()),
            // A division: its lines counted, the picture under them (S95).
            Original::Picture(_) if self.family(index) == Some(Family::Division) => {
                let check = self.division(index, artwork).ok_or_else(nothing)?;
                let corners = corners(self).ok_or_else(nothing)?;
                Compared::Made(Comparison::of(DivisionCompared { corners, check }))
            }
            _ if let Some((reading, unread)) = self.ask_read(index) => {
                Compared::Painted(reading, unread)
            }
            // A colour copy or a whole copy: its picture over it as strongly as the slider sets;
            // a whole copy keeps its own division's numbers.
            Original::Picture(_) if !self.drawn(index) || self.whole_copy(index) => {
                let corners = corners(self).ok_or_else(nothing)?;
                let metrics = (self.whole_copy(index))
                    .then(|| self.division(index, artwork))
                    .flatten()
                    .map(|check| check.metrics)
                    .unwrap_or_default();
                let shown = Comparison::of(PictureCompared { corners, metrics });
                Compared::Painted(Reading::Shown, shown)
            }
            Original::Lines(lines) => {
                let shown = Comparison::of(LinesCompared {
                    lines: lines.clone(),
                });
                Compared::Drawn(self.ask_fit(index, artwork, frame), shown)
            }
            Original::Picture(_) => {
                let picture = self.sheet.pictures.get(index).and_then(Option::as_ref);
                let picture = picture.ok_or_else(nothing)?;
                let shown = Comparison::of(ShapeCompared {
                    corners: picture.fit.corners(),
                    outline: picture.outline.clone(),
                });
                Compared::Drawn(self.ask_fit(index, artwork, frame), shown)
            }
            geometry => {
                let checked = self.checked(index, geometry, (artwork, frame));
                Compared::Checked(checked.ok_or_else(nothing)?)
            }
        })
    }

    /// Whether exercise `index` is a whole copy.
    fn whole_copy(&self, index: usize) -> bool {
        self.family(index) == Some(Family::WholeCopy)
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
        if let Some(Compared::Drawn(fitting, _)) = &mut self.compared
            && *fitting == Fitting::Asked(sequence)
        {
            *fitting = Fitting::Found(fit);
        }
    }

    /// The fit to find, once: the runtime's worker takes it.
    pub(crate) fn fit_request(&mut self) -> Option<FitJob> {
        self.fit_job.take()
    }

    /// The numbers a first Compare keeps as `compared` found them: a colour's, an estimate's, a
    /// point in space's, a solid's, an object's or a division's error or counts (a whole copy its
    /// own division's); a painted copy's come with its pixels.
    fn numbers(&self, _artwork: &Artwork, _index: usize, compared: &Compared) -> Metrics {
        compared.shown().metrics()
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
