//! `CanvasTarget` and `RegionEvaluator`: the single entry that draws a frame request into
//! regions of a canvas target. Since S65 (DECISIONS D92) every scene is drawn by the raster core
//! (`canvas_raster_fill.rs`), the stroke being drawn too (`drawing_frame.rs`).
//!
//! What remains of the evaluator, the per-pixel interpreter over binned op lists (D82), and why
//! (S66 deletes it): the reference image the tests, harness `canvas-check` and `export-check`
//! (`Offscreen::evaluated_canvas`) and the device's `bench raster` compare the raster core with,
//! and its benchmark (`Core::{evaluated_image, time_evaluation}`, `evaluator_bench.rs`, `cost.rs`,
//! runtime `bench.rs`, harness `predict.rs`): a target made by `Core::reference`
//! (`CanvasTarget::reference`) evaluated whole as one piece (`evaluate_piece`,
//! `evaluator_dispatch.rs`, `bounded.rs`), its scene binned (`plan.rs`, `plan_bin.rs`) and
//! dispatched (`program*.rs`, evaluate.wgsl), its smudges' and filters' materials the evaluator's
//! (`Smudges::evaluating`, `smudge/material.rs`). One product path still bins: a filter's block
//! under a transform record is where `Plan::material` lists anything (`filter.rs`,
//! `content_rect`). Nothing else reaches it.

use openpaint_core::{ActiveStroke, Artwork, RegionMask};

use crate::canvas_raster::CanvasRaster;
use crate::core::Core;
use crate::drawing::Drawing;
use crate::error::RenderError;
use crate::filter::Filters;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::image::RgbaImage;
use crate::output::Output;
use crate::program::{Bindings, SceneOps};
use crate::readback::{FORMAT, read_rgba, read_texel, target_texture};
use crate::scene::{ActivePlace, Scene};
use crate::screen::{VIEW_BYTES, blit_bind_group, selection_texture};
use crate::smudge::Smudges;
use crate::waiting::Focus;

/// A texture holding the canvas at one target pixel per canvas pixel (8 bits per channel, or
/// for the screen 10), kept between draws so later ones update only what changed.
pub struct CanvasTarget {
    texture: wgpu::Texture,
    /// Binds the texture and the view for showing it on screen (`CanvasBlit`).
    shown: wgpu::BindGroup,
    /// The view the screen shows the canvas through, written each frame it is drawn.
    view: wgpu::Buffer,
    /// The evaluator's bindings: a reference's only (S65: evaluated whole by the evaluator, its
    /// smudges' and filters' materials too).
    bindings: Option<Bindings>,
    scene: Option<SceneCache>,
    /// The selection the screen shows over the canvas (`show_selection`).
    selection: Option<RegionMask>,
    /// The smudges' residuals and the filters' results the scene draws (D46, D59).
    smudges: Smudges,
    filters: Filters,
    /// The canvas pixels draws changed since a view cache last took them (S32).
    changed: Vec<PixelRect>,
    /// Drawing on the 1x canvas by the raster core (S63, `drawing.rs`); the canvas pixels the
    /// version under the stroke changed this frame (its mirrored copies), which its view tiers
    /// draw too; the canvas pixels changes reached since the view tiers last took them (their
    /// drawings' snapshots there go).
    pub(crate) drawing: Drawing,
    pub(crate) moved: Vec<PixelRect>,
    dirtied: Vec<PixelRect>,
    /// Where the screen looks: the waiting tiles it shows go first (S53).
    focus: Option<Focus>,
    /// Nothing was drawn into it yet: its pixels are undefined.
    blank: bool,
    /// Shown complete from a display cache (S54): residuals evaluated so far while it warms.
    warming: Option<usize>,
    /// The raster core's side (S62): what waits for it, its targets, the worker's tiles.
    pub(crate) raster: CanvasRaster,
    /// Bytes its snapshots under the next stroke may grow to (S49: what the GPU memory budget
    /// leaves them).
    snapshot_room: u64,
    /// Its draws are a worker's: each region waited for (S53, `pace_dispatches`).
    paced: bool,
}

/// More changed rectangles than this are kept as the one rectangle holding them.
const CHANGED_KEPT: usize = 256;

impl CanvasTarget {
    /// A target of `size` pixels for the screen, in the device's display format (S37: ten bits
    /// per channel where it has them, opaque); `read` and `pick` give the 8-bit values an
    /// 8-bit target would hold, exactly. Its content is undefined until a region is drawn.
    pub fn new(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        Self::of(core, (size, core.gpu.display), false)
    }

    /// As `new`, in 8 bits per channel whatever the device shows (tests: what a ten-bit canvas
    /// reads as).
    #[cfg(test)]
    pub(crate) fn eight_bits(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        Self::of(core, (size, FORMAT), false)
    }

    /// An 8-bit target of `size` pixels the evaluator evaluates whole: a reference image
    /// (`Core::reference`, S62; the evaluator's one use since S65).
    pub(crate) fn reference(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        Self::of(core, (size, FORMAT), true)
    }

