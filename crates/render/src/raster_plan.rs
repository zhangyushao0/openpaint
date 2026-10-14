//! The raster probe's plan on the CPU (S57, `raster.rs`): a scene of one layer's strokes as the
//! instances its render passes draw. Each stroke reaching the target gets a region of the scratch
//! atlas, its spans' output bounds rounded out to whole composite tiles (two regions for a stroke
//! on paper: its density and its contact); its spans become one instance per piece (continuous:
//! `Op::span_pieces` at the output's scale, as the evaluator cuts them) or per stamp (stamped:
//! the stamps the span holds), and its tiles one instance each for the composite. Strokes go into
//! a batch in scene order while their regions fit the atlas, shelf by shelf; a batch is two
//! render passes, its coverage into the atlas and its composites onto the layer.

use std::ops::Range;

use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::protocol::{Blend, Op};
use crate::scene::{Item, Scene};

/// Side of a composite tile in target pixels (`RASTER_TILE` in raster.wgsl).
pub(crate) const TILE: u32 = 32;

/// One instance as the probe's vertex stages read it: whole numbers, then signed ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Instance {
    pub(crate) a: [u32; 4],
    pub(crate) b: [i32; 4],
}

/// How a stroke's instances lay into its region of the atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Coverage {
    /// Pieces of a covering stroke: the largest coverage.
    Cover,
    /// Pieces of an accumulating one: their densities summed; and its paper contact, the largest.
    Density,
    Contact,
    /// Stamps, kept the largest or summed.
    StampCover,
    StampDensity,
}

/// One draw into the atlas: the instances of one stroke's region, cut to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CoverageDraw {
    pub(crate) coverage: Coverage,
    /// The region: x, y, width, height in atlas pixels.
    pub(crate) scissor: [u32; 4],
    pub(crate) instances: Range<u32>,
}

/// One stroke's composite: its tiles' instances, by its paint op's blend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompositeDraw {
    pub(crate) blend: Blend,
    pub(crate) tiles: Range<u32>,
}

/// The strokes whose regions share the atlas: one coverage pass, one composite pass.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Batch {
    pub(crate) coverage: Vec<CoverageDraw>,
    pub(crate) composite: Vec<CompositeDraw>,
}

/// What the plan holds, counted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RasterCounts {
    /// Strokes reaching the target, of the scene's.
    pub strokes: usize,
    pub scene_strokes: usize,
    /// Batches (two render passes each; one more pass lays the layer on the paper).
    pub batches: usize,
    /// Piece instances (a paper stroke's twice: density and contact) and stamp instances.
    pub pieces: usize,
    pub stamps: usize,
    /// Composite tile instances.
    pub tiles: usize,
    /// Atlas pixels the regions take, summed over the batches.
    pub region_pixels: u64,
}

#[derive(Debug, Default)]
pub(crate) struct RasterPlan {
    pub(crate) instances: Vec<Instance>,
    pub(crate) tiles: Vec<Instance>,
    pub(crate) batches: Vec<Batch>,
    /// The layer's opacity (its end onto the canvas).
    pub(crate) opacity: f32,
    pub(crate) counts: RasterCounts,
}

/// A stroke of the scene: its span items and its paint op's item.
struct StrokeItems {
    spans: Vec<usize>,
    paint: usize,
}

/// A stroke as it reaches the target: its spans that do, its tiles, its box (whole tiles).
struct Laid {
    spans: Vec<usize>,
    tiles: Vec<[u32; 2]>,
    rect: PixelRect,
    coverage: Coverage,
    contact: bool,
}

