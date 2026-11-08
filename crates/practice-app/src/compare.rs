//! Compare on a practice sheet (D99): the original laid over the copy as the copy is now, as often
//! as the hand asks. How the copy is compared is its kind's (`Exercise::compare`, S109b): the
//! copy is read off the artwork as the kind asks (`checks.rs`) and the kind says what it
//! found: shown at once (a colour's colours, a division's lines, a picture laid over the copy), its
//! original laid over it and fitted onto it for Aligned (a line's, a group's, a single shape's
//! outline: the fit found off the frame loop, `FitJob`, and shown when ready; none to be had, in
//! place in both), checked as geometry against its exact answer at once on the frame loop (a
//! Construction to lay over it), or read from its pixels off the frame loop
//! (`read.rs`). Whatever was found is a comparison (`openpaint_practice::Comparison`,
//! S109) whose marks and card the screen draws (`overlay.rs`).
//!
//! The first Compare stores the exercise's result: when, the sheet's next stroke id (every stroke
//! in the frame from it on is a correction) and the numbers its comparison keeps (a painted copy's
//! when its pixels arrive), which judge the eye before the answer was seen; every later one
//! counts. A drawn exercise's pen turns blue. Anything laid in the frame afterwards takes the
//! comparison away: corrections are made by eye.

use std::sync::Arc;

use openpaint_core::{Artwork, RegionMask};
use openpaint_practice::{
    Asked, Compare, Comparison, ExerciseResult, Fit, Frame, Reading, ToFit, Tools,
};

use super::checks::CopyOn;
use super::read::ReadState;
use super::{PracticeSheet, unavailable};
use crate::copy::{ALL_STROKES, holds_copy, line_pieces};
use crate::pictures::FitJob;
use crate::projection::{Pen, PracticeNotice};
use crate::refused::Refused;
use openpaint_app::Availability;
use openpaint_app::LayerRefusal;

/// The comparison over the shown copy, as Compare found the copy.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Compared {
    /// A drawn copy (lines, a group or a shape) and the original's fit onto it.
    Drawn(Fitting, Comparison),
    /// Shown as found: a colour's colours, a division's lines (S95: lines drawn after it are the
    /// division's own, no correcting pen), a picture laid over the copy.
    Made(Comparison),
    /// A painted copy: how it is read, what its pixels read as, and what shows until they are.
    Painted(Reading, ReadState, Comparison),
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
            Self::Painted(_, ReadState::Read(read), _) => &read.comparison,
            Self::Drawn(_, shown)
            | Self::Made(shown)
            | Self::Painted(_, _, shown)
            | Self::Checked(shown) => shown,
        }
    }
}

impl PracticeSheet {
    /// The comparison shown (`on`) or taken away.
    pub(super) fn compare(&mut self, on: bool, artwork: &Artwork, now: u64) -> Result<(), Refused> {
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
                let metrics = compared.shown().metrics();
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

    /// Exercise `index`'s copy in `frame` of `artwork` as its kind compares it now: shown, fitted
    /// on the worker, checked, or read on the worker; nothing laid, nothing to compare.
    fn compared_now(
        &mut self,
        index: usize,
        artwork: &Artwork,
        frame: &Frame,
    ) -> Result<Compared, Refused> {
        let nothing = || unavailable(Availability::NothingLeft);
        let sheet = Arc::clone(&self.sheet);
        let exercise = self
            .record
            .exercises
            .get(index)
            .ok_or_else(nothing)?
            .clone();
        let copy = CopyOn {
            artwork,
            frame,
            corrected_from: self.record.result(index).and_then(|r| r.corrected_from),
        };
        let asked = Asked {
            exercise: &exercise,
            original: &sheet.originals[index],
            picture: sheet.pictures.get(index).and_then(Option::as_deref),
            copy: &copy,
        };
        Ok(match exercise.compare(&asked).ok_or_else(nothing)? {
            Compare::Made(shown) => Compared::Made(shown),
            Compare::Checked(shown) => Compared::Checked(shown),
            Compare::Fitted(shown, fit) => {
                Compared::Drawn(self.ask_fit(fit, artwork, frame), shown)
            }
            Compare::Read(reading) => {
                let (state, unread) = self.ask_read(index, &reading);
                Compared::Painted(reading, state, unread)
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
        let removed =
            self.sheet.originals[index].picture().is_some() && self.sheet.pictures[index].is_none();
        let held = self.record.result(index).is_some()
            || (self.pair(index)).is_some_and(|pair| holds_copy(artwork, &pair.copy));
        if held && !removed {
            Availability::Available
        } else {
            Availability::NothingLeft
        }
    }

    /// The fit of `fit`'s original onto the copy's drawn lines in `frame` (the sketch pen's left
    /// out), handed to the worker; none to be had without an original to fit.
    fn ask_fit(&mut self, fit: Option<ToFit>, artwork: &Artwork, frame: &Frame) -> Fitting {
        let Some(fit) = fit else {
            self.fit_job = None;
            return Fitting::Unavailable;
        };
        self.fits += 1;
        let copy = line_pieces(artwork, frame, ALL_STROKES);
        self.fit_job = Some(FitJob::of(self.fits, fit, copy));
        Fitting::Asked(self.fits)
    }

    /// The fit for the Compare numbered `sequence` was found (`None`: the copy had no length);
    /// a later Compare's or none waiting drops it.
    pub fn fitted(&mut self, sequence: u64, fit: Option<Fit>) {
        if let Some(Compared::Drawn(fitting, _)) = &mut self.compared
            && *fitting == Fitting::Asked(sequence)
        {
            *fitting = Fitting::Found(fit);
        }
    }

    /// The fit to find, once: the runtime's worker takes it.
    pub fn fit_request(&mut self) -> Option<FitJob> {
        self.fit_job.take()
    }

    /// The region a fill of the shown exercise's frame covers where its tools fill (a colour's,
    /// a painted copy's, a value scale's cell's, value masses'; a whole copy's while a painting
    /// pen is chosen, its colour blocks, S96): the frame, the picture's shape (S99) (with
    /// `selection`); or why none: no fill among its tools, or the active layer of `artwork` hidden
    /// or locked (the notice says so). Filling takes the comparison away; a value scale's next
    /// cell comes in hand.
    pub(crate) fn fill_region(
        &mut self,
        artwork: &Artwork,
        selection: Option<&RegionMask>,
    ) -> Result<RegionMask, Refused> {
        let tools = self.shown_facts().map(|facts| facts.tools);
        let fills = matches!(
            tools,
            Some(Tools::Colour | Tools::Cells | Tools::Painting | Tools::Greys)
        );
        let blocks = tools == Some(Tools::Copying) && self.pen.paints();
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
