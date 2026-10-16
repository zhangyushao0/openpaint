//! Where a stroke lies in a raster region (S57's probe, product code since S59; `raster_plan.rs`):
//! its spans reaching the region, the composite tiles they meet and their box in whole tiles,
//! which becomes its region of the atlas, placed shelf by shelf, and each span's instances there.

use std::ops::Range;

use crate::geom::PixelRect;
use crate::output::Output;
use crate::protocol::Op;
use crate::raster_plan::{Coverage, Instance, RasterPlan};
use crate::scene::Scene;

/// Side of a composite tile in region pixels (`RASTER_TILE` in raster.wgsl).
pub(crate) const TILE: u32 = 32;

/// A stroke as it reaches a region: its spans that do, its tiles, its box (whole tiles).
pub(crate) struct Laid {
    pub(crate) spans: Vec<usize>,
    pub(crate) tiles: Vec<[u32; 2]>,
    pub(crate) rect: PixelRect,
    pub(crate) coverage: Coverage,
    /// It lays its density through the paper: a second region holds its contact.
    pub(crate) contact: bool,
}

impl Laid {
    /// Its box's width and height.
    pub(crate) fn side(&self) -> [u32; 2] {
        [0, 1].map(|axis| (self.rect.max[axis] - self.rect.min[axis]) as u32)
    }
}

/// How the stroke of span items `spans` and paint op item `paint` reaches a region of `size` at
/// `output`: `None` when it does not.
pub(crate) fn lay(
    scene: &Scene,
    (spans, paint): (Range<usize>, usize),
    (output, size): (Output, [u32; 2]),
    marks: &mut Marks,
) -> Option<Laid> {
    let region = PixelRect::full(size);
    marks.next();
    let mut reaching = Vec::new();
    let mut tiles = Vec::new();
    let mut rect: Option<PixelRect> = None;
    for span in spans {
        let Some(bounds) = scene.items[span]
            .bounds()
            .filter(|bounds| !bounds.is_empty())
        else {
            continue;
        };
        let reached = output.place(&bounds.pixels()).intersect(&region);
        if reached.is_empty() {
            continue;
        }
        reaching.push(span);
        let whole = marks.mark(&reached, &mut tiles);
        rect = Some(rect.map_or(whole, |rect| rect.union(&whole)));
    }
    let first = scene.items[*reaching.first()?].op();
    let coverage = match (first.is_stamped(), first.covers()) {
        (true, true) => Coverage::StampCover,
        (true, false) => Coverage::StampDensity,
        (false, true) => Coverage::Cover,
        (false, false) => Coverage::Density,
    };
    Some(Laid {
        spans: reaching,
        tiles,
        rect: rect?.intersect(&region),
        contact: coverage == Coverage::Density && scene.items[paint].op().on_paper(),
        coverage,
    })
}

/// Which composite tiles of the region a stroke reached: a mark per tile, renewed per stroke.
pub(crate) struct Marks {
    columns: u32,
    marks: Vec<u32>,
    stroke: u32,
}

impl Marks {
    pub(crate) fn new([width, height]: [u32; 2]) -> Self {
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

    /// Marks the tiles `rect` (inside the region) meets, adding those new to `tiles`; returns
    /// their box in region pixels.
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

/// The instances of span `op` (item `index`) laid at `place` in the atlas: a stamp each, or a
/// piece each at `scale`.
pub(crate) fn laid_span(
    plan: &mut RasterPlan,
    (op, index): (Op, usize),
    place: [i32; 2],
    scale: f32,
) {
    let [x, y] = place;
    let b = [x, y, 0, 0];
    let index = index as u32;
    if op.is_stamped() {
        if let Some((first, last)) = op.stamp_indices() {
            (plan.instances).extend((first..=last).map(|n| Instance {
                a: [index, n, 0, 0],
                b,
            }));
            plan.counts.stamps += (last - first + 1) as usize;
        }
        return;
    }
    let count = op.span_pieces(scale);
    let lone = u32::from(count == 1);
    (plan.instances).extend((0..count).map(|k| Instance {
        a: [index, k, count, lone],
        b,
    }));
    plan.counts.pieces += count as usize;
}

/// Places regions in the atlas shelf by shelf, left to right, top to bottom.
pub(crate) struct Packer {
    size: [u32; 2],
    x: u32,
    y: u32,
    shelf: u32,
}

impl Packer {
    pub(crate) fn new(size: [u32; 2]) -> Self {
        Self {
            size,
            x: 0,
            y: 0,
            shelf: 0,
        }
    }

    /// Where `count` regions of `side` pixels go; `None` once the atlas is full.
    pub(crate) fn places(&mut self, side: [u32; 2], count: usize) -> Option<Vec<[u32; 2]>> {
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
