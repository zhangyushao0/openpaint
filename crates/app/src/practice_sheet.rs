//! The open practice sheet's flow (`practice.rs`): which exercise shows, where the pen may draw
//! (strokes and fills only in its empty frame, clipped to it), what the eyedropper may read (an
//! original only once compared), the exercise's own tools or the full interface, the pen a line
//! is drawn with, Compare and its overlay (`practice_compare.rs`), Next (the view glides to the
//! next pair at its zoom, `practice_view.rs`; after the last the session ends over the whole
//! sheet), the instructions (by themselves the first time a kind comes up), the summary and the
//! notices. The session record changes here and goes to the store whenever it did
//! (`save_request`).

use std::sync::Arc;

use openpaint_core::{Artwork, CanvasSize, Color, RegionMask, StrokeStyle, StrokeTool};
use openpaint_practice::{Original, Pair, SessionRecord, summary};

use crate::error::AppError;
use crate::practice::{Pen, PracticeIntent, PracticeNotice, PracticeProjection, SheetView};
use crate::practice_copy::{MARGIN, holds_copy};
use crate::practice_pens::PracticePens;
use crate::projection::{Availability, EditorAction};
use crate::view::{CanvasView, Viewport};

#[path = "practice_compare.rs"]
mod compare;
#[path = "practice_view.rs"]
mod view;

pub use view::PRACTICE_ROOM;

use compare::Compared;

/// What the eyedropper reads at a point of a practice sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PracticePick {
    /// The canvas, as anywhere.
    Canvas,
    /// Nothing: an original not compared yet.
    Refused,
    /// A compared swatch's colour.
    Colour(Color),
}

#[derive(Debug)]
pub(crate) struct PracticeSheet {
    record: SessionRecord,
    /// The finished sessions of its stage before it, for the summary.
    before: Vec<SessionRecord>,
    sheet: Arc<SheetView>,
    canvas: CanvasSize,
    pens: PracticePens,
    /// The shown exercise's empty frame, which strokes and fills stay inside.
    clip: Option<RegionMask>,
    /// While comparing: the check as the last Compare found the copy.
    compared: Option<Compared>,
    /// A line's original lies over the copy aligned onto it, else in place.
    aligned: bool,
    /// The full interface shows, else the exercise's own tools.
    all_tools: bool,
    /// The pen lines are drawn with, and the last of Sketch and Draw chosen.
    pen: Pen,
    drawing: Pen,
    instructions: Option<bool>,
    /// Whether the instructions for lines and for colours showed by themselves already.
    told: [bool; 2],
    notice: Option<PracticeNotice>,
    /// The record changed since the store last got it.
    changed: bool,
}

impl PracticeSheet {
    pub(crate) fn new(
        (record, before): (SessionRecord, Vec<SessionRecord>),
        canvas: CanvasSize,
        pens: PracticePens,
    ) -> Self {
        let sheet = Arc::new(SheetView::of(&record));
        let done = |lines: bool| {
            before.iter().chain([&record]).any(|session| {
                session.results.iter().any(|result| {
                    let kind = session.exercises.get(result.index as usize);
                    kind.is_some_and(|e| e.kind.is_line() == lines)
                })
            })
        };
        let mut practice = Self {
            told: [done(true), done(false)],
            record,
            before,
            sheet,
            canvas,
            pens,
            clip: None,
            compared: None,
            aligned: false,
            all_tools: false,
            pen: Pen::Draw,
            drawing: Pen::Draw,
            instructions: None,
            notice: None,
            changed: false,
        };
        practice.shown();
        practice
    }

    /// The exercise shown, unless the session is done.
    fn current(&self) -> Option<usize> {
        let index = self.record.current as usize;
        (self.record.finished.is_none() && index < self.sheet.pairs.len()).then_some(index)
    }

    fn pair(&self, index: usize) -> Option<&Pair> {
        self.sheet.pairs.get(index)
    }

    fn is_line(&self, index: usize) -> bool {
        matches!(self.sheet.originals.get(index), Some(Original::Line(_)))
    }

    /// An exercise came to show: its frame becomes the clip; a line compared before is
    /// corrected in blue, another drawn with the pen last chosen; its kind's instructions show by
    /// themselves the first time.
    fn shown(&mut self) {
        self.clip = self.current().and_then(|index| {
            let frame = self.sheet.pairs[index].copy;
            let [x, y] = frame.origin.map(|v| v as f32);
            let side = frame.side as f32;
            let corners = [[x, y], [x + side, y], [x + side, y + side], [x, y + side]];
            RegionMask::polygon(self.canvas, &corners)
        });
        let compared = (self.current()).is_some_and(|index| self.record.result(index).is_some());
        self.pen = if compared { Pen::Correct } else { self.drawing };
        if let Some(index) = self.current() {
            let line = self.is_line(index);
            let told = &mut self.told[usize::from(!line)];
            if !*told {
                *told = true;
                self.instructions = Some(line);
            }
        }
    }

