//! The raster core's plan on the CPU (S57's probe made product code, S59): one region of a scene's
//! output as the render passes that draw it. Each stroke reaching the region gets a region of the
//! scratch atlas (`raster_lay.rs`), its pieces and stamps lay their coverage there, combined by
//! the blend unit, and its tiles composite it onto the layer by the evaluator's paint; fills and
//! patches draw onto the layer as quads; a smudge copies what it reaches of the layer (or mask)
//! and lays it back moved by its residual (S61, D88); a mask's units draw onto a target of their
//! own; each layer's end composites it onto the canvas (or into its clipping chain) by its mode,
//! through its mask, a copy of what lies under it first where fixed-function blending cannot
//! (Overlay). Strokes go into a batch in scene order while their regions fit the atlas; a batch
//! is a coverage pass and a pass of the layer's draws, and a smudge ends one. The order of every
//! draw is the scene's, so a pixel's value depends only on what reaches it: a part of a region
//! draws what the whole does there.

use std::ops::Range;

use crate::error::RenderError;
use crate::geom::{Bounds, PixelRect};
use crate::output::Output;
use crate::protocol::raster::Edit;
pub use crate::raster_kinds::RasterCounts;
pub(crate) use crate::raster_kinds::{
    Coverage, CoverageDraw, Draw, DrawKind, EndBlend, Instance, Pass, Scratch,
};
use crate::raster_lay::{Laid, Marks, Packer, laid_span, lay};
use crate::raster_route::{RasterScene, Unit};
use crate::scene::Scene;

#[derive(Debug, Default)]
pub(crate) struct RasterPlan {
    pub(crate) instances: Vec<Instance>,
    pub(crate) passes: Vec<Pass>,
    /// The region's size in pixels.
    pub(crate) size: [u32; 2],
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
    /// Where each of the scene's smudges' residuals starts in the arena bound (`smudge::Held`).
    bases: &'a [Option<u32>],
    output: Output,
    plan: RasterPlan,
    atlas: [u32; 2],
    packer: Packer,
    marks: Marks,
    batch: Batch,
    /// Where draws go now: the layer, or its mask after its `BeginMask` (a material: the canvas).
    target: Scratch,
    /// The layer and the mask were cleared, and drawn on where the region is.
    cleared: [bool; 2],
    touched: [bool; 2],
    chain: Chain,
    /// A smudge whose residual the arena lacks is left out (a material, S62), not refused.
    lacking: bool,
}

/// The scene a plan draws: its items, its raster units and its smudges' residuals' bases.
type Drawn<'a> = (&'a Scene, &'a RasterScene, &'a [Option<u32>]);

impl RasterPlan {
    /// The plan of `raster` (`scene`'s units, its smudges' residuals at `bases`) over a region of
    /// `size` pixels at `output`, with an atlas of `atlas` pixels.
    ///
    /// # Errors
    /// `Invalid` for a stroke's region larger than the atlas, or a smudge reaching the region
    /// whose residual is not in the arena.
    pub(crate) fn build(
        (scene, raster, bases): Drawn<'_>,
        (output, size): (Output, [u32; 2]),
        atlas: [u32; 2],
    ) -> Result<Self, RenderError> {
        let builder = Builder::new((scene, bases), (output, size), atlas, Scratch::Layer);
        builder.lay(&raster.units)
    }

    /// A smudge's material (S62): the units laying the scene's `items` (its layer's paint or mask
    /// before it) drawn onto the canvas, which starts as nothing (the plan's paper): that layer's
    /// own pixels, premultiplied, which the finish rounds to 8 bits. A smudge before it whose
    /// residual the arena lacks reaches none of the pixels the material is wanted at (the
    /// residuals it reads are made present first, `engine_needs.rs`) and is left out; the finish
    /// writes only those pixels.
    ///
    /// # Errors
    /// As `build`.
    pub(crate) fn material(
        (scene, raster, bases): Drawn<'_>,
        items: &Range<usize>,
        (output, size): (Output, [u32; 2]),
        atlas: [u32; 2],
    ) -> Result<Self, RenderError> {
        let mut builder = Builder::new((scene, bases), (output, size), atlas, Scratch::Canvas);
        builder.lacking = true;
        builder.lay(raster.within(items))
    }
}

impl<'a> Builder<'a> {
    fn new(
        (scene, bases): (&'a Scene, &'a [Option<u32>]),
        (output, size): (Output, [u32; 2]),
        atlas: [u32; 2],
        target: Scratch,
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
            target,
            cleared: [false; 2],
            touched: [false; 2],
            chain: Chain::Empty,
            lacking: false,
        }
    }

    /// The plan of `units`, in order.
    fn lay<'u>(
        mut self,
        units: impl IntoIterator<Item = &'u Unit>,
    ) -> Result<RasterPlan, RenderError> {
        self.plan.passes.push(Pass::Clear(Scratch::Canvas));
        for unit in units {
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
                Unit::BeginMask => {
                    self.flush();
                    self.target = Scratch::Mask;
                }
                Unit::End(item) => self.end(*item),
            }
        }
        self.flush();
        let mut plan = self.plan;
        plan.counts.passes = plan.passes.len() + 1;
        Ok(plan)
    }
}

