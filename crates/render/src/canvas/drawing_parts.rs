//! The regions a frame of a stroke's drawing draws (`drawing_frame.rs`): what the frame
//! changes, as regions as large as the frame loop's targets, one pass chain each, wherever the
//! output's tiles lie (the canvas's side by side, a view tier's each in a slot of its own: a
//! region finishes into each tile it meets, at that tile's place); a region per tile only where
//! the box would meet a tile not drawn now. Each region is drawn from the snapshots of the tiles
//! it meets (taken now when one has none, or laying the strokes committed since the one it has),
//! the stroke laid at its unit from its kept coverage and its tail, then the units above it. One
//! chain whatever the tiles: a frame crossing a tier tile's border cost a chain a tile (S132).

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
use crate::raster::{Finish, Placed};
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
        // A tile's targets when every region fits them (a snapshot part is a tile).
        let small =
            (regions.iter()).all(|region| region.size().iter().all(|side| *side <= TIER_TILE));
        let mut work = StrokeWork {
            skipped,
            small,
            ..StrokeWork::default()
        };
        if regions.is_empty() {
            return Ok(work);
        }
        let atlas = self.stroke_atlas(core, small)?;
        let (needed, resumes) = settle(drawing, &regions, size);
        let base = drawing.base.as_ref().map_or(0, |base| base.at);
        work.snapshots = needed.len();
        work.laid_again = (resumes.iter())
            .filter(|resume| resume.restored.is_some())
            .map(|resume| base.saturating_sub(resume.at))
            .sum();
        // The residuals the regions read, and the snapshots brought up (whole tiles).
        let near: Vec<PixelRect> = (regions.iter().copied())
            .chain(needed.iter().map(|tile| tile_rect(*tile, size)))
            .map(|rect| around_output(&rect, scale))
            .collect();
        self.reach(core, request.artwork, &near)?;
        let paper = paper(request);
        for tile in needed {
            let at = (tile, size, atlas);
            let part = self.snapshot(core, drawing, at, (scene, scale, paper))?;
            batch.push(part);
        }
        for (region, resume) in regions.into_iter().zip(resumes) {
            let place = (onto, scene, paper);
            let at = (region, size, atlas);
            let part = self.region(drawing, at, place, (resume, pen))?;
            batch.push(part);
            work.parts.push(region);
            work.drawn
                .extend(meeting(region, size).map(|(tile, _)| tile));
        }
        work.drawn.sort_unstable();
        work.drawn.dedup();
        Ok(work)
    }

    /// Output pixels `region` of an output of `size` pixels drawn onto `onto` from `resume`, with
    /// an atlas of `atlas` pixels: its targets set from the snapshots of the tiles it meets when
    /// restored, the stroke (`pen`) from its kept coverage there.
    fn region<'a>(
        &self,
        drawing: &Drawing,
        (region, size, atlas): (PixelRect, [u32; 2], [u32; 2]),
        (onto, scene, paper): (&Onto<'a>, &Scene, [f32; 4]),
        (resume, pen): (Resume, Option<&Pen>),
    ) -> Result<Part<'a>, RenderError> {
        let held = self.held().ok_or(NO_SCENE)?;
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
        Ok(Part {
            plan,
            output,
            paper,
            finish: Some(finish(onto, region, size)?),
        })
    }

    /// Tile `tile`'s snapshot brought up to the drawing's place, with an atlas of `atlas` pixels:
    /// taken from the paper when it has none, else the strokes committed since it was taken laid
    /// onto it, in place.
    fn snapshot<'a>(
        &mut self,
        core: &Core,
        drawing: &mut Drawing,
        (tile, size, atlas): ([u32; 2], [u32; 2], [u32; 2]),
        (scene, scale, paper): (&Scene, u32, [f32; 4]),
    ) -> Result<Part<'a>, RenderError> {
        let base = drawing.base.clone().ok_or(NO_SCENE)?;
        let rect = tile_rect(tile, size);
        let output = Output::at(scale, rect.min.map(i32::unsigned_abs))?;
        let side = rect.size();
        let held = self.held().ok_or(NO_SCENE)?;
        let units = (scene, &held.bases);
        let under = drawing.under.get(&tile).cloned();
        let from = under.as_ref().map(|under| *under.stands.end());
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

