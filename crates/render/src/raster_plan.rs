//! The raster core's plan on the CPU (S57's probe made product code, S59): one region of a scene's
//! output as the render passes that draw it. Each stroke reaching the region gets a region of the
//! scratch atlas (`raster_lay.rs`), its pieces and stamps lay their coverage there, combined by
//! the blend unit, and its tiles composite it onto the layer by the evaluator's paint; fills and
//! patches draw onto the layer as quads; a smudge copies what it reaches of the layer (or mask)
//! and lays it back moved by its residual (S61, D88); a mask draws onto a target of its own; each
//! layer's end composites it onto the canvas (or its clipping chain) by its mode, through its
//! mask, from a copy of what lies under it where fixed-function blending cannot (Overlay). S65
//! (D92): adjustments and filters rewrite the layer from a copy of it, a transform record's
//! content lies in its frame, a moved selection's or a merged layer's content draws on a level of
//! its own laid onto the one under it at its end, a group's end fades the canvas over a copy of
//! what lay under the group (`raster_plan_records.rs`, `raster_plan_ends.rs`). Strokes go into a
//! batch in scene order while their regions fit the atlas; a batch is a coverage pass and a pass
//! of the layer's draws; a smudge or a record ends one. Draws keep the scene's order, so a part of
//! a region draws what the whole does there. S63: the stroke being drawn lays from its kept
//! coverage; a plan may start from snapshots of the units before it (`raster_plan_drawing.rs`).

use std::ops::Range;

use crate::affine::Affine;
use crate::error::RenderError;
use crate::geom::Bounds;
use crate::output::Output;
use crate::protocol::raster::Edit;
pub use crate::raster_kinds::RasterCounts;
pub(crate) use crate::raster_kinds::{
    AtlasCopy, Coverage, CoverageDraw, Draw, DrawKind, EndBlend, Instance, Pass, SCRATCH, Scratch,
    SlotAt, SlotCopy,
};
use crate::raster_lay::{Laid, Marks, Packer, Placing, laid_span, lay};
use crate::raster_scene::Unit;
use crate::scene::Scene;
use crate::smudge::Bases;

#[derive(Debug, Default)]
pub(crate) struct RasterPlan {
    pub(crate) instances: Vec<Instance>,
    pub(crate) passes: Vec<Pass>,
    /// The region's size in pixels.
    pub(crate) size: [u32; 2],
    /// Where the region's pixels lie on the canvas beside its output (S65: a material seen from a
    /// transform record, its content's map onto the canvas; else the identity).
    pub(crate) base: Affine,
    pub(crate) counts: RasterCounts,
}

/// A chain's state while its layers are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chain {
    /// None open, or its base reached nothing here (nothing clips onto it).
    Empty,
    Held,
}

/// The strokes and edits of a batch not laid out yet.
#[derive(Default)]
struct Batch {
    coverage: Vec<CoverageDraw>,
    draws: Vec<Draw>,
}

struct Builder<'a> {
    scene: &'a Scene,
    /// Where each of the scene's smudges' residuals and filters' results starts in the arena
    /// bound (`smudge::Held`).
    bases: &'a Bases,
    output: Output,
    plan: RasterPlan,
    atlas: [u32; 2],
    packer: Packer,
    marks: Marks,
    batch: Batch,
    /// Where the units laid now lie (S65: the frame they lie in).
    placing: Placing,
    /// The base's footprint: one over its canvas pixels per pixel (1 without a base).
    base_foot: f32,
    /// The paint draws go to at level 0: the layer's, or a material's (the canvas).
    paint: Scratch,
    /// The level the units lie on (S65: 0, or a moved selection's or merged layer's content), and
    /// whether on its mask (after its `BeginMask`).
    depth: u8,
    on_mask: bool,
    /// Per scratch target: cleared, and drawn on where the region is.
    cleared: [bool; SCRATCH],
    touched: [bool; SCRATCH],
    chain: Chain,
    /// The group whose layers end now (S65): what lay under it is copied (`Some`), and whether
    /// one of its ends reached the region.
    group: Option<bool>,
    /// A smudge or filter whose result the arena lacks is left out (a material, S62), not refused.
    lacking: bool,
    /// S63: the stroke being drawn, laid at its unit.
    drawing: Option<&'a InProgress>,
}

/// The scene a plan draws and its smudges' residuals' and filters' results' bases.
type Drawn<'a> = (&'a Scene, &'a Bases);

impl RasterPlan {
    /// The plan of `scene` (its smudges' residuals and filters' results at `bases`) over a
    /// region of `size` pixels at `output`, with an atlas of `atlas` pixels.
    ///
    /// # Errors
    /// `Invalid` for a stroke's region larger than the atlas, a smudge or filter reaching the
    /// region whose result is not in the arena, or moves stacked deeper than the evaluator's.
    pub(crate) fn build(
        (scene, bases): Drawn<'_>,
        (output, size): (Output, [u32; 2]),
        atlas: [u32; 2],
    ) -> Result<Self, RenderError> {
        let builder = Builder::new((scene, bases), (output, size), atlas, Scratch::Layer);
        builder.lay(&scene.raster.units)
    }

