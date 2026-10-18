//! The canvas's waiting tiles handed to the raster worker and its answers applied (S62, DECISIONS
//! D89; `canvas_raster.rs`). Up to `AHEAD` tiles wait with the worker, the screen's middle first,
//! each with the smudge residuals it reads present (a tile reading one still being evaluated
//! waits, a share of it evaluated each frame); while the screen is touched a frame hands over only
//! what its share takes (S60). Each is drawn into a slot of a staging texture, which the frame
//! loop copies into the canvas once answered, unless a change reached the tile meanwhile: then it
//! is handed over again.

use std::sync::Arc;

use super::super::CanvasTarget;
use super::paper;
use crate::canvas_raster::{AHEAD, Handed};
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::raster::SceneBuffers;
use crate::smudge::around_output;
use crate::view_raster_jobs::{JobInto, RasterJob, RasterOrder};
use crate::view_tiles::TIER_TILE;

impl CanvasTarget {
    /// Hands the worker the next waiting tiles, as many as may wait with it.
    pub(in crate::evaluator) fn hand_over(
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
        let held = self
            .held()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let format = self.texture.format();
        let missing = || RenderError::Invalid("no scene was built");
        for tile in taken {
            let Some((slot, into)) = self.raster.slot(core, format)? else {
                break;
            };
            let cache = self.scene.as_ref().ok_or_else(missing)?;
            let id = self.raster.next_id();
            let size = [0, 1].map(|axis| (tile.max[axis] - tile.min[axis]).unsigned_abs());
            let job = RasterJob {
                id,
                scene: Arc::clone(&cache.scene),
                buffers: SceneBuffers::of(&cache.ops, held.clone()),
                output: Output::at(1, tile.min.map(i32::unsigned_abs))?,
                size,
                paper: paper(request),
                into: JobInto::Tile {
                    view: into,
                    format,
                    at: [slot * TIER_TILE, 0],
                },
            };
            self.raster.orders.push(RasterOrder::Draw(Box::new(job)));
            self.raster.handed.push(Handed {
                id,
                rect: tile,
                slot,
                stale: false,
            });
        }
        Ok(())
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
    pub(in crate::evaluator) fn apply_answers(&mut self, core: &Core) -> Vec<PixelRect> {
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
            let [width, height] =
                [0, 1].map(|axis| (handed.rect.max[axis] - handed.rect.min[axis]).unsigned_abs());
            let [x, y] = handed.rect.min.map(i32::unsigned_abs);
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    origin: wgpu::Origin3d {
                        x: handed.slot * TIER_TILE,
                        y: 0,
                        z: 0,
                    },
                    ..staging.as_image_copy()
                },
                wgpu::TexelCopyTextureInfo {
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    ..self.texture.as_image_copy()
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
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
