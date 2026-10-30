//! The open practice sheet's flow (`practice.rs`): which exercise shows, where the pen may draw
//! (strokes and fills only in its empty frame, or a value scale's cell, clipped to it), the
//! exercise's own tools or the full interface, the pen drawn exercises are drawn with (a single
//! line, combined lines, a single shape) or painted ones painted with (a colour combination, a
//! value scale), Compare and its overlay (`practice_compare.rs`), a painted copy's reading and the
//! eyedropper (`practice_read.rs`), Next (the view glides to the next pair at its zoom,
//! `practice_view.rs`; after the last the session ends over the whole sheet), the instructions
//! (by themselves the first time a family comes up), the summary and the notices; the session's
//! pictures as the store reads them. A long colour copy is a sheet of one pair, the full
//! interface showing at first and no Next. The session record changes here and goes to the store
//! whenever it did (`save_request`).

use std::collections::HashSet;
use std::sync::Arc;

use openpaint_core::{Artwork, CanvasSize, RegionMask, StrokeStyle};
use openpaint_practice::{
    Family, Frame, ItemRef, Original, Pair, SessionRecord, Tally, seconds, summary,
};

use crate::error::AppError;
use crate::practice::{Pen, PracticeIntent, PracticeNotice, PracticeProjection, SheetView};
use crate::practice_copy::MARGIN;
use crate::practice_pens::PracticePens;
use crate::practice_pictures::{FitJob, PictureImage};
use crate::projection::{Availability, EditorAction};
use crate::view::CanvasView;

/// A practice sheet opening: its session's record, the finished sessions of its stage before it
/// (for the summary), the sheet as the screen draws it, and what every other record adds up to
/// (for the seals its finish stamps).
pub type Practising = (SessionRecord, Vec<SessionRecord>, SheetView, Tally);

#[path = "practice_checks.rs"]
mod checks;
#[path = "practice_compare.rs"]
mod compare;
#[path = "practice_sheet_pens.rs"]
mod pens;
#[path = "practice_read.rs"]
mod read;
#[path = "practice_time.rs"]
mod time;
#[path = "practice_view.rs"]
mod view;

pub(crate) use pens::Layering;
pub use read::{CopyRead, PracticePick, ReadJob};
pub use time::Finished;
pub use view::PRACTICE_ROOM;

use compare::Compared;
use read::Painting;

#[derive(Debug)]
pub(crate) struct PracticeSheet {
    record: SessionRecord,
    /// The finished sessions of its stage before it, for the summary.
    before: Vec<SessionRecord>,
    sheet: Arc<SheetView>,
    canvas: CanvasSize,
    pens: PracticePens,
    /// What strokes and fills stay inside: the shown exercise's empty frame, or a value scale's
    /// cell.
    clip: Option<RegionMask>,
    /// While comparing: the check as the last Compare found the copy.
    compared: Option<Compared>,
    /// A line's original lies over the copy aligned onto it, else in place.
    aligned: bool,
    /// The full interface shows, else the exercise's own tools.
    all_tools: bool,
    /// The pen the exercise draws or paints with now, the last of Sketch and Draw chosen, the
    /// last painting pen chosen and the last division's pen chosen (S95).
    pen: Pen,
    drawing: Pen,
    painting: Pen,
    dividing: Pen,
    instructions: Option<Family>,
    /// Whether each family's instructions (`Family::ALL`'s order) showed by themselves already,
    /// or need not: a family compared before or in the warm-up.
    told: [bool; Family::ALL.len()],
    notice: Option<PracticeNotice>,
    /// The record changed since the store last got it.
    changed: bool,
    /// The app was hidden (or the sheet just opened) since the last use, the time the store last
    /// got, and what every other record adds up to (`practice_time.rs`).
    hidden: bool,
    time_saved: u64,
    others: Tally,
    /// The aligned fit for the worker, and how many were asked.
    fit_job: Option<FitJob>,
    fits: u64,
    /// A painted exercise's state: how much of the original shows, the area compared, the
    /// value scale's cell, the copy's pixels asked for, the eyedropper's comparison.
    painting_state: Painting,
    /// The pictures read (none: unreadable) and those asked of the store.
    images: Vec<(ItemRef, Option<Arc<PictureImage>>)>,
    asked: HashSet<ItemRef>,
}

