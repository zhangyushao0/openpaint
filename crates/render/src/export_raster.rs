//! An export's pieces drawn by the raster core (S59, DECISIONS D86): each piece of a band in
//! regions of at most `REGION` pixels, each rounded into the piece's 8-bit texture where it lies,
//! then the piece read back. A pixel depends only on what reaches it, so the regions, the pieces
//! and the bands make what one region over the whole output would.

use std::time::{Duration, Instant};

use openpaint_core::Color;

use crate::core::Core;
use crate::error::RenderError;
use crate::image::RgbaImage;
use crate::output::Output;
use crate::program::{SceneOps, scene_ops};
use crate::raster::{ATLAS, Finish, RasterTargets, SceneBuffers};
use crate::raster_plan::RasterPlan;
use crate::raster_route::RasterScene;
use crate::readback::{FORMAT, read_rect, target_texture};
use crate::scene::Scene;

/// The largest region drawn at once: 8 MB a float target.
const REGION: [u32; 2] = [1024, 512];

/// An export's scene for the raster core, its targets and its 8-bit piece.
pub(crate) struct RasterExport {
    scene: Scene,
    raster: RasterScene,
    ops: SceneOps,
    targets: RasterTargets,
    piece: wgpu::Texture,
}

impl RasterExport {
    /// The export of `scene` (drawn as `raster`) in pieces of up to `piece` pixels.
    ///
    /// # Errors
    /// As the GPU: sizes the device cannot hold.
    pub(crate) fn new(
        core: &Core,
        (scene, raster): (Scene, RasterScene),
        piece: [u32; 2],
    ) -> Result<Self, RenderError> {
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let region = [0, 1].map(|axis| REGION[axis].min(piece[axis]));
        Ok(Self {
            ops: scene_ops(&core.gpu, &scene, None),
            scene,
            raster,
            targets: RasterTargets::new(core, region, ATLAS)?,
            piece: target_texture(&core.gpu, (piece, FORMAT), usage)?,
        })
    }

    /// The output's `size` pixels from output pixel `at` at `scale`, on `paper`, drawn and read
    /// back; the time spent drawing (CPU) and waiting for the pixels.
    ///
    /// # Errors
    /// As the raster core and the readback.
    pub(crate) fn piece(
        &mut self,
        core: &Core,
        (scale, [left, top]): (u32, [u32; 2]),
        [columns, rows]: [u32; 2],
        paper: Color,
    ) -> Result<(RgbaImage, [Duration; 2]), RenderError> {
        let started = Instant::now();
        let view = self.piece.create_view(&Default::default());
        let background = [paper.r(), paper.g(), paper.b(), 1.0];
        let [width, height] = self.targets.capacity();
        for y in (0..rows).step_by(height as usize) {
            for x in (0..columns).step_by(width as usize) {
                let size = [width.min(columns - x), height.min(rows - y)];
                let output = Output::at(scale, [left + x, top + y])?;
                let scene = (&self.scene, &self.raster);
                let plan = RasterPlan::build(scene, (output, size), self.targets.atlas())?;
                let finish = Finish {
                    view: &view,
                    format: FORMAT,
                    at: [x, y],
                };
                let scene = SceneBuffers::of(&self.ops, 0);
                let drawn = (&scene, &plan);
                (self.targets).draw(core, drawn, (output, background), &finish, None)?;
                core.evaluate.tally.rastered(1);
            }
        }
        let submitted = Instant::now();
        let image = read_rect(&core.gpu, &self.piece, [0, 0], [columns, rows])?;
        Ok((image, [submitted - started, submitted.elapsed()]))
    }
}
