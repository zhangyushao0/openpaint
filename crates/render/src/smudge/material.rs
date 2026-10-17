//! What a smudge carries: its layer's pixels just before it, into tiles of a texture of their own
//! (premultiplied, transparent elsewhere, 8 bits like the canvas): drawn by the raster core for a
//! scene it draws (S62, `material_raster.rs`), else evaluated by the single evaluator from the
//! layer's earlier ops in the target's own scene. A pixel never depends on which regions are drawn
//! with it (D82, D86), so a tile made now or later holds the same.

use std::ops::Range;

use super::{Arena, held_in};
use crate::affine::Affine;
use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::gpu_clock::{GpuClock, Timing};
use crate::output::Output;
use crate::plan::Plan;
use crate::program::{Bindings, Dispatch, SceneOps};
use crate::protocol::{Op, TileRecord};
use crate::raster::RasterTargets;
use crate::readback::{FORMAT, target_texture};
use crate::scene::Scene;

#[path = "material_raster.rs"]
mod raster;
pub(crate) use raster::{MaterialRegion, regions};

pub(crate) struct Material {
    view: wgpu::TextureView,
    bindings: Bindings,
    /// The raster core's targets drawing it (S62), made on first use.
    raster: Option<RasterTargets>,
}

impl Material {
    /// A texture of the canvas's `size`; its pixels are undefined until evaluated.
    pub(crate) fn new(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        let usage = wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT;
        let texture = target_texture(&core.gpu, (size, FORMAT), usage)?;
        Ok(Self {
            view: texture.create_view(&Default::default()),
            bindings: Bindings::without_kept(&core.gpu, &texture),
            raster: None,
        })
    }

    pub(crate) const fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Bytes of its texture, bindings and raster targets.
    pub(crate) fn bytes(&self) -> u64 {
        let [width, height] = self.bindings.size();
        let raster = self.raster.as_ref().map_or(0, RasterTargets::bytes);
        u64::from(width) * u64::from(height) * 4 + self.bindings.bytes() + raster
    }

    /// Makes the scene's items in `range` (one layer's ops before a smudge) over `rects`, on
    /// nothing and at full opacity: that layer's own pixels there, where the smudge's pixels lie
    /// on the canvas by `to_canvas` (under transform records, S31; the evaluator's scenes only).
    /// The layer's earlier smudges' residuals are the arena's (`arena`, with `empty` for the
    /// in-progress one).
    pub(crate) fn make(
        &mut self,
        core: &Core,
        (scene, ops): (&Scene, &SceneOps),
        (range, to_canvas): (Range<usize>, Affine),
        rects: &[PixelRect],
        (arena, empty): (&Arena, &wgpu::Buffer),
    ) -> Result<(), RenderError> {
        if rects.is_empty() {
            return Ok(());
        }
        if scene.rastered().is_some() {
            let held = held_in(arena, empty, scene);
            return self.draw(core, (scene, ops), &range, rects, held);
        }
        let plan = self.plan(scene, (range, to_canvas), rects);
        let residuals = (([arena.buffer(), empty]), None);
        self.evaluate_tiles(core, ops, (&plan, &plan.tiles), to_canvas, residuals)
    }

    /// The plan the evaluator's material over `rects` is evaluated by.
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