    fn of(
        core: &Core,
        size: ([u32; 2], wgpu::TextureFormat),
        reference: bool,
    ) -> Result<Self, RenderError> {
        // Copied into for a display cache's pixels (S54) and the raster worker's tiles, drawn into
        // by the raster core (S62), written by the gallery picture's enlarging (S54) and, for a
        // reference, by the evaluator.
        let usage = wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT;
        // First: it rejects sizes the device cannot hold.
        let texture = target_texture(&core.gpu, size, usage)?;
        let view = core.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("canvas view"),
            size: VIEW_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let none = selection_texture(core, &[], 1);
        Ok(Self {
            shown: blit_bind_group(core, &texture, &view, &none),
            view,
            bindings: reference.then(|| Bindings::new(&core.gpu, &texture)),
            raster: CanvasRaster::new(&texture),
            texture,
            scene: None,
            selection: None,
            smudges: if reference {
                Smudges::evaluating(core)
            } else {
                Smudges::new(core)
            },
            filters: Filters::default(),
            changed: Vec::new(),
            drawing: Drawing::default(),
            moved: Vec::new(),
            dirtied: Vec::new(),
            focus: None,
            blank: true,
            warming: None,
            snapshot_room: u64::MAX,
            paced: false,
        })
    }

    /// The canvas pixels draws changed since the last call (the stroke being drawn, what the
    /// caller passed): what a view tier draws again.
    pub(crate) fn take_changed(&mut self) -> Vec<PixelRect> {
        std::mem::take(&mut self.changed)
    }

    /// Changes reached the canvas pixels `rects` (S63: kept for the view tiers' drawings).
    pub(crate) fn dirtied(&mut self, rects: &[PixelRect]) {
        self.dirtied.extend_from_slice(rects);
        if self.dirtied.len() > CHANGED_KEPT
            && let Some(all) = self.dirtied.iter().copied().reduce(|a, b| a.union(&b))
        {
            self.dirtied = vec![all];
        }
    }

    /// The canvas pixels changes reached since the last call.
    pub(crate) fn take_dirtied(&mut self) -> Vec<PixelRect> {
        std::mem::take(&mut self.dirtied)
    }

    /// The scene and its ops on the GPU the last draw drew, when it drew `artwork` with the
    /// in-progress stroke `active`.
    pub(crate) fn scene_of(
        &self,
        artwork: &Artwork,
        active: Option<&ActiveStroke>,
    ) -> Option<(&std::sync::Arc<Scene>, &SceneOps, u64)> {
        let place = ActivePlace::of_request(artwork, active);
        self.scene
            .as_ref()
            .filter(|cache| cache.artwork.same_version(artwork) && cache.active == place)
            .map(|cache| (&cache.scene, &cache.ops, cache.serial))
    }

    /// The screen shows `selection` over the canvas from the next frame on: an outline around
    /// it and light stripes outside; `None` shows none. Uploads only when it changes. The
    /// canvas's pixels are untouched: the selection is tool state, not artwork.
    pub fn show_selection(&mut self, core: &Core, selection: Option<&RegionMask>) {
        let same = match (&self.selection, selection) {
            (Some(shown), Some(new)) => shown.shares(new),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        let texture = match selection {
            Some(mask) => selection_texture(core, mask.words(), mask.stride()),
            None => selection_texture(core, &[], 1),
        };
        self.shown = blit_bind_group(core, &self.texture, &self.view, &texture);
        self.selection = selection.cloned();
    }

    /// The shown selection's rectangle: top-left pixel and size.
    pub(crate) fn selection_rect(&self) -> Option<([u32; 2], [u32; 2])> {
        self.selection
            .as_ref()
            .map(|mask| (mask.origin(), mask.size()))
    }

    /// `[width, height]` in pixels.
    #[must_use]
    pub fn size(&self) -> [u32; 2] {
        [self.texture.width(), self.texture.height()]
    }

    /// Waits for every submitted draw and copies the target into memory.
    pub fn read(&self, core: &Core) -> Result<RgbaImage, RenderError> {
        read_rgba(&core.gpu, &self.texture)
    }

    /// Render picking: waits for every submitted draw and reads the RGBA pixel at `[x, y]` (the
    /// colour the canvas shows there, everything composited); `None` outside the target. The
    /// same bytes `read` gives at that position.
    pub fn pick(&self, core: &Core, at: [u32; 2]) -> Result<Option<[u8; 4]>, RenderError> {
        read_texel(&core.gpu, &self.texture, at)
    }

    pub(crate) fn shown(&self) -> &wgpu::BindGroup {
        &self.shown
    }

    pub(crate) fn view_buffer(&self) -> &wgpu::Buffer {
        &self.view
    }

    pub(crate) fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Tiles wait to be drawn (S62: by the raster core, the worker's included). The target shows
    /// them as they were until then; `read` and `pick` read them so.
    #[must_use]
    pub fn pending(&self) -> bool {
        self.raster.pending()
    }

    /// Where the screen looks on the canvas from now on: the waiting tiles it shows are drawn
    /// first, nearest its middle (S53); `None`: rows from the top.
    pub fn look_at(&mut self, focus: Option<Focus>) {
        self.focus = focus;
    }

    /// Its draws are a worker's: each region waited for before the next (S53), so the screen's
    /// frames find the GPU free between them (the brush editor's strip, brush pictures).
    pub fn pace_dispatches(&mut self) {
        self.paced = true;
    }
}

/// Draws frame requests into one target.
pub struct RegionEvaluator<'a> {
    core: &'a Core,
    target: &'a mut CanvasTarget,
}

impl<'a> RegionEvaluator<'a> {
    #[must_use]
    pub fn new(core: &'a Core, target: &'a mut CanvasTarget) -> Self {
        Self { core, target }
    }

