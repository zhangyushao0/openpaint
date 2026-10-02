//! `RegionEvaluator`: the single entry that evaluates a frame request into regions of a target.
//! Every consumer (screen, export, picking) goes through it.

use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::image::RgbaImage;
use crate::plan::Plan;
use crate::readback::{read_rgba, target_texture};
use crate::scene::Scene;

/// An 8-bit RGBA texture holding the canvas at one target pixel per canvas pixel. It keeps its
/// pixels between evaluations, so later evaluations may update only the regions that changed.
pub struct CanvasTarget {
    texture: wgpu::Texture,
}

impl CanvasTarget {
    /// A target of `size` pixels. Its content is undefined until a region is evaluated.
    pub fn new(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        let usage = wgpu::TextureUsages::STORAGE_BINDING;
        Ok(Self {
            texture: target_texture(&core.gpu, size, usage)?,
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
}

/// Evaluates frame requests into one target.
pub struct RegionEvaluator<'a> {
    core: &'a Core,
    target: &'a CanvasTarget,
}

impl<'a> RegionEvaluator<'a> {
    #[must_use]
    pub fn new(core: &'a Core, target: &'a CanvasTarget) -> Self {
        Self { core, target }
    }

    /// Recomputes every pixel of `regions` (clipped to the target, widened to whole tiles) from
    /// `request`; pixels outside keep their values. A pixel's result does not depend on which
    /// regions are evaluated with it, so updating only what changed equals evaluating it all.
    ///
    /// # Errors
    /// `Invalid` when the target's size is not the canvas size or the active stroke's layer is
    /// not in the artwork.
    pub fn evaluate(
        &self,
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
        let scene = Scene::build(request)?;
        let plan = Plan::build(&scene, size, regions);
        if plan.tiles.is_empty() {
            return Ok(());
        }
        let paper = request.background;
        let background = [paper.r(), paper.g(), paper.b(), 1.0];
        let program = &self.core.evaluate;
        program.dispatch(
            &self.core.gpu,
            &scene,
            &plan,
            background,
            &self.target.texture,
        )
    }
}
