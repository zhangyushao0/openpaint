//! `RegionEvaluator`: the single entry that evaluates a frame request into regions of a target.
//! Every consumer (screen, export, picking) goes through it.

use openpaint_core::{Artwork, LayerId};

use crate::active::ActiveTrack;
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::image::RgbaImage;
use crate::plan::Plan;
use crate::program::{Bindings, COVERAGE_FORMAT, Dispatch, scene_ops};
use crate::readback::{read_rgba, read_texel, target_texture};
use crate::scene::{Fresh, Scene};
use crate::screen::blit_bind_group;

/// An 8-bit RGBA texture holding the canvas at one target pixel per canvas pixel. It keeps its
/// pixels between evaluations, so later evaluations update only the regions that changed. It
/// also keeps the in-progress stroke's coverage so far and the last scene's ops on the GPU.
pub struct CanvasTarget {
    texture: wgpu::Texture,
    /// Binds the texture for showing it on screen (`CanvasBlit`).
    shown: wgpu::BindGroup,
    bindings: Bindings,
    track: ActiveTrack,
    scene: Option<SceneCache>,
}

/// One artwork version's scene and its ops on the GPU, reused while the version stays.
struct SceneCache {
    artwork: Artwork,
    active: Option<LayerId>,
    scene: Scene,
    ops: wgpu::Buffer,
}

impl CanvasTarget {
    /// A target of `size` pixels. Its content is undefined until a region is evaluated.
    pub fn new(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        let usage = wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING;
        // First: it rejects sizes the device cannot hold.
        let texture = target_texture(&core.gpu, size, usage)?;
        let coverage = core.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("active coverage"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COVERAGE_FORMAT,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        Ok(Self {
            shown: blit_bind_group(core, &texture),
            bindings: Bindings::new(&core.gpu, &texture, &coverage),
            texture,
            track: ActiveTrack::default(),
            scene: None,
        })
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

    pub(crate) fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// The scene of `artwork` with the in-progress stroke's place on `active`, built and
    /// uploaded only when either differs from the last call.
    fn scene(
        &mut self,
        core: &Core,
        artwork: &Artwork,
        active: Option<LayerId>,
    ) -> Result<&SceneCache, RenderError> {
        let reuse = self
            .scene
            .as_ref()
            .is_some_and(|cache| cache.artwork.same_version(artwork) && cache.active == active);
        if !reuse {
            let scene = Scene::build(artwork, active)?;
            let ops = scene_ops(&core.gpu, &scene);
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
        let result = self.dispatch(request, regions);
        if result.is_err() {
            self.target.track.forget();
        }
        result
    }

    fn dispatch(
        &mut self,
        request: &FrameRequest<'_>,
        regions: &[PixelRect],
    ) -> Result<(), RenderError> {
        let core = self.core;
        let layer = request.active.map(|active| active.layer());
        let drawn = self
            .target
            .scene(core, request.artwork, layer)?
            .scene
            .has_active();
        let step = self
            .target
            .track
            .advance(request.artwork, request.active, drawn);
        let fresh = match request.active.filter(|_| drawn) {
            Some(active) => Fresh::new(active, step.fresh.start),
            None => Fresh::default(),
        };
        let mut all = regions.to_vec();
        all.extend(step.redraw.into_iter().chain(step.fresh_region));
        if all.is_empty() && !step.reset {
            return Ok(());
        }
        let size = self.target.size();
        let target = &mut *self.target;
        let cache = target
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let plan = Plan::build(&cache.scene, &fresh, size, &all);
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
                background: [paper.r(), paper.g(), paper.b(), 1.0],
                reset: step.reset,
            },
        )
    }
}
