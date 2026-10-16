//! A view tier's tiles handed to the raster worker and its answers taken (S60, DECISIONS D87;
//! `view_raster.rs`, `view_raster_jobs.rs`). Up to `AHEAD` tiles wait with the worker, so it
//! always has the next when the GPU's queue has room; while the screen is touched, a frame hands
//! over only what its share takes (estimated at what the last tile cost). The missing tiles
//! nearest the screen's middle go first, then the tiles the evaluator drew. A tile is shown once its
//! answer says it was drawn, into the slot and the atlas it was handed over with, and no change
//! reached it meanwhile: one that did waits to be drawn again (the canvas shows there), as a tile
//! the worker dropped does.

use std::sync::Arc;

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::CanvasTarget;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::raster::SceneBuffers;
use crate::view_atlas::Tier;
use crate::view_background::next_missing;
use crate::view_needed::Needed;
use crate::view_raster::{Issued, RasterTier, paper};
use crate::view_raster_jobs::{RasterAnswer, RasterDone, RasterJob, RasterOrder};
use crate::view_tier::{ViewCache, ViewFrame, ViewWork};
use crate::view_tiles::{TIER_TILE, TileMap};

/// Tiles handed to the worker and not answered yet, at most: the frame loop hands tiles over a
/// vsync apart, and the worker plans and encodes a sparse one in a millisecond or less (a dense
/// one about 3.5 ms on the tablet), so it always has the next.
pub(crate) const AHEAD: usize = 8;

/// What a tile is expected to cost the GPU before any was drawn, ns: a dense one on the tablet.
const FIRST: f64 = 6_000_000.0;