    /// Draws every pixel of `regions` (clipped to the target) from `request` now; pixels
    /// outside keep their values. A pixel's result does not depend on which regions are drawn
    /// with it, so updating only what changed equals drawing it all.
    ///
    /// The in-progress stroke needs no regions: the raster core draws where its new samples
    /// change the canvas, erases a stroke that disappeared while the artwork stayed the same
    /// version (cancelled), and draws the committed stroke at its lift (S63). Every other
    /// artwork change is the caller's to pass as regions.
    ///
    /// # Errors
    /// `Invalid` when the target's size is not the canvas size or the active stroke's layer is
    /// not in the artwork.
    pub fn evaluate(
        &mut self,
        request: &FrameRequest<'_>,
        regions: &[PixelRect],
    ) -> Result<(), RenderError> {
        self.canvas(request, regions, false)
    }

    /// As `evaluate`, for one frame of the screen (S49, S62): a change too large for what is
    /// left of the frame waits, and the raster worker draws its tiles over the next frames
    /// (`CanvasTarget::pending`); the stroke being drawn never waits.
    ///
    /// # Errors
    /// As `evaluate`.
    pub fn evaluate_frame(
        &mut self,
        request: &FrameRequest<'_>,
        regions: &[PixelRect],
    ) -> Result<(), RenderError> {
        self.canvas(request, regions, true)
    }

    fn canvas(
        &mut self,
        request: &FrameRequest<'_>,
        regions: &[PixelRect],
        frame: bool,
    ) -> Result<(), RenderError> {
        let canvas = request.artwork.canvas();
        let size = self.target.size();
        if size != [canvas.width(), canvas.height()] {
            return Err(RenderError::Invalid(
                "the target's size differs from the canvas size",
            ));
        }
        if self.target.bindings.is_some() {
            return Err(RenderError::Invalid(
                "a reference is evaluated as one piece",
            ));
        }
        let started = std::time::Instant::now();
        let drawn = self.drawn(request, regions, frame);
        crate::profile::evaluated(started.elapsed());
        let changed = &mut self.target.changed;
        changed.extend(drawn?);
        if changed.len() > CHANGED_KEPT
            && let Some(all) = changed.iter().copied().reduce(|a, b| a.union(&b))
        {
            *changed = vec![all];
        }
        Ok(())
    }

    /// Evaluates the whole of a reference target as the piece of the output at `output` (its
    /// scale and the output pixel at its top-left) by the evaluator, every op at the output
    /// pixel's centre mapped back to the canvas (S65: the evaluator's one use).
    ///
    /// # Errors
    /// `Invalid` for a request with an in-progress stroke or a target that is no reference.
    pub(crate) fn evaluate_piece(
        &mut self,
        request: &FrameRequest<'_>,
        output: Output,
    ) -> Result<(), RenderError> {
        if request.active.is_some() || self.target.bindings.is_none() {
            return Err(RenderError::Invalid(
                "only a reference is evaluated as a piece, with no stroke in progress",
            ));
        }
        self.dispatch(request, output)
    }
}

#[path = "evaluator_dispatch.rs"]
mod dispatching;

#[path = "evaluator_scene.rs"]
mod scene_cache;
use scene_cache::SceneCache;

#[path = "evaluator_budget.rs"]
mod budget;
#[path = "evaluator_residuals.rs"]
mod residuals;

#[path = "evaluator_cache.rs"]
mod cache;
pub use cache::CanvasCopy;

#[path = "canvas_raster_fill.rs"]
mod raster_fill;
pub(crate) use raster_fill::{Planned, paper};

#[path = "drawing_frame.rs"]
mod drawing_frame;
pub(crate) use drawing_frame::Onto;
#[path = "drawing_ahead.rs"]
mod drawing_ahead;
pub(crate) use drawing_ahead::TAKING;

#[path = "evaluator_bench.rs"]
mod bench;
pub(crate) use bench::WARM_ENOUGH;
pub use bench::{Listed, Timed};

#[cfg(test)]
#[path = "evaluator_display_tests.rs"]
mod display_tests;
