//! A view tier's tiles handed to the raster worker and its answers taken (S60, DECISIONS D87;
//! `view_raster.rs`, `view_raster_jobs.rs`). Up to `AHEAD` tiles wait with the worker, so it
//! always has the next when the GPU's queue has room; while the screen is touched, a frame hands
//! over only what its share takes (estimated at what the last tile cost). The missing tiles
//! nearest the screen's middle go first, then the tiles the evaluator drew. A tile is shown once its
//! answer says it was drawn, into the slot and the atlas it was handed over with, and no change
//! reached it meanwhile: one that did waits to be drawn again (the canvas shows there), as a tile
//! the worker dropped does. Since S62 the orders go out and the answers come in through the
//! canvas (`canvas_raster.rs`), which hands the worker its own tiles too.

use std::sync::Arc;

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::{CanvasTarget, paper};
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::raster::SceneBuffers;
use crate::smudge::around_output;
use crate::view_atlas::Tier;
use crate::view_background::next_missing;
use crate::view_needed::Needed;
use crate::view_raster::{Issued, RasterTier, residuals_near};
use crate::view_raster_jobs::{RasterDone, RasterJob, RasterOrder};
use crate::view_tier::{ViewCache, ViewFrame, ViewWork};
use crate::view_tiles::{TIER_TILE, TileMap};

/// Tiles handed to the worker and not answered yet, at most: the frame loop hands tiles over a
/// vsync apart, and the worker plans and encodes a sparse one in a millisecond or less (a dense
/// one about 3.5 ms on the tablet), so it always has the next.
pub(crate) const AHEAD: usize = 8;