/// Where output pixels `region` (of an output of `size` pixels) go in `onto`: each tile's part
/// at its tile's place, as one place where the tiles lie side by side there (the canvas).
fn finish<'a>(
    onto: &Onto<'a>,
    region: PixelRect,
    size: [u32; 2],
) -> Result<Finish<'a>, RenderError> {
    let mut into = Vec::new();
    for (tile, part) in meeting(region, size) {
        let outside = RenderError::Invalid("a stroke's region meets a tile not drawn now");
        let place = (onto.place)(tile).ok_or(outside)?;
        // Where the region's top-left lands when this tile lands at its place.
        let shift = [0, 1].map(|axis| place[axis] as i32 - (tile[axis] * TIER_TILE) as i32);
        let at = [0, 1].map(|axis| region.min[axis] + shift[axis]);
        let only = PixelRect {
            min: [0, 1].map(|axis| part.min[axis] + shift[axis]),
            max: [0, 1].map(|axis| part.max[axis] + shift[axis]),
        };
        into.push(Placed {
            at,
            only: vec![only],
        });
    }
    if into.windows(2).all(|pair| pair[0].at == pair[1].at) {
        into.truncate(1);
        if let Some(whole) = into.first_mut() {
            whole.only.clear();
        }
    }
    Ok(Finish {
        view: onto.view,
        format: onto.format,
        into,
        texture: Some(onto.texture),
    })
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
    let Some(all) = parts
        .iter()
        .map(|(_, part)| *part)
        .reduce(|a, b| a.union(&b))
    else {
        return Ok((Vec::new(), skipped));
    };
    // The box of every part, cut into equal pieces the targets hold, each the box of what it
    // holds; where that box meets a tile not drawn now, its parts a region each.
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
            let held: Vec<PixelRect> = (parts.iter())
                .map(|(_, part)| part.intersect(&piece))
                .filter(|part| !part.is_empty())
                .collect();
            let Some(joined) = held.iter().copied().reduce(|a, b| a.union(&b)) else {
                continue;
            };
            if meeting(joined, size).all(|(tile, _)| (onto.place)(tile).is_some()) {
                regions.push(joined);
            } else {
                regions.extend(held);
            }
        }
    }
    Ok((regions, skipped))
}

/// The tiles whose snapshot a frame brings up to the drawing's place (taken anew, or the strokes
/// committed since laid onto it), and where each of `regions` (of an output of `size` pixels)
/// resumes: at a unit every tile it meets stands before, the latest (the region lays the strokes
/// since), when they have one; else the tiles lacking strokes are brought up and it is the
/// place's; from the paper without a place.
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
    // The units a tile's snapshot stands before; the place's alone once brought up.
    let stands = |needed: &BTreeSet<[u32; 2]>, tile: &[u32; 2]| match drawing.under.get(tile) {
        Some(under) if !needed.contains(tile) => (*under.stands.start(), *under.stands.end()),
        _ => (base.at, base.at),
    };
    loop {
        let mut more: Vec<[u32; 2]> = Vec::new();
        for met in &tiles {
            if agreed(met.iter().map(|tile| stands(&needed, tile))).is_none() {
                more.extend(met.iter().filter(|tile| stands(&needed, tile).1 != base.at));
            }
        }
        if more.is_empty() {
            break;
        }
        needed.extend(more);
    }
    let resumes = (tiles.iter())
        .map(|met| Resume {
            at: agreed(met.iter().map(|tile| stands(&needed, tile))).unwrap_or(base.at),
            restored: Some(base.restored),
        })
        .collect();
    (needed.into_iter().collect(), resumes)
}

/// The latest unit every one of the snapshots standing before units `stands` (first, last)
/// stands before: the least last, when no first is later; `None` when they share none or are
/// none.
pub(super) fn agreed(stands: impl IntoIterator<Item = (usize, usize)>) -> Option<usize> {
    let (first, last) = (stands.into_iter()).reduce(|(a, b), (c, d)| (a.max(c), b.min(d)))?;
    (first <= last).then_some(last)
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

#[cfg(test)]
mod tests {
    use openpaint_core::{Artwork, CanvasSize};

    use super::{PixelRect, agreed, settle};
    use crate::canvas::drawing::{Base, Drawing, Under};
    use crate::raster_plan::Restored;

    #[test]
    fn a_region_resumes_where_its_tiles_stand_together_else_brings_up_the_lagging() {
        assert_eq!(agreed([(2, 5), (4, 7)]), Some(5), "the latest shared");
        assert_eq!(agreed([(1, 1), (3, 7)]), None, "none shared");
        assert_eq!(agreed([]), None);
        let artwork = Artwork::new(CanvasSize::new(8 * 256, 256).expect("size"));
        let restored = Restored {
            on_mask: false,
            chain: false,
            group: false,
        };
        let mut drawing = Drawing::default();
        let place = (artwork.active(), false);
        let (at, epoch) = (7, 0);
        drawing.base = Some(Base {
            artwork,
            place,
            at,
            restored,
            epoch,
        });
        // Tile n stands before these units; tile 4 has no snapshot.
        let stands = [(2, 5), (4, 7), (1, 1), (3, 7), (0, 0), (6, 7), (2, 3)];
        for (column, (first, last)) in (0u32..).zip(stands).filter(|(column, _)| *column != 4) {
            let slots = Vec::new();
            let under = Under {
                slots,
                stands: first..=last,
                made: 0,
            };
            drawing.under.insert([column, 0], under);
        }
        // A region over the tiles `a` to `b` (columns).
        let across = |(a, b): (i32, i32)| PixelRect {
            min: [a * 256 + 10, 10],
            max: [b * 256 + 60, 20],
        };
        let regions = [(0, 1), (2, 3), (3, 3), (4, 5), (6, 6)].map(across);
        let (needed, resumes) = settle(&drawing, &regions, [8 * 256, 256]);
        let at: Vec<usize> = resumes.iter().map(|resume| resume.at).collect();
        // Tiles 0 and 1 meet at 5; tile 2 shares no unit with tile 3: brought up; tile 3 alone
        // stands before the place; tile 4 is taken; tile 6 keeps its lag.
        assert_eq!(at, [5, 7, 7, 7, 3]);
        assert_eq!(needed, [[2, 0], [4, 0]]);
    }
}