    /// A smudge's or a filter's material (S62): the units laying the scene's `items` (one
    /// layer's paint or mask before it) drawn onto the canvas, which starts as nothing (the
    /// plan's paper): that layer's own pixels, premultiplied, which the finish rounds to 8 bits;
    /// their pixels lie on the canvas by `base` (S65: under a transform record). A smudge or
    /// filter before it whose result the arena lacks reaches none of the pixels the material is
    /// wanted at (what it reads is made present first) and is left out.
    ///
    /// # Errors
    /// As `build`.
    pub(crate) fn material(
        (scene, bases): Drawn<'_>,
        (items, base): (&Range<usize>, Affine),
        (output, size): (Output, [u32; 2]),
        atlas: [u32; 2],
    ) -> Result<Self, RenderError> {
        let mut builder = Builder::new((scene, bases), (output, size), atlas, Scratch::Canvas);
        builder.lacking = true;
        builder.based(base);
        builder.lay(scene.raster.within(items))
    }
}

impl<'a> Builder<'a> {
    fn new(
        (scene, bases): (&'a Scene, &'a Bases),
        (output, size): (Output, [u32; 2]),
        atlas: [u32; 2],
        paint: Scratch,
    ) -> Self {
        Builder {
            scene,
            bases,
            output,
            plan: RasterPlan {
                size,
                ..RasterPlan::default()
            },
            atlas,
            packer: Packer::new(atlas),
            marks: Marks::new(size),
            batch: Batch::default(),
            placing: Placing::new(output, size),
            base_foot: 1.0,
            paint,
            depth: 0,
            on_mask: false,
            cleared: [false; SCRATCH],
            touched: [false; SCRATCH],
            chain: Chain::Empty,
            group: None,
            lacking: false,
            drawing: None,
        }
    }

