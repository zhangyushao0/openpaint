//! The stroke being drawn in one region's plan (`raster_plan/`): one
//! pass of the atlas, cleared first. Per kind of coverage (a pencil's density and contact), the
//! stroke's region of the atlas (its box in whole composite tiles) is drawn from its kept
//! coverage, read from the slots of the output tiles it meets where they hold kept spans, then the
//! spans settled since its last frame and its tail are laid over it, so it holds what the stroke's
//! spans laid in one go would; where spans settled, a second region is drawn the same way without
//! the tail and copied back into the slots after the pass, so the kept coverage gains exactly the
//! settled spans, in order. Then its tiles are composited onto the layer (or its mask).

use super::Builder;
use super::drawing::StrokeDrawn;
use crate::error::RenderError;
use crate::geom::{Bounds, PixelRect};
use crate::protocol::Op;
use crate::raster_kinds::{AtlasCopy, Coverage, CoverageDraw, Draw, DrawKind, Instance, Pass};
use crate::raster_lay::{Packer, laid_span};
use crate::view_tiles::TIER_TILE;

/// A composite cell's side in output pixels (`raster_lay::TILE`).
const CELL: i32 = 32;

/// Where the stroke being drawn reaches a region: its pixels, its composite tiles, and the spans
/// settled since and of its tail reaching them (each its op's place and the op).
struct Reached {
    rect: PixelRect,
    tiles: Vec<[u32; 2]>,
    settled: Vec<(u32, Op)>,
    tail: Vec<(u32, Op)>,
}

