//! What a smudge carries: its layer's pixels just before it, evaluated by the single evaluator
//! from the layer's earlier ops in the target's own scene, into tiles of a texture of their own
//! (premultiplied, transparent elsewhere, 8 bits like the canvas). A pixel never depends on
//! which regions are evaluated with it (D82), so a tile evaluated now or later holds the same.

use std::ops::Range;

use crate::affine::Affine;
use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::gpu_clock::{GpuClock, Timing};
use crate::output::Output;
use crate::plan::Plan;
use crate::program::{Bindings, Dispatch, SceneOps};
use crate::protocol::{Op, TileRecord};
use crate::readback::{FORMAT, target_texture};
use crate::scene::Scene;

pub(crate) struct Material {
    view: wgpu::TextureView,
    bindings: Bindings,
}

impl Material {
    /// A texture of the canvas's `size`; its pixels are undefined until evaluated.
    pub(crate) fn new(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        let usage = wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING;
        let texture = target_texture(&core.gpu, (size, FORMAT), usage)?;
        Ok(Self {
            view: texture.create_view(&Default::default()),
            bindings: Bindings::without_kept(&core.gpu, &texture),
        })
    }

    pub(crate) const fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Bytes of its texture and bindings.
    pub(crate) fn bytes(&self) -> u64 {
        let [width, height] = self.bindings.size();
        u64::from(width) * u64::from(height) * 4 + self.bindings.bytes()
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
        let plan = self.plan(scene, (range, to_canvas), rects);
        self.evaluate_tiles(
            core,
            ops,
            (&plan, &plan.tiles),
            to_canvas,
            (residuals, None),
        )
    }

    /// The plan `evaluate` evaluates over `rects`.
    pub(crate) fn plan(
        &self,
        scene: &Scene,
        (range, to_canvas): (Range<usize>, Affine),
        rects: &[PixelRect],
    ) -> Plan {
        Plan::material(scene, (range, to_canvas), self.bindings.size(), rects)
    }

    /// Evaluates `tiles` of `plan` (from `plan`) into the texture: a residual evaluated over
    /// frames as background work takes its material a share at a time (S53), timed by `timing`.
    pub(crate) fn evaluate_tiles(
        &mut self,
        core: &Core,
        ops: &SceneOps,
        (plan, tiles): (&Plan, &[TileRecord]),
        to_canvas: Affine,
        (residuals, timing): ([&wgpu::Buffer; 2], Option<(&GpuClock, Timing)>),
    ) -> Result<(), RenderError> {
        core.evaluate.dispatch(
            &core.gpu,
            &mut self.bindings,
            &Dispatch {
                ops,
                fresh: &[Op::end_layer(1.0)],
                plan,
                only: Some(tiles),
                output: Output::CANVAS,
                background: [0.0; 4],
                reset: false,
                residuals,
                base: to_canvas,
                snapshot: None,
                timing,
            },
        )
    }
}