impl RasterPlan {
    /// The plan of `scene` (one plain layer of strokes, else `Invalid`) over a target of `size`
    /// pixels at `output`, with an atlas of `atlas` pixels.
    pub(crate) fn build(
        scene: &Scene,
        output: Output,
        size: [u32; 2],
        atlas: [u32; 2],
    ) -> Result<Self, RenderError> {
        let (strokes, opacity) = strokes(scene)?;
        let mut plan = Self {
            opacity,
            ..Self::default()
        };
        plan.counts.scene_strokes = strokes.len();
        let mut packer = Packer::new(atlas);
        let mut batch = Batch::default();
        let mut marks = Marks::new(size);
        for stroke in &strokes {
            let Some(laid) = lay(scene, stroke, (output, size), &mut marks) else {
                continue;
            };
            let side = [
                (laid.rect.max[0] - laid.rect.min[0]) as u32,
                (laid.rect.max[1] - laid.rect.min[1]) as u32,
            ];
            let regions = 1 + usize::from(laid.contact);
            let at = match packer.places(side, regions) {
                Some(at) => at,
                None => {
                    plan.batches.push(std::mem::take(&mut batch));
                    packer = Packer::new(atlas);
                    (packer.places(side, regions)).ok_or(RenderError::Invalid(
                        "a stroke's region is larger than the probe's atlas",
                    ))?
                }
            };
            plan.add(scene, (stroke, &laid), (output, &at), &mut batch);
        }
        if !batch.coverage.is_empty() {
            plan.batches.push(batch);
        }
        plan.counts.batches = plan.batches.len();
        plan.counts.tiles = plan.tiles.len();
        Ok(plan)
    }

    /// A stroke's instances and draws, its regions at `at` in the atlas.
    fn add(
        &mut self,
        scene: &Scene,
        (stroke, laid): (&StrokeItems, &Laid),
        (output, at): (Output, &[[u32; 2]]),
        batch: &mut Batch,
    ) {
        let side = [
            laid.rect.max[0] - laid.rect.min[0],
            laid.rect.max[1] - laid.rect.min[1],
        ];
        let shift = |at: [u32; 2]| {
            [
                at[0] as i32 - laid.rect.min[0],
                at[1] as i32 - laid.rect.min[1],
            ]
        };
        let places: Vec<[i32; 2]> = at.iter().map(|at| shift(*at)).collect();
        let kinds: &[Coverage] = if laid.contact {
            &[Coverage::Density, Coverage::Contact]
        } else {
            std::slice::from_ref(&laid.coverage)
        };
        for ((coverage, place), at) in kinds.iter().zip(&places).zip(at) {
            let start = self.instances.len() as u32;
            for &span in &laid.spans {
                self.span(scene.items[span].op(), span, *place, output.scale as f32);
            }
            batch.coverage.push(CoverageDraw {
                coverage: *coverage,
                scissor: [at[0], at[1], side[0] as u32, side[1] as u32],
                instances: start..self.instances.len() as u32,
            });
            self.counts.region_pixels += (side[0] * side[1]) as u64;
        }
        let start = self.tiles.len() as u32;
        let contact = places.get(1).copied().unwrap_or([0, 0]);
        let [x, y] = places[0];
        for [tx, ty] in &laid.tiles {
            self.tiles.push(Instance {
                a: [stroke.paint as u32, *tx, *ty, u32::from(laid.contact)],
                b: [x, y, contact[0], contact[1]],
            });
        }
        let paint = scene.items[stroke.paint].op();
        batch.composite.push(CompositeDraw {
            blend: paint.paint_blend(),
            tiles: start..self.tiles.len() as u32,
        });
        self.counts.strokes += 1;
    }

    /// The instances of span `op` (item `index`): a stamp each, or a piece each at `scale`.
    fn span(&mut self, op: Op, index: usize, [x, y]: [i32; 2], scale: f32) {
        let b = [x, y, 0, 0];
        let index = index as u32;
        if op.is_stamped() {
            if let Some((first, last)) = op.stamp_indices() {
                self.instances.extend((first..=last).map(|n| Instance {
                    a: [index, n, 0, 0],
                    b,
                }));
                self.counts.stamps += (last - first + 1) as usize;
            }
            return;
        }
        let count = op.span_pieces(scale);
        let lone = u32::from(count == 1);
        self.instances.extend((0..count).map(|k| Instance {
            a: [index, k, count, lone],
            b,
        }));
        self.counts.pieces += count as usize;
    }
}

/// The scene's strokes in order and its layer's opacity; `Invalid` for anything but one plain
/// layer of finished strokes.
fn strokes(scene: &Scene) -> Result<(Vec<StrokeItems>, f32), RenderError> {
    let refused = RenderError::Invalid("the raster probe draws one plain layer of strokes only");
    let mut strokes = Vec::new();
    let mut spans = Vec::new();
    let mut opacity = None;
    for (index, item) in scene.items.iter().enumerate() {
        match item {
            Item::Segment { .. } if opacity.is_none() => spans.push(index),
            Item::EndStroke(_) if opacity.is_none() => strokes.push(StrokeItems {
                spans: std::mem::take(&mut spans),
                paint: index,
            }),
            Item::EndLayer { op, .. } if opacity.is_none() && op.layer_end().plain => {
                opacity = Some(op.layer_end().opacity);
            }
            _ => return Err(refused),
        }
    }
    Ok((strokes, opacity.unwrap_or(1.0)))
}

