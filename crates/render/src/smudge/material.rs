//! What a smudge carries: its layer's pixels just before it, evaluated by the single evaluator
//! from the layer's earlier ops in the target's own scene, into tiles of a texture of their own
//! (premultiplied, transparent elsewhere, 8 bits like the canvas). A pixel never depends on
//! which regions are evaluated with it (D16), so a tile evaluated now or later holds the same.

use std::ops::Range;

use crate::affine::Affine;
use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::plan::Plan;
use crate::program::{Bindings, Dispatch, SceneOps};
use crate::protocol::Op;
use crate::readback::target_texture;
use crate::scene::Scene;

pub(crate) struct Material {
    view: wgpu::TextureView,
    bindings: Bindings,
}

impl Material {
    /// A texture of the canvas's `size`; its pixels are undefined until evaluated.
    pub(crate) fn new(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        let usage = wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING;
        let texture = target_texture(&core.gpu, size, usage)?;
        Ok(Self {
            view: texture.create_view(&Default::default()),
            bindings: Bindings::without_kept(&core.gpu, &texture),
        })
    }

    pub(crate) const fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Evaluates the scene's items in `range` (one layer's ops before a smudge) over `rects`,
    /// on nothing and at full opacity: that layer's own pixels there, where the smudge's pixels
    /// lie on the canvas by `to_canvas` (under transform records, S31). `residuals` are the
    /// arena and an empty in-progress residual, for the layer's earlier smudges.
    pub(crate) fn evaluate(
        &mut self,
        core: &Core,
        (scene, ops): (&Scene, &SceneOps),
        (range, to_canvas): (Range<usize>, Affine),
        rects: &[PixelRect],
        residuals: [&wgpu::Buffer; 2],
    ) -> Result<(), RenderError> {
        if rects.is_empty() {
            return Ok(());
        }
        let size = self.bindings.size();
        let plan = Plan::material(scene, (range, to_canvas), size, rects);
        core.evaluate.dispatch(
            &core.gpu,
            &mut self.bindings,
            &Dispatch {
                ops,
                fresh: &[Op::end_layer(1.0)],
                plan: &plan,
                only: None,
                output: Output::CANVAS,
                background: [0.0; 4],
                reset: false,
                residuals,
                base: to_canvas,
                snapshot: None,
            },
        )
    }
}
