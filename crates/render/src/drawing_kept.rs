//! The kept coverage of the stroke being drawn (S63, DECISIONS D90; `drawing_frame.rs`): every
//! frame lays the spans that settled since into the slots of the output tiles they reach, in
//! order, by the blend unit (max or add), a coverage page cleared first when its slots were freed
//! since it was laid into; the cells they reach noted, so a region composites the stroke only
//! where it lies. The frame's batch begins with it.

use super::{CanvasTarget, Onto, Part, Pen, meeting};
use crate::core::Core;
use crate::drawing::Drawing;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::raster_lay::laid_span;
use crate::raster_plan::{Coverage, CoverageDraw, Pass, RasterPlan};
use crate::view_tiles::TIER_TILE;

impl CanvasTarget {
    /// The tiles the stroke reaches given their kept coverage; the spans settled since laid into
    /// it (and the cells they reach noted), the pages freed since they were laid into cleared
    /// first: the batch's first region.
    pub(super) fn lay_kept<'a>(
        &mut self,
        core: &Core,
        drawing: &mut Drawing,
        (pen, onto): (&Pen, &Onto<'a>),
        batch: &mut Vec<Part<'a>>,
    ) -> Result<(), RenderError> {
        if pen.smudge.is_some() {
            return Ok(());
        }
        let atlas = self.raster_atlas(core)?;
        let format = core.raster_pipelines()?.atlas;
        let scale = onto.scale;
        let output = Output::at(scale, [0, 0])?;
        let size = self.size().map(|side| side * scale);
        let mut plan = RasterPlan {
            size: [1, 1],
            ..RasterPlan::default()
        };
        let mut draws: Vec<(usize, CoverageDraw)> = Vec::new();
        for (index, (at, op, bounds)) in pen.settled.iter().chain(&pen.tail).enumerate() {
            for (tile, part) in meeting(output.place(&bounds.pixels()), size) {
                if bounds.is_empty() || (onto.place)(tile).is_none() {
                    continue;
                }
                let slots = drawing.kept(core, tile, (format, atlas))?;
                if index >= pen.settled.len() {
                    continue;
                }
                let origin = tile.map(|at| at * TIER_TILE);
                if let Some(held) = drawing.kept.get_mut(&tile) {
                    held.cells |= cells(&part, origin);
                }
                let side = [0, 1].map(|axis| TIER_TILE.min(size[axis] - origin[axis]));
                let kinds = if pen.contact {
                    vec![(Coverage::Density, slots[0]), (Coverage::Contact, slots[1])]
                } else {
                    vec![(pen.coverage, slots[0])]
                };
                for (coverage, slot) in kinds {
                    let place = [0, 1].map(|axis| slot.at[axis] as i32 - origin[axis] as i32);
                    let start = plan.instances.len() as u32;
                    laid_span(&mut plan, (*op, *at as usize), place, scale as f32);
                    let draw = CoverageDraw {
                        coverage,
                        scissor: [slot.at[0], slot.at[1], side[0], side[1]],
                        instances: start..plan.instances.len() as u32,
                    };
                    draws.push((slot.page, draw));
                }
            }
        }
        cover_passes(&mut plan, (draws, drawing.take_stale()));
        if !plan.passes.is_empty() {
            batch.push(Part {
                plan,
                output,
                paper: [0.0; 4],
                finish: None,
            });
        }
        Ok(())
    }
}

/// A kept coverage plan's passes: per page, `draws` laid into it, cleared first when it is among
/// the `stale` pages (cleared whether or not anything is laid there now).
fn cover_passes(plan: &mut RasterPlan, (draws, stale): (Vec<(usize, CoverageDraw)>, Vec<usize>)) {
    let mut pages: Vec<usize> = (draws.iter().map(|(page, _)| *page))
        .chain(stale.iter().copied())
        .collect();
    pages.sort_unstable();
    pages.dedup();
    for page in pages {
        let draws = (draws.iter())
            .filter(|(at, _)| *at == page)
            .map(|(_, draw)| draw.clone())
            .collect();
        let clear = stale.contains(&page);
        plan.passes.push(Pass::Cover { page, clear, draws });
    }
}

/// The 32-px cells of the output tile at `origin` that output pixels `part` (inside it) meet.
fn cells(part: &PixelRect, origin: [u32; 2]) -> u64 {
    let cell = 32;
    let [x0, y0] = [0, 1].map(|axis| (part.min[axis] - origin[axis] as i32) / cell);
    let [x1, y1] = [0, 1].map(|axis| (part.max[axis] - origin[axis] as i32 - 1) / cell);
    let mut cells = 0;
    for y in y0.max(0)..=y1.min(7) {
        for x in x0.max(0)..=x1.min(7) {
            cells |= 1 << (y * 8 + x);
        }
    }
    cells
}
