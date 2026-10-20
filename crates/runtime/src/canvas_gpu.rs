//! The open canvas's GPU side: evaluating what changed into its target a frame at a time (S49:
//! a large change fills in over frames), the view tiers, the gallery picture, what its derived
//! caches hold and the GPU memory budget's steps on them, and making all of it again on a new
//! device after a loss. The session (artwork, history, the stroke being drawn) is never touched.

use std::time::Instant;

use openpaint_app::{Session, Thumbnail};
use openpaint_core::StrokeTool;
use openpaint_render::{
    CacheBytes, CanvasTarget, Core, Focus, FrameRequest, PixelRect, RasterAnswer, RasterOrder,
    RenderError, ViewCache, ViewFrame, changed_regions,
};

use super::{Canvas, PAPER, THUMBNAIL_MAX};
use crate::budget::Unit;
use crate::error::RuntimeError;
use crate::pictures::Drawn;

/// What the GPU memory budget leaves the canvas's caches to grow into this frame (`budget.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rooms {
    pub(crate) tiers: u64,
    pub(crate) snapshots: u64,
    /// The smudge residuals' room in the arena (S51).
    pub(crate) arena: u64,
}

/// What the budget's steps gave back, in bytes: the tiers, the snapshots, the arena's blocks not
/// shown, the residuals beyond their room.
pub(crate) type Freed = [u64; 4];

impl Canvas {
    /// Brings the target up to date with what the session shows (the current version, or a
    /// fill's preview over it), within one frame's work: what does not fit waits for the next
    /// frames (S49). Committing the stroke being drawn is followed by the target itself (S63);
    /// any other change passes the pixels that differ between the version shown before it and
    /// the one shown now; the first draw covers the whole canvas.
    pub(crate) fn evaluate(&mut self, core: &Core) -> Result<(), RuntimeError> {
        self.evaluated_now(core, true)
    }

    /// Evaluates everything still to show now, every waiting tile too, before the target is
    /// read (a pick, the gallery picture); the smudge residuals that took at once go back within
    /// their room after it (S51).
    pub(crate) fn settle(&mut self, core: &Core) -> Result<(), RuntimeError> {
        if self.evaluated && self.changed_from.is_none() && !self.target.pending() {
            return Ok(());
        }
        self.evaluated_now(core, false)?;
        self.target.settle_arena(core)?;
        Ok(())
    }

    /// `evaluate`, within one `frame`'s work or whole.
    fn evaluated_now(&mut self, core: &Core, frame: bool) -> Result<(), RuntimeError> {
        let mut regions = if self.evaluated {
            Vec::new()
        } else {
            vec![PixelRect::full(self.target.size())]
        };
        if let Some(before) = self.changed_from.take() {
            regions.extend(changed_regions(&before, self.session.shown()));
        }
        let request = request(&self.session);
        if frame {
            self.target.draw_frame(core, &request, &regions)?;
        } else {
            self.target.draw(core, &request, &regions)?;
        }
        self.target.show_selection(core, self.session.selection());
        self.evaluated = true;
        self.filled(core);
        Ok(())
    }

    /// Tiles of the target still wait to be evaluated: frames are wanted until none does.
    pub(crate) fn filling(&self) -> bool {
        self.target.pending()
    }

    /// Shown complete from its display cache, its smudge residuals are still being evaluated
    /// (S54): frames are wanted until they are.
    pub(crate) fn warming(&self) -> bool {
        self.target.warming()
    }

    /// The snapshots under the next stroke are being taken ahead (S63): frames are wanted until
    /// they are.
    pub(crate) fn taking_ahead(&self) -> bool {
        self.target.taking_ahead() || self.view.taking_ahead()
    }

    /// A stroke is being drawn (the pen is on the glass).
    pub(crate) fn drawing(&self) -> bool {
        self.session.active().is_some()
    }

