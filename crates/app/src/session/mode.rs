//! What a mode (the other product standing on this canvas, D112) sets and does on the session it
//! owns; the session holds no mode and calls nothing back. A mode sets data: strokes keep the
//! pen's own samples (`keep_raw`), strokes and fills stay inside its bounds beside the selection
//! (`set_bounds`; a fill's region is found within their box), the next stroke's style is its pen
//! (`set_brush`), its view (`view_mut`). And it takes steps: its layers amended into the version
//! shown (`amend`), a version of its own recorded (`record`), the history cut (`keep_only_current`),
//! and before them what an intent or any edit ends (`make_way`, `make_way_for_step`).

use openpaint_core::{Artwork, RegionMask};

use super::{Ends, Session};
use crate::intent::Intent;
use crate::view::CanvasView;

impl Session {
    /// Strokes keep the pen's own samples from now on: no StreamLine, QuickShape or guide.
    pub const fn keep_raw(&mut self) {
        self.strokes.set_raw();
    }

    /// What strokes and fills stay inside beside the selection (only it where the two do not
    /// meet); a fill's region is found within its box. None: the whole canvas.
    pub fn set_bounds(&mut self, bounds: Option<RegionMask>) {
        self.bounds = bounds;
    }

    /// How the artwork shows, to move it.
    pub const fn view_mut(&mut self) -> &mut CanvasView {
        &mut self.view
    }

    /// `artwork` in place of the current version, never a step of its own; nothing while a
    /// stroke is drawn.
    pub fn amend(&mut self, artwork: Artwork) {
        if self.strokes.active().is_none() {
            self.history.amend(artwork);
        }
    }

    /// `artwork` as the next version, one step of history (a fill being placed is dropped).
    pub fn record(&mut self, artwork: Artwork) {
        self.fills.interrupt();
        self.history.record(artwork);
    }

    /// Undo and redo reach no version before the current one from now on (a fill still being
    /// found is dropped with them).
    pub fn keep_only_current(&mut self) {
        self.fills.interrupt();
        self.history.keep_only_current();
    }

    /// Ends what `intent` ends before it applies (`drafts.rs`), as `dispatch` does first. A mode
    /// that may refuse the intent (a pen landing) runs it before it decides; run again by
    /// `dispatch`, it ends nothing more.
    pub fn make_way(&mut self, intent: &Intent) {
        self.end(Ends::by(intent, self.transforms.active()));
    }

    /// Ends what any step of a mode's own ends, as any edit: the shape snapped last, an
    /// adjustment under way (kept as it shows), a transform (committed where it shows).
    pub fn make_way_for_step(&mut self) {
        self.end(Ends::STEP);
    }

    /// What strokes and fills stay inside: the selection and the bounds (only the bounds where
    /// the two do not meet).
    pub(super) fn clip(&self) -> Option<RegionMask> {
        let selected = self.selection.mask();
        match self.bounds.as_ref() {
            Some(bounds) => selected
                .and_then(|selection| bounds.intersect(selection))
                .or_else(|| Some(bounds.clone())),
            None => selected.cloned(),
        }
    }

    /// The bounds' box, `(origin, size)` in canvas pixels: a fill's region is found within it, so
    /// a flood never leaks round the bounds through the paper outside.
    pub(super) fn bounds_box(&self) -> Option<([u32; 2], [u32; 2])> {
        let bounds = self.bounds.as_ref()?;
        Some((bounds.origin(), bounds.size()))
    }
}
