//! The open practice sheet's flow (`practice.rs`): which exercise shows, where the pen may draw
//! (strokes and fills only in its empty frame, clipped to it; a checked frame takes none), what
//! the eyedropper may read (an original only once checked), Compare (the first time it scores
//! the copy and stores the result), Next (the view glides to the next pair at its zoom; after the
//! last the session ends over the whole sheet), the instructions (by themselves the first time a
//! kind comes up), the summary and the notices. The session record changes here and goes to the
//! store whenever it did (`save_request`).

use std::sync::Arc;

use openpaint_core::{Artwork, CanvasSize, Color, RegionMask};
use openpaint_practice::{
    Metrics, Original, Pair, SessionRecord, key_point_matches, score_colour, score_line, summary,
};

use crate::error::AppError;
use crate::practice::{Overlay, PracticeIntent, PracticeNotice, PracticeProjection, SheetView};
use crate::practice_copy::{MARGIN, colour_copy, holds_copy, line_copy};
use crate::projection::{Availability, EditorAction};
use crate::view::{CanvasView, FIT_MARGIN, MAX_ZOOM, MIN_ZOOM, Viewport};

/// Points at the view's right kept for the practice card (its width and margins): the pair, and
/// the whole sheet once done, lie in the room left of it.
pub const PRACTICE_ROOM: f32 = 416.0;
/// At first a pair takes at most this share of the room's width and its frames this share of
/// its height.
const PAIR_WIDTH: f32 = 0.85;
const FRAME_HEIGHT: f32 = 0.45;

/// What the eyedropper reads at a point of a practice sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PracticePick {
    /// The canvas, as anywhere.
    Canvas,
    /// Nothing: an original not checked yet.
    Refused,
    /// A checked swatch's colour.
    Colour(Color),
}

#[derive(Debug)]
pub(crate) struct PracticeSheet {
    record: SessionRecord,
    /// The finished sessions of its stage before it, for the summary.
    before: Vec<SessionRecord>,
    sheet: Arc<SheetView>,
    canvas: CanvasSize,
    /// The shown exercise's empty frame, which strokes and fills stay inside.
    clip: Option<RegionMask>,
    comparing: bool,
    instructions: Option<bool>,
    /// Whether the instructions for lines and for colours showed by themselves already.
    told: [bool; 2],
    notice: Option<PracticeNotice>,
    /// The record changed since the store last got it.
    changed: bool,
}