impl PracticeSheet {
    pub(crate) fn new(
        (record, before, view, others): Practising,
        canvas: CanvasSize,
        pens: PracticePens,
    ) -> Self {
        let sheet = Arc::new(view);
        // Told: compared before or in this session, or warmed up with (only kinds met are); a
        // drill's How shows first every time it opens (S91).
        let warm_up = &record.exercises[..(record.warm_up as usize).min(record.exercises.len())];
        let done = |family: Family| {
            let compared = before.iter().chain([&record]).any(|session| {
                session.results.iter().any(|result| {
                    let kind = session.exercises.get(result.index as usize);
                    kind.is_some_and(|e| e.kind.family() == family)
                })
            });
            let warmed = warm_up.iter().any(|e| e.kind.family() == family);
            (compared || warmed) && record.drilled() != Some(family)
        };
        let mut practice = Self {
            told: Family::ALL.map(done),
            all_tools: record.is_project(),
            hidden: true,
            time_saved: seconds(&record.time),
            others,
            record,
            before,
            sheet,
            canvas,
            pens,
            clip: None,
            compared: None,
            aligned: false,
            pen: Pen::Draw,
            drawing: Pen::Draw,
            painting: Pen::Big,
            dividing: Pen::Main,
            instructions: None,
            notice: None,
            changed: false,
            fit_job: None,
            fits: 0,
            painting_state: Painting::default(),
            images: Vec::new(),
            asked: HashSet::new(),
        };
        practice.shown();
        practice
    }

    /// The next picture of the session to read for the screen, once each.
    pub(crate) fn picture_request(&mut self) -> Option<ItemRef> {
        let item = (self.sheet.pictures.iter().flatten())
            .map(|picture| &picture.item)
            .find(|item| !self.asked.contains(*item))?
            .clone();
        self.asked.insert(item.clone());
        Some(item)
    }

    /// The store read `item`'s picture (`None`: it could not).
    pub(crate) fn picture_read(&mut self, item: ItemRef, image: Option<PictureImage>) {
        self.images.retain(|(kept, _)| *kept != item);
        self.images.push((item, image.map(Arc::new)));
    }

    /// The kind a drill drills (One more starts another of it); none on a session's sheet.
    pub(crate) fn drill(&self) -> Option<Family> {
        self.record.drilled()
    }

    /// The exercise shown, unless the session is done.
    fn current(&self) -> Option<usize> {
        let index = self.record.current as usize;
        (self.record.finished.is_none() && index < self.sheet.pairs.len()).then_some(index)
    }

    fn pair(&self, index: usize) -> Option<&Pair> {
        self.sheet.pairs.get(index)
    }

    fn family(&self, index: usize) -> Option<Family> {
        let exercise = self.record.exercises.get(index)?;
        Some(exercise.kind.family())
    }

    /// Whether exercise `index` is drawn with the pens as lines (lines, a group, a single shape).
    fn drawn(&self, index: usize) -> bool {
        self.family(index).is_some_and(Family::drawn)
    }

    /// Whether Aligned can be chosen for exercise `index`: lines, or a shape's picture with an
    /// outline.
    fn aligned_offered(&self, index: usize) -> bool {
        match self.sheet.originals.get(index) {
            Some(Original::Lines(_)) => true,
            Some(Original::Picture(_)) => {
                self.drawn(index)
                    && (self.sheet.pictures.get(index))
                        .is_some_and(|p| p.as_ref().is_some_and(|p| p.outline.is_some()))
            }
            _ => false,
        }
    }

    /// An exercise came to show: its frame (a value scale's first cell) becomes the clip; lines
    /// compared before are corrected in blue, others drawn with the pen last chosen, painted
    /// exercises painted with the painting pen last chosen; its family's instructions show by
    /// themselves the first time.
    fn shown(&mut self) {
        let frame = self.current().map(|index| self.sheet.pairs[index].copy);
        self.clip = frame.and_then(|frame| self.region(&frame));
        let compared = (self.current()).is_some_and(|index| self.record.result(index).is_some());
        let family = self.current().and_then(|index| self.family(index));
        self.pen = match family {
            Some(family) if !family.drawn() => self.painting,
            Some(family) if family.divided() => self.dividing,
            _ if compared => Pen::Correct,
            _ => self.drawing,
        };
        self.painted_shown();
        if let Some(family) = family {
            let at = Family::ALL.iter().position(|f| *f == family).unwrap_or(0);
            if !self.told[at] {
                self.told[at] = true;
                self.instructions = Some(family);
            }
        }
    }

    /// `frame`'s region of the canvas.
    fn region(&self, frame: &Frame) -> Option<RegionMask> {
        let corners = frame.corners().map(|p| p.map(|v| v as f32));
        RegionMask::polygon(self.canvas, &corners)
    }

    /// What strokes and fills stay inside.
    pub(crate) fn clip(&self) -> Option<&RegionMask> {
        self.clip.as_ref()
    }

