//! What a smudge or a filter reads: its layer's pixels just before it, into tiles of a texture of
//! their own (premultiplied, transparent elsewhere, 8 bits like the canvas), drawn by the raster
//! core (S62, `material_raster.rs`; every scene since S65, D92); a reference's (the evaluator's
//! image the raster core is compared with) evaluated by the evaluator from the layer's earlier
//! ops. A pixel never depends on which regions are drawn with it (D82, D86), so a tile made now
//! or later holds the same.

use std::ops::Range;

use super::{Arena, held_in};
use crate::affine::Affine;
use crate::core::Core;
use crate::error::RenderError;
use crate::filter::Filters;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::plan::Plan;
use crate::program::{Bindings, Dispatch, SceneOps};
use crate::protocol::Op;
use crate::raster::RasterTargets;
use crate::readback::{FORMAT, target_texture};
use crate::scene::Scene;

#[path = "material_raster.rs"]
mod raster;
pub(crate) use raster::{MaterialRegion, regions};

pub(crate) struct Material {
    view: wgpu::TextureView,
    size: [u32; 2],
    /// The evaluator's bindings: a reference's material (S65), which it evaluates.
    bindings: Option<Bindings>,
    /// The raster core's targets drawing it (S62), made on first use.
    raster: Option<RasterTargets>,
}

impl Material {
    /// A texture of the canvas's `size`, `evaluated` by the evaluator (a reference's) or drawn by
    /// the raster core; its pixels are undefined until made.
    pub(crate) fn new(core: &Core, size: [u32; 2], evaluated: bool) -> Result<Self, RenderError> {
        let written = if evaluated {
            wgpu::TextureUsages::STORAGE_BINDING
        } else {
            wgpu::TextureUsages::RENDER_ATTACHMENT
        };
        let usage = written | wgpu::TextureUsages::TEXTURE_BINDING;
        let texture = target_texture(&core.gpu, (size, FORMAT), usage)?;
        Ok(Self {
            view: texture.create_view(&Default::default()),
            size,
            bindings: evaluated.then(|| Bindings::new(&core.gpu, &texture)),
            raster: None,
        })
    }

    pub(crate) const fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Bytes of its texture, bindings and raster targets.
    pub(crate) fn bytes(&self) -> u64 {
        let [width, height] = self.size;
        let bindings = self.bindings.as_ref().map_or(0, Bindings::bytes);
        let raster = self.raster.as_ref().map_or(0, RasterTargets::bytes);
        u64::from(width) * u64::from(height) * 4 + bindings + raster
    }

    /// Makes the scene's items in `range` (one layer's before a smudge or filter) over `rects`,
    /// on nothing and at full opacity: that layer's own pixels there, where its pixels lie on the
    /// canvas by `to_canvas` (under a transform record, S31). The layer's earlier smudges'
    /// residuals and filters' results are the arena's (`arena`, with `empty` for the in-progress
    /// residual), as `filters` names the filters'.
    pub(crate) fn make(
        &mut self,
        core: &Core,
        (scene, ops): (&Scene, &SceneOps),
        (range, to_canvas): (Range<usize>, Affine),
        rects: &[PixelRect],
        (arena, empty, filters): (&Arena, &wgpu::Buffer, &Filters),
    ) -> Result<(), RenderError> {
        if rects.is_empty() {
            return Ok(());
        }
        let Some(bindings) = self.bindings.as_mut() else {
            let held = held_in(arena, empty, (scene, filters));
            return self.draw(core, (scene, ops), (&range, to_canvas), rects, held);
        };
        let plan = Plan::material(scene, (range, to_canvas), self.size, rects);
        core.evaluate.dispatch(
            &core.gpu,
            bindings,
            &Dispatch {
                ops,
                fresh: &[Op::end_layer(1.0)],
                plan: &plan,
                output: Output::CANVAS,
                background: [0.0; 4],
                residuals: [arena.buffer(), empty],
                base: to_canvas,
                timing: None,
            },
        )
    }
}
