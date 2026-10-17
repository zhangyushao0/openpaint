//! A smudge's material drawn by the raster core (S62, DECISIONS D89): for a scene the raster core
//! draws, its layer's pixels just before the smudge are the units laying that layer's items before
//! it, drawn onto a canvas of nothing (`RasterPlan::material`) and rounded to 8 bits as the
//! evaluator's material was, over one region per 256-px cell of the canvas (the box of the
//! residual's tiles in it), only those tiles written. A scene routed to the evaluator keeps the
//! evaluator's material (`material.rs`; S64 takes it), so routed scenes stay as they were.

use std::collections::BTreeMap;
use std::ops::Range;

use super::Material;
use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::program::SceneOps;
use crate::raster::{Finish, RasterTargets, SceneBuffers};
use crate::raster_plan::RasterPlan;
use crate::readback::FORMAT;
use crate::scene::Scene;
use crate::smudge::Held;
use crate::view_tiles::TIER_TILE;

/// The atlas of a material's raster targets, pixels a side: room for several of a cell's
/// strokes' regions.
const ATLAS: u32 = 1024;

/// Where a material is drawn: a region (canvas pixels) and the residual's tiles in it that the
/// finish writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MaterialRegion {
    pub(crate) region: PixelRect,
    pub(crate) tiles: Vec<PixelRect>,
}

/// The regions of the residual's tiles `rects` (canvas pixels): one per 256-px cell of the
/// canvas they lie in, rows from the top.
pub(crate) fn regions(rects: &[PixelRect]) -> Vec<MaterialRegion> {
    let side = TIER_TILE as i32;
    let mut cells: BTreeMap<[i32; 2], MaterialRegion> = BTreeMap::new();
    for rect in rects.iter().filter(|rect| !rect.is_empty()) {
        let [x, y] = rect.min.map(|at| at.div_euclid(side));
        cells
            .entry([y, x])
            .and_modify(|cell| {
                cell.region = cell.region.union(rect);
                cell.tiles.push(*rect);
            })
            .or_insert_with(|| MaterialRegion {
                region: *rect,
                tiles: vec![*rect],
            });
    }
    cells.into_values().collect()
}

impl Material {
    /// The plan of `region` of the material of `scene`'s `items` (one layer's before a smudge),
    /// its earlier smudges' residuals at `bases`, and the output it lies at; the raster targets
    /// made on first use.
    ///
    /// # Errors
    /// `Invalid` for a scene the evaluator draws; as the raster core.
    pub(crate) fn plan_region(
        &mut self,
        core: &Core,
        (scene, items, bases): (&Scene, &Range<usize>, &[Option<u32>]),
        region: &MaterialRegion,
    ) -> Result<(RasterPlan, Output), RenderError> {
        let raster =
            (scene.rastered()).ok_or(RenderError::Invalid("the scene goes to the evaluator"))?;
        if self.raster.is_none() {
            self.raster = Some(RasterTargets::new(core, [TIER_TILE; 2], ATLAS)?);
        }
        let atlas = self.raster.as_ref().map_or([0; 2], RasterTargets::atlas);
        let rect = region.region;
        let output = Output::at(1, rect.min.map(i32::unsigned_abs))?;
        let size = [0, 1].map(|axis| (rect.max[axis] - rect.min[axis]).unsigned_abs());
        let units = (scene, raster, bases);
        let plan = RasterPlan::material(units, items, (output, size), atlas)?;
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
            at: region.region.min.map(i32::unsigned_abs),
            only: &region.tiles,
        };
        targets.draw(core, (buffers, plan), (*output, [0.0; 4]), &finish, None)?;
        core.evaluate.tally.rastered(1);
        Ok(())
    }

    /// Draws the material of `scene`'s `items` over the residual's tiles `rects` now, the
    /// earlier residuals as `held` holds them.
    ///
    /// # Errors
    /// As the raster core.
    pub(crate) fn draw(
        &mut self,
        core: &Core,
        (scene, ops): (&Scene, &SceneOps),
        items: &Range<usize>,
        rects: &[PixelRect],
        held: Held,
    ) -> Result<(), RenderError> {
        let buffers = SceneBuffers::of(ops, held);
        for region in regions(rects) {
            let units = (scene, items, buffers.bases());
            let planned = self.plan_region(core, units, &region)?;
            self.draw_region(core, &planned, &region, &buffers)?;
        }
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
