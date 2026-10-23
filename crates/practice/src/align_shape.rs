//! A single shape's outline laid onto a copy (S80, Compare's aligned overlay), moved and evenly
//! sized, never turned, as a line is (`align.rs`). A copy of a shape holds more than its outline:
//! the lines inside (folds, features, construction) lie off the outline on purpose, so they must
//! not pull it in.
//!
//! The fit starts from the two sides' boxes (the lines inside a copy lie within its outline, so its
//! box is the outline's): the scale their areas ask and their middles laid on each other. Each
//! round pairs every point of the outline as placed with the nearest point of the copy, leaving
//! out pairs much farther apart than most (a part not drawn yet), and the copy's points with the
//! nearest point of the placed outline, keeping only the nearer half of them (the lines inside lie
//! farther from the outline than the copy's own outline does), then solves the similarity the pairs
//! ask and keeps it without its turn. Nearest points are looked up in a grid of the points, so a
//! busy copy fits in milliseconds; it runs off the frame loop all the same.

use crate::align::{Fit, Similarity, solve};
use crate::path::{Point, distance, resample};

/// Steps the outline and the copy are compared at, in frame sides.
const SPACING: f64 = 0.01;
/// Rounds at most, and the change (frame sides) a round ends the fit under.
const ROUNDS: usize = 60;
const SETTLED: f64 = 1e-9;
/// The outline's pairs farther apart than this many times their median (and at least
/// `LEAST_TRIM` frame sides) are left out; of the copy's pairs the nearer `COPY_KEPT` are kept.
const TRIM: f64 = 2.5;
const LEAST_TRIM: f64 = 0.02;
const COPY_KEPT: f64 = 0.5;
/// The scales a fit may take.
const SCALES: [f64; 2] = [0.2, 5.0];
/// The grid's cell, in frame sides.
const CELL: f64 = 0.02;

