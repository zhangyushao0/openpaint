//! The regions a frame of a stroke's drawing draws (`drawing_frame.rs`): what the frame
//! changes, as regions as large as the frame loop's targets where the output's tiles lie side by
//! side (the canvas), else a region per tile; each drawn from the snapshots of the tiles it meets
//! (taken now when one has none, or laying the strokes committed since the one it has), the
//! stroke laid at its unit from its kept coverage and its tail, then the units above it.

use std::collections::BTreeSet;

use super::{Onto, Part, Pen, StrokeWork};
use crate::canvas::NO_SCENE;
use crate::canvas::drawing::{Drawing, slot_copies as copies};
use crate::canvas::{CanvasTarget, paper};
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::raster::Finish;
use crate::raster_plan::{Pass, RasterPlan, Resume, SlotCopy};
use crate::scene::Scene;
use crate::smudge::around_output;
use crate::view_tiles::{TIER_TILE, meeting, tile_rect};

impl CanvasTarget {
    /// What `changed` (canvas pixels) reaches of `onto`'s output, as regions pushed onto `batch`:
    /// each from the snapshots of the tiles it meets (from the paper without them) with `pen`
    /// (when a stroke is drawn) laid at its unit; the tiles `onto` does not draw now passed over.
    pub(super) fn draw_parts<'a>(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        drawing: &mut Drawing,
        (onto, scene): (&Onto<'a>, &Scene),
        (changed, pen): (&[PixelRect], Option<&Pen>),
        batch: &mut Vec<Part<'a>>,
    ) -> Result<StrokeWork, RenderError> {
        let scale = onto.scale;
        let size = self.size().map(|side| side * scale);
        self.raster_atlas(core)?;
        let capacity = (self.raster.targets.as_ref()).map_or([TIER_TILE; 2], |t| t.capacity());
        let (regions, skipped) = regions(changed, (onto, size), capacity)?;
        let mut work = StrokeWork {
            skipped,
            ..StrokeWork::default()
        };
        if regions.is_empty() {
            return Ok(work);
        }
        let (needed, resumes) = settle(drawing, &regions, size);
        // The residuals the regions read, and the snapshots brought up (whole tiles).
        let near: Vec<PixelRect> = (regions.iter().copied())
            .chain(needed.iter().map(|tile| tile_rect(*tile, size)))
            .map(|rect| around_output(&rect, scale))
            .collect();
        self.reach(core, request.artwork, &near)?;
        let paper = paper(request);
        for tile in needed {
            let part = self.snapshot(core, drawing, (tile, size), (scene, scale, paper))?;
            batch.push(part);
        }
        for (region, resume) in regions.into_iter().zip(resumes) {
            let place = (onto, scene, paper);
            let part = self.region(core, drawing, (region, size), place, (resume, pen))?;
            batch.push(part);
            work.parts.push(region);
            work.drawn
                .extend(meeting(region, size).map(|(tile, _)| tile));
        }
        work.drawn.sort_unstable();
        work.drawn.dedup();
        Ok(work)
    }

    /// Output pixels `region` of an output of `size` pixels drawn onto `onto` from `resume`: its
    /// targets set from the snapshots of the tiles it meets when restored, the stroke (`pen`)
    /// from its kept coverage there.
    fn region<'a>(
        &mut self,
        core: &Core,
        drawing: &Drawing,
        (region, size): (PixelRect, [u32; 2]),
        (onto, scene, paper): (&Onto<'a>, &Scene, [f32; 4]),
        (resume, pen): (Resume, Option<&Pen>),
    ) -> Result<Part<'a>, RenderError> {
        let held = self.held().ok_or(NO_SCENE)?;
        let atlas = self.raster_atlas(core)?;
        let min = region.min.map(i32::unsigned_abs);
        let output = Output::at(onto.scale, min)?;
        let side = region.size();
        let tiles: Vec<[u32; 2]> = meeting(region, size).map(|(tile, _)| tile).collect();
        let in_progress = pen.map(|pen| pen.in_progress(drawing, &tiles));
        let units = (scene, &held.bases);
        let drawn = (resume, in_progress.as_ref());
        let mut plan = RasterPlan::drawing(units, (output, side), atlas, drawn)?;
        if resume.restored.is_some() {
            plan.passes
                .insert(0, Pass::Restore(restores(drawing, region, size)));
        }
        let first = tiles.first().copied().unwrap_or_default();
        let corner = first.map(|at| at * TIER_TILE);
        let place = (onto.place)(first).ok_or(NO_SCENE)?;
        let finish = Finish {
            view: onto.view,
            format: onto.format,
            at: [0, 1].map(|axis| place[axis] + min[axis] - corner[axis]),
            only: &[],
        };
        Ok(Part {
            plan,
            output,
            paper,
            finish: Some(finish),
        })
    }

    /// Tile `tile`'s snapshot brought up to the drawing's place: taken from the paper when it has
    /// none, else the strokes committed since it was taken laid onto it, in place.
    fn snapshot<'a>(
        &mut self,
        core: &Core,
        drawing: &mut Drawing,
        (tile, size): ([u32; 2], [u32; 2]),
        (scene, scale, paper): (&Scene, u32, [f32; 4]),
    ) -> Result<Part<'a>, RenderError> {
        let base = drawing.base.clone().ok_or(NO_SCENE)?;
        let rect = tile_rect(tile, size);
        let output = Output::at(scale, rect.min.map(i32::unsigned_abs))?;
        let side = rect.size();
        let held = self.held().ok_or(NO_SCENE)?;
        let atlas = self.raster_atlas(core)?;
        let units = (scene, &held.bases);
        let under = drawing.under.get(&tile).cloned();
        let from = under.as_ref().map(|under| under.at);
        let range = (from, base.at, base.restored);
        let mut plan = RasterPlan::snapshot(units, (output, side), atlas, range)?;
        let slots = match under {
            Some(under) => {
                let restore = copies(&under.slots, ([0, 0], [0, 0], side));
                plan.passes.insert(0, Pass::Restore(restore));
                under.slots
            }
            None => {
                let format = core.raster_pipelines()?.layer;
                drawing.snapshot_slots(core, (&base.restored.targets(), format))?
            }
        };
        plan.passes
            .push(Pass::Keep(copies(&slots, ([0, 0], [0, 0], side))));
        drawing.set_under(tile, (slots, base.at));
        core.tally.snapshot();
        Ok(Part {
            plan,
            output,
            paper,
            finish: None,
        })
    }
}

