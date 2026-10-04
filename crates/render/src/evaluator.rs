//! `RegionEvaluator`: the single entry that evaluates a frame request into regions of a target.
//! Every consumer (screen, export, picking) goes through it.

use openpaint_core::{Artwork, LayerId, RegionMask, StrokeTool};

use crate::active::ActiveTrack;
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::image::RgbaImage;
use crate::output::Output;
use crate::plan::Plan;
use crate::program::{Bindings, Dispatch, SceneOps, scene_ops};
use crate::readback::{read_rgba, read_texel, target_texture};
use crate::scene::{ActivePlace, Fresh, Scene};
use crate::screen::{VIEW_BYTES, blit_bind_group, selection_texture};
use crate::smudge::Smudges;

/// An 8-bit RGBA texture holding the canvas at one target pixel per canvas pixel. It keeps its
/// pixels between evaluations, so later evaluations update only the regions that changed. It
/// also keeps the in-progress stroke's coverage so far and the last scene's ops on the GPU.
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
    /// The smudges' residuals the scene draws (D46).
    smudges: Smudges,
}

/// One artwork version's scene and its ops on the GPU, reused while the version stays.
struct SceneCache {
    artwork: Artwork,
    active: Option<ActivePlace>,
    scene: Scene,
    ops: SceneOps,
}

impl CanvasTarget {
    /// A target of `size` pixels. Its content is undefined until a region is evaluated.
    pub fn new(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        Self::with_kept(core, size, true)
    }

    /// A target of `size` pixels for a piece of a scaled output (export): it never draws an
    /// in-progress stroke, so it keeps no coverage of one.
    pub(crate) fn piece(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        Self::with_kept(core, size, false)
    }

    fn with_kept(core: &Core, size: [u32; 2], kept: bool) -> Result<Self, RenderError> {
        let usage = wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING;
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
        })
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

    /// The scene of `artwork` with the in-progress stroke's place `active`, built and uploaded
    /// only when either differs from the last call, with every smudge it draws evaluated.
    fn scene(
        &mut self,
        core: &Core,
        artwork: &Artwork,
        active: Option<ActivePlace>,
    ) -> Result<&SceneCache, RenderError> {
        let reuse = self
            .scene
            .as_ref()
            .is_some_and(|cache| cache.artwork.same_version(artwork) && cache.active == active);
        if !reuse {
            let known = self.scene.as_ref().map(|cache| cache.scene.edges.clone());
            let scene = Scene::build_knowing(artwork, active.as_ref(), known.unwrap_or_default())?;
            let patches = self.scene.as_ref().and_then(|cache| {
                let (old, new) = (&cache.scene.patches, &scene.patches);
                let same = old.len() == new.len() && old.iter().zip(new).all(|(a, b)| a.shares(b));
                same.then(|| cache.ops.patches.clone())
            });
            let ops = scene_ops(&core.gpu, &scene, patches);
            self.smudges.prepare(core, (&scene, &ops), artwork)?;
            self.scene = Some(SceneCache {
                artwork: artwork.clone(),
                active,
                scene,
                ops,
            });
        }
        self.scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))
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
    /// # Errors
    /// `Invalid` when the target's size is not the canvas size or the active stroke's layer is
    /// not in the artwork.
    pub fn evaluate(
        &mut self,
        request: &FrameRequest<'_>,
        regions: &[PixelRect],
    ) -> Result<(), RenderError> {
        let canvas = request.artwork.canvas();
        let size = self.target.size();
        if size != [canvas.width(), canvas.height()] {
            return Err(RenderError::Invalid(
                "the target's size differs from the canvas size",
            ));
        }
        let result = self.dispatch(request, Output::CANVAS, regions);
        if result.is_err() {
            self.target.track.forget();
        }
        result
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
        self.dispatch(request, output, &[region])
    }

    /// Evaluates `layer` of `artwork` alone over the whole target, at full opacity, on nothing
    /// (transparent black), whether the layer is shown or not: premultiplied layer pixels for a
    /// fill to sample. With `mask`, its mask alone on white: the mask's grey, opaque. The
    /// target then holds no frame of the artwork.
    pub(crate) fn evaluate_layer(
        &mut self,
        artwork: &Artwork,
        layer: LayerId,
        mask: bool,
    ) -> Result<(), RenderError> {
        let size = self.target.size();
        let canvas = artwork.canvas();
        if size != [canvas.width(), canvas.height()] {
            return Err(RenderError::Invalid(
                "the target's size differs from the canvas size",
            ));
        }
        let scene = Scene::layer(artwork, layer, mask)?;
        let ops = scene_ops(&self.core.gpu, &scene, None);
        let target = (size, Output::CANVAS);
        let plan = Plan::build(&scene, &Fresh::default(), target, &[PixelRect::full(size)]);
        self.target.scene = None;
        self.target.track.forget();
        let smudges = &mut self.target.smudges;
        smudges.end_active();
        smudges.prepare(self.core, (&scene, &ops), artwork)?;
        self.core.evaluate.dispatch(
            &self.core.gpu,
            &mut self.target.bindings,
            &Dispatch {
                ops: &ops,
                fresh: &[],
                plan: &plan,
                output: Output::CANVAS,
                background: if mask { [1.0; 4] } else { [0.0; 4] },
                reset: false,
                residuals: smudges.residuals(),
            },
        )
    }

    fn dispatch(
        &mut self,
        request: &FrameRequest<'_>,
        output: Output,
        regions: &[PixelRect],
    ) -> Result<(), RenderError> {
        let core = self.core;
        let place = request.active.map(ActivePlace::of);
        let scene = &self.target.scene(core, request.artwork, place)?.scene;
        let (drawn, clip) = (scene.has_active(), scene.active_clip);
        let step = self
            .target
            .track
            .advance(request.artwork, request.active, drawn);
        let fresh = match request.active.filter(|_| drawn) {
            Some(active) => Fresh::new(active, step.fresh.start, clip),
            None => Fresh::default(),
        };
        let mut all = regions.to_vec();
        all.extend(step.redraw.into_iter().chain(step.fresh_region));
        let size = self.target.size();
        let target = &mut *self.target;
        let cache = target
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let smudging = request
            .active
            .filter(|active| drawn && active.style().tool() == StrokeTool::Smudge);
        match smudging {
            Some(active) => {
                let scene = (&cache.scene, &cache.ops);
                let restarted = target
                    .smudges
                    .advance(core, scene, request.artwork, active)?;
                all.extend(restarted);
            }
            None => target.smudges.end_active(),
        }
        if all.is_empty() && !step.reset {
            return Ok(());
        }
        let plan = Plan::build(&cache.scene, &fresh, (size, output), &all);
        if plan.tiles.is_empty() && !step.reset {
            return Ok(());
        }
        let paper = request.background;
        core.evaluate.dispatch(
            &core.gpu,
            &mut target.bindings,
            &Dispatch {
                ops: &cache.ops,
                fresh: &fresh.ops,
                plan: &plan,
                output,
                background: [paper.r(), paper.g(), paper.b(), 1.0],
                reset: step.reset,
                residuals: target.smudges.residuals(),
            },
        )
    }
}