    /// Whether a pen landing at `point` (canvas pixels) may draw; if not, the notice says why.
    /// Drawing in the frame takes the comparison away: a correction is made by eye. On a value
    /// scale the cell it lands in is the one it paints.
    pub(crate) fn may_draw(&mut self, point: [f32; 2]) -> bool {
        let point = point.map(f64::from);
        let notice = match self.current() {
            None => Some(PracticeNotice::Done),
            Some(index) => {
                let frame = self.sheet.pairs[index].copy;
                if frame.contains(point, MARGIN * frame.unit()) {
                    self.take_cell(index, frame.to_frame(point))
                } else {
                    Some(PracticeNotice::DrawInFrame)
                }
            }
        };
        // A stroke that may begin lets an earlier refusal's notice go.
        self.notice = notice;
        if notice.is_none() {
            self.laid();
        }
        notice.is_none()
    }

    /// Pairs shown: up to the one shown, or all once the session is done.
    fn revealed(&self) -> usize {
        match self.current() {
            Some(index) => index + 1,
            None => self.sheet.pairs.len(),
        }
    }

    /// Applies `intent` at `now` (seconds since 1970 UTC); the view moves for Next. A fill is
    /// the session's (`fill_frame`).
    pub(crate) fn handle(
        &mut self,
        intent: PracticeIntent,
        artwork: &Artwork,
        (view, now): (&mut CanvasView, u64),
    ) -> Result<(), AppError> {
        match intent {
            PracticeIntent::Instructions(on) => {
                self.instructions = on
                    .then(|| self.current().and_then(|i| self.family(i)))
                    .flatten();
            }
            PracticeIntent::Compare(on) => self.compare(on, artwork, now)?,
            PracticeIntent::Aligned(on) => {
                let offered = self.current().is_some_and(|i| self.aligned_offered(i));
                if on && !offered {
                    return Err(unavailable(Availability::NothingLeft));
                }
                self.aligned = on;
            }
            PracticeIntent::Shown(share) => self.painting_state.shown = share.clamp(0.0, 1.0),
            PracticeIntent::Area(area) => self.choose_area(area)?,
            PracticeIntent::Pen(pen) => self.choose(pen)?,
            PracticeIntent::AllTools(on) => self.all_tools = on,
            PracticeIntent::Next(viewport) => self.next(view, &viewport, now)?,
            PracticeIntent::OneMore => {
                if self.current().is_some() {
                    return Err(unavailable(Availability::Running));
                }
            }
            PracticeIntent::Fill(_) => return Err(unavailable(Availability::NothingLeft)),
            PracticeIntent::Dismiss => self.notice = None,
        }
        Ok(())
    }

    /// The record for the store, once after each change (its time included).
    pub(crate) fn save_request(&mut self) -> Option<SessionRecord> {
        let changed = std::mem::take(&mut self.changed);
        changed.then(|| {
            self.time_saved = seconds(&self.record.time);
            self.record.clone()
        })
    }

    pub(crate) fn projection(&self, artwork: &Artwork, brush: &StrokeStyle) -> PracticeProjection {
        let current = self.current();
        let result = current.and_then(|index| self.record.result(index));
        let compare = self.compare_offered(current, artwork);
        let place = current.map_or((0, 0), |index| self.record.place(index));
        let before: Vec<&SessionRecord> = self.before.iter().collect();
        let pen = self.pen_style(brush).map(|style| (self.pen, style.size()));
        PracticeProjection {
            sheet: Arc::clone(&self.sheet),
            revealed: self.revealed(),
            stage: self.record.stage,
            number: self.record.number,
            drill: self.record.drilled(),
            current,
            family: current.and_then(|index| self.family(index)),
            warming_up: current.is_some_and(|index| self.record.warming_up(index)),
            place,
            last: current.is_some_and(|index| index + 1 == self.sheet.pairs.len()),
            compare,
            comparing: self.compared.is_some(),
            aligned: self.aligned,
            all_tools: self.all_tools,
            pen,
            result: result.cloned(),
            overlay: current.and_then(|index| self.overlay(index)),
            instructions: self.instructions,
            summary: current.is_none().then(|| summary(&self.record, &before)),
            finished: self.finished(),
            notice: self.notice,
            aligned_offered: current.is_some_and(|index| self.aligned_offered(index)),
            fitting: self.fitting(),
            images: self.images.clone(),
            cell: self.cell(),
            probe: self.probe(),
        }
    }
}

fn unavailable(availability: Availability) -> AppError {
    AppError::Unavailable {
        action: EditorAction::Practice,
        availability,
    }
}
