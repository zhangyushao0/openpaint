//! A raster plan with the stroke being drawn (`raster_plan.rs`). The stroke
//! lays at its unit (`Unit::Active`) from what its drawing kept: its coverage so far in a slot per
//! output tile, to which the spans settled since and its tail (the spans later samples may still
//! change, never kept) are added in the atlas, so its region of the atlas holds what the stroke's
//! spans laid in one go would (`raster_plan_kept.rs`). A plan may start from snapshots of the
//! units before a unit (`Resume`): their passes are not drawn, the targets hold what those units
//! left over the whole region (nothing where they drew nothing), so the plan goes on as one drawn
//! from the paper whose units drew everywhere in the region: a layer ending with nothing where it
//! drew nothing, as a part of a region draws what the whole does.

use super::{Builder, Chain, Drawn, RasterPlan};
use crate::error::RenderError;
use crate::geom::{Bounds, PixelRect};
use crate::output::Output;
use crate::protocol::{LayerInto, Op};
use crate::raster_kinds::{Coverage, Pass, Scratch, SlotAt};
use crate::raster_scene::Unit;
use crate::scene::{Item, Scene};

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

/// A stroke being drawn: its paint op, the spans settled since its last frame and its tail's
/// (each its op's place in the op buffer, the op, its bounds in canvas pixels), how its spans
/// lay, and its kept coverage in the output tiles the region meets.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StrokeDrawn {
    pub(crate) paint: (u32, Op),
    pub(crate) settled: Vec<(u32, Op, Bounds)>,
    pub(crate) tail: Vec<(u32, Op, Bounds)>,
    pub(crate) coverage: Coverage,
    pub(crate) contact: bool,
    pub(crate) kept: Vec<KeptTile>,
}

/// A stroke's kept coverage in one output tile: its slots (coverage or density, and contact), the
/// tile's top-left output pixel, which of its 32-px cells (8 a row) the slots hold the kept spans
/// in, and which the spans settled since reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeptTile {
    pub(crate) slots: [SlotAt; 2],
    pub(crate) origin: [u32; 2],
    pub(crate) held: u64,
    pub(crate) laid: u64,
}

/// What a snapshot's targets hold besides the canvas and the layer: the mask (the stroke's place
/// is a layer's mask), the clipping chain (its layer clips onto a base), what lay under its group
/// (a layer of its group ended onto the canvas before it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Restored {
    pub(crate) on_mask: bool,
    pub(crate) chain: bool,
    pub(crate) group: bool,
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
        if self.group {
            targets.push(Scratch::Group);
        }
        targets
    }

    /// Where a stroke lays in `scene` (its unit `at`, the stroke's place's), on the mask or not.
    pub(crate) fn at(scene: &Scene, (at, on_mask): (usize, bool)) -> Self {
        let raster = &scene.raster;
        let end = (raster.units.iter().skip(at)).find_map(|unit| match unit {
            Unit::End(item) => Some(*item),
            _ => None,
        });
        let chain = end.is_some_and(|item| {
            matches!(scene.items[item], Item::EndLayer { .. })
                && scene.items[item].op().layer_end().into == LayerInto::Clipped
        });
        let group = raster.group_open_at(scene, at);
        Self {
            on_mask,
            chain,
            group,
        }
    }
}

/// Where a plan starts: from the paper, or with the targets as snapshots hold them after the units
/// before `at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Resume {
    pub(crate) at: usize,
    pub(crate) restored: Option<Restored>,
}

impl RasterPlan {
    /// The plan of a region of `size` pixels at `output` with the stroke being drawn `drawing`
    /// laid at its unit; from snapshots of the units before `resume.at` when restored (the caller
    /// sets the targets from them first).
    ///
    /// # Errors
    /// As `build`.
    pub(crate) fn drawing(
        (scene, bases): Drawn<'_>,
        (output, size): (Output, [u32; 2]),
        atlas: [u32; 2],
        (resume, drawing): (Resume, Option<&InProgress>),
    ) -> Result<Self, RenderError> {
        let raster = &scene.raster;
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
        (scene, bases): Drawn<'_>,
        (output, size): (Output, [u32; 2]),
        atlas: [u32; 2],
        (from, to, restored): (Option<usize>, usize, Restored),
    ) -> Result<Self, RenderError> {
        let raster = &scene.raster;
        let mut builder = Builder::new((scene, bases), (output, size), atlas, Scratch::Layer);
        let to = to.min(raster.units.len());
        let from = match from {
            Some(from) => {
                builder.restored(restored);
                from.min(to)
            }
            None => {
                builder.plan.passes.push(Pass::Clear(Scratch::Canvas));
                builder.cleared[Scratch::Canvas.index()] = true;
                0
            }
        };
        for unit in &raster.units[from..to] {
            builder.unit(unit)?;
        }
        builder.flush();
        // What a target holds where its units drew nothing is no unit's: nothing.
        let unset = [
            (Scratch::Layer, !builder.cleared[Scratch::Layer.index()]),
            (
                Scratch::Mask,
                restored.on_mask && !builder.cleared[Scratch::Mask.index()],
            ),
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
        self.on_mask = restored.on_mask;
        for target in [Scratch::Canvas, Scratch::Layer] {
            (self.cleared[target.index()], self.touched[target.index()]) = (true, true);
        }
        let mask = Scratch::Mask.index();
        (self.cleared[mask], self.touched[mask]) = (restored.on_mask, restored.on_mask);
        self.chain = if restored.chain {
            Chain::Held
        } else {
            Chain::Empty
        };
        // Its end then runs: where none of its layers reached, it leaves the canvas as it is.
        self.group = restored.group.then_some(true);
    }

    /// The stroke being drawn, where its unit is.
    pub(super) fn active(&mut self) -> Result<(), RenderError> {
        match self.drawing {
            Some(InProgress::Stroke(stroke)) => self.drawn_stroke(stroke),
            Some(InProgress::Smudge { op, reach }) => {
                self.smudge_drawn(*op as usize, reach);
                Ok(())
            }
            None => Ok(()),
        }
    }
}