    /// The plan of `units`, in order.
    fn lay<'u>(
        mut self,
        units: impl IntoIterator<Item = &'u Unit>,
    ) -> Result<RasterPlan, RenderError> {
        self.plan.passes.push(Pass::Clear(Scratch::Canvas));
        self.cleared[Scratch::Canvas.index()] = true;
        for unit in units {
            self.unit(unit)?;
        }
        Ok(self.done())
    }

    /// One unit laid out.
    fn unit(&mut self, unit: &Unit) -> Result<(), RenderError> {
        match unit {
            Unit::Stroke {
                spans,
                paint,
                bounds,
            } => self.stroke(spans, *paint, *bounds)?,
            Unit::Edit { item, bounds } => self.edit(*item, *bounds),
            Unit::Smudge {
                item,
                smudge,
                reach,
            } => self.smudge((*item, *smudge), reach)?,
            Unit::Adjust { item, bounds } => self.adjust(*item, *bounds),
            Unit::Filter {
                item,
                filter,
                bounds,
            } => self.filter((*item, *filter), *bounds)?,
            Unit::BeginMask(_) => {
                self.flush();
                self.on_mask = true;
            }
            Unit::End(item) => self.end(*item),
            Unit::EndGroup(item) => self.end_group(*item),
            Unit::Frame(item) => self.frame(*item),
            Unit::BeginMove(_) => self.begin_move()?,
            Unit::EndMove(item) => self.end_move(*item)?,
            Unit::EndNested(item) => self.end_nested(*item)?,
            Unit::Active(_) => self.active()?,
        }
        Ok(())
    }

    /// The last batch's passes; the plan.
    fn done(mut self) -> RasterPlan {
        self.flush();
        self.material_done();
        let mut plan = self.plan;
        plan.counts.passes = plan.passes.len() + 1;
        plan
    }

    /// The paint draws go to now: the level's.
    fn paint(&self) -> Scratch {
        if self.depth == 0 {
            self.paint
        } else {
            Scratch::Level(self.depth)
        }
    }

    /// Where draws go now: the level's paint, or its mask.
    fn target(&self) -> Scratch {
        if self.on_mask {
            Scratch::Mask
        } else {
            self.paint()
        }
    }

    fn stroke(
        &mut self,
        spans: &Range<usize>,
        paint: usize,
        bounds: Option<Bounds>,
    ) -> Result<(), RenderError> {
        self.plan.counts.scene_strokes += 1;
        if bounds.is_none_or(|bounds| self.placing.reach(&bounds).is_empty()) {
            return Ok(());
        }
        let Some(laid) = lay(
            self.scene,
            (spans.clone(), paint),
            &self.placing,
            &mut self.marks,
        ) else {
            return Ok(());
        };
        let regions = 1 + usize::from(laid.contact);
        let at = match self.packer.places(laid.side(), regions) {
            Some(at) => at,
            None => {
                self.flush();
                (self.packer.places(laid.side(), regions)).ok_or(RenderError::Invalid(
                    "a stroke's region is larger than the raster core's atlas",
                ))?
            }
        };
        self.add_stroke(paint, &laid, &at);
        Ok(())
    }

    /// A stroke's instances and draws, its regions at `at` in the atlas.
    fn add_stroke(&mut self, paint: usize, laid: &Laid, at: &[[u32; 2]]) {
        let side = laid.side();
        let shift = |at: &[u32; 2]| {
            [
                at[0] as i32 - laid.rect.min[0],
                at[1] as i32 - laid.rect.min[1],
            ]
        };
        let places: Vec<[i32; 2]> = at.iter().map(shift).collect();
        let kinds: &[Coverage] = if laid.contact {
            &[Coverage::Density, Coverage::Contact]
        } else {
            std::slice::from_ref(&laid.coverage)
        };
        let lay_at = (self.placing.foot_scale, self.placing.frame);
        for ((coverage, place), at) in kinds.iter().zip(&places).zip(at) {
            let start = self.plan.instances.len() as u32;
            for &span in &laid.spans {
                let op = self.scene.items[span].op();
                laid_span(&mut self.plan, (op, span), *place, lay_at);
            }
            self.batch.coverage.push(CoverageDraw {
                coverage: *coverage,
                scissor: [at[0], at[1], side[0], side[1]],
                instances: start..self.plan.instances.len() as u32,
            });
            self.plan.counts.region_pixels += u64::from(side[0]) * u64::from(side[1]);
        }
        let start = self.plan.instances.len() as u32;
        let contact = places.get(1).copied().unwrap_or([0, 0]);
        let [x, y] = places[0];
        // The contact flag, then the frame (S65).
        let flags = u32::from(laid.contact) | self.placing.frame << 1;
        for [tx, ty] in &laid.tiles {
            self.plan.instances.push(Instance {
                a: [paint as u32, *tx, *ty, flags],
                b: [x, y, contact[0], contact[1]],
            });
        }
        self.plan.counts.tiles += laid.tiles.len();
        let blend = self.scene.items[paint].op().paint_blend();
        self.batch.draws.push(Draw {
            kind: DrawKind::Stroke(blend),
            instances: start..self.plan.instances.len() as u32,
        });
        self.plan.counts.strokes += 1;
        self.touched[self.target().index()] = true;
    }

    /// A fill's or a patch's quad over the pixels it reaches.
    fn edit(&mut self, item: usize, bounds: Bounds) {
        let rect = self.placing.reach(&bounds);
        if rect.is_empty() {
            return;
        }
        let op = self.scene.items[item].op();
        let kind = match op.edit() {
            Edit::Patch => DrawKind::Patch,
            _ => DrawKind::Fill(op.paint_blend()),
        };
        let draw = self.quad(item, rect, kind);
        self.batch.draws.push(draw);
        let [width, height] = [0, 1].map(|axis| (rect.max[axis] - rect.min[axis]) as u64);
        self.plan.counts.edits += 1;
        self.plan.counts.edit_pixels += width * height;
        self.touched[self.target().index()] = true;
    }

    /// One instance drawing `kind` for op `item` over `rect`, in the frame the units lie in.
    fn quad(&mut self, item: usize, rect: crate::geom::PixelRect, kind: DrawKind) -> Draw {
        let start = self.plan.instances.len() as u32;
        self.plan.instances.push(Instance {
            a: [item as u32, self.placing.frame, 0, 0],
            b: [rect.min[0], rect.min[1], rect.max[0], rect.max[1]],
        });
        Draw {
            kind,
            instances: start..start + 1,
        }
    }

    /// The batch's passes: its coverage, then its draws onto the target (cleared first once).
    fn flush(&mut self) {
        let batch = std::mem::take(&mut self.batch);
        if !batch.coverage.is_empty() {
            self.plan.passes.push(Pass::Coverage(batch.coverage));
            self.plan.counts.batches += 1;
            self.packer = Packer::new(self.atlas);
        }
        if !batch.draws.is_empty() {
            let target = self.target();
            let clear = !self.cleared[target.index()];
            self.cleared[target.index()] = true;
            self.plan.passes.push(Pass::Draw {
                target,
                clear,
                draws: batch.draws,
            });
        }
    }

    /// `target` cleared to nothing unless it was in the region (what lies under a level's end,
    /// what an adjustment or filter rewrites).
    fn ensure_cleared(&mut self, target: Scratch) {
        if !self.cleared[target.index()] {
            self.plan.passes.push(Pass::Clear(target));
            self.cleared[target.index()] = true;
        }
    }
}

#[path = "raster_plan_ends.rs"]
mod ends;

#[path = "raster_plan_records.rs"]
mod records;

#[path = "raster_plan_drawing.rs"]
mod drawing;
pub(crate) use drawing::{InProgress, KeptTile, Restored, Resume, StrokeDrawn};

#[path = "raster_plan_kept.rs"]
mod kept;
