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

use openpaint_core::{Artwork, CanvasSize, RegionMask, StrokeStyle, StrokeTool};
use openpaint_practice::{Family, Frame, ItemRef, Original, Pair, SessionRecord, summary};

use crate::error::AppError;
use crate::practice::{Pen, PracticeIntent, PracticeNotice, PracticeProjection, SheetView};
use crate::practice_copy::{MARGIN, holds_copy};
use crate::practice_pens::PracticePens;
use crate::practice_pictures::{FitJob, PictureImage};
use crate::projection::{Availability, EditorAction};
use crate::view::{CanvasView, Viewport};

/// A practice sheet opening: its session's record, the finished sessions of its stage before it
/// (for the summary) and the sheet as the screen draws it.
pub type Practising = (SessionRecord, Vec<SessionRecord>, SheetView);

#[path = "practice_compare.rs"]
mod compare;
#[path = "practice_read.rs"]
mod read;
#[path = "practice_view.rs"]
mod view;

pub use read::{PracticePick, ReadJob};
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
    /// The pen the exercise draws or paints with now, the last of Sketch and Draw chosen and the
    /// last painting pen chosen.
    pen: Pen,
    drawing: Pen,
    painting: Pen,
    instructions: Option<Family>,
    /// Whether each family's instructions (`Family::ALL`'s order) showed by themselves already,
    /// or need not: a family compared before or in the warm-up.
    told: [bool; 7],
    notice: Option<PracticeNotice>,
    /// The record changed since the store last got it.
    changed: bool,
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
        (record, before, view): Practising,
        canvas: CanvasSize,
        pens: PracticePens,
    ) -> Self {
        let sheet = Arc::new(view);
        // Told: compared before or in this session, or warmed up with (only kinds met are).
        let warm_up = &record.exercises[..(record.warm_up as usize).min(record.exercises.len())];
        let done = |family: Family| {
            let compared = before.iter().chain([&record]).any(|session| {
                session.results.iter().any(|result| {
                    let kind = session.exercises.get(result.index as usize);
                    kind.is_some_and(|e| e.kind.family() == family)
                })
            });
            compared || warm_up.iter().any(|e| e.kind.family() == family)
        };
        let mut practice = Self {
            told: Family::ALL.map(done),
            all_tools: record.is_project(),
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
        self.painting_state = Painting::default();
        let frame = self.current().map(|index| self.sheet.pairs[index].copy);
        self.clip = frame.and_then(|frame| self.region(&frame));
        let compared = (self.current()).is_some_and(|index| self.record.result(index).is_some());
        let family = self.current().and_then(|index| self.family(index));
        self.pen = match family {
            Some(family) if !family.drawn() => self.painting,
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

    /// The style the next stroke takes instead of the toolbox's `brush`: the pen chosen, while
    /// the exercise's own tools show and the toolbox paints (its eraser, held or chosen,
    /// erases); a painting pen in `brush`'s colour. A colour is filled, not painted.
    pub(crate) fn pen_style(&self, brush: &StrokeStyle) -> Option<StrokeStyle> {
        let family = self.family(self.current()?)?;
        let pens = !self.all_tools && family != Family::Colour && brush.tool() == StrokeTool::Paint;
        pens.then(|| self.pens.stroke(self.pen, brush.color()))
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

    /// The shown exercise is drawn or painted with `pen` from now on: a drawing pen for lines and
    /// shapes (the blue one only once compared), a painting pen for painted exercises.
    fn choose(&mut self, pen: Pen) -> Result<(), AppError> {
        let index = self
            .current()
            .ok_or(unavailable(Availability::NothingLeft))?;
        let compared = self.record.result(index).is_some();
        let fits = if self.drawn(index) {
            !pen.paints() && (pen != Pen::Correct || compared)
        } else {
            pen.paints()
        };
        if !fits {
            return Err(unavailable(Availability::NothingLeft));
        }
        self.pen = pen;
        if pen.paints() {
            self.painting = pen;
        } else if pen != Pen::Correct {
            self.drawing = pen;
        }
        Ok(())
    }

    fn next(
        &mut self,
        view: &mut CanvasView,
        viewport: &Viewport,
        now: u64,
    ) -> Result<(), AppError> {
        let Some(index) = self.current() else {
            return Ok(());
        };
        if self.record.is_project() {
            // A colour copy is worked on whenever the user likes; it never ends with Next.
            return Err(unavailable(Availability::NothingLeft));
        }
        (self.compared, self.instructions, self.notice) = (None, None, None);
        self.fit_job = None;
        if index + 1 < self.sheet.pairs.len() {
            self.record.current += 1;
            if let Some(pair) = self.pair(index + 1) {
                *view = view::centred(view, pair.centre(), (self.canvas, viewport));
            }
        } else {
            self.record.finished = Some(now);
            *view = self.whole(viewport);
        }
        self.changed = true;
        self.shown();
        Ok(())
    }

    /// The record for the store, once after each change.
    pub(crate) fn save_request(&mut self) -> Option<SessionRecord> {
        std::mem::take(&mut self.changed).then(|| self.record.clone())
    }

    pub(crate) fn projection(&self, artwork: &Artwork, brush: &StrokeStyle) -> PracticeProjection {
        let current = self.current();
        let result = current.and_then(|index| self.record.result(index));
        let held = |index: usize| {
            self.pair(index)
                .is_some_and(|pair| holds_copy(artwork, &pair.copy))
        };
        let compare = match current {
            Some(index) if result.is_some() || held(index) => Availability::Available,
            _ => Availability::NothingLeft,
        };
        let place = current.map_or((0, 0), |index| self.record.place(index));
        let before: Vec<&SessionRecord> = self.before.iter().collect();
        let pen = self.pen_style(brush).map(|style| (self.pen, style.size()));
        PracticeProjection {
            sheet: Arc::clone(&self.sheet),
            revealed: self.revealed(),
            stage: self.record.stage,
            number: self.record.number,
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
