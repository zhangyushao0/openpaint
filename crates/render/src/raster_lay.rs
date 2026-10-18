//! Where a stroke lies in a raster region (S57's probe, product code since S59; `raster_plan.rs`):
//! its spans reaching the region, the composite tiles they meet and their box in whole tiles,
//! which becomes its region of the atlas, placed shelf by shelf, and each span's instances there.
//! S65: a unit inside a transform record's content lies in its frame's pixels (`Placing`).

use std::ops::Range;

use crate::affine::Affine;
use crate::geom::{Bounds, PixelRect};
use crate::output::Output;
use crate::protocol::Op;
use crate::raster_plan::{Coverage, Instance, RasterPlan};
use crate::scene::Scene;
use crate::scene_cleared::intersect;

/// Side of a composite tile in region pixels (`RASTER_TILE` in raster.wgsl).
pub(crate) const TILE: u32 = 32;

/// No pixels.
const NONE: PixelRect = PixelRect {
    min: [0, 0],
    max: [0, 0],
};

/// Where the units laid now lie in a region (S65): bounds in the pixels of the frame they lie in,
/// cut to its clip, mapped onto the canvas and, for a material seen from a transform record, into
/// the material's pixels, placed at the region's output; the output pixels per content pixel a
/// span is flattened at there; the frame a draw names (its item plus one, 0 on the canvas).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Placing {
    pub(crate) output: Output,
    pub(crate) size: [u32; 2],
    /// Canvas pixels into the plan's (a material's base map back; `None`: the same).
    pub(crate) unbase: Option<Affine>,
    /// Content pixels into the plan's (`None`: the same).
    pub(crate) map: Option<Affine>,
    pub(crate) clip: Option<Bounds>,
    pub(crate) foot_scale: f32,
    pub(crate) frame: u32,
}

impl Placing {
    /// Units on the canvas of a region of `size` pixels at `output`.
    pub(crate) fn new(output: Output, size: [u32; 2]) -> Self {
        Self {
            output,
            size,
            unbase: None,
            map: None,
            clip: None,
            foot_scale: output.scale as f32,
            frame: 0,
        }
    }

    /// The region's pixels content `bounds` can reach.
    pub(crate) fn reach(&self, bounds: &Bounds) -> PixelRect {
        if bounds.is_empty() {
            return NONE;
        }
        let bounds = match self.clip {
            Some(clip) => match intersect(*bounds, clip) {
                Some(cut) => cut,
                None => return NONE,
            },
            None => *bounds,
        };
        let rect = self.map.map_or(bounds, |map| map.bounds(bounds)).pixels();
        self.placed(&rect)
    }

    /// The region's pixels the canvas pixels `rect` reach.
    pub(crate) fn reach_canvas(&self, rect: &PixelRect) -> PixelRect {
        if rect.is_empty() {
            return NONE;
        }
        let rect = match self.unbase {
            Some(unbase) => {
                let [x0, y0] = rect.min.map(|v| v as f32);
                let [x1, y1] = rect.max.map(|v| v as f32);
                unbase.bounds(Bounds([x0, y0, x1, y1])).pixels()
            }
            None => *rect,
        };
        self.placed(&rect)
    }

    fn placed(&self, rect: &PixelRect) -> PixelRect {
        let region = PixelRect::full(self.size);
        self.output.place(rect).intersect(&region)
    }
}

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

/// How the stroke of span items `spans` and paint op item `paint` reaches a region `placing`
/// places it in: `None` when it does not.
pub(crate) fn lay(
    scene: &Scene,
    (spans, paint): (Range<usize>, usize),
    placing: &Placing,
    marks: &mut Marks,
) -> Option<Laid> {
    let region = PixelRect::full(placing.size);
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
        let reached = placing.reach(&bounds);
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

    pub(crate) fn next(&mut self) {
        self.stroke += 1;
    }

    /// Marks the tiles `rect` (inside the region) meets, adding those new to `tiles`; returns
    /// their box in region pixels.
    pub(crate) fn mark(&mut self, rect: &PixelRect, tiles: &mut Vec<[u32; 2]>) -> PixelRect {
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
/// piece each at `foot_scale` output pixels per pixel of the span's, in frame `frame` (0: on the
/// canvas).
pub(crate) fn laid_span(
    plan: &mut RasterPlan,
    (op, index): (Op, usize),
    place: [i32; 2],
    (foot_scale, frame): (f32, u32),
) {
    let [x, y] = place;
    let b = [x, y, frame.cast_signed(), 0];
    let scale = foot_scale;
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