/// How a stroke reaches a target of `size` at `output`: `None` when it does not.
fn lay(
    scene: &Scene,
    stroke: &StrokeItems,
    (output, size): (Output, [u32; 2]),
    marks: &mut Marks,
) -> Option<Laid> {
    let target = PixelRect::full(size);
    marks.next();
    let mut spans = Vec::new();
    let mut tiles = Vec::new();
    let mut rect: Option<PixelRect> = None;
    for &span in &stroke.spans {
        let Some(bounds) = scene.items[span]
            .bounds()
            .filter(|bounds| !bounds.is_empty())
        else {
            continue;
        };
        let reached = output.place(&bounds.pixels()).intersect(&target);
        if reached.is_empty() {
            continue;
        }
        spans.push(span);
        let whole = marks.mark(&reached, &mut tiles);
        rect = Some(rect.map_or(whole, |rect| rect.union(&whole)));
    }
    let first = scene.items[*spans.first()?].op();
    let paint = scene.items[stroke.paint].op();
    let coverage = match (first.is_stamped(), first.covers()) {
        (true, true) => Coverage::StampCover,
        (true, false) => Coverage::StampDensity,
        (false, true) => Coverage::Cover,
        (false, false) => Coverage::Density,
    };
    Some(Laid {
        spans,
        tiles,
        rect: rect?.intersect(&target),
        contact: coverage == Coverage::Density && paint.on_paper(),
        coverage,
    })
}

/// Which composite tiles of the target a stroke reached: a mark per tile, renewed per stroke.
struct Marks {
    columns: u32,
    marks: Vec<u32>,
    stroke: u32,
}

impl Marks {
    fn new([width, height]: [u32; 2]) -> Self {
        let columns = width.div_ceil(TILE);
        Self {
            columns,
            marks: vec![0; (columns * height.div_ceil(TILE)) as usize],
            stroke: 0,
        }
    }

    fn next(&mut self) {
        self.stroke += 1;
    }

    /// Marks the tiles `rect` (inside the target) meets, adding those new to `tiles`; returns
    /// their box in target pixels.
    fn mark(&mut self, rect: &PixelRect, tiles: &mut Vec<[u32; 2]>) -> PixelRect {
        let tile = TILE as i32;
        let low = [rect.min[0] / tile, rect.min[1] / tile];
        let high = [(rect.max[0] - 1) / tile, (rect.max[1] - 1) / tile];
        for ty in low[1]..=high[1] {
            for tx in low[0]..=high[0] {
                let at = (ty as u32 * self.columns + tx as u32) as usize;
                if self.marks[at] != self.stroke {
                    self.marks[at] = self.stroke;
                    tiles.push([tx as u32 * TILE, ty as u32 * TILE]);
                }
            }
        }
        PixelRect {
            min: [low[0] * tile, low[1] * tile],
            max: [(high[0] + 1) * tile, (high[1] + 1) * tile],
        }
    }
}

/// Places regions in the atlas shelf by shelf, left to right, top to bottom.
struct Packer {
    size: [u32; 2],
    x: u32,
    y: u32,
    shelf: u32,
}

impl Packer {
    fn new(size: [u32; 2]) -> Self {
        Self {
            size,
            x: 0,
            y: 0,
            shelf: 0,
        }
    }

    /// Where `count` regions of `side` pixels go; `None` once the atlas is full.
    fn places(&mut self, side: [u32; 2], count: usize) -> Option<Vec<[u32; 2]>> {
        (0..count).map(|_| self.place(side)).collect()
    }

    fn place(&mut self, [width, height]: [u32; 2]) -> Option<[u32; 2]> {
        if width > self.size[0] {
            return None;
        }
        if self.x + width > self.size[0] {
            self.y += self.shelf;
            self.x = 0;
            self.shelf = 0;
        }
        if self.y + height > self.size[1] {
            return None;
        }
        let at = [self.x, self.y];
        self.x += width;
        self.shelf = self.shelf.max(height);
        Some(at)
    }
}
