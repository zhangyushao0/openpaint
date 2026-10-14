//! `RegionEvaluator`: the single entry that evaluates a frame request into regions of a target.
//! Every consumer (screen, export, picking) goes through it.

use openpaint_core::{ActiveStroke, Artwork, RegionMask};

use crate::active::ActiveTrack;
use crate::active_snapshot::ActiveSnapshot;
use crate::core::Core;
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
use crate::waiting::{Focus, Waiting};

/// A texture holding the canvas at one target pixel per canvas pixel (8 bits per channel, or
/// for the screen 10), kept between evaluations so later ones update only what changed, with
/// the in-progress stroke's coverage.
pub struct CanvasTarget {
    texture: wgpu::Texture,
    /// Binds the texture and the view for showing it on screen (`CanvasBlit`).
    shown: wgpu::BindGroup,
    /// The view the screen shows the canvas through, written each frame it is drawn.
    view: wgpu::Buffer,
    bindings: Bindings,
    track: ActiveTrack,
    scene: Option<SceneCache>,
    /// The selection the screen shows over the canvas (`show_selection`).
    selection: Option<RegionMask>,
    /// The smudges' residuals and the filters' results the scene draws (D46, D59).
    smudges: Smudges,
    filters: Filters,
    /// The canvas pixels evaluations changed since a view cache last took them (S32).
    changed: Vec<PixelRect>,
    /// What the last evaluation cost (S48).
    work: EvaluationWork,
    /// The in-progress stroke's layer before it, per tile (S48, `active_snapshot.rs`).
    held: Option<ActiveSnapshot>,
    /// Its tiles lack strokes committed since: frames lay them (S56, `catch_up`).
    catching: bool,
    /// Whole tiles the screen's frames left for the next ones (S49, S53: `evaluate_frame`).
    waiting: Waiting,
    /// Where the screen looks: the waiting tiles it shows go first (S53).
    focus: Option<Focus>,
    /// Nothing was evaluated into it yet: its pixels are undefined.
    blank: bool,
    /// Shown complete from a display cache (S54): residuals evaluated so far while it warms.
    warming: Option<usize>,
}

/// More changed rectangles than this are kept as the one rectangle holding them.
const CHANGED_KEPT: usize = 256;

impl CanvasTarget {
    /// A target of `size` pixels for the screen, in the device's display format (S37: ten bits
    /// per channel where it has them, opaque); `read` and `pick` give the 8-bit values an
    /// 8-bit target would hold, exactly. Its content is undefined until a region is evaluated.
    pub fn new(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        Self::with_kept(core, (size, core.gpu.display), true)
    }

    /// An 8-bit target of `size` pixels for a piece of a scaled output (export) or one layer's
    /// pixels (their alpha too): it never draws an in-progress stroke, so it keeps no coverage
    /// of one.
    pub(crate) fn exact(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        Self::with_kept(core, (size, FORMAT), false)
    }

    fn with_kept(
        core: &Core,
        size: ([u32; 2], wgpu::TextureFormat),
        kept: bool,
    ) -> Result<Self, RenderError> {
        // Copied into for a display cache's pixels (S54).
        let usage = wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST;
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
            bindings: if kept {
                Bindings::new(&core.gpu, &texture)
            } else {
                Bindings::without_kept(&core.gpu, &texture)
            },
            texture,
            track: ActiveTrack::default(),
            scene: None,
            selection: None,
            smudges: Smudges::new(core),
            filters: Filters::default(),
            changed: Vec::new(),
            work: EvaluationWork::default(),
            held: None,
            catching: false,
            waiting: Waiting::new(size.0),
            focus: None,
            blank: true,
            warming: None,
        })
    }

    /// The canvas pixels evaluations changed since the last call (the in-progress stroke's new
    /// samples, what the caller passed, what a dropped or committed stroke left): what a view
    /// tier evaluates again.
    pub(crate) fn take_changed(&mut self) -> Vec<PixelRect> {
        std::mem::take(&mut self.changed)
    }

    /// The scene and its ops on the GPU the last evaluation drew, when it drew `artwork` with
    /// the in-progress stroke `active`: what a view tier evaluates too, with the same smudges'
    /// residuals (`residuals`).
    pub(crate) fn scene_of(
        &self,
        artwork: &Artwork,
        active: Option<&ActiveStroke>,
    ) -> Option<(&Scene, &SceneOps)> {
        let place = ActivePlace::of_request(artwork, active);
        self.scene
            .as_ref()
            .filter(|cache| cache.artwork.same_version(artwork) && cache.active == place)
            .map(|cache| (&cache.scene, &cache.ops))
    }

    /// The smudge arena and the in-progress smudge's residual, as the last evaluation left
    /// them.
    pub(crate) fn residuals(&self) -> [&wgpu::Buffer; 2] {
        self.smudges.residuals()
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

    /// Waits for every submitted evaluation and copies the target into memory.
    pub fn read(&self, core: &Core) -> Result<RgbaImage, RenderError> {
        read_rgba(&core.gpu, &self.texture)
    }

    /// Render picking: waits for every submitted evaluation and reads the RGBA pixel at `[x, y]`
    /// (the colour the canvas shows there, everything composited); `None` outside the target.
    /// The same bytes `read` gives at that position.
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

    /// What the last evaluation into this target cost (`paint-timing`, S48).
    #[must_use]
    pub const fn work(&self) -> EvaluationWork {
        self.work
    }

    /// The snapshot of the in-progress stroke's layer lacks strokes committed since, which the
    /// screen's next frames lay within their budget (S56): frames are wanted until it does not.
    #[must_use]
    pub const fn catching_up(&self) -> bool {
        self.catching
    }
}

