//! `CanvasTarget`: a texture holding an artwork's canvas, and the one entry that draws a frame
//! request into regions of it (`draw`, `draw_frame`). The raster core draws every scene: changes
//! at once when they fit the frame, else as the raster worker's tiles (`canvas_raster_fill.rs`),
//! the stroke being drawn where it changes the canvas (`drawing_frame.rs`). Its scene, smudge
//! residuals and filter results are kept between draws (`canvas_scene.rs`, `canvas_budget.rs`).

use openpaint_core::{ActiveStroke, Artwork, RegionMask};

use crate::canvas_raster::CanvasRaster;
use crate::core::Core;
use crate::drawing::Drawing;
use crate::error::RenderError;
use crate::filter::Filters;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::image::RgbaImage;
use crate::readback::{read_rgba, read_texel, target_texture};
use crate::scene::{ActivePlace, Scene};
use crate::scene_ops::SceneOps;
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
    scene: Option<SceneCache>,
    /// The selection the screen shows over the canvas (`show_selection`).
    selection: Option<RegionMask>,
    /// The smudges' residuals and the filters' results the scene draws.
    smudges: Smudges,
    filters: Filters,
    /// The canvas pixels draws changed since a view cache last took them.
    changed: Vec<PixelRect>,
    /// Drawing on the 1x canvas (`drawing.rs`); the canvas pixels the version under the
    /// stroke changed this frame (its mirrored copies), which its view tiers draw too; the canvas
    /// pixels changes reached since the view tiers last took them (their drawings' snapshots
    /// there go).
    pub(crate) drawing: Drawing,
    pub(crate) moved: Vec<PixelRect>,
    dirtied: Vec<PixelRect>,
    /// Where the screen looks: the waiting tiles it shows go first.
    focus: Option<Focus>,
    /// Nothing was drawn into it yet: its pixels are undefined.
    blank: bool,
    /// Shown complete from a display cache: residuals derived so far while it warms.
    warming: Option<usize>,
    /// What waits to be drawn, the frame loop's raster targets, the worker's tiles.
    pub(crate) raster: CanvasRaster,
    /// Bytes its snapshots under the next stroke may grow to (what the GPU memory budget
    /// leaves them).
    snapshot_room: u64,
    /// Its draws are a worker's: each region waited for (`pace_draws`).
    paced: bool,
}

/// More changed rectangles than this are kept as the one rectangle holding them.
const CHANGED_KEPT: usize = 256;

/// The error for a draw that needs the scene before one was built.
pub(crate) const NO_SCENE: RenderError = RenderError::Invalid("no scene was built");

/// `rects` added to the changed rectangles `list`, kept as one past `CHANGED_KEPT`.
fn kept(list: &mut Vec<PixelRect>, rects: &[PixelRect]) {
    list.extend_from_slice(rects);
    if list.len() > CHANGED_KEPT
        && let Some(all) = list.iter().copied().reduce(|a, b| a.union(&b))
    {
        *list = vec![all];
    }
}

impl CanvasTarget {
    /// A target of `size` pixels for the screen, in the device's display format (ten bits
    /// per channel where it has them, opaque); `read` and `pick` give the 8-bit values an
    /// 8-bit target would hold, exactly. Its content is undefined until a region is drawn.
    pub fn new(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        Self::of(core, (size, core.gpu.display))
    }

    /// As `new`, in 8 bits per channel whatever the device shows (tests: what a ten-bit canvas
    /// reads as).
    #[cfg(test)]
    pub(crate) fn eight_bits(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        Self::of(core, (size, crate::readback::FORMAT))
    }

    fn of(core: &Core, size: ([u32; 2], wgpu::TextureFormat)) -> Result<Self, RenderError> {
        // Copied into for a display cache's pixels and the raster worker's tiles, drawn into
        // by the raster core, written by the gallery picture's enlarging.
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
            raster: CanvasRaster::new(&texture),
            texture,
            scene: None,
            selection: None,
            smudges: Smudges::new(core),
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

    /// Draws every pixel of `regions` (clipped to the target) from `request` now; pixels
    /// outside keep their values. A pixel's result does not depend on which regions are drawn
    /// with it, so updating only what changed equals drawing it all.
    ///
    /// The in-progress stroke needs no regions: the raster core draws where its new samples
    /// change the canvas, erases a stroke that disappeared while the artwork stayed the same
    /// version (cancelled), and draws the committed stroke at its lift. Every other
    /// artwork change is the caller's to pass as regions.
    ///
    /// # Errors
    /// `Invalid` when the target's size is not the canvas size or the active stroke's layer is
    /// not in the artwork.
    pub fn draw(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        regions: &[PixelRect],
    ) -> Result<(), RenderError> {
        self.canvas(core, request, (regions, false))
    }

    /// As `draw`, for one frame of the screen: a change too large for what is left of
    /// the frame waits, and the raster worker draws its tiles over the next frames (`pending`);
    /// the stroke being drawn never waits.
    ///
    /// # Errors
    /// As `draw`.
    pub fn draw_frame(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        regions: &[PixelRect],
    ) -> Result<(), RenderError> {
        self.canvas(core, request, (regions, true))
    }

    fn canvas(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        (regions, frame): (&[PixelRect], bool),
    ) -> Result<(), RenderError> {
        let canvas = request.artwork.canvas();
        if self.size() != [canvas.width(), canvas.height()] {
            return Err(RenderError::Invalid(
                "the target's size differs from the canvas size",
            ));
        }
        let started = std::time::Instant::now();
        let drawn = self.drawn(core, request, regions, frame);
        crate::profile::drawn(started.elapsed());
        kept(&mut self.changed, &drawn?);
        Ok(())
    }

    /// The canvas pixels draws changed since the last call (the stroke being drawn, what the
    /// caller passed): what a view tier draws again.
    pub(crate) fn take_changed(&mut self) -> Vec<PixelRect> {
        std::mem::take(&mut self.changed)
    }

    /// Changes reached the canvas pixels `rects` (kept for the view tiers' drawings).
    pub(crate) fn dirtied(&mut self, rects: &[PixelRect]) {
        kept(&mut self.dirtied, rects);
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

    /// Tiles wait to be drawn (the raster worker's included). The target shows them as they
    /// were until then; `read` and `pick` read them so.
    #[must_use]
    pub fn pending(&self) -> bool {
        self.raster.pending()
    }

    /// Where the screen looks on the canvas from now on: the waiting tiles it shows are drawn
    /// first, nearest its middle; `None`: rows from the top.
    pub fn look_at(&mut self, focus: Option<Focus>) {
        self.focus = focus;
    }

    /// Its draws are a worker's: each region waited for before the next, so the screen's
    /// frames find the GPU free between them (the brush editor's strip, brush pictures).
    pub fn pace_draws(&mut self) {
        self.paced = true;
    }
}

#[path = "canvas_scene.rs"]
mod scene_cache;
use scene_cache::SceneCache;

#[path = "canvas_budget.rs"]
mod budget;
#[path = "canvas_residuals.rs"]
mod residuals;

#[path = "canvas_cache.rs"]
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

#[cfg(test)]
#[path = "canvas_display_tests.rs"]
mod display_tests;
