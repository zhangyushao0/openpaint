//! What a smudge or a filter reads: its layer's pixels just before it, in a texture of the
//! canvas's size (premultiplied, transparent elsewhere, 8 bits like the canvas). They are the
//! units laying that layer's items before it, drawn by the raster core onto a canvas of nothing
//! (`RasterPlan::material`, where a transform record's content maps them) and rounded to 8 bits,
//! over one region per 256-px cell of the canvas (the box of the rectangles wanted in it), only
//! those rectangles written. A pixel never depends on which regions are drawn with it, so a tile
//! made now or later holds the same.

use std::collections::BTreeMap;
use std::ops::Range;

use super::{Arena, held_in};
use crate::affine::Affine;
use crate::core::Core;
use crate::error::RenderError;
use crate::filter::Filters;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::raster::{Finish, Placed, RasterTargets, SceneBuffers};
use crate::raster_plan::RasterPlan;
use crate::readback::{FORMAT, target_texture};
use crate::scene::Scene;
use crate::scene_ops::SceneOps;
use crate::smudge::Bases;
use crate::view_tiles::TIER_TILE;

/// The atlas of a material's raster targets, pixels a side: room for several of a cell's
/// strokes' regions.
const ATLAS: u32 = 1024;

pub(crate) struct Material {
    view: wgpu::TextureView,
    size: [u32; 2],
    /// The raster core's targets drawing it, made on first use.
    raster: Option<RasterTargets>,
}

impl Material {
    /// A texture of the canvas's `size`; its pixels are undefined until made.
    pub(crate) fn new(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let texture = target_texture(&core.gpu, (size, FORMAT), usage)?;
        Ok(Self {
            view: texture.create_view(&Default::default()),
            size,
            raster: None,
        })
    }

    pub(crate) const fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Bytes of its texture and raster targets.
    pub(crate) fn bytes(&self) -> u64 {
        let [width, height] = self.size;
        let raster = self.raster.as_ref().map_or(0, RasterTargets::bytes);
        u64::from(width) * u64::from(height) * 4 + raster
    }

    /// Makes the scene's items in `range` (one layer's before a smudge or filter) over `rects`,
    /// on nothing and at full opacity: that layer's own pixels there, where its pixels lie on the
    /// canvas by `to_canvas` (under a transform record). The layer's earlier smudges'
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
        let buffers = SceneBuffers::of(ops, held_in(arena, empty, (scene, filters)));
        for region in regions(rects) {
            let units = ((scene, buffers.bases()), &range, to_canvas);
            let planned = self.plan_region(core, units, &region)?;
            self.draw_region(core, &planned, &region, &buffers)?;
        }
        Ok(())
    }
}

/// Where a material is drawn: a region (canvas pixels) and the residual's tiles in it that the
/// finish writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MaterialRegion {
    pub(crate) region: PixelRect,
    pub(crate) tiles: Vec<PixelRect>,
}

/// The regions of the rectangles `rects` (a residual's tiles, a filter's read; material pixels):
/// one per 256-px cell of the canvas, each rectangle cut at the cells' edges, rows from the top.
pub(crate) fn regions(rects: &[PixelRect]) -> Vec<MaterialRegion> {
    let side = TIER_TILE as i32;
    let mut cells: BTreeMap<[i32; 2], MaterialRegion> = BTreeMap::new();
    for whole in rects.iter().filter(|rect| !rect.is_empty()) {
        let [x0, y0] = whole.min.map(|at| at.div_euclid(side));
        let [x1, y1] = whole.max.map(|at| (at - 1).div_euclid(side));
        for (x, y) in (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| (x, y))) {
            let cell = PixelRect {
                min: [x * side, y * side],
                max: [(x + 1) * side, (y + 1) * side],
            };
            let rect = whole.intersect(&cell);
            cells
                .entry([y, x])
                .and_modify(|cell| {
                    cell.region = cell.region.union(&rect);
                    cell.tiles.push(rect);
                })
                .or_insert_with(|| MaterialRegion {
                    region: rect,
                    tiles: vec![rect],
                });
        }
    }
    cells.into_values().collect()
}

impl Material {
    /// The plan of `region` of the material of `scene`'s `items` (one layer's before a smudge or
    /// filter) whose pixels lie on the canvas by `base`, its earlier smudges' residuals and
    /// filters' results at `bases`, and the output it lies at; the raster targets made on first
    /// use.
    ///
    /// # Errors
    /// As the raster core.
    pub(crate) fn plan_region(
        &mut self,
        core: &Core,
        ((scene, bases), items, base): ((&Scene, &Bases), &Range<usize>, Affine),
        region: &MaterialRegion,
    ) -> Result<(RasterPlan, Output), RenderError> {
        if self.raster.is_none() {
            self.raster = Some(RasterTargets::new(core, [TIER_TILE; 2], ATLAS)?);
        }
        let atlas = self.raster.as_ref().map_or([0; 2], RasterTargets::atlas);
        let rect = region.region;
        let output = Output::at(1, rect.min.map(i32::unsigned_abs))?;
        let size = rect.size();
        let plan = RasterPlan::material((scene, bases), (items, base), (output, size), atlas)?;
        Ok((plan, output))
    }

    /// Draws `planned` (`plan_region`'s, of `region`) of the scene in `buffers` into the
    /// material's texture: only the region's tiles.
    ///
    /// # Errors
    /// As the raster core.
    pub(crate) fn draw_region(
        &mut self,
        core: &Core,
        (plan, output): &(RasterPlan, Output),
        region: &MaterialRegion,
        buffers: &SceneBuffers,
    ) -> Result<(), RenderError> {
        let Some(targets) = self.raster.as_mut() else {
            return Err(RenderError::Invalid("no raster targets for the material"));
        };
        let finish = Finish {
            view: &self.view,
            format: FORMAT,
            into: vec![Placed {
                at: region.region.min,
                only: region.tiles.clone(),
            }],
            texture: None,
        };
        targets.draw(core, (buffers, plan), (*output, [0.0; 4]), &finish)?;
        core.tally.rastered(1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn residual_tiles_are_drawn_a_region_per_canvas_cell() {
        let tile = |x: i32, y: i32| PixelRect {
            min: [x, y],
            max: [x + 32, y + 32],
        };
        let found = regions(&[tile(224, 0), tile(256, 0), tile(192, 32), tile(256, 288)]);
        assert_eq!(found.len(), 3, "{found:?}");
        assert_eq!(
            found[0].region,
            PixelRect {
                min: [192, 0],
                max: [256, 64]
            }
        );
        assert_eq!(found[0].tiles, [tile(224, 0), tile(192, 32)]);
        assert_eq!(found[1].tiles, [tile(256, 0)]);
        assert_eq!(found[2].tiles, [tile(256, 288)]);
    }
}