/// Hands the worker the tiles `needed` that `tier` lacks, or holds as the evaluator drew them, as
/// many as may wait with it (`AHEAD`; while the screen is touched, within the frame's share).
/// Nothing is handed over while a stroke is drawn.
///
/// # Errors
/// As `Output::at`; a slot or the atlas that cannot be had.
pub(crate) fn fill_raster(
    core: &Core,
    (tier, needed): (&mut Tier, &Needed),
    (canvas, request): (&CanvasTarget, &FrameRequest<'_>),
    raster: &mut RasterTier,
    now: f64,
) -> Result<(), RenderError> {
    let Some((scene, ops, serial)) = canvas.scene_serial(request.artwork, request.active) else {
        return Ok(());
    };
    let Some((_, Ok(routed))) = &raster.scene else {
        return Ok(());
    };
    if request.active.is_some() {
        return Ok(());
    }
    let routed = Arc::clone(routed);
    // An evaluation of a tile begun before the scene came here is left.
    tier.fill = None;
    let idle = core.pace().idle();
    let scale = tier.scale();
    while raster.issued.len() < AHEAD {
        let drawing = raster.drawing(scale);
        let Some(tile) = next_tile(core, tier, needed, &drawing)? else {
            break;
        };
        let Some(slot) = tier.map.slot(tile) else {
            break;
        };
        // While the screen is touched, the frame's share: tiles estimated at what the last cost.
        if !idle && !core.pace().promise(raster.expected()) {
            break;
        }
        let rect = tier.map.tile_rect(tile);
        let size = [0, 1].map(|axis| (rect.max[axis] - rect.min[axis]).max(0) as u32);
        let id = raster.next;
        raster.next += 1;
        let job = RasterJob {
            id,
            scene: Arc::clone(scene),
            raster: Arc::clone(&routed),
            buffers: SceneBuffers::of(ops, serial),
            output: Output::at(scale, rect.min.map(|at| at.max(0) as u32))?,
            size,
            paper: paper(request),
            into: tier.atlas_view().clone(),
            format: tier.atlas().format(),
            at: tier.map.slot_origin(slot),
        };
        raster.orders.push(RasterOrder::Draw(Box::new(job)));
        let grown = tier.grown;
        raster.issued.push(Issued {
            id,
            scale,
            tile,
            slot,
            grown,
            stale: false,
        });
        raster.used_at = now;
    }
    Ok(())
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
    /// What the raster worker is asked to do since the last call, in order (S60): the runtime
    /// hands it over.
    pub fn take_orders(&mut self) -> Vec<RasterOrder> {
        self.raster.take_orders()
    }

    /// The raster worker's answer to an order: a tile's applied by the next `update`; an atlas
    /// kept for the next new tier.
    pub fn answer(&mut self, answer: RasterAnswer) {
        match answer {
            RasterAnswer::Tile(done) => self.raster.answer(done),
            RasterAnswer::Atlas(atlas) => {
                // Kept within the room the GPU memory budget leaves the tiers now.
                let bytes = u64::from(atlas.width()) * u64::from(atlas.height()) * 4;
                if self.bytes() + bytes <= self.room {
                    self.spare = Some(atlas);
                }
            }
        }
    }

    /// An atlas is to be made ahead once the screen rests (S60): frames are wanted until it is
    /// asked for.
    #[must_use]
    pub const fn wants_spare(&self) -> bool {
        self.wanting
    }

    /// While nothing touches the screen and an atlas is wanted ahead (before the first tier of the
    /// canvas, or since the tiers were let go), the raster worker is asked to make one: as many
    /// slots as a tier needs showing the screen `frame` upright (its tiles and a ring), when the
    /// GPU memory budget leaves the tiers room for it. Only the first: making one holds the
    /// device's allocator for tens of ms, which the frame loop would wait for at a random moment,
    /// where a later tier's own atlas costs its first frame that once the view stands still.
    pub(crate) fn ask_spare(&mut self, core: &Core, frame: &ViewFrame) {
        if !self.wanting || !core.pace().idle() {
            return;
        }
        self.wanting = false;
        let tiles = frame.size.map(|side| side.div_ceil(TIER_TILE) + 3);
        let slots = tiles[0] * tiles[1];
        let (across, rows) = crate::view_atlas::atlas_for(slots);
        let bytes = u64::from(across * rows) * u64::from(TIER_TILE * TIER_TILE) * 4;
        if self.bytes() + bytes <= self.room {
            self.raster.orders.push(RasterOrder::Atlas(slots));
        }
    }

    /// The worker's answers applied: a tile drawn into the slot and atlas it was handed over
    /// with shows (when the scene still goes to the raster core and no change reached it, else
    /// it waits to be drawn again); one dropped stays as it was.
    pub(crate) fn take_answers(&mut self, core: &Core, rastered: bool, work: &mut ViewWork) {
        let shown = self.current.as_ref().map(Tier::scale);
        let mut filled = 0;
        for (issued, drawn) in self.raster.answered() {
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
    /// The orders for the worker since the last call, in order.
    pub(crate) fn take_orders(&mut self) -> Vec<RasterOrder> {
        std::mem::take(&mut self.orders)
    }

    /// The worker answered.
    pub(crate) fn answer(&mut self, done: RasterDone) {
        self.answers.push(done);
    }

    /// The worker's answers since the last call: each tile it was handed with whether it drew it.
    pub(crate) fn answered(&mut self) -> Vec<(Issued, bool)> {
        let mut answered = Vec::new();
        for done in std::mem::take(&mut self.answers) {
            self.drawer = done.bytes;
            if done.drawn {
                self.last = done.time;
            }
            if let Some(at) = self.issued.iter().position(|issued| issued.id == done.id) {
                answered.push((self.issued.remove(at), done.drawn));
            }
        }
        answered
    }

    /// What the next tile is expected to cost the GPU, ns: what the last drawn did.
    pub(crate) fn expected(&self) -> f64 {
        if self.last > 0.0 { self.last } else { FIRST }
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
        let mut raster = RasterTier::default();
        raster.issued = vec![issued(1, [0, 0]), issued(2, [3, 1])];
        assert_eq!(raster.drawing(4), [[0, 0], [3, 1]]);
        assert!(raster.drawing(8).is_empty(), "another tier's");
        assert_eq!(raster.expected(), FIRST, "before any tile was drawn");
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
        raster.answer(done(2, true));
        raster.answer(done(1, false));
        raster.answer(done(9, true));
        let answered = raster.answered();
        assert_eq!(answered.len(), 2, "the unknown job's answer is passed over");
        assert_eq!(
            (answered[0].0.tile, answered[0].0.stale, answered[0].1),
            ([3, 1], true, true)
        );
        assert_eq!(
            (answered[1].0.tile, answered[1].0.stale, answered[1].1),
            ([0, 0], false, false)
        );
        assert!(raster.issued.is_empty() && raster.drawer == 7);
        assert_eq!(
            raster.expected(),
            2_000_000.0,
            "what the last drawn tile cost"
        );
    }
}
