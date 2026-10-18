//! A raster plan with the stroke being drawn (S63, DECISIONS D90; `raster_plan.rs`). The stroke
//! lays at its unit (`Unit::Active`) from what its drawing kept: its coverage so far in a slot per
//! output tile, to which its tail (the spans later samples may still change) is added in the atlas,
//! never kept, so its region of the atlas holds what the stroke's spans laid in one go would. A plan
//! may start from snapshots of the units before a unit (`Resume`): their passes are not drawn, the
//! targets hold what those units left over the whole region (nothing where they drew nothing), so
//! the plan goes on as one drawn from the paper whose units drew everywhere in the region: a layer
//! ending with nothing where it drew nothing, as a part of a region draws what the whole does.

use super::{Builder, Chain, Drawn, RasterPlan};
use crate::error::RenderError;
use crate::geom::{Bounds, PixelRect};
use crate::output::Output;
use crate::protocol::{LayerInto, Op};
use crate::raster_kinds::{
    AtlasCopy, Coverage, CoverageDraw, Draw, DrawKind, Instance, Pass, Scratch, SlotAt,
};
use crate::raster_lay::{Packer, laid_span};
use crate::raster_route::{RasterScene, Unit};
use crate::scene::{Item, Scene};
use crate::view_tiles::TIER_TILE;

/// The stroke being drawn as one region's plan lays it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InProgress {
    Stroke(StrokeDrawn),
    /// A smudge: its op (whose residual is the in-progress one) and the canvas pixels it can
    /// change.
    Smudge {
        op: u32,
        reach: PixelRect,
    },
}

/// A stroke being drawn: its paint op and its tail's spans (each its op's place in the op buffer,
/// the op, its bounds in canvas pixels), how its spans lay, and its kept coverage in the output
/// tiles the region meets.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StrokeDrawn {
    pub(crate) paint: (u32, Op),
    pub(crate) tail: Vec<(u32, Op, Bounds)>,
    pub(crate) coverage: Coverage,
    pub(crate) contact: bool,
    pub(crate) kept: Vec<KeptTile>,
}

/// A stroke's kept coverage in one output tile: its slots (coverage or density, and contact), the
/// tile's top-left output pixel, and which of its 32-px cells (8 a row) the kept spans reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeptTile {
    pub(crate) slots: [SlotAt; 2],
    pub(crate) origin: [u32; 2],
    pub(crate) cells: u64,
}

/// What a snapshot's targets hold besides the canvas and the layer: the mask (the stroke's place
/// is a layer's mask), the clipping chain (its layer clips onto a base).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Restored {
    pub(crate) on_mask: bool,
    pub(crate) chain: bool,
}

impl Restored {
    /// The float targets a snapshot holds.
    pub(crate) fn targets(self) -> Vec<Scratch> {
        let mut targets = vec![Scratch::Canvas, Scratch::Layer];
        if self.on_mask {
            targets.push(Scratch::Mask);
        }
        if self.chain {
            targets.push(Scratch::Chain);
        }
        targets
    }

    /// Where a stroke lays in `raster` (its unit `at`, the stroke's place's), on the mask or not.
    pub(crate) fn at(scene: &Scene, raster: &RasterScene, (at, on_mask): (usize, bool)) -> Self {
        let end = (raster.units.iter().skip(at)).find_map(|unit| match unit {
            Unit::End(item) => Some(*item),
            _ => None,
        });
        let chain = end.is_some_and(|item| {
            matches!(scene.items[item], Item::EndLayer { .. })
                && scene.items[item].op().layer_end().into == LayerInto::Clipped
        });
        Self { on_mask, chain }
    }
}

/// Where a plan starts: from the paper, or with the targets as snapshots hold them after the units
/// before `at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Resume {
    pub(crate) at: usize,
    pub(crate) restored: Option<Restored>,
}

/// A composite cell's side in output pixels (`raster_lay::TILE`).
const CELL: i32 = 32;

/// Where the stroke being drawn reaches a region: its pixels, its composite tiles, its tail's
/// spans reaching them (each its op's place and the op).
type Reached = (PixelRect, Vec<[u32; 2]>, Vec<(u32, Op)>);

impl RasterPlan {
    /// The plan of a region of `size` pixels at `output` with the stroke being drawn `drawing`
    /// laid at its unit; from snapshots of the units before `resume.at` when restored (the caller
    /// sets the targets from them first).
    ///
    /// # Errors
    /// As `build`.
    pub(crate) fn drawing(
        (scene, raster, bases): Drawn<'_>,
        (output, size): (Output, [u32; 2]),
        atlas: [u32; 2],
        (resume, drawing): (Resume, Option<&InProgress>),
    ) -> Result<Self, RenderError> {
        let mut builder = Builder::new((scene, bases), (output, size), atlas, Scratch::Layer);
        builder.drawing = drawing;
        let Some(restored) = resume.restored else {
            return builder.lay(&raster.units);
        };
        builder.restored(restored);
        for unit in raster.units.iter().skip(resume.at) {
            builder.unit(unit)?;
        }
        Ok(builder.done())
    }

