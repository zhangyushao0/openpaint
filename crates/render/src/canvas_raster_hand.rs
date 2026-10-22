//! The canvas's waiting tiles handed to the raster worker and its answers applied
//! (`canvas_raster.rs`). Up to `AHEAD` tiles wait with the worker, the screen's middle first, each
//! with the smudge residuals it reads present (a tile reading one still being evaluated waits, a
//! share of it evaluated each frame); while the screen is touched a frame hands over only what its
//! share takes. Each is drawn into a slot of a staging texture, which the frame loop copies into
//! the canvas once answered, unless a change reached the tile meanwhile: then it is handed over
//! again.

use std::sync::Arc;

use super::super::CanvasTarget;
use super::paper;
use crate::canvas::NO_SCENE;
use crate::canvas_raster::Handed;
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::raster::SceneBuffers;
use crate::raster_worker::AHEAD;
use crate::raster_worker::{JobInto, RasterJob, RasterOrder};
use crate::readback::{extent, place};
use crate::smudge::{Held, around_output};
use crate::view_tiles::TIER_TILE;

impl CanvasTarget {
    /// Hands the worker the next waiting tiles, as many as may wait with it.
    pub(in crate::canvas) fn hand_over(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
    ) -> Result<(), RenderError> {
        let taken = self.take_tiles(core, request)?;
        if taken.is_empty() {
            return Ok(());
        }
        let near: Vec<PixelRect> = taken.iter().map(|tile| around_output(tile, 1)).collect();
        self.reach(core, request.artwork, &near)?;
        let held = self.held().ok_or(NO_SCENE)?;
        let format = self.texture.format();
        for tile in taken {
            let Some((slot, into)) = self.raster.slot(core, format)? else {
                break;
            };
            let into = JobInto::Tile {
                view: into,
                format,
                at: [slot * TIER_TILE, 0],
            };
            let output = Output::at(1, tile.min.map(i32::unsigned_abs))?;
            let id = self.order(request, (output, tile.size()), held.clone(), into)?;
            self.raster.handed.push(Handed {
                id,
                rect: tile,
                slot,
                stale: false,
            });
        }
        Ok(())
    }

    /// Orders the raster worker to draw the region of `size` pixels at `output` of the scene the
    /// last draw drew, on `request`'s paper, its residuals as `held` holds them, `into` its
    /// destination; the job's id.
    pub(crate) fn order(
        &mut self,
        request: &FrameRequest<'_>,
        (output, size): (Output, [u32; 2]),
        held: Held,
        into: JobInto,
    ) -> Result<u64, RenderError> {
        let cache = self.scene.as_ref().ok_or(NO_SCENE)?;
        let id = self.raster.next_id();
        let job = RasterJob {
            id,
            scene: Arc::clone(&cache.scene),
            buffers: SceneBuffers::of(&cache.ops, held),
            output,
            size,
            paper: paper(request),
            into,
        };
        self.raster.orders.push(RasterOrder::Draw(Box::new(job)));
        Ok(id)
    }

    /// The tiles to hand over now: the next waiting ones not with the worker while it has room
    /// for them and, while the screen is touched, the frame's share (each estimated at what the
    /// last cost), until one reads a residual still being evaluated (a share of it is, now).
    fn take_tiles(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
    ) -> Result<Vec<PixelRect>, RenderError> {
        let whole = PixelRect::full(self.size());
        let idle = core.pace().idle();
        let mut taken: Vec<PixelRect> = Vec::new();
        while self.raster.handed.len() + taken.len() < AHEAD {
            let busy: Vec<PixelRect> = (self.raster.handed.iter())
                .map(|handed| handed.rect)
                .chain(self.raster.done.iter().map(|(handed, _)| handed.rect))
                .chain(taken.iter().copied())
                .collect();
            let skip = |rect: &PixelRect| busy.contains(&rect.intersect(&whole));
            let Some(tile) = self.raster.waiting.first(self.focus, skip) else {
                break;
            };
            let tile = tile.intersect(&whole);
            if !self.residuals_ready(core, request, &[around_output(&tile, 1)])? {
                break;
            }
            if !idle && !core.pace().promise(self.raster.expected()) {
                break;
            }
            taken.push(tile);
        }
        Ok(taken)
    }

    /// The worker's answers applied: each tile it drew that no change reached copied from its
    /// slot into the canvas, no longer waiting; one dropped or reached waits still. Returns the
    /// canvas pixels copied.
    pub(in crate::canvas) fn apply_answers(&mut self, core: &Core) -> Vec<PixelRect> {
        let done = std::mem::take(&mut self.raster.done);
        let Some(staging) = self.raster.staging().cloned() else {
            return Vec::new();
        };
        let mut encoder = core.gpu.device.create_command_encoder(&Default::default());
        let mut applied = Vec::new();
        for (handed, drawn) in done {
            if !drawn || handed.stale {
                continue;
            }
            encoder.copy_texture_to_texture(
                place(&staging, [handed.slot * TIER_TILE, 0]),
                place(&self.texture, handed.rect.min.map(i32::unsigned_abs)),
                extent(handed.rect.size()),
            );
            self.raster.waiting.remove(&[handed.rect]);
            applied.push(handed.rect);
        }
        if !applied.is_empty() {
            core.gpu.submit([encoder.finish()]);
        }
        applied
    }
}