/// Evaluates frame requests into one target.
pub struct RegionEvaluator<'a> {
    core: &'a Core,
    target: &'a mut CanvasTarget,
}

impl<'a> RegionEvaluator<'a> {
    #[must_use]
    pub fn new(core: &'a Core, target: &'a mut CanvasTarget) -> Self {
        Self { core, target }
    }

    /// Recomputes every pixel of `regions` (clipped to the target, widened to whole tiles) from
    /// `request`; pixels outside keep their values. A pixel's result does not depend on which
    /// regions are evaluated with it, so updating only what changed equals evaluating it all.
    ///
    /// The in-progress stroke needs no regions: the evaluator adds the pixels its new samples
    /// can change, erases a stroke that disappeared while the artwork stayed the same version
    /// (cancelled), and draws the samples a committed stroke gained since its last evaluation;
    /// the rest of a committed stroke already shows exactly as the new version would. Every
    /// other artwork change is the caller's to pass as regions.
    ///
    /// Tiles an earlier `evaluate_frame` left are evaluated too. The work goes to the GPU in
    /// bounded dispatches (S49, `bounded.rs`).
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

    /// As `evaluate`, for one frame of the screen (S49): when the regions' work passes what one
    /// frame takes, the tiles beyond wait for the next calls (`CanvasTarget::pending`), whole
    /// tiles in order, so a large change fills in over frames and the frame stays short. The
    /// in-progress stroke's pixels never wait, and while a stroke is drawn nothing does.
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
        let started = std::time::Instant::now();
        let dispatched = self.dispatch(request, Output::CANVAS, (regions, frame));
        crate::profile::evaluated(started.elapsed());
        match dispatched {
            Ok(evaluated) => {
                let changed = &mut self.target.changed;
                changed.extend(evaluated);
                if changed.len() > CHANGED_KEPT
                    && let Some(all) = changed.iter().copied().reduce(|a, b| a.union(&b))
                {
                    *changed = vec![all];
                }
                Ok(())
            }
            Err(error) => {
                self.target.track.forget();
                Err(error)
            }
        }
    }

    /// Evaluates the whole target as the piece of the output at `output` (its scale and the
    /// output pixel at its top-left), or the part of it inside `region` (target pixels): every
    /// op at the output pixel's centre mapped back to the canvas. At `Output::CANVAS` it is
    /// `evaluate` over `region`. The scene and the smudges' residuals are kept for the next
    /// piece of the same version.
    ///
    /// # Errors
    /// `Invalid` for a request with an in-progress stroke (only the canvas draws one).
    pub(crate) fn evaluate_piece(
        &mut self,
        request: &FrameRequest<'_>,
        output: Output,
        region: PixelRect,
    ) -> Result<(), RenderError> {
        if request.active.is_some() {
            return Err(RenderError::Invalid(
                "an output piece never draws an in-progress stroke",
            ));
        }
        self.dispatch(request, output, (&[region], false)).map(drop)
    }
}

#[path = "evaluator_dispatch.rs"]
mod dispatching;

#[path = "evaluator_scene.rs"]
mod scene_cache;
pub use scene_cache::EvaluationWork;
use scene_cache::SceneCache;

#[path = "evaluator_background.rs"]
mod background;
#[path = "evaluator_budget.rs"]
mod budget;
#[path = "evaluator_layer.rs"]
mod layer;
#[path = "evaluator_residuals.rs"]
mod residuals;

#[path = "evaluator_cache.rs"]
mod cache;
pub use cache::CanvasCopy;

#[path = "evaluator_bench.rs"]
mod bench;
pub(crate) use bench::WARM_ENOUGH;
pub use bench::{Listed, Timed};

#[cfg(test)]
#[path = "evaluator_display_tests.rs"]
mod display_tests;

#[cfg(test)]
#[path = "evaluator_snapshot_tests.rs"]
mod snapshot_tests;