    /// Counts a frame of the canvas filling in after it opened, and logs the time, the frames
    /// and the raster regions it took once it is full.
    fn filled(&mut self, core: &Core) {
        let Some(opening) = self.opened.as_mut() else {
            return;
        };
        opening.frames += 1;
        if self.target.pending() {
            return;
        }
        log::info!(
            "canvas complete {} ms after it opened, over {} frames and {} raster regions",
            opening.at.elapsed().as_millis(),
            opening.frames,
            core.counts().rastered - opening.rastered
        );
        self.opened = None;
    }

    /// The gallery picture of what the canvas shows now, every waiting tile evaluated first.
    pub(crate) fn thumbnail(&mut self, core: &Core) -> Result<Thumbnail, RuntimeError> {
        self.settle(core)?;
        let image = core.thumbnail(&self.target, THUMBNAIL_MAX)?;
        let size = image.size();
        Thumbnail::new(size, image.into_pixels()).ok_or(RuntimeError::Render(RenderError::Invalid(
            "a thumbnail of the wrong length",
        )))
    }

    /// Brings the view tiers up to date with the target, just evaluated, for showing it
    /// through `frame` at `now` (seconds on the frame clock, S32); call after `evaluate`.
    pub(crate) fn update_view(
        &mut self,
        core: &Core,
        (frame, now): (&ViewFrame, f64),
    ) -> Result<(), RuntimeError> {
        let request = request(&self.session);
        self.view_pending = self
            .view
            .update(core, &mut self.target, &request, (frame, now))?
            .pending();
        self.shown = Some(*frame);
        Ok(())
    }

    /// As `update_view`, through the view the last frame drawn showed (S60: a frame call between
    /// the frames drawn).
    pub(crate) fn update_view_again(&mut self, core: &Core, now: f64) -> Result<(), RuntimeError> {
        match self.shown {
            Some(frame) => self.update_view(core, (&frame, now)),
            None => Ok(()),
        }
    }

    /// What the canvas and its view tiers ask of the raster worker since the last call (S60,
    /// S62).
    pub(crate) fn raster_orders(&mut self) -> Vec<RasterOrder> {
        self.target.take_orders()
    }

    /// The raster worker answered (S60): a tile goes to the canvas (its own, or its tiers'), an
    /// atlas made ahead to the tiers.
    pub(crate) fn raster_answer(&mut self, answer: RasterAnswer) {
        match answer {
            RasterAnswer::Tile(done) => self.target.answer(done),
            RasterAnswer::Atlas(atlas) => self.view.answer_atlas(atlas),
        }
    }

