//! The screen canvas's background work (S53, DECISIONS D80): each frame without a stroke being
//! drawn takes, of the tiles that wait (an artwork opening, an undo, a layer edit), what the
//! frame budget allows (`pace.rs`), nearest the screen's middle first (`waiting.rs`). Tiles are
//! binned in batches of `BATCH_TILES`, kept with the scene across the frames of one version, so
//! a frame bins only when its batch ran out: never the whole canvas again every frame (S49 did,
//! and on the tablet that binning alone took most of a 170 ms frame).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use openpaint_core::{ActiveStroke, Artwork};

use super::CanvasTarget;
use super::scene_cache::Batch;
use crate::affine::Affine;
use crate::bounded::tile_rects;
use crate::core::Core;
use crate::cost::tile_cost;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::plan::Plan;
use crate::program::Dispatch;
use crate::protocol::TileRecord;
use crate::scene::{ActivePlace, Fresh};
use crate::smudge::{Wanted, reached};
use crate::waiting::Focus;

/// Waiting tiles binned at once: a block about 256 pixels square around the screen's middle.
const BATCH_TILES: usize = 256;
/// Batches one frame takes at most (cheap tiles: the paper, a few strokes), and the CPU time it
/// bins for at most before it starts another.
const BATCHES: usize = 64;
const BINNING: Duration = Duration::from_millis(2);

impl CanvasTarget {
    /// Tiles wait to be evaluated: a frame's evaluation left them for the next (S49). The
    /// target shows them as they were until then; `read` and `pick` read them so.
    #[must_use]
    pub fn pending(&self) -> bool {
        !self.waiting.is_empty()
    }

    /// Where the screen looks on the canvas from now on: the waiting tiles it shows are
    /// evaluated first, nearest its middle (S53); `None`: rows from the top.
    pub fn look_at(&mut self, focus: Option<Focus>) {
        self.focus = focus;
    }

    /// Its evaluations go in small dispatches, each waited for before the next (S53): a target a
    /// worker evaluates while the screen draws on (the layer panel's pictures, the brush
    /// editor's strip), so the screen's frames find the GPU free between them.
    pub fn pace_dispatches(&mut self) {
        self.bindings.paced = true;
    }

    /// The serial of the scene the last evaluation drew and what its items cost at an output
    /// pixel (S53), when it drew `artwork` with the in-progress stroke `active`.
    pub(crate) fn scene_costs(
        &self,
        artwork: &Artwork,
        active: Option<&ActiveStroke>,
    ) -> Option<(u64, &[f32])> {
        let place = active.map(ActivePlace::of);
        self.scene
            .as_ref()
            .filter(|cache| cache.artwork.same_version(artwork) && cache.active == place)
            .map(|cache| (cache.serial, cache.costs.as_slice()))
    }

    /// One frame's share of the waiting tiles: binned in batches, taken while the frame budget
    /// holds their estimates, each batch's share dispatched (and its GPU time measured). A new
    /// target first shows the paper where its tiles wait. Returns the pixels evaluated.
    ///
    /// # Errors
    /// As the evaluator's dispatch.
    pub(super) fn background(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
    ) -> Result<Vec<PixelRect>, RenderError> {
        if self.blank {
            self.paper(core, request)?;
        }
        let mut evaluated = Vec::new();
        let started = Instant::now();
        for _ in 0..BATCHES {
            if self.waiting.is_empty() || core.pace().spent() {
                break;
            }
            if !self.batch(core, started.elapsed() < BINNING)? {
                break;
            }
            // Tiles reading a residual the arena lacks wait for it; the others go on.
            self.defer_blocked(core, request);
            let taken = self.take(core);
            if taken.is_empty() {
                if self.batch_left() {
                    break;
                }
                continue;
            }
            self.dispatch_share(core, request, &taken)?;
            let rects = tile_rects(&taken);
            self.waiting.remove(&rects);
            evaluated.extend(rects);
        }
        // What the frame has left goes to a residual tiles put aside wait for, or once nothing
        // waits to one a canvas shown from its display cache lacks (S54).
        if self.waiting.has_deferred() && !core.pace().spent() {
            self.derive_deferred(core, request)?;
        } else if self.waiting.is_empty() && !core.pace().spent() {
            self.warm(core, request)?;
        }
        Ok(evaluated)
    }

