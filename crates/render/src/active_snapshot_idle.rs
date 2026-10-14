//! Frames without a stroke lay the strokes committed to the snapshot's layer onto the tiles that
//! lack them (S56, DECISIONS D83), as background work within the frame budget (D81): the tiles
//! that lack the fewest first, each frame as many as its budget takes. Before, the next stroke's
//! first frames laid the previous stroke over every tile they needed at once (60 to 73 ms on the
//! tablet for Opaque Paint at its largest size); now they find them up to date. A tile laid now
//! holds what one laid by a stroke's frame holds (`Plan::appended` either way).

use std::collections::HashSet;
use std::ops::Range;

use openpaint_core::LayerId;

use super::ActiveSnapshot;
use crate::affine::Affine;
use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::gpu_clock::{GpuClock, Timing};
use crate::output::Output;
use crate::plan::Plan;
use crate::program::{Bindings, Dispatch, SceneOps};
use crate::protocol::{Op, TILE, TileRecord};
use crate::scene::Scene;

/// Tiles a frame bins strokes over at most: what a frame's budget lays at most (each tile a
/// piece of about 10 to 40 us of the tablet's GPU).
const BEHIND: usize = 256;

impl ActiveSnapshot {
    /// The layer it is a snapshot of: its id, and whether of its mask.
    pub(crate) const fn place(&self) -> (LayerId, bool) {
        (self.layer, self.on_mask)
    }

    /// Its buffer's bytes and how many items each tile holds (tests).
    #[cfg(test)]
    pub(crate) fn contents(&self, core: &Core) -> (Vec<u8>, Vec<u32>) {
        (self.tiles.read(core), self.applied.clone())
    }

    /// Whether a tile it holds lacks strokes appended since it was laid.
    pub(crate) fn lacking(&self) -> bool {
        let items = u32::try_from(self.items).unwrap_or(u32::MAX);
        (0..self.applied.len()).any(|tile| self.tiles.holds(tile) && self.applied[tile] < items)
    }

    /// Up to `BEHIND` of the tiles it holds that lack the most strokes appended since (the
    /// layer's content is the items `prefix` of `scene`, `hold` given), and the plan laying those
    /// strokes over them on a target of `size`; the tiles they miss count as laid at once. `None`
    /// when no tile lacks any, or none of those the strokes reach.
    pub(crate) fn behind(
        &mut self,
        scene: &Scene,
        prefix: &Range<usize>,
        size: [u32; 2],
    ) -> Option<Plan> {
        let items = u32::try_from(prefix.len()).unwrap_or(u32::MAX);
        let lacks = |tile: usize| self.tiles.holds(tile) && self.applied[tile] < items;
        let from = (0..self.applied.len())
            .filter(|&tile| lacks(tile))
            .map(|tile| self.applied[tile])
            .min()?;
        // A frame bins a few hundred tiles, not every tile a large stroke reached.
        let group: Vec<usize> = (0..self.applied.len())
            .filter(|&tile| lacks(tile) && self.applied[tile] == from)
            .take(BEHIND)
            .collect();
        let side = TILE as i32;
        let rects: Vec<PixelRect> = (group.iter())
            .map(|&tile| {
                let [x, y] = self.tiles.origin_of(tile).map(|at| at as i32);
                PixelRect {
                    min: [x, y],
                    max: [x + side, y + side],
                }
            })
            .collect();
        let gained = prefix.start + from as usize..prefix.end;
        let plan = Plan::appended(scene, gained, size, &rects);
        let reached: HashSet<usize> = (plan.tiles.iter())
            .map(|tile| self.tiles.tile_of(tile.origin))
            .collect();
        for tile in group.into_iter().filter(|tile| !reached.contains(tile)) {
            self.applied[tile] = items;
        }
        (!plan.tiles.is_empty()).then_some(plan)
    }

    /// Lays the strokes `plan` lists over its tiles `taken` (the snapshot written, not the
    /// target), measured by `timing`; they count as laid.
    pub(crate) fn lay(
        &mut self,
        core: &Core,
        (ops, bindings, residuals): (&SceneOps, &mut Bindings, [&wgpu::Buffer; 2]),
        (plan, taken): (&Plan, &[TileRecord]),
        timing: Option<(&GpuClock, Timing)>,
    ) -> Result<(), RenderError> {
        core.evaluate.dispatch(
            &core.gpu,
            bindings,
            &Dispatch {
                ops,
                fresh: &[Op::end_layer(1.0), Op::resume()],
                plan,
                only: Some(taken),
                output: Output::CANVAS,
                background: [0.0; 4],
                reset: false,
                residuals,
                base: Affine::IDENTITY,
                snapshot: Some(self.tiles.reading(true)),
                timing,
            },
        )?;
        let items = u32::try_from(self.items).unwrap_or(u32::MAX);
        for tile in taken {
            let at = self.tiles.tile_of(tile.origin);
            self.applied[at] = items;
        }
        Ok(())
    }
}
