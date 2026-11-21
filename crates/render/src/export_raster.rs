//! An export's pieces drawn by the raster core: each piece of a band in regions of at most
//! `REGION` pixels, each rounded into the piece's 8-bit texture where it lies, then the piece read
//! back. A pixel depends only on what reaches it, so the regions, the pieces and the bands make
//! what one region over the whole output would. The export's smudges and filters read residuals
//! and results of its own, made near each region before it is drawn, each once. A layer sample
//! (one layer on nothing, a mask's grey on white) is such an export at 1x of its layer's scene,
//! one piece the canvas's size (`sample.rs`).

use std::time::{Duration, Instant};

use openpaint_core::Artwork;

use crate::core::Core;
use crate::error::RenderError;
use crate::filter::Filters;
use crate::geom::PixelRect;
use crate::image::RgbaImage;
use crate::output::Output;
use crate::raster::{ATLAS, Finish, RasterTargets, SceneBuffers};
use crate::raster_plan::RasterPlan;
use crate::readback::{FORMAT, read_rect, target_texture, wait};
use crate::scene::Scene;
use crate::scene_ops::{SceneOps, scene_ops};
use crate::smudge::{Smudges, around_output};

/// The largest region drawn at once: 8 MB a float target.
const REGION: [u32; 2] = [1024, 512];
/// The largest region a worker draws at once while the screen draws on (`paced`): a canvas
/// tile's, as the frame loop's background pieces are, so a frame queued behind it waits for
/// little (a layer's picture over a dense layer took up to 50 ms of GPU a 1024 x 512 region on
/// the tablet, D97).
const PACED_REGION: [u32; 2] = [256, 256];

/// An export's scene for the raster core, its smudges' residuals, its targets and its 8-bit
/// piece.
pub(crate) struct RasterExport {
    scene: Scene,
    ops: SceneOps,
    smudges: Smudges,
    filters: Filters,
    targets: RasterTargets,
    piece: wgpu::Texture,
}

impl RasterExport {
    /// The export of `scene` in pieces of up to `piece` pixels, its regions a worker's while the
    /// screen draws on when `paced` (`draw`).
    ///
    /// # Errors
    /// As the GPU: sizes the device cannot hold.
    pub(crate) fn new(
        core: &Core,
        scene: Scene,
        (piece, paced): ([u32; 2], bool),
    ) -> Result<Self, RenderError> {
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let most = if paced { PACED_REGION } else { REGION };
        let region = [0, 1].map(|axis| most[axis].min(piece[axis]));
        Ok(Self {
            ops: scene_ops(&core.gpu, &scene, (None, None)),
            scene,
            smudges: Smudges::new(core),
            filters: Filters::default(),
            targets: RasterTargets::new(core, region, ATLAS)?,
            piece: target_texture(&core.gpu, (piece, FORMAT), usage)?,
        })
    }

    /// The piece's texture, as the last `draw` left it.
    pub(crate) const fn texture(&self) -> &wgpu::Texture {
        &self.piece
    }

    /// `artwork`'s output: its `size` pixels from output pixel `at` at `scale`, on `background`
    /// (premultiplied), drawn and read back; the time spent drawing (CPU) and waiting for the
    /// pixels.
    ///
    /// # Errors
    /// As the raster core, the residuals' evaluation and the readback.
    pub(crate) fn piece(
        &mut self,
        core: &Core,
        (artwork, scale, at): (&Artwork, u32, [u32; 2]),
        size: [u32; 2],
        background: [f32; 4],
    ) -> Result<(RgbaImage, [Duration; 2]), RenderError> {
        let started = Instant::now();
        self.draw(core, (artwork, scale, at), size, (background, false))?;
        let submitted = Instant::now();
        let image = read_rect(&core.gpu, &self.piece, [0, 0], size)?;
        Ok((image, [submitted - started, submitted.elapsed()]))
    }

    /// As `piece`, a worker's while the screen draws on: each region waited for before the next.
    ///
    /// # Errors
    /// As `piece`.
    pub(crate) fn paced_piece(
        &mut self,
        core: &Core,
        at: (&Artwork, u32, [u32; 2]),
        size: [u32; 2],
        background: [f32; 4],
    ) -> Result<RgbaImage, RenderError> {
        self.draw(core, at, size, (background, true))?;
        read_rect(&core.gpu, &self.piece, [0, 0], size)
    }

    /// As `piece`, drawn into the piece's texture and not read back; `paced` (a worker's while
    /// the screen draws on), each region waited for before the next.
    ///
    /// # Errors
    /// As the raster core and the residuals' evaluation.
    pub(crate) fn draw(
        &mut self,
        core: &Core,
        (artwork, scale, [left, top]): (&Artwork, u32, [u32; 2]),
        [columns, rows]: [u32; 2],
        (background, paced): ([f32; 4], bool),
    ) -> Result<(), RenderError> {
        let view = self.piece.create_view(&Default::default());
        let [width, height] = self.targets.capacity();
        for y in (0..rows).step_by(height as usize) {
            for x in (0..columns).step_by(width as usize) {
                let size = [width.min(columns - x), height.min(rows - y)];
                let output = Output::at(scale, [left + x, top + y])?;
                let buffers = self.region(core, artwork, (output, size))?;
                let finish = Finish::at(&view, FORMAT, [x, y]);
                let units = (&self.scene, buffers.bases());
                let plan = RasterPlan::build(units, (output, size), self.targets.atlas())?;
                let drawn = (&buffers, &plan);
                (self.targets).draw(core, drawn, (output, background), &finish)?;
                core.tally.rastered(1);
                if paced {
                    wait(&core.gpu, "a worker's raster region")?;
                }
            }
        }
        Ok(())
    }

    /// The scene's buffers for a region of `size` pixels at `output`, the residuals it reads
    /// made present first (the smudge engine's, their material the raster core's).
    fn region(
        &mut self,
        core: &Core,
        artwork: &Artwork,
        (output, size): (Output, [u32; 2]),
    ) -> Result<SceneBuffers, RenderError> {
        let [x, y] = output.origin.map(|at| at as i32);
        let rect = PixelRect {
            min: [x, y],
            max: [x + size[0] as i32, y + size[1] as i32],
        };
        let near = [around_output(&rect, output.scale)];

        let scene = (&self.scene, &self.ops);
        let wanted = (&mut self.filters, &near[..]);
        self.smudges.prepare(core, scene, artwork, wanted)?;
        let held = self.smudges.held(&self.scene, &self.filters);
        Ok(SceneBuffers::of(&self.ops, held))
    }
}