impl PracticeSheet {
    pub(crate) fn new(
        record: SessionRecord,
        before: Vec<SessionRecord>,
        canvas: CanvasSize,
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
            clip: None,
            comparing: false,
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

    /// An exercise came to show: its frame becomes the clip; its kind's instructions show by
    /// themselves the first time.
    fn shown(&mut self) {
        self.clip = self.current().and_then(|index| {
            let frame = self.sheet.pairs[index].copy;
            let [x, y] = frame.origin.map(|v| v as f32);
            let side = frame.side as f32;
            let corners = [[x, y], [x + side, y], [x + side, y + side], [x, y + side]];
            RegionMask::polygon(self.canvas, &corners)
        });
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

    /// Whether a pen landing at `point` (canvas pixels) may draw; if not, the notice says why.
    pub(crate) fn may_draw(&mut self, point: [f32; 2]) -> bool {
        let point = point.map(f64::from);
        let notice = match self.current() {
            None => Some(PracticeNotice::Done),
            Some(index) => {
                let frame = self.sheet.pairs[index].copy;
                if !frame.contains(point, MARGIN * frame.side) {
                    Some(PracticeNotice::DrawInFrame)
                } else if self.record.result(index).is_some() {
                    Some(PracticeNotice::Checked)
                } else {
                    None
                }
            }
        };
        // A stroke that may begin lets an earlier refusal's notice go.
        self.notice = notice;
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

    /// Applies `intent` at `now` (seconds since 1970 UTC); the view moves for Next.
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
            PracticeIntent::Next(viewport) => self.next(view, &viewport, now),
            PracticeIntent::OneMore => {
                if self.current().is_some() {
                    return Err(unavailable(Availability::Running));
                }
            }
            PracticeIntent::Dismiss => self.notice = None,
        }
        Ok(())
    }

    fn compare(&mut self, on: bool, artwork: &Artwork, now: u64) -> Result<(), AppError> {
        let Some(index) = self.current() else {
            return Err(unavailable(Availability::NothingLeft));
        };
        if on && self.record.result(index).is_none() {
            let metrics = self
                .score(artwork, index)
                .ok_or(unavailable(Availability::NothingLeft))?;
            self.record
                .results
                .push(openpaint_practice::ExerciseResult {
                    index: index as u32,
                    checked_at: now,
                    metrics,
                });
            self.changed = true;
        }
        self.comparing = on;
        self.instructions = None;
        Ok(())
    }

    fn next(&mut self, view: &mut CanvasView, viewport: &Viewport, now: u64) {
        let Some(index) = self.current() else {
            return;
        };
        (self.comparing, self.instructions, self.notice) = (false, None, None);
        if index + 1 < self.sheet.pairs.len() {
            self.record.current += 1;
            if let Some(pair) = self.pair(index + 1) {
                *view = centred(view, pair.centre(), (self.canvas, viewport));
            }
        } else {
            self.record.finished = Some(now);
            *view = self.whole(viewport);
        }
        self.changed = true;
        self.shown();
    }

    /// The numbers of exercise `index`'s copy now, if it holds anything.
    fn score(&self, artwork: &Artwork, index: usize) -> Option<Metrics> {
        let frame = self.pair(index)?.copy;
        match &self.sheet.originals[index] {
            Original::Line(line) => {
                score_line(line, &line_copy(artwork, &frame)).map(|s| s.metrics())
            }
            Original::Colour(rgb) => {
                score_colour(*rgb, &colour_copy(artwork, &frame)).map(|s| s.metrics())
            }
        }
    }

    /// The record for the store, once after each change.
    pub(crate) fn save_request(&mut self) -> Option<SessionRecord> {
        std::mem::take(&mut self.changed).then(|| self.record.clone())
    }

    /// The view a sheet opens with: the shown pair (or the whole sheet, once done) in the middle
    /// of `viewport`, large but leaving room round it.
    pub(crate) fn opening_view(&self, viewport: &Viewport) -> CanvasView {
        let Some(pair) = self.current().and_then(|index| self.pair(index)) else {
            return self.whole(viewport);
        };
        let ([width, height], _) = room(viewport);
        let pair_width = (pair.copy.origin[0] + pair.copy.side - pair.original.origin[0]) as f32;
        let side = pair.copy.side as f32;
        let scale = (PAIR_WIDTH * width / pair_width).min(FRAME_HEIGHT * height / side);
        self.at_scale(scale, pair.centre(), viewport)
    }

    /// The whole sheet upright in the room left of the card.
    fn whole(&self, viewport: &Viewport) -> CanvasView {
        let [cw, ch] = [self.canvas.width(), self.canvas.height()].map(|side| side as f32);
        let ([width, height], _) = room(viewport);
        let scale = (width / cw).min(height / ch) * FIT_MARGIN;
        let middle = [f64::from(cw) / 2.0, f64::from(ch) / 2.0];
        self.at_scale(scale, middle, viewport)
    }

    /// Upright at `scale` screen pixels per canvas pixel, `point` in the room's middle.
    fn at_scale(&self, scale: f32, point: [f64; 2], viewport: &Viewport) -> CanvasView {
        let [cw, ch] = [self.canvas.width(), self.canvas.height()].map(|side| side as f32);
        let [vw, vh] = viewport.size.map(|side| side.max(1.0));
        let fit = (vw / cw).min(vh / ch) * FIT_MARGIN;
        let zoom = (scale / fit).clamp(MIN_ZOOM, MAX_ZOOM);
        let start = CanvasView::new([0.0; 2], zoom, 0.0).unwrap_or(CanvasView::FIT);
        centred(&start, point, (self.canvas, viewport))
    }

    pub(crate) fn projection(&self, artwork: &Artwork) -> PracticeProjection {
        let current = self.current();
        let checked = current.and_then(|index| self.record.result(index));
        let held = |index: usize| {
            self.pair(index)
                .is_some_and(|pair| holds_copy(artwork, &pair.copy))
        };
        let compare = match current {
            Some(_) if checked.is_some() => Availability::Available,
            Some(index) if held(index) => Availability::Available,
            _ => Availability::NothingLeft,
        };
        let (line, place) = current.map_or((false, (0, 0)), |index| {
            (self.is_line(index), self.record.place(index))
        });
        let before: Vec<&SessionRecord> = self.before.iter().collect();
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
            comparing: self.comparing && checked.is_some(),
            result: checked.map(|result| result.metrics.clone()),
            overlay: current
                .filter(|_| self.comparing && checked.is_some())
                .and_then(|index| self.overlay(artwork, index)),
            instructions: self.instructions,
            summary: current.is_none().then(|| summary(&self.record, &before)),
            notice: self.notice,
        }
    }

    /// What lies over exercise `index`'s copy while comparing.
    fn overlay(&self, artwork: &Artwork, index: usize) -> Option<Overlay> {
        let frame = self.pair(index)?.copy;
        let canvas = |p: [f64; 2]| frame.to_canvas(p).map(|v| v as f32);
        match &self.sheet.originals[index] {
            Original::Line(line) => {
                let copy = line_copy(artwork, &frame);
                let matches = key_point_matches(line, &copy)
                    .into_iter()
                    .map(|(key, near)| (canvas(key), canvas(near)))
                    .collect();
                Some(Overlay::Line(matches))
            }
            Original::Colour(original) => {
                let picks = colour_copy(artwork, &frame).picks;
                Some(Overlay::Colour {
                    original: *original,
                    first: *picks.first()?,
                    last: *picks.last()?,
                })
            }
        }
    }
}

fn unavailable(availability: Availability) -> AppError {
    AppError::Unavailable {
        action: EditorAction::Practice,
        availability,
    }
}

/// The room left of the practice card in `viewport` (at least half of it): its size and its
/// middle, in screen pixels.
fn room(viewport: &Viewport) -> ([f32; 2], [f32; 2]) {
    let [vw, vh] = viewport.size.map(|side| side.max(1.0));
    let width = (vw - PRACTICE_ROOM * viewport.pixels_per_point).max(vw / 2.0);
    let middle = [
        viewport.origin[0] + width / 2.0,
        viewport.origin[1] + vh / 2.0,
    ];
    ([width, vh], middle)
}

/// `view` moved so the canvas point `point` shows in the middle of the room left of the card in
/// `viewport`, its zoom and turn kept.
fn centred(
    view: &CanvasView,
    point: [f64; 2],
    (canvas, viewport): (CanvasSize, &Viewport),
) -> CanvasView {
    let still = CanvasView::new([0.0; 2], view.zoom(), view.rotation()).unwrap_or(CanvasView::FIT);
    let mapping = still.mapping(canvas, viewport);
    let [x, y] = mapping.to_screen(point.map(|v| v as f32));
    let (_, middle) = room(viewport);
    let ppp = viewport.pixels_per_point.max(f32::MIN_POSITIVE);
    let pan = [(middle[0] - x) / ppp, (middle[1] - y) / ppp];
    CanvasView::new(pan, view.zoom(), view.rotation()).unwrap_or(*view)
}