    /// Where the screen shows the canvas through `frame`: its waiting tiles there go first, the
    /// screen's middle first (S53).
    pub(crate) fn look_at(&mut self, frame: &ViewFrame) {
        let [a, b, c, d, e, f] = frame.to_canvas;
        let map = |[x, y]: [f32; 2]| [a * x + b * y + c, d * x + e * y + f];
        let [x0, y0] = frame.origin.map(|at| at as f32);
        let [x1, y1] = [x0 + frame.size[0] as f32, y0 + frame.size[1] as f32];
        let corners = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]].map(map);
        let low = corners
            .iter()
            .fold([f32::MAX; 2], |m, p| [m[0].min(p[0]), m[1].min(p[1])]);
        let high = corners
            .iter()
            .fold([f32::MIN; 2], |m, p| [m[0].max(p[0]), m[1].max(p[1])]);
        // `as` saturates: a view far off the canvas stays a rectangle.
        let shown = PixelRect {
            min: low.map(|at| at.floor() as i32),
            max: high.map(|at| at.ceil() as i32),
        };
        let middle = map([(x0 + x1) / 2.0, (y0 + y1) / 2.0]);
        self.target.look_at(Some(Focus { middle, shown }));
    }

    /// The view tiers still change what shows: tiles missing, or a tier about to start.
    pub(crate) fn view_pending(&self) -> bool {
        self.view_pending
    }

    /// What its derived caches hold on the GPU (pictures are the runtime's: 0).
    pub(crate) fn bytes(&self) -> CacheBytes {
        let held = self.target.bytes();
        CacheBytes {
            tiers: self.view.bytes(),
            raster: self.target.raster_held().map_or(0, |(bytes, _)| bytes),
            // S63: the stroke's drawings, the canvas's and the tier shown's.
            snapshots: held.snapshots + self.view.drawing_bytes(),
            ..held
        }
    }

    /// The budget's steps on its caches (S58): the units `gone` let go (the pictures are the
    /// runtime's), then each cache held within its room to grow, the residuals within theirs
    /// (S51).
    pub(crate) fn hold_within(
        &mut self,
        core: &Core,
        rooms: Rooms,
        gone: &[Unit],
    ) -> Result<Freed, RuntimeError> {
        let mut freed = [0; 4];
        for unit in gone {
            match unit {
                Unit::Tier(scale) => freed[0] += self.view.let_go(*scale),
                Unit::Raster => freed[0] += self.target.let_go_raster(),
                Unit::Snapshots => {
                    freed[1] += self.target.set_snapshot_room(0) + self.view.let_go_drawing();
                }
                Unit::Arena => freed[2] += self.target.shrink_arena(core)?,
                Unit::Pictures => {}
            }
        }
        freed[0] += self.view.set_room(rooms.tiers);
        freed[1] += self.target.set_snapshot_room(rooms.snapshots);
        freed[3] = self.target.set_arena_room(core, rooms.arena)?;
        Ok(freed)
    }

    /// Lets the view tiers, the snapshots and the arena's blocks no scene shows go at once (the
    /// app no longer shows, S58); each is made again when needed.
    pub(crate) fn release(&mut self, core: &Core) -> Result<Freed, RuntimeError> {
        let snapshots = self.target.set_snapshot_room(0) + self.view.let_go_drawing();
        let tiers = self.view.release() + self.target.let_go_raster();
        let arena = self.target.shrink_arena(core)?;
        Ok([tiers, snapshots, arena, 0])
    }

    /// The view tiers (the budget weighs them, S58).
    pub(crate) fn view(&self) -> &ViewCache {
        &self.view
    }

    /// The canvas shows a preview over the current version (a stroke's mirrored copies, a
    /// transform, an adjustment, a fill being placed): it evaluates from its snapshot.
    pub(crate) fn previewing(&self) -> bool {
        !self.session.shown().same_version(self.session.artwork())
    }

    /// A smudge is being drawn: it works in the arena's material.
    pub(crate) fn smudging(&self) -> bool {
        self.session
            .active()
            .is_some_and(|active| active.style().tool() == StrokeTool::Smudge)
    }

    /// Makes the target and the view tiers again on `core`, a new device after the last was lost
    /// (S49): the whole canvas is evaluated anew from the next frame on, as on opening; the
    /// session, its history and the stroke being drawn stay as they are (the stroke is taken up
    /// from its first sample), and the layer pictures are made again.
    pub(crate) fn rebuild(&mut self, core: &Core) -> Result<(), RuntimeError> {
        self.target = CanvasTarget::new(core, self.target.size())?;
        self.view = ViewCache::new(core);
        self.view_pending = false;
        self.shown = None;
        self.evaluated = false;
        self.changed_from = None;
        self.pictures = Drawn::default();
        self.opened = Some(Opening::start(core));
        Ok(())
    }
}

/// A canvas opening (or made again on a new device) until the whole of it was drawn: when, how
/// many frames so far, and the device's raster regions then (S49's log).
pub(crate) struct Opening {
    at: Instant,
    frames: u32,
    rastered: u64,
}

impl Opening {
    pub(crate) fn start(core: &Core) -> Self {
        Self {
            at: Instant::now(),
            frames: 0,
            rastered: core.counts().rastered,
        }
    }
}

/// What the canvas evaluates: what `session` shows, the stroke being drawn, on the paper.
fn request(session: &Session) -> FrameRequest<'_> {
    FrameRequest {
        artwork: session.shown(),
        active: session.active(),
        background: PAPER,
    }
}