/// The fit that lays `outline` (closed lines, frame units) best over the lines of `copy`; `None`
/// when either has no extent.
#[must_use]
pub fn align_shape(outline: &[&[Point]], copy: &[&[Point]]) -> Option<Fit> {
    let originals: Vec<Point> = outline.iter().flat_map(|l| resample(l, SPACING)).collect();
    let copies: Vec<Point> = copy.iter().flat_map(|l| resample(l, SPACING)).collect();
    let (o_low, o_high) = bounds(&originals)?;
    let (c_low, c_high) = bounds(&copies)?;
    let area = |low: Point, high: Point| (high[0] - low[0]) * (high[1] - low[1]);
    let (o_area, c_area) = (area(o_low, o_high), area(c_low, c_high));
    if o_area <= 0.0 || c_area <= 0.0 {
        return None;
    }
    let scale = (c_area / o_area).sqrt().clamp(SCALES[0], SCALES[1]);
    let middle = |low: Point, high: Point| [(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0];
    let (o_mid, c_mid) = (middle(o_low, o_high), middle(c_low, c_high));
    let shift = [c_mid[0] - scale * o_mid[0], c_mid[1] - scale * o_mid[1]];
    let mut placed = Similarity {
        scale,
        turn: [1.0, 0.0],
        shift,
    };
    let mut fit = Fit { scale, shift };
    let copy_grid = Grid::of(&copies);
    for _ in 0..ROUNDS {
        let pairs = pairs(&placed, &originals, (&copies, &copy_grid));
        let Some((next, unturned)) = solve(&pairs) else {
            break;
        };
        let next = Similarity {
            scale: next.scale.clamp(SCALES[0], SCALES[1]),
            ..next
        };
        let moved = next.moved(&placed);
        (placed, fit) = (next, unturned);
        if moved < SETTLED {
            break;
        }
    }
    Some(fit)
}

/// The least and greatest corner of `points`; `None` for none.
fn bounds(points: &[Point]) -> Option<(Point, Point)> {
    let first = *points.first()?;
    Some(points.iter().fold((first, first), |(low, high), p| {
        (
            [low[0].min(p[0]), low[1].min(p[1])],
            [high[0].max(p[0]), high[1].max(p[1])],
        )
    }))
}

/// The pairs `(outline's, copy's)` a round fits under `placed`: the outline's points with the
/// copy's nearest (trimmed), and the copy's nearer half with the placed outline's nearest.
fn pairs(
    placed: &Similarity,
    originals: &[Point],
    (copies, copy_grid): (&[Point], &Grid),
) -> Vec<(Point, Point)> {
    let laid: Vec<Point> = originals.iter().map(|p| placed.apply(*p)).collect();
    let mut outward: Vec<(f64, Point, Point)> = Vec::with_capacity(originals.len());
    for (o, at) in originals.iter().zip(&laid) {
        if let Some(c) = copy_grid.nearest(copies, *at) {
            outward.push((distance(*at, c), *o, c));
        }
    }
    let mut gaps: Vec<f64> = outward.iter().map(|(gap, ..)| *gap).collect();
    gaps.sort_by(f64::total_cmp);
    let limit = gaps
        .get(gaps.len() / 2)
        .map_or(f64::INFINITY, |median| (TRIM * median).max(LEAST_TRIM));
    let laid_grid = Grid::of(&laid);
    let mut inward: Vec<(f64, Point, Point)> = Vec::with_capacity(copies.len());
    for c in copies {
        if let Some(index) = laid_grid.nearest_index(&laid, *c) {
            inward.push((distance(laid[index], *c), originals[index], *c));
        }
    }
    inward.sort_by(|a, b| a.0.total_cmp(&b.0));
    inward.truncate((COPY_KEPT * inward.len() as f64).ceil() as usize);
    (outward.into_iter())
        .filter(|(gap, ..)| *gap <= limit)
        .chain(inward.into_iter().filter(|(gap, ..)| *gap <= limit))
        .map(|(_, o, c)| (o, c))
        .collect()
}

/// Points in square cells of `CELL`, for nearest-point lookups.
struct Grid {
    low: Point,
    across: usize,
    down: usize,
    cells: Vec<Vec<usize>>,
}

impl Grid {
    fn of(points: &[Point]) -> Self {
        let (low, high) = bounds(points).unwrap_or(([0.0, 0.0], [0.0, 0.0]));
        let cells_of = |extent: f64| ((extent / CELL).floor() as usize + 1).min(4096);
        let (across, down) = (cells_of(high[0] - low[0]), cells_of(high[1] - low[1]));
        let mut cells = vec![Vec::new(); across * down];
        let mut grid = Self {
            low,
            across,
            down,
            cells: Vec::new(),
        };
        for (index, p) in points.iter().enumerate() {
            let (x, y) = grid.cell(*p);
            cells[y * across + x].push(index);
        }
        grid.cells = cells;
        grid
    }

    /// The cell holding `p`, clamped to the grid.
    fn cell(&self, p: Point) -> (usize, usize) {
        let at = |v: f64, low: f64, cells: usize| {
            (((v - low) / CELL).floor().max(0.0) as usize).min(cells - 1)
        };
        (
            at(p[0], self.low[0], self.across),
            at(p[1], self.low[1], self.down),
        )
    }

    /// The point of `points` nearest `p`.
    fn nearest(&self, points: &[Point], p: Point) -> Option<Point> {
        self.nearest_index(points, p).map(|index| points[index])
    }

    /// The index of the point of `points` nearest `p`: rings of cells round `p`'s, until a ring
    /// lies farther than the nearest found.
    fn nearest_index(&self, points: &[Point], p: Point) -> Option<usize> {
        let (cx, cy) = self.cell(p);
        let mut best: Option<(usize, f64)> = None;
        for ring in 0..self.across.max(self.down) {
            if best.is_some_and(|(_, d)| d < (ring.saturating_sub(1)) as f64 * CELL) {
                break;
            }
            let (x0, x1) = (cx.saturating_sub(ring), (cx + ring).min(self.across - 1));
            let (y0, y1) = (cy.saturating_sub(ring), (cy + ring).min(self.down - 1));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let on_ring = x == x0 || x == x1 || y == y0 || y == y1;
                    if !on_ring {
                        continue;
                    }
                    for &index in &self.cells[y * self.across + x] {
                        let d = distance(points[index], p);
                        if best.is_none_or(|(_, b)| d < b) {
                            best = Some((index, d));
                        }
                    }
                }
            }
        }
        best.map(|(index, _)| index)
    }
}
