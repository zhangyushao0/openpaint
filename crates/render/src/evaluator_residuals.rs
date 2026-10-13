//! The screen canvas's waiting tiles and the smudge residuals they read (S53): a tile that reads a
//! residual the arena lacks is put aside while that residual is evaluated over frames as
//! background work (`engine_derive.rs`), first in scene order, and the tiles that read none go
//! on; once nothing they read is lacking, the tiles put aside are taken again. So an artwork
//! opening shows its paint everywhere but near its smudges while their residuals are evaluated,
//! instead of the paper until all are.

use super::CanvasTarget;
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::protocol::{TILE, TileRecord};
use crate::smudge::{Wanted, reached};

impl CanvasTarget {
    /// Puts aside the batch's tiles that read a residual the arena lacks (when every residual
    /// they read fits the device's binding whole; else `prepare` evaluates them at once as
    /// before).
    pub(super) fn defer_blocked(&mut self, core: &Core, request: &FrameRequest<'_>) {
        let Some(cache) = self.scene.as_mut() else {
            return;
        };
        let Some(batch) = cache.batch.as_mut() else {
            return;
        };
        let tiles: Vec<TileRecord> = batch.left.iter().map(|(tile, _)| *tile).collect();
        let near = reached(&tiles, 1);
        let wanted = Wanted::Near(&near);
        let lacking = self.smudges.lacking(
            core,
            (&cache.scene, request.artwork),
            (&mut self.filters, &wanted),
        );
        let Some(lacking) = lacking else {
            return;
        };
        let side = TILE as i32;
        let blocked = |tile: &TileRecord| {
            // What its evaluation reads, as `reached` widens a tile.
            let read = reached(std::slice::from_ref(tile), 1);
            (read.iter()).any(|read| {
                lacking
                    .reach
                    .iter()
                    .any(|reach| !reach.intersect(read).is_empty())
            })
        };
        let aside: Vec<PixelRect> = (batch.left.iter())
            .filter(|(tile, _)| blocked(tile))
            .map(|(tile, _)| {
                let [x, y] = tile.origin.map(|at| at as i32);
                PixelRect {
                    min: [x, y],
                    max: [x + side, y + side],
                }
            })
            .collect();
        batch.left.retain(|(tile, _)| !blocked(tile));
        self.waiting.defer(&aside);
    }

    /// Goes on evaluating the first residual the tiles put aside read, within what is left of
    /// the frame budget; once none they read is lacking, they are taken again.
    pub(super) fn derive_deferred(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
    ) -> Result<(), RenderError> {
        let cache = self
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let near = self.waiting.deferred_rects();
        let wanted = Wanted::Near(&near);
        let scene = (
            &cache.scene,
            &cache.ops,
            cache.serial,
            cache.costs.as_slice(),
        );
        // Each residual at most once a frame: what one placed must not let another go.
        for _ in 0..=cache.scene.smudges.len() {
            let lacking = self.smudges.lacking(
                core,
                (&cache.scene, request.artwork),
                (&mut self.filters, &wanted),
            );
            let Some(lacking) = lacking else {
                self.waiting.resume();
                return Ok(());
            };
            let deriving = (lacking.first, &lacking.needed, &mut self.filters);
            if !self
                .smudges
                .derive(core, scene, request.artwork, deriving)?
            {
                return Ok(());
            }
        }
        Ok(())
    }
}