    /// What strokes and fills stay inside.
    pub(crate) fn clip(&self) -> Option<&RegionMask> {
        self.clip.as_ref()
    }

    /// The style the next stroke takes instead of the toolbox's `brush`: the pen chosen, while a
    /// line shows with its own tools and the toolbox paints (its eraser, held or chosen, erases).
    pub(crate) fn pen_style(&self, brush: &StrokeStyle) -> Option<StrokeStyle> {
        let index = self.current()?;
        let pens = !self.all_tools && self.is_line(index) && brush.tool() == StrokeTool::Paint;
        pens.then(|| self.pens.style(self.pen).clone())
    }

    /// Whether a pen landing at `point` (canvas pixels) may draw; if not, the notice says why.
    /// Drawing in the frame takes the comparison away: a correction is made by eye.
    pub(crate) fn may_draw(&mut self, point: [f32; 2]) -> bool {
        let point = point.map(f64::from);
        let notice = match self.current() {
            None => Some(PracticeNotice::Done),
            Some(index) => {
                let frame = self.sheet.pairs[index].copy;
                (!frame.contains(point, MARGIN * frame.side)).then_some(PracticeNotice::DrawInFrame)
            }
        };
        // A stroke that may begin lets an earlier refusal's notice go.
        self.notice = notice;
        if notice.is_none() {
            self.compared = None;
        }
        notice.is_none()
    }

    /// What the eyedropper reads at `point` (canvas pixels).
    pub(crate) fn pick(&mut self, point: [f32; 2]) -> PracticePick {
        let point = point.map(f64::from);
        let revealed = self.revealed();
        let over = self.sheet.pairs[..revealed]
            .iter()
            .position(|pair| pair.original.contains(point, 0.0));
        let Some(index) = over else {
            return PracticePick::Canvas;
        };
        if self.record.result(index).is_none() {
            self.notice = Some(PracticeNotice::PickAfterCompare);
            return PracticePick::Refused;
        }
        match self.sheet.originals[index] {
            Original::Colour(rgb) => {
                let [r, g, b] = rgb.map(|c| f32::from(c) / 255.0);
                Color::rgb(r, g, b).map_or(PracticePick::Canvas, PracticePick::Colour)
            }
            Original::Line(_) => PracticePick::Canvas,
        }
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
                    .then(|| self.current().map(|i| self.is_line(i)))
                    .flatten();
            }
            PracticeIntent::Compare(on) => self.compare(on, artwork, now)?,
            PracticeIntent::Aligned(on) => self.aligned = on,
            PracticeIntent::Pen(pen) => self.choose(pen)?,
            PracticeIntent::AllTools(on) => self.all_tools = on,
            PracticeIntent::Next(viewport) => self.next(view, &viewport, now),
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

    /// Lines are drawn with `pen` from now on; the blue one only once the line was compared.
    fn choose(&mut self, pen: Pen) -> Result<(), AppError> {
        let compared = (self.current()).is_some_and(|index| self.record.result(index).is_some());
        if pen == Pen::Correct && !compared {
            return Err(unavailable(Availability::NothingLeft));
        }
        self.pen = pen;
        if pen != Pen::Correct {
            self.drawing = pen;
        }
        Ok(())
    }

    fn next(&mut self, view: &mut CanvasView, viewport: &Viewport, now: u64) {
        let Some(index) = self.current() else {
            return;
        };
        (self.compared, self.instructions, self.notice) = (None, None, None);
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
        let (line, place) = current.map_or((false, (0, 0)), |index| {
            (self.is_line(index), self.record.place(index))
        });
        let before: Vec<&SessionRecord> = self.before.iter().collect();
        let pen = self.pen_style(brush).map(|style| (self.pen, style.size()));
        PracticeProjection {
            sheet: Arc::clone(&self.sheet),
            revealed: self.revealed(),
            stage: self.record.stage,
            number: self.record.number,
            current,
            line,
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
        }
    }
}

fn unavailable(availability: Availability) -> AppError {
    AppError::Unavailable {
        action: EditorAction::Practice,
        availability,
    }
}
