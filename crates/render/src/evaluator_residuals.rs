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

    /// Whether every residual `tiles` (of an output at `scale`) read is in the arena; when one is
    /// lacking, goes on evaluating a residual within what is left of the frame budget instead
    /// (while the canvas warms, the one warming takes, so the two never start each other's over),
    /// and the tiles wait (S54: a view tier over a canvas shown from its display cache fills
    /// without a frame evaluating every residual it reads at once).
    ///
    /// # Errors
    /// As the evaluation's.
    pub(crate) fn residuals_ready(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        (tiles, scale): (&[TileRecord], u32),
    ) -> Result<bool, RenderError> {
        let near = reached(tiles, scale);
        let cache = self
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let wanted = Wanted::Near(&near);
        let scene = (&cache.scene, request.artwork);
        let filters = (&mut self.filters, &wanted);
        if self.smudges.lacking(core, scene, filters).is_none() {
            return Ok(true);
        }
        let whole = [PixelRect::full(self.size())];
        if self.warming.is_none() || self.derive_lacking(core, request, &whole)?.is_none() {
            self.derive_lacking(core, request, &near)?;
        }
        Ok(false)
    }

    /// After a canvas shown complete from its display cache (S54): while nothing waits, the
    /// residuals of the whole canvas are evaluated within the frame budget, as many as there are
    /// smudges at most, so a stroke, a tier or an undo near a smudge later finds them at hand, as
    /// after a fill.
    pub(super) fn warm(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
    ) -> Result<(), RenderError> {
        let Some(done) = self.warming else {
            return Ok(());
        };
        let smudges = (self.scene.as_ref()).map_or(0, |cache| cache.scene.smudges.len());
        let whole = [PixelRect::full(self.size())];
        match self.derive_lacking(core, request, &whole)? {
            Some(true) if done + 1 < smudges => self.warming = Some(done + 1),
            None | Some(true) => self.warming = None,
            Some(false) => {}
        }
        Ok(())
    }

    /// Goes on evaluating the first residual `near` reads that the arena lacks (and fits
    /// whole): `None` when none is lacking (or they do not fit, so `prepare` takes them at
    /// once), else whether it is in the arena now.
    fn derive_lacking(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        near: &[PixelRect],
    ) -> Result<Option<bool>, RenderError> {
        let cache = self
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let wanted = Wanted::Near(near);
        let lacking = self.smudges.lacking(
            core,
            (&cache.scene, request.artwork),
            (&mut self.filters, &wanted),
        );
        let Some(lacking) = lacking else {
            return Ok(None);
        };
        let scene = (
            &cache.scene,
            &cache.ops,
            cache.serial,
            cache.costs.as_slice(),
        );
        let deriving = (lacking.first, &lacking.needed, &mut self.filters);
        self.smudges
            .derive(core, scene, request.artwork, deriving)
            .map(Some)
    }
}