impl Builder<'_> {
    /// The region's pixels the canvas pixels `rect` reach.
    fn reach(&self, rect: &PixelRect) -> PixelRect {
        let region = PixelRect::full(self.plan.size);
        self.output.place(rect).intersect(&region)
    }

    fn stroke(
        &mut self,
        spans: &Range<usize>,
        paint: usize,
        bounds: Option<Bounds>,
    ) -> Result<(), RenderError> {
        self.plan.counts.scene_strokes += 1;
        if bounds.is_none_or(|bounds| self.reach(&bounds.pixels()).is_empty()) {
            return Ok(());
        }
        let place = (self.output, self.plan.size);
        let Some(laid) = lay(self.scene, (spans.clone(), paint), place, &mut self.marks) else {
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
        let scale = self.output.scale as f32;
        for ((coverage, place), at) in kinds.iter().zip(&places).zip(at) {
            let start = self.plan.instances.len() as u32;
            for &span in &laid.spans {
                let op = self.scene.items[span].op();
                laid_span(&mut self.plan, (op, span), *place, scale);
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
        for [tx, ty] in &laid.tiles {
            self.plan.instances.push(Instance {
                a: [paint as u32, *tx, *ty, u32::from(laid.contact)],
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
        self.touched[self.slot()] = true;
    }

    /// A fill's or a patch's quad over the pixels it reaches.
    fn edit(&mut self, item: usize, bounds: Bounds) {
        let rect = self.reach(&bounds.pixels());
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
        self.touched[self.slot()] = true;
    }

    /// A smudge (op item `item`, the scene's smudge number `smudge`) over the region's pixels the
    /// canvas pixels `reach` reach: what is laid there so far copied, then laid back moved by its
    /// residual (D88). The layer or mask starts as nothing here when nothing was laid yet: the
    /// smudge may carry paint in from outside.
    fn smudge(
        &mut self,
        (item, smudge): (usize, usize),
        reach: &PixelRect,
    ) -> Result<(), RenderError> {
        let rect = self.reach(reach);
        if rect.is_empty() {
            return Ok(());
        }
        let Some(base) = self.bases.get(smudge).copied().flatten() else {
            if self.lacking {
                return Ok(());
            }
            return Err(RenderError::Invalid(
                "a smudge's residual is not in the arena",
            ));
        };
        self.flush();
        let slot = self.slot();
        if !self.cleared[slot] {
            self.plan.passes.push(Pass::Clear(self.target));
            self.cleared[slot] = true;
        }
        let start = self.plan.instances.len() as u32;
        self.plan.instances.push(Instance {
            a: [item as u32, base, 0, 0],
            b: [rect.min[0], rect.min[1], rect.max[0], rect.max[1]],
        });
        let draw = Draw {
            kind: DrawKind::Smudge,
            instances: start..start + 1,
        };
        let [x, y] = rect.min.map(|at| at.max(0) as u32);
        let [width, height] = [0, 1].map(|axis| (rect.max[axis] - rect.min[axis]) as u32);
        let target = self.target;
        let rect = [x, y, width, height];
        self.plan.passes.push(Pass::Smudge { target, rect, draw });
        self.plan.counts.smudges += 1;
        self.plan.counts.smudge_pixels += u64::from(width) * u64::from(height);
        self.touched[slot] = true;
        Ok(())
    }

    /// One instance drawing `kind` for op `item` over `rect`.
    fn quad(&mut self, item: usize, rect: PixelRect, kind: DrawKind) -> Draw {
        let start = self.plan.instances.len() as u32;
        self.plan.instances.push(Instance {
            a: [item as u32, 0, 0, 0],
            b: [rect.min[0], rect.min[1], rect.max[0], rect.max[1]],
        });
        Draw {
            kind,
            instances: start..start + 1,
        }
    }

    /// The layer or the mask: which `cleared` and `touched` slot draws go to.
    fn slot(&self) -> usize {
        usize::from(self.target == Scratch::Mask)
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
            let slot = self.slot();
            let clear = !self.cleared[slot];
            self.cleared[slot] = true;
            self.plan.passes.push(Pass::Draw {
                target: self.target,
                clear,
                draws: batch.draws,
            });
        }
    }
}

#[path = "raster_plan_ends.rs"]
mod ends;
