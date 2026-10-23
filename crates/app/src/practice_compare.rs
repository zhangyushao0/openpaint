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
//! (`practice_read.rs`).
//!
//! The first Compare stores the exercise's result: when, the sheet's next stroke id (every stroke
//! in the frame from it on is a correction) and a colour's numbers (a painted copy's when its
//! pixels arrive), which judge the eye before the answer was seen; every later one counts. A
//! drawn exercise's pen turns blue. Anything laid in the frame afterwards takes the comparison
//! away: corrections are made by eye.

use openpaint_core::{Artwork, RegionMask};
use openpaint_practice::{
    ExerciseResult, Family, Fit, Frame, Metrics, Original, Point, Rgb8, score_colour,
};

use super::read::{CopyRead, Reading};
use super::{PracticeSheet, unavailable};
use crate::error::AppError;
use crate::layer_rules::LayerRefusal;
use crate::practice::{Overlay, Pen, PracticeNotice, SHAPE_SHOWN};
use crate::practice_copy::{colour_copy, holds_copy, line_pieces};
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
        let compared = match &self.sheet.originals[index] {
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
            _ if !holds_copy(artwork, &frame) => return Err(nothing()),
            Original::Areas(_) | Original::Scale(_) => Compared::Painted(self.ask_read(index)),
            Original::Picture(_) if !self.drawn(index) => Compared::Painted(Reading::Shown),
            Original::Lines(_) | Original::Picture(_) => {
                Compared::Drawn(self.ask_fit(index, artwork, &frame))
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
        if matches!(compared, Compared::Drawn(_)) {
            self.pen = Pen::Correct;
        }
        self.compared = Some(compared);
        Ok(())
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
            copy: line_pieces(artwork, frame, &self.pens),
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

    /// A colour's numbers now; a drawn exercise has none, a painted one's come with its pixels.
    fn numbers(&self, artwork: &Artwork, index: usize) -> Metrics {
        let frame = self.sheet.pairs[index].copy;
        match &self.sheet.originals[index] {
            Original::Colour(rgb) => (score_colour(*rgb, &colour_copy(artwork, &frame)))
                .map(|score| score.metrics())
                .unwrap_or_default(),
            _ => Metrics::new(),
        }
    }

    /// The region a fill of the shown colour's, colour combination's or value scale's cell's
    /// frame covers (with `selection`), or why none: not one of those shown, or the active layer
    /// of `artwork` hidden or locked (the notice says so). Filling takes the comparison away; a
    /// value scale's next cell comes in hand.
    pub(crate) fn fill_region(
        &mut self,
        artwork: &Artwork,
        selection: Option<&RegionMask>,
    ) -> Result<RegionMask, AppError> {
        let family = self.current().and_then(|index| self.family(index));
        let fills = matches!(family, Some(Family::Colour | Family::Areas | Family::Scale));
        let Some(clip) = self.clip.clone().filter(|_| fills) else {
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
            (Compared::Painted(_), Original::Picture(_)) => {
                // A colour copy compares colours: the picture alone, no outline.
                let (shown, _, _) = self.painted_overlay();
                self.picture_overlay(index, |p| laid(Fit::SAME, p), (shown, false))
            }
            (Compared::Painted(reading), Original::Areas(_)) => {
                let (shown, chosen, outline) = self.painted_overlay();
                let read = match reading {
                    Reading::Read(CopyRead::Areas(read)) => Some(std::sync::Arc::clone(read)),
                    _ => None,
                };
                Some(Overlay::Areas {
                    shown,
                    read,
                    chosen,
                    outline,
                })
            }
            (Compared::Painted(reading), Original::Scale(_)) => {
                Some(Overlay::Scale(match reading {
                    Reading::Read(CopyRead::Scale(check)) => Some(std::sync::Arc::clone(check)),
                    _ => None,
                }))
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

    /// Exercise `index`'s picture over its copy, its points laid by `laid`, `shown` strong, with
    /// its outline where it has one if `outlined`.
    fn picture_overlay(
        &self,
        index: usize,
        laid: impl Fn(&Point) -> [f32; 2],
        (shown, outlined): (f32, bool),
    ) -> Option<Overlay> {
        let picture = self.sheet.pictures[index].as_ref()?;
        let [x, y] = picture.fit.origin;
        let [w, h] = picture.fit.size;
        let corners = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
        let outline = picture.outline.iter().filter(|_| outlined).flatten();
        Some(Overlay::Picture {
            corners: corners.map(|p| laid(&p)),
            outline: outline
                .map(|line| line.iter().map(&laid).collect())
                .collect(),
            shown,
        })
    }
}