/// The regions `changed` (canvas pixels) makes of an output of `size` pixels drawn onto `onto`,
/// at most `capacity` pixels each, and the tiles passed over (not drawn now).
fn regions(
    changed: &[PixelRect],
    (onto, size): (&Onto<'_>, [u32; 2]),
    capacity: [u32; 2],
) -> Result<(Vec<PixelRect>, Vec<[u32; 2]>), RenderError> {
    let output = Output::at(onto.scale, [0, 0])?;
    let mut parts: Vec<([u32; 2], PixelRect)> = Vec::new();
    for (tile, part) in changed
        .iter()
        .flat_map(|rect| meeting(output.place(rect), size))
    {
        if part.is_empty() {
            continue;
        }
        match parts.iter_mut().find(|(held, _)| *held == tile) {
            Some((_, held)) => *held = held.union(&part),
            None => parts.push((tile, part)),
        }
    }
    let mut skipped = Vec::new();
    parts.retain(|(tile, _)| {
        let shown = (onto.place)(*tile).is_some();
        if !shown {
            skipped.push(*tile);
        }
        shown
    });
    if !onto.joined {
        return Ok((parts.into_iter().map(|(_, part)| part).collect(), skipped));
    }
    let Some(all) = parts
        .iter()
        .map(|(_, part)| *part)
        .reduce(|a, b| a.union(&b))
    else {
        return Ok((Vec::new(), skipped));
    };
    // The box of every part, cut into equal pieces the targets hold, each the box of what it holds.
    let side = all.size();
    let count = [0, 1].map(|axis| side[axis].div_ceil(capacity[axis]).max(1));
    let mut regions = Vec::new();
    for row in 0..count[1] {
        for column in 0..count[0] {
            let at = [column, row];
            let edge = |axis: usize, k: u32| all.min[axis] + (side[axis] * k / count[axis]) as i32;
            let piece = PixelRect {
                min: [0, 1].map(|axis| edge(axis, at[axis])),
                max: [0, 1].map(|axis| edge(axis, at[axis] + 1)),
            };
            let held = (parts.iter())
                .map(|(_, part)| part.intersect(&piece))
                .filter(|part| !part.is_empty())
                .reduce(|a, b| a.union(&b));
            regions.extend(held);
        }
    }
    Ok((regions, skipped))
}

/// The tiles whose snapshot a frame brings up to the drawing's place (taken anew, or the strokes
/// committed since laid onto it), and where each of `regions` (of an output of `size` pixels)
/// resumes: every tile a region meets stands before one unit, the one it resumes at, so those
/// lacking strokes another lacks too are left as they are (the region lays the strokes since),
/// and the rest are brought up; from the paper without a place.
fn settle(
    drawing: &Drawing,
    regions: &[PixelRect],
    size: [u32; 2],
) -> (Vec<[u32; 2]>, Vec<Resume>) {
    let Some(base) = &drawing.base else {
        let cold = Resume {
            at: 0,
            restored: None,
        };
        return (Vec::new(), vec![cold; regions.len()]);
    };
    let tiles: Vec<Vec<[u32; 2]>> = (regions.iter())
        .map(|region| meeting(*region, size).map(|(tile, _)| tile).collect())
        .collect();
    let mut needed: BTreeSet<[u32; 2]> = (tiles.iter().flatten())
        .filter(|tile| !drawing.under.contains_key(*tile))
        .copied()
        .collect();
    let at = |needed: &BTreeSet<[u32; 2]>, tile: &[u32; 2]| match drawing.under.get(tile) {
        Some(under) if !needed.contains(tile) => under.at,
        _ => base.at,
    };
    loop {
        let mut more: Vec<[u32; 2]> = Vec::new();
        for met in &tiles {
            let first = met.first().map(|tile| at(&needed, tile));
            if met.iter().any(|tile| Some(at(&needed, tile)) != first) {
                more.extend(met.iter().filter(|tile| at(&needed, tile) != base.at));
            }
        }
        if more.is_empty() {
            break;
        }
        needed.extend(more);
    }
    let resumes = (tiles.iter())
        .map(|met| Resume {
            at: met.first().map_or(base.at, |tile| at(&needed, tile)),
            restored: Some(base.restored),
        })
        .collect();
    (needed.into_iter().collect(), resumes)
}

/// The restores of output pixels `region` (of an output of `size` pixels) from the snapshots of
/// the tiles it meets.
fn restores(drawing: &Drawing, region: PixelRect, size: [u32; 2]) -> Vec<SlotCopy> {
    let mut all = Vec::new();
    for (tile, part) in meeting(region, size) {
        let Some(under) = drawing.under.get(&tile) else {
            continue;
        };
        let corner = tile.map(|at| (at * TIER_TILE) as i32);
        let offset = [0, 1].map(|axis| (part.min[axis] - corner[axis]) as u32);
        let at = [0, 1].map(|axis| (part.min[axis] - region.min[axis]) as u32);
        let side = part.size();
        all.extend(copies(&under.slots, (offset, at, side)));
    }
    all
}
