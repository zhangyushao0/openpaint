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
//! ask and keeps it without its turn. Nearest points are looked up in a grid of the points
//! (`Grid`, each a lone point), so a busy copy fits in milliseconds; it runs off the frame loop
//! all the same.

use crate::align::{Fit, Similarity, solve};
use crate::grid::{Grid, bounds};
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
    let copy_points: Vec<&[Point]> = copies.iter().map(std::slice::from_ref).collect();
    let copy_grid = Grid::of(&copy_points);
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
        if let Some(near) = copy_grid.nearest(*at) {
            outward.push((distance(*at, near.at), *o, near.at));
        }
    }
    let mut gaps: Vec<f64> = outward.iter().map(|(gap, ..)| *gap).collect();
    gaps.sort_by(f64::total_cmp);
    let limit = gaps
        .get(gaps.len() / 2)
        .map_or(f64::INFINITY, |median| (TRIM * median).max(LEAST_TRIM));
    let laid_points: Vec<&[Point]> = laid.iter().map(std::slice::from_ref).collect();
    let laid_grid = Grid::of(&laid_points);
    let mut inward: Vec<(f64, Point, Point)> = Vec::with_capacity(copies.len());
    for c in copies {
        if let Some(near) = laid_grid.nearest(*c) {
            inward.push((distance(laid[near.line], *c), originals[near.line], *c));
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
