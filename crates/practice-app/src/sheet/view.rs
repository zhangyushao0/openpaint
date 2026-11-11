//! Where a practice sheet shows (`mod.rs`): every view of a pair fits its row (`Layout::row_rect`,
//! exactly a one-row sheet), so a quick pinch fits the row in hand and the zoom's limits are a
//! row's whatever the sheet's size. It opens on the shown pair, large but with room round it,
//! Next glides to the next pair at the zoom kept, a finished lesson shows the rows used (the
//! paper laid ahead below them left out), and One more glides to the next lesson's first pair at
//! the zoom the view had before Finish; always in the room left of `PRACTICE_ROOM`, where the
//! card and the tools of a colour lie. A long colour copy's views fit its whole sheet.

use openpaint_core::CanvasSize;
use openpaint_practice::Layout;

use super::{PracticeSheet, unavailable};
use crate::refused::Refused;
use openpaint_app::Availability;
use openpaint_app::{CanvasView, FIT_MARGIN, Fitted, MAX_ZOOM, MIN_ZOOM, Viewport};

/// Points at the view's right kept for the practice card (its width and margins): the pair, and
/// the rows used once done, lie in the room left of it.
pub const PRACTICE_ROOM: f32 = 416.0;
/// At first a pair takes at most this share of the room's width and its frames this share of
/// its height.
const PAIR_WIDTH: f32 = 0.85;
const FRAME_HEIGHT: f32 = 0.45;
/// A colour copy's frames take at most this share of the room's height.
const STUDY_HEIGHT: f32 = 0.8;

impl PracticeSheet {
    /// On to the next exercise, the view gliding to its pair within `viewport` at its zoom (its
    /// row fitted); after the lesson's last it ends at `now` and the rows used show, the zoom
    /// kept for One more.
    pub(super) fn next(
        &mut self,
        view: &mut CanvasView,
        viewport: &Viewport,
        now: u64,
    ) -> Result<(), Refused> {
        let Some(index) = self.current() else {
            return Ok(());
        };
        // A colour copy is worked on whenever the user likes; it never ends with Next.
        let copy = self.record.is_copy();
        let ends = self.ends(index);
        let Some(lesson) = self.record.lesson_mut().filter(|_| !copy) else {
            return Err(unavailable(Availability::NothingLeft));
        };
        if ends {
            lesson.finished = Some(now);
        } else {
            lesson.at = lesson.at.saturating_add(1);
        }
        (self.compared, self.instructions, self.notice) = (None, None, None);
        self.fit_job = None;
        if ends {
            self.zoom = Some(view.zoom());
            *view = self.whole(viewport);
        } else if let Some(pair) = self.pair(index + 1) {
            let row = view.within(self.row(index + 1));
            *view = centred(&row, pair.centre(), (self.canvas, viewport));
        }
        self.changed = true;
        self.shown();
        Ok(())
    }

    /// The view a sheet opens with: the shown pair (or the rows used, once done) in the middle
    /// of `viewport`, large but leaving room round it.
    pub(crate) fn opening_view(&self, viewport: &Viewport) -> CanvasView {
        let Some(index) = self.current() else {
            return self.whole(viewport);
        };
        let Some(pair) = self.pair(index) else {
            return self.whole(viewport);
        };
        let ([width, height], _) = room(viewport);
        let pair_width = (pair.copy.origin[0] + pair.copy.size[0] - pair.original.origin[0]) as f32;
        let frame_height = pair.copy.size[1] as f32;
        // A colour copy's pair fills the room: it is the whole sheet.
        let tall = if self.record.is_copy() {
            STUDY_HEIGHT
        } else {
            FRAME_HEIGHT
        };
        let scale = (PAIR_WIDTH * width / pair_width).min(tall * height / frame_height);
        self.at_scale(scale, (pair.centre(), self.row(index)), viewport)
    }

    /// The view One more glides to: the new lesson's first pair at the zoom the view had before
    /// Finish (the opening's when the sheet opened done).
    pub(super) fn one_more_view(&self, viewport: &Viewport) -> CanvasView {
        let (Some(index), Some(zoom)) = (self.current(), self.zoom) else {
            return self.opening_view(viewport);
        };
        let Some(pair) = self.pair(index) else {
            return self.opening_view(viewport);
        };
        let still = CanvasView::new([0.0; 2], zoom, 0.0).unwrap_or(CanvasView::FIT);
        centred(
            &still.within(self.row(index)),
            pair.centre(),
            (self.canvas, viewport),
        )
    }

    /// The rows used upright in the room left of the card (a colour copy's whole sheet).
    pub(super) fn whole(&self, viewport: &Viewport) -> CanvasView {
        let used = (!self.record.is_copy()).then(|| {
            let [width, height] = Layout::used(&self.record);
            Fitted {
                origin: [0, 0],
                size: [
                    width.min(self.canvas.width()),
                    height.min(self.canvas.height()),
                ],
            }
        });
        let [cw, ch] = used.map_or([self.canvas.width(), self.canvas.height()], |f| f.size);
        let [cw, ch] = [cw, ch].map(|side| side.max(1) as f32);
        let ([width, height], _) = room(viewport);
        let scale = (width / cw).min(height / ch) * FIT_MARGIN;
        let middle = [f64::from(cw) / 2.0, f64::from(ch) / 2.0];
        self.at_scale(scale, (middle, used), viewport)
    }

    /// The rectangle a view of exercise `index` fits: its row; none (the whole canvas) on a long
    /// colour copy.
    fn row(&self, index: usize) -> Option<Fitted> {
        let slot = self
            .record
            .laid(index)
            .filter(|_| !self.record.is_copy())?
            .slot;
        let [x, y, width, height] = Layout::row_rect(usize::from(slot));
        Some(Fitted {
            origin: [x, y].map(|v| v as u32),
            size: [width, height].map(|v| v as u32),
        })
    }

    /// Upright at `scale` screen pixels per canvas pixel, fitting `fits`, `point` in the room's
    /// middle.
    fn at_scale(
        &self,
        scale: f32,
        (point, fits): ([f64; 2], Option<Fitted>),
        viewport: &Viewport,
    ) -> CanvasView {
        let fitted = CanvasView::fitting(fits);
        let fit = fitted.mapping(self.canvas, viewport).scale();
        let zoom = (scale / fit).clamp(MIN_ZOOM, MAX_ZOOM);
        let start = CanvasView::new([0.0; 2], zoom, 0.0).unwrap_or(CanvasView::FIT);
        centred(&start.within(fits), point, (self.canvas, viewport))
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
/// `viewport`, its zoom, turn and rectangle kept.
pub(super) fn centred(
    view: &CanvasView,
    point: [f64; 2],
    (canvas, viewport): (CanvasSize, &Viewport),
) -> CanvasView {
    let still = CanvasView::new([0.0; 2], view.zoom(), view.rotation())
        .map_or(*view, |still| still.within(view.fits()));
    let mapping = still.mapping(canvas, viewport);
    let [x, y] = mapping.to_screen(point.map(|v| v as f32));
    let (_, middle) = room(viewport);
    let ppp = viewport.pixels_per_point.max(f32::MIN_POSITIVE);
    let pan = [(middle[0] - x) / ppp, (middle[1] - y) / ppp];
    CanvasView::new(pan, view.zoom(), view.rotation()).map_or(*view, |v| v.within(view.fits()))
}