    /// The plan a snapshot of the units before `to` over a region takes, `restored` naming its
    /// targets: from the paper, or (`from`) from the snapshot of the units before an earlier unit,
    /// laying those since; the layer, the mask and the chain cleared where they drew nothing.
    /// The caller restores the earlier snapshot first and keeps the targets after.
    ///
    /// # Errors
    /// As `build`.
    pub(crate) fn snapshot(
        (scene, raster, bases): Drawn<'_>,
        (output, size): (Output, [u32; 2]),
        atlas: [u32; 2],
        (from, to, restored): (Option<usize>, usize, Restored),
    ) -> Result<Self, RenderError> {
        let mut builder = Builder::new((scene, bases), (output, size), atlas, Scratch::Layer);
        let to = to.min(raster.units.len());
        let from = match from {
            Some(from) => {
                builder.restored(restored);
                from.min(to)
            }
            None => {
                builder.plan.passes.push(Pass::Clear(Scratch::Canvas));
                0
            }
        };
        for unit in &raster.units[from..to] {
            builder.unit(unit)?;
        }
        builder.flush();
        // What a target holds where its units drew nothing is no unit's: nothing.
        let unset = [
            (Scratch::Layer, !builder.cleared[0]),
            (Scratch::Mask, restored.on_mask && !builder.cleared[1]),
            (
                Scratch::Chain,
                restored.chain && builder.chain != Chain::Held,
            ),
        ];
        for (target, unset) in unset {
            if unset {
                builder.plan.passes.push(Pass::Clear(target));
            }
        }
        Ok(builder.done())
    }
}

impl Builder<'_> {
    /// The targets as snapshots set them: every one holding what the units before drew over the
    /// whole region, draws going to the mask when the stroke's place is one, the chain held.
    fn restored(&mut self, restored: Restored) {
        self.target = if restored.on_mask {
            Scratch::Mask
        } else {
            Scratch::Layer
        };
        self.cleared = [true, restored.on_mask];
        self.touched = [true, restored.on_mask];
        self.chain = if restored.chain {
            Chain::Held
        } else {
            Chain::Empty
        };
    }

    /// The stroke being drawn, where its unit is.
    pub(super) fn active(&mut self) -> Result<(), RenderError> {
        match self.drawing {
            Some(InProgress::Stroke(stroke)) => self.drawn_stroke(stroke),
            Some(InProgress::Smudge { op, reach }) => {
                self.smudge_from((*op as usize, 0), reach);
                Ok(())
            }
            None => Ok(()),
        }
    }

    /// The stroke being drawn: its kept coverage copied into its atlas regions, its tail laid
    /// there, its tiles composited onto the layer (or mask), each a batch of its own.
    fn drawn_stroke(&mut self, stroke: &StrokeDrawn) -> Result<(), RenderError> {
        self.flush();
        let Some((rect, tiles, reaching)) = self.drawn_reach(stroke) else {
            return Ok(());
        };
        let side = [0, 1].map(|axis| (rect.max[axis] - rect.min[axis]) as u32);
        self.packer = Packer::new(self.atlas);
        let kinds: &[Coverage] = if stroke.contact {
            &[Coverage::Density, Coverage::Contact]
        } else {
            std::slice::from_ref(&stroke.coverage)
        };
        let at = (self.packer.places(side, kinds.len())).ok_or(RenderError::Invalid(
            "a stroke's region is larger than the raster core's atlas",
        ))?;
        let mut copies = Vec::new();
        let mut draws = Vec::new();
        let scale = self.output.scale as f32;
        for (kind, (coverage, at)) in kinds.iter().zip(&at).enumerate() {
            copies.extend(self.kept_copies(stroke, (kind, *at), rect));
            let place = [0, 1].map(|axis| at[axis] as i32 - rect.min[axis]);
            let start = self.plan.instances.len() as u32;
            for &(index, op) in &reaching {
                laid_span(&mut self.plan, (op, index as usize), place, scale);
            }
            draws.push(CoverageDraw {
                coverage: *coverage,
                scissor: [at[0], at[1], side[0], side[1]],
                instances: start..self.plan.instances.len() as u32,
            });
            self.plan.counts.region_pixels += u64::from(side[0]) * u64::from(side[1]);
        }
        self.plan.passes.push(Pass::Kept { copies, draws });
        self.plan.counts.batches += 1;
        self.drawn_tiles(stroke, (&at, rect), &tiles);
        Ok(())
    }

    /// The kept coverage of kind `kind` (its slot) of each tile meeting the region's pixels
    /// `rect`, copied to where they lie in the atlas region at `at` holding `rect`.
    fn kept_copies(
        &self,
        stroke: &StrokeDrawn,
        (kind, at): (usize, [u32; 2]),
        rect: PixelRect,
    ) -> Vec<AtlasCopy> {
        let origin = self.output.origin.map(|at| at as i32);
        let side = TIER_TILE as i32;
        let mut copies = Vec::new();
        for kept in &stroke.kept {
            let min = [0, 1].map(|axis| kept.origin[axis] as i32 - origin[axis]);
            let tile = PixelRect {
                min,
                max: [min[0] + side, min[1] + side],
            };
            let part = tile.intersect(&rect);
            if part.is_empty() {
                continue;
            }
            copies.push(AtlasCopy {
                slot: kept.slots[kind],
                offset: [0, 1].map(|axis| (part.min[axis] - min[axis]) as u32),
                size: [0, 1].map(|axis| (part.max[axis] - part.min[axis]) as u32),
                to: [0, 1].map(|axis| at[axis] + (part.min[axis] - rect.min[axis]) as u32),
            });
        }
        copies
    }

    /// The region's pixels the stroke being drawn reaches (its kept cells' and its tail's, in
    /// whole composite tiles), those tiles, and its tail's spans reaching them.
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
            for cell in (0..64).filter(|cell| kept.cells & (1 << cell) != 0) {
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
        let mut reaching = Vec::new();
        for &(index, op, bounds) in &stroke.tail {
            let reached = self.output.place(&bounds.pixels()).intersect(&region);
            if !bounds.is_empty() && !reached.is_empty() {
                reaching.push((index, op));
                mark(self, &reached);
            }
        }
        Some((rect?.intersect(&region), tiles, reaching))
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
        self.plan.counts.strokes += 1;
        self.flush();
        let slot = self.slot();
        self.touched[slot] = true;
    }
}
