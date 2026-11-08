//! Where a practice sheet shows (`sheet.rs`): it opens on the shown pair, large but with
//! room round it, Next glides to the next pair at the zoom kept, and a finished session shows
//! the whole sheet; always in the room left of `PRACTICE_ROOM`, where the card and the tools of a
//! colour lie.

use openpaint_core::CanvasSize;

use super::{PracticeSheet, unavailable};
use crate::refused::Refused;
use openpaint_app::Availability;
use openpaint_app::{CanvasView, FIT_MARGIN, MAX_ZOOM, MIN_ZOOM, Viewport};

/// Points at the view's right kept for the practice card (its width and margins): the pair, and
/// the whole sheet once done, lie in the room left of it.
pub const PRACTICE_ROOM: f32 = 416.0;
/// At first a pair takes at most this share of the room's width and its frames this share of
/// its height.
const PAIR_WIDTH: f32 = 0.85;
const FRAME_HEIGHT: f32 = 0.45;
/// A colour copy's frames take at most this share of the room's height.
const STUDY_HEIGHT: f32 = 0.8;

impl PracticeSheet {
    /// On to the next exercise, the view gliding to its pair within `viewport` at its zoom; after
    /// the last the session ends at `now` and the whole sheet shows.
    pub(super) fn next(
        &mut self,
        view: &mut CanvasView,
        viewport: &Viewport,
        now: u64,
    ) -> Result<(), Refused> {
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
                *view = centred(view, pair.centre(), (self.canvas, viewport));
            }
        } else {
            self.record.finished = Some(now);
            *view = self.whole(viewport);
        }
        self.changed = true;
        self.shown();
        Ok(())
    }

    /// The view a sheet opens with: the shown pair (or the whole sheet, once done) in the middle
    /// of `viewport`, large but leaving room round it.
    pub(crate) fn opening_view(&self, viewport: &Viewport) -> CanvasView {
        let Some(pair) = self.current().and_then(|index| self.pair(index)) else {
            return self.whole(viewport);
        };
        let ([width, height], _) = room(viewport);
        let pair_width = (pair.copy.origin[0] + pair.copy.size[0] - pair.original.origin[0]) as f32;
        let frame_height = pair.copy.size[1] as f32;
        // A colour copy's pair fills the room: it is the whole sheet.
        let tall = if self.record.is_project() {
            STUDY_HEIGHT
        } else {
            FRAME_HEIGHT
        };
        let scale = (PAIR_WIDTH * width / pair_width).min(tall * height / frame_height);
        self.at_scale(scale, pair.centre(), viewport)
    }

    /// The whole sheet upright in the room left of the card.
    pub(super) fn whole(&self, viewport: &Viewport) -> CanvasView {
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
pub(super) fn centred(
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