    /// Whether the batch still has tiles.
    fn batch_left(&self) -> bool {
        (self.scene.as_ref())
            .is_some_and(|cache| (cache.batch.as_ref()).is_some_and(|batch| !batch.left.is_empty()))
    }

    /// Whether a batch has tiles left: the scene's, or (when `bin`) the next waiting tiles binned.
    fn batch(&mut self, core: &Core, bin: bool) -> Result<bool, RenderError> {
        let size = self.size();
        let cache = self
            .scene
            .as_mut()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        if cache
            .batch
            .as_ref()
            .is_some_and(|batch| !batch.left.is_empty())
        {
            return Ok(true);
        }
        if !bin {
            return Ok(false);
        }
        let rects = self.waiting.next(self.focus, BATCH_TILES);
        if rects.is_empty() {
            // Every waiting tile is put aside for a residual.
            return Ok(false);
        }
        let target = (size, Output::CANVAS);
        let plan = Plan::build(&cache.scene, &Fresh::default(), target, &rects);
        core.evaluate.tally.binned(plan.indices.len());
        // Binned in the order the rectangles came: the screen's.
        let left: VecDeque<(TileRecord, f64)> = (plan.tiles.iter())
            .map(|tile| (*tile, tile_cost(&plan, tile, &cache.costs)))
            .collect();
        cache.batch = Some(Batch { plan, left });
        Ok(true)
    }

    /// The batch's tiles the frame budget takes, in order.
    fn take(&mut self, core: &Core) -> Vec<TileRecord> {
        let Some(batch) = self.scene.as_mut().and_then(|cache| cache.batch.as_mut()) else {
            return Vec::new();
        };
        let mut pace = core.pace();
        let mut taken = Vec::new();
        while let Some((tile, cost)) = batch.left.front().copied() {
            if !pace.take(cost) {
                break;
            }
            batch.left.pop_front();
            taken.push(tile);
        }
        taken
    }

    /// Evaluates `taken` tiles of the batch, the residuals near them made present first, timed.
    fn dispatch_share(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        taken: &[TileRecord],
    ) -> Result<(), RenderError> {
        let cache = self
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let batch = cache
            .batch
            .as_ref()
            .ok_or(RenderError::Invalid("no batch was binned"))?;
        let near = reached(taken, 1);
        let filters = (&mut self.filters, &Wanted::Near(&near));
        let scene = (&cache.scene, &cache.ops);
        self.smudges
            .prepare(core, scene, request.artwork, filters)?;
        let mut pace = core.pace();
        let estimate = pace.close();
        let timing = pace.time(estimate);
        let paper = request.background;
        let dispatched = core.evaluate.dispatch(
            &core.gpu,
            &mut self.bindings,
            &Dispatch {
                ops: &cache.ops,
                fresh: &[],
                plan: &batch.plan,
                only: Some(taken),
                output: Output::CANVAS,
                background: [paper.r(), paper.g(), paper.b(), 1.0],
                reset: false,
                residuals: self.smudges.residuals(),
                base: Affine::IDENTITY,
                snapshot: None,
                timing: timing.and_then(|timing| Some((pace.clock()?, timing))),
            },
        );
        if let Some(timing) = timing {
            pace.finish(&core.gpu, timing);
        }
        dispatched
    }

    /// Every waiting tile of a new target shows the paper until it is evaluated (S49).
    fn paper(&mut self, core: &Core, request: &FrameRequest<'_>) -> Result<(), RenderError> {
        self.blank = false;
        let cache = self
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let rects = self.waiting.next(None, usize::MAX);
        let plan = Plan::default();
        // Tiles listing no op: each shows the paper alone.
        let tiles: Vec<TileRecord> = rects
            .iter()
            .map(|rect| {
                let origin = rect.min.map(i32::unsigned_abs);
                TileRecord {
                    origin,
                    output: origin,
                    start: 0,
                    count: 0,
                }
            })
            .collect();
        let paper = request.background;
        core.evaluate.dispatch(
            &core.gpu,
            &mut self.bindings,
            &Dispatch {
                ops: &cache.ops,
                fresh: &[],
                plan: &plan,
                only: Some(&tiles),
                output: Output::CANVAS,
                background: [paper.r(), paper.g(), paper.b(), 1.0],
                reset: false,
                residuals: self.smudges.residuals(),
                base: Affine::IDENTITY,
                snapshot: None,
                timing: None,
            },
        )
    }
}