impl Builder<'_> {
    /// The stroke being drawn: its regions of the atlas in one pass, its tiles composited onto
    /// the layer (or mask) from the first region of each kind.
    pub(super) fn drawn_stroke(&mut self, stroke: &StrokeDrawn) -> Result<(), RenderError> {
        self.flush();
        let Some(reached) = self.drawn_reach(stroke) else {
            return Ok(());
        };
        let rect = reached.rect;
        let side = rect.size();
        self.packer = Packer::new(self.atlas);
        let kinds: &[Coverage] = if stroke.contact {
            &[Coverage::Density, Coverage::Contact]
        } else {
            std::slice::from_ref(&stroke.coverage)
        };
        let keeping = !reached.settled.is_empty();
        let count = kinds.len() * (1 + usize::from(keeping));
        let at = (self.packer.places(side, count)).ok_or(RenderError::Invalid(
            "a stroke's region is larger than the raster core's atlas",
        ))?;
        let (shown, back) = at.split_at(kinds.len());
        let origin = self.output.origin.map(|at| at as i32);
        let (mut held, mut draws, mut keep) = (Vec::new(), Vec::new(), Vec::new());
        for (kind, coverage) in kinds.iter().enumerate() {
            let mut regions = vec![(shown[kind], &reached.tail[..])];
            if keeping {
                regions.push((back[kind], &[]));
                keep.extend(kept_back(stroke, (kind, back[kind]), (rect, origin)));
            }
            for (at, tail) in regions {
                held.extend(self.held_quads(stroke, (kind, at), rect));
                let place = [0, 1].map(|axis| at[axis] as i32 - rect.min[axis]);
                let start = self.plan.instances.len() as u32;
                // Its place lies on the canvas, in no frame.
                let scale = (self.output.scale as f32, 0);
                for &(index, op) in reached.settled.iter().chain(tail) {
                    laid_span(&mut self.plan, (op, index as usize), place, scale);
                }
                draws.push(CoverageDraw {
                    coverage: *coverage,
                    scissor: [at[0], at[1], side[0], side[1]],
                    instances: start..self.plan.instances.len() as u32,
                });
                self.plan.counts.region_pixels += u64::from(side[0]) * u64::from(side[1]);
            }
        }
        let held = self.grouped(held);
        self.plan.passes.push(Pass::Kept { held, draws, keep });
        self.drawn_tiles(stroke, (shown, rect), &reached.tiles);
        Ok(())
    }

    /// The kept coverage of kind `kind` of each tile meeting the region's pixels `rect`, read
    /// from its slot into the atlas region at `at` holding `rect`: an instance per tile holding
    /// kept spans there, with its slot's page.
    fn held_quads(
        &self,
        stroke: &StrokeDrawn,
        (kind, at): (usize, [u32; 2]),
        rect: PixelRect,
    ) -> Vec<(usize, Instance)> {
        let origin = self.output.origin.map(|at| at as i32);
        let mut quads = Vec::new();
        for kept in stroke.kept.iter().filter(|kept| kept.held != 0) {
            let (tile, part) = tile_part(kept.origin, origin, rect);
            if part.is_empty() {
                continue;
            }
            let slot = kept.slots[kind];
            // An atlas pixel's page pixel, less the atlas pixel.
            let shift = [0, 1].map(|axis| {
                slot.at[axis] as i32 - tile.min[axis] - at[axis] as i32 + rect.min[axis]
            });
            let corner =
                |p: [i32; 2]| [0, 1].map(|axis| at[axis] as i32 + p[axis] - rect.min[axis]);
            let [x0, y0] = corner(part.min);
            let [x1, y1] = corner(part.max);
            let cells = [kept.held as u32, (kept.held >> 32) as u32];
            let instance = Instance {
                a: [
                    cells[0],
                    cells[1],
                    shift[0].cast_unsigned(),
                    shift[1].cast_unsigned(),
                ],
                b: [x0, y0, x1, y1],
            };
            quads.push((slot.page, instance));
        }
        quads
    }

    /// `quads` added to the plan's instances page by page: each page's instances.
    fn grouped(&mut self, mut quads: Vec<(usize, Instance)>) -> Vec<(usize, std::ops::Range<u32>)> {
        quads.sort_by_key(|(page, _)| *page);
        let mut held: Vec<(usize, std::ops::Range<u32>)> = Vec::new();
        for (page, instance) in quads {
            let at = self.plan.instances.len() as u32;
            self.plan.instances.push(instance);
            match held.last_mut() {
                Some((last, range)) if *last == page => range.end = at + 1,
                _ => held.push((page, at..at + 1)),
            }
        }
        held
    }

    /// The region's pixels the stroke being drawn reaches (the cells its kept and settled spans
    /// reach and its tail's, in whole composite tiles), those tiles, and its settled and tail
    /// spans reaching them.
    fn drawn_reach(&mut self, stroke: &StrokeDrawn) -> Option<Reached> {
        let region = PixelRect::full(self.plan.size);
        let origin = self.output.origin.map(|at| at as i32);
        self.marks.next();
        let mut tiles = Vec::new();
        let mut rect: Option<PixelRect> = None;
        let mut mark = |builder: &mut Self, reached: &PixelRect| {
            let whole = builder.marks.mark(reached, &mut tiles);
            rect = Some(rect.map_or(whole, |rect| rect.union(&whole)));
        };
        for kept in &stroke.kept {
            let cells = kept.held | kept.laid;
            for cell in (0..64).filter(|cell| cells & (1 << cell) != 0) {
                let [x, y] = [cell % 8, cell / 8].map(|at| at * CELL);
                let min = [
                    kept.origin[0] as i32 + x - origin[0],
                    kept.origin[1] as i32 + y - origin[1],
                ];
                let cell = PixelRect {
                    min,
                    max: [min[0] + CELL, min[1] + CELL],
                };
                let reached = cell.intersect(&region);
                if !reached.is_empty() {
                    mark(self, &reached);
                }
            }
        }
        // The settled spans lie in the cells their tiles hold now.
        let output = self.output;
        let reach = |bounds: &Bounds| output.place(&bounds.pixels()).intersect(&region);
        let settled = (stroke.settled.iter())
            .filter(|(_, _, bounds)| !bounds.is_empty() && !reach(bounds).is_empty())
            .map(|&(index, op, _)| (index, op))
            .collect();
        let mut tail = Vec::new();
        for &(index, op, bounds) in &stroke.tail {
            let reached = reach(&bounds);
            if !bounds.is_empty() && !reached.is_empty() {
                tail.push((index, op));
                mark(self, &reached);
            }
        }
        Some(Reached {
            rect: rect?.intersect(&region),
            tiles,
            settled,
            tail,
        })
    }

    /// The stroke's composite tiles onto the layer (or mask) from its atlas regions `at` holding
    /// the region's pixels `rect`.
    fn drawn_tiles(
        &mut self,
        stroke: &StrokeDrawn,
        (at, rect): (&[[u32; 2]], PixelRect),
        tiles: &[[u32; 2]],
    ) {
        let place = |at: &[u32; 2]| [0, 1].map(|axis| at[axis] as i32 - rect.min[axis]);
        let [x, y] = place(&at[0]);
        let contact = at.get(1).map_or([0, 0], place);
        let start = self.plan.instances.len() as u32;
        let (paint, op) = stroke.paint;
        for [tx, ty] in tiles {
            self.plan.instances.push(Instance {
                a: [paint, *tx, *ty, u32::from(stroke.contact)],
                b: [x, y, contact[0], contact[1]],
            });
        }
        self.plan.counts.tiles += tiles.len();
        self.batch.draws.push(Draw {
            kind: DrawKind::Stroke(op.paint_blend()),
            instances: start..self.plan.instances.len() as u32,
        });
        self.flush();
        self.touched[self.target().index()] = true;
    }
}

/// The copies of kind `kind` of the atlas region at `at` holding the region's pixels `rect` (its
/// top-left output pixel `origin`) into the slots of the tiles the settled spans reach there.
fn kept_back(
    stroke: &StrokeDrawn,
    (kind, at): (usize, [u32; 2]),
    (rect, origin): (PixelRect, [i32; 2]),
) -> Vec<AtlasCopy> {
    let mut copies = Vec::new();
    for kept in stroke.kept.iter().filter(|kept| kept.laid != 0) {
        let (tile, part) = tile_part(kept.origin, origin, rect);
        if part.is_empty() {
            continue;
        }
        copies.push(AtlasCopy {
            at: [0, 1].map(|axis| at[axis] + (part.min[axis] - rect.min[axis]) as u32),
            slot: kept.slots[kind],
            offset: [0, 1].map(|axis| (part.min[axis] - tile.min[axis]) as u32),
            size: part.size(),
        });
    }
    copies
}

/// The output tile at output pixel `corner` in region pixels (the region's top-left output pixel
/// `origin`), and its part inside `rect`.
fn tile_part(corner: [u32; 2], origin: [i32; 2], rect: PixelRect) -> (PixelRect, PixelRect) {
    let side = TIER_TILE as i32;
    let min = [0, 1].map(|axis| corner[axis] as i32 - origin[axis]);
    let tile = PixelRect {
        min,
        max: [min[0] + side, min[1] + side],
    };
    (tile, tile.intersect(&rect))
}