/// Hands the worker the tiles `needed` that `tier` lacks, or holds as the evaluator drew them, as
/// many as may wait with it (`AHEAD`; while the screen is touched, within the frame's share),
/// each with the smudge residuals it reads present (S61: a tile reading one still being
/// evaluated waits). Nothing is handed over while a stroke is drawn.
///
/// # Errors
/// As `Output::at`; a slot or the atlas that cannot be had; as the residuals' evaluation.
pub(crate) fn fill_raster(
    core: &Core,
    (tier, needed): (&mut Tier, &Needed),
    (canvas, request): (&mut CanvasTarget, &FrameRequest<'_>),
    raster: &mut RasterTier,
    now: f64,
) -> Result<(), RenderError> {
    if request.active.is_some() || !canvas.rastered(request) {
        return Ok(());
    }
    // An evaluation of a tile begun before the scene came here is left.
    tier.fill = None;
    let taken = take_tiles(core, (tier, needed), (canvas, request), raster)?;
    if taken.is_empty() {
        return Ok(());
    }
    let scale = tier.scale();
    let near: Vec<PixelRect> = (taken.iter())
        .map(|(_, _, rect)| around_output(rect, scale))
        .collect();
    let Some(held) = residuals_near(core, (canvas, request), &near)? else {
        return Ok(());
    };
    let Some((scene, ops, _)) = canvas.scene_serial(request.artwork, None) else {
        return Ok(());
    };
    let (scene, buffers) = (Arc::clone(scene), SceneBuffers::of(ops, held));
    for (tile, slot, rect) in taken {
        let size = [0, 1].map(|axis| (rect.max[axis] - rect.min[axis]).max(0) as u32);
        let id = canvas.raster.next_id();
        let job = RasterJob {
            id,
            scene: Arc::clone(&scene),
            buffers: buffers.clone(),
            output: Output::at(scale, rect.min.map(|at| at.max(0) as u32))?,
            size,
            paper: paper(request),
            into: tier.atlas_view().clone(),
            format: tier.atlas().format(),
            at: tier.map.slot_origin(slot),
        };
        canvas.raster.orders.push(RasterOrder::Draw(Box::new(job)));
        let grown = tier.grown;
        raster.issued.push(Issued {
            id,
            scale,
            tile,
            slot,
            grown,
            stale: false,
        });
        canvas.raster.used_at = now;
    }
    Ok(())
}

/// The tiles to hand over now, with their slots and output pixels: the next ones while the worker
/// has room for them and, while the screen is touched, the frame's share (each estimated at what
/// the last cost), until one reads a residual still being evaluated (a share of it is, now).
fn take_tiles(
    core: &Core,
    (tier, needed): (&mut Tier, &Needed),
    (canvas, request): (&mut CanvasTarget, &FrameRequest<'_>),
    raster: &RasterTier,
) -> Result<Vec<([u32; 2], u32, PixelRect)>, RenderError> {
    let idle = core.pace().idle();
    let scale = tier.scale();
    let mut taken: Vec<([u32; 2], u32, PixelRect)> = Vec::new();
    while raster.issued.len() + taken.len() < AHEAD {
        let mut drawing = raster.drawing(scale);
        drawing.extend(taken.iter().map(|(tile, _, _)| *tile));
        let Some(tile) = next_tile(core, tier, needed, &drawing)? else {
            break;
        };
        let Some(slot) = tier.map.slot(tile) else {
            break;
        };
        let rect = tier.map.tile_rect(tile);
        let near = [around_output(&rect, scale)];
        if !canvas.residuals_ready(core, request, &near)? {
            break;
        }
        if !idle && !core.pace().promise(canvas.raster.expected()) {
            break;
        }
        taken.push((tile, slot, rect));
    }
    Ok(taken)
}

/// The next tile to hand over: the first needed one not ready (given a slot), else the first
/// needed one the evaluator drew; never one `drawing` already.
fn next_tile(
    core: &Core,
    tier: &mut Tier,
    needed: &Needed,
    drawing: &[[u32; 2]],
) -> Result<Option<[u32; 2]>, RenderError> {
    if let Some(tile) = next_missing(core, tier, needed, drawing)? {
        return Ok(Some(tile));
    }
    Ok(needed
        .ordered()
        .find(|tile| tier.is_ready(*tile) && tier.is_evaluated(*tile) && !drawing.contains(tile)))
}

impl ViewCache {
    /// The raster worker made an atlas ahead: kept for the next new tier, within the room the GPU
    /// memory budget leaves the tiers now.
    pub fn answer_atlas(&mut self, atlas: wgpu::Texture) {
        let bytes = u64::from(atlas.width()) * u64::from(atlas.height()) * 4;
        if self.bytes() + bytes <= self.room {
            self.spare = Some(atlas);
        }
    }

    /// An atlas is to be made ahead once the screen rests (S60): frames are wanted until it is
    /// asked for.
    #[must_use]
    pub const fn wants_spare(&self) -> bool {
        self.wanting
    }

    /// While nothing touches the screen and an atlas is wanted ahead (before the first tier of the
    /// canvas, or since the tiers were let go), the raster worker is asked (through `canvas`) to
    /// make one: as many slots as a tier needs showing the screen `frame` upright (its tiles and a
    /// ring), when the GPU memory budget leaves the tiers room for it. Only the first: making one
    /// holds the device's allocator for tens of ms, which the frame loop would wait for at a
    /// random moment, where a later tier's own atlas costs its first frame that once the view
    /// stands still. A tier made before then wants none any more (frames were asked for until one
    /// was, for ever).
    pub(crate) fn ask_spare(&mut self, core: &Core, canvas: &mut CanvasTarget, frame: &ViewFrame) {
        if self.current.is_some() || self.previous.is_some() || !self.kept.is_empty() {
            self.wanting = false;
        }
        if !self.wanting || !core.pace().idle() {
            return;
        }
        self.wanting = false;
        let tiles = frame.size.map(|side| side.div_ceil(TIER_TILE) + 3);
        let slots = tiles[0] * tiles[1];
        let (across, rows) = crate::view_atlas::atlas_for(slots);
        let bytes = u64::from(across * rows) * u64::from(TIER_TILE * TIER_TILE) * 4;
        if self.bytes() + bytes <= self.room {
            canvas.raster.orders.push(RasterOrder::Atlas(slots));
        }
    }

    /// The worker's answers to the tiers' tiles (`canvas` keeps them) applied: a tile drawn into
    /// the slot and atlas it was handed over with shows (when the scene still goes to the raster
    /// core and no change reached it, else it waits to be drawn again); one dropped stays as it
    /// was.
    pub(crate) fn take_answers(
        &mut self,
        core: &Core,
        canvas: &mut CanvasTarget,
        rastered: bool,
        work: &mut ViewWork,
    ) {
        let shown = self.current.as_ref().map(Tier::scale);
        let mut filled = 0;
        let answers = std::mem::take(&mut canvas.raster.answers);
        for (issued, drawn) in self.raster.answered(answers) {
            let mut tiers = (self.current.iter_mut())
                .chain(self.previous.iter_mut())
                .chain(self.kept.iter_mut());
            let Some(tier) = tiers.find(|tier| tier.scale() == issued.scale) else {
                continue;
            };
            let held = tier.map.slot(issued.tile) == Some(issued.slot);
            if !drawn || !held || tier.grown != issued.grown {
                continue;
            }
            if issued.stale || !rastered {
                tier.set_ready(issued.tile, false);
                continue;
            }
            tier.set_ready(issued.tile, true);
            tier.set_evaluated(issued.tile, false);
            filled += 1;
            if shown == Some(issued.scale) {
                work.filled.push(issued.tile);
            }
        }
        core.evaluate.tally.filled(filled);
    }
}

impl RasterTier {
    /// The tiles `answers` answer, each with whether the worker drew it.
    pub(crate) fn answered(&mut self, answers: Vec<RasterDone>) -> Vec<(Issued, bool)> {
        let mut answered = Vec::new();
        for done in answers {
            if let Some(at) = self.issued.iter().position(|issued| issued.id == done.id) {
                answered.push((self.issued.remove(at), done.drawn));
            }
        }
        answered
    }

    /// The tiles of the tier at `scale` handed over and not answered.
    pub(crate) fn drawing(&self, scale: u32) -> Vec<[u32; 2]> {
        (self.issued.iter())
            .filter(|issued| issued.scale == scale)
            .map(|issued| issued.tile)
            .collect()
    }

    /// The tiles of `map`'s tier handed over that `changed` (canvas pixels) reaches: they are
    /// drawn again once answered.
    pub(crate) fn reached(&mut self, map: &TileMap, changed: &[PixelRect]) -> Vec<[u32; 2]> {
        let scale = map.scale();
        let mut reached = Vec::new();
        for issued in (self.issued.iter_mut()).filter(|issued| issued.scale == scale) {
            let met = changed.iter().any(|rect| {
                (map.meeting(&map.place(rect)))
                    .any(|(tile, part)| tile == issued.tile && !part.is_empty())
            });
            if met {
                issued.stale = true;
                reached.push(issued.tile);
            }
        }
        reached
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Output;

    fn issued(id: u64, tile: [u32; 2]) -> Issued {
        Issued {
            id,
            scale: 4,
            tile,
            slot: id as u32,
            grown: 0,
            stale: false,
        }
    }

    #[test]
    fn answers_find_their_tiles_and_a_change_marks_the_tiles_it_reaches_stale() {
        let mut raster = RasterTier {
            issued: vec![issued(1, [0, 0]), issued(2, [3, 1])],
        };
        assert_eq!(raster.drawing(4), [[0, 0], [3, 1]]);
        assert!(raster.drawing(8).is_empty(), "another tier's");
        // Canvas pixels 200..210 at 4x lie in output 800..840: view tile 3 across, row 1 (256..).
        let map = TileMap::new([1000, 800], Output::at(4, [0, 0]).expect("4x"), (2, 1));
        let changed = [PixelRect {
            min: [200, 70],
            max: [210, 80],
        }];
        assert_eq!(raster.reached(&map, &changed), [[3, 1]]);
        let done = |id, drawn| RasterDone {
            id,
            drawn,
            time: 2_000_000.0,
            bytes: 7,
        };
        let answered = raster.answered(vec![done(2, true), done(1, false), done(9, true)]);
        assert_eq!(answered.len(), 2, "the unknown job's answer is passed over");
        assert_eq!(
            (answered[0].0.tile, answered[0].0.stale, answered[0].1),
            ([3, 1], true, true)
        );
        assert_eq!(
            (answered[1].0.tile, answered[1].0.stale, answered[1].1),
            ([0, 0], false, false)
        );
        assert!(raster.issued.is_empty());
    }
}
