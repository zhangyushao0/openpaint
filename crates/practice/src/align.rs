//! The original laid onto a copy (Compare's aligned overlay): moved and evenly sized onto the
//! copy's best fit, never turned, so a copy's angle errors still show while its place and size no
//! longer do. The original is one line or a group of them (combined lines, fitted together as one
//! shape); both are in frame units.
//!
//! The fit is least squares on corresponding points, the correspondences found as iterative
//! closest points do. It starts from the two sides' centres and spreads along their length
//! (`centre_and_spread`: exact for a moved and sized copy). Each round pairs every point of the
//! original as placed with the nearest point of the copy and every point of the copy with the
//! nearest point of the placed original (both ways, so the original can neither shrink onto a part
//! of the copy nor grow past it), leaves out pairs much farther apart than most (a stray mark, an
//! overshoot) and solves the similarity those pairs ask (scale, turn and shift, Procrustes in
//! closed form). The turn is fitted only so the pairs are right and the scale is the size alone
//! (fitting a scale without the turn would shrink the original under any copy drawn at another
//! angle); the result drops it: the scale, and the shift that lays the pairs' centre of the
//! original on theirs of the copy. A copy in any number of pieces works; one with no length gives
//! no fit.
//!
//! A group may be compared before all its lines are drawn (its key lines first). Fitted whole, the
//! undrawn lines would pull the original small onto the drawn ones; so the part of the original
//! fitted is the one that leaves the least between it and the copy, both ways (each point of
//! either to the other, on the mean): the whole group, the group without one line, or one line
//! alone. The lines drawn land on theirs, and the rest lie where those put them.
//!
//! Each side's lines are indexed once (`Grid`), so a nearest point is found among the few segments
//! round it rather than among all of them (S88: a group's fit took 0.4 s on the tablet, now about
//! 16 ms). The copy's raw pen samples are indexed as they are: resampled, their chords cut the
//! curve, and a known move and size would no longer come back exactly. The placed original's
//! nearest point to a copy's is the original's own nearest to that point taken back through the
//! placement, which a similarity keeps (distances only scale), so the original is indexed once,
//! not every round.

use crate::grid::Grid;
use crate::path::{Point, centre_and_spread, distance, resample};

/// Steps the two lines are compared at, in frame sides.
const SPACING: f64 = 0.01;
/// Rounds at most (choosing the part a copy holds, then fitting it), and the change (frame sides)
/// a round ends the fit under.
const SEARCH_ROUNDS: usize = 15;
const ROUNDS: usize = 40;
const SETTLED: f64 = 1e-10;
/// Pairs farther apart than this many times the median distance (and at least `LEAST_TRIM`
/// frame sides) are left out of a round.
const TRIM: f64 = 3.0;
const LEAST_TRIM: f64 = 0.02;
/// The scales a fit may take: a copy sized past these is shown at them.
const SCALES: [f64; 2] = [0.2, 5.0];

/// A translation and a uniform scale: `p` goes to `scale p + shift`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    pub scale: f64,
    pub shift: Point,
}

impl Fit {
    /// Leaves every point where it is.
    pub const SAME: Self = Self {
        scale: 1.0,
        shift: [0.0, 0.0],
    };

    #[must_use]
    pub fn apply(&self, p: Point) -> Point {
        [
            self.scale * p[0] + self.shift[0],
            self.scale * p[1] + self.shift[1],
        ]
    }
}

/// A round's placement: `p` goes to `scale R(turn) p + shift`, the turn as its cosine and sine.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Similarity {
    pub(crate) scale: f64,
    pub(crate) turn: [f64; 2],
    pub(crate) shift: Point,
}

impl Similarity {
    pub(crate) fn apply(&self, p: Point) -> Point {
        let [cos, sin] = self.turn;
        [
            self.scale * (cos * p[0] - sin * p[1]) + self.shift[0],
            self.scale * (sin * p[0] + cos * p[1]) + self.shift[1],
        ]
    }

    /// The point that `apply` takes to `q`.
    fn invert(&self, q: Point) -> Point {
        let [cos, sin] = self.turn;
        let [x, y] = [
            (q[0] - self.shift[0]) / self.scale,
            (q[1] - self.shift[1]) / self.scale,
        ];
        [cos * x + sin * y, -sin * x + cos * y]
    }

    pub(crate) fn moved(&self, other: &Self) -> f64 {
        (self.scale - other.scale).abs()
            + distance(self.turn, other.turn)
            + distance(self.shift, other.shift)
    }
}

/// One side of a fit: its lines, the points along them it is paired by, and the lines indexed for
/// nearest points.
struct Side<'a> {
    lines: &'a [&'a [Point]],
    points: Vec<Point>,
    grid: Grid,
}

impl<'a> Side<'a> {
    /// `lines` paired by points every `SPACING` along them.
    fn of(lines: &'a [&'a [Point]]) -> Self {
        let points = lines.iter().flat_map(|l| resample(l, SPACING)).collect();
        Self::with(lines, points)
    }

    fn with(lines: &'a [&'a [Point]], points: Vec<Point>) -> Self {
        Self {
            lines,
            points,
            grid: Grid::of(lines),
        }
    }
}

/// The fit that lays the lines of `original` together best over the lines of `copy`, or the part
/// of them the copy holds; `None` when either has no length.
#[must_use]
pub fn align(original: &[&[Point]], copy: &[&[Point]]) -> Option<Fit> {
    let part = held(original, copy);
    let lines: Vec<&[Point]> = part.iter().map(|&line| original[line]).collect();
    fitted(&Side::of(&lines), &Side::of(copy), ROUNDS)
}

/// Which of `original`'s lines the copy holds: of the parts tried, the one whose fit leaves the
/// least between it and the copy (the first of equals), looked at with the lines thinned to
/// `SPACING`; every line of one line or of no fit.
fn held(original: &[&[Point]], copy: &[&[Point]]) -> Vec<usize> {
    let every: Vec<usize> = (0..original.len()).collect();
    if original.len() < 2 {
        return every;
    }
    let thin = |lines: &[&[Point]]| -> Vec<Vec<Point>> {
        lines.iter().map(|line| resample(line, SPACING)).collect()
    };
    let (original, copy) = (thin(original), thin(copy));
    let copy_lines: Vec<&[Point]> = copy.iter().map(Vec::as_slice).collect();
    let copy = Side::with(&copy_lines, copy.concat());
    let tried = parts(original.len()).into_iter().filter_map(|part| {
        let lines: Vec<&[Point]> = part.iter().map(|&line| original[line].as_slice()).collect();
        let fit = fitted(&Side::of(&lines), &copy, SEARCH_ROUNDS)?;
        Some((mismatch(&lines, fit, &copy), part))
    });
    tried
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map_or(every, |(_, part)| part)
}

/// The parts of an original of `count` lines tried, by their lines, the largest first: all of it,
/// then without each line (from three lines on), then each line alone.
fn parts(count: usize) -> Vec<Vec<usize>> {
    let mut parts = vec![(0..count).collect::<Vec<usize>>()];
    if count > 2 {
        parts.extend((0..count).map(|out| (0..count).filter(|line| *line != out).collect()));
    }
    if count > 1 {
        parts.extend((0..count).map(|line| vec![line]));
    }
    parts
}

/// How far `lines` laid by `fit` and the copy lie from each other: the mean distance of the
/// copy's points to them and theirs to the copy.
fn mismatch(lines: &[&[Point]], fit: Fit, copy: &Side<'_>) -> f64 {
    let laid: Vec<Vec<Point>> = (lines.iter())
        .map(|line| line.iter().map(|p| fit.apply(*p)).collect())
        .collect();
    let laid_lines: Vec<&[Point]> = laid.iter().map(Vec::as_slice).collect();
    let laid_points: Vec<Point> = laid.iter().flat_map(|l| resample(l, SPACING)).collect();
    let mean = |points: &[Point], to: &Grid| {
        let gaps = points
            .iter()
            .filter_map(|p| to.nearest(*p).map(|near| near.distance));
        gaps.sum::<f64>() / points.len().max(1) as f64
    };
    mean(&copy.points, &Grid::of(&laid_lines)) + mean(&laid_points, &copy.grid)
}

/// The fit that lays the lines of `original` together best over the lines of `copy`, in at most
/// `rounds`; `None` when either has no length.
fn fitted(original: &Side<'_>, copy: &Side<'_>, rounds: usize) -> Option<Fit> {
    let (original_centre, original_spread) = centre_and_spread(original.lines)?;
    let (copy_centre, copy_spread) = centre_and_spread(copy.lines)?;
    if original_spread <= 0.0 || copy_spread <= 0.0 {
        return None;
    }
    let scale = (copy_spread / original_spread).clamp(SCALES[0], SCALES[1]);
    let mut placed = Similarity {
        scale,
        turn: [1.0, 0.0],
        shift: [
            copy_centre[0] - scale * original_centre[0],
            copy_centre[1] - scale * original_centre[1],
        ],
    };
    let mut fit = Fit {
        scale,
        shift: placed.shift,
    };
    for _ in 0..rounds {
        let pairs = trimmed(&placed, pairs(&placed, original, copy));
        let Some((next, unturned)) = solve(&pairs) else {
            break;
        };
        let moved = next.moved(&placed);
        (placed, fit) = (next, unturned);
        if moved < SETTLED {
            break;
        }
    }
    Some(fit)
}

/// Corresponding points `(original's, copy's)` under `placed`: each of the original's points with
/// the copy's nearest point to where it is placed, and each of the copy's points with the placed
/// original's nearest point, taken back to the original (the original's nearest to the copy's
/// point taken back: `placed` only scales distances).
fn pairs(placed: &Similarity, original: &Side<'_>, copy: &Side<'_>) -> Vec<(Point, Point)> {
    let mut pairs = Vec::with_capacity(original.points.len() + copy.points.len());
    for &o in &original.points {
        if let Some(near) = copy.grid.nearest(placed.apply(o)) {
            pairs.push((o, near.at));
        }
    }
    for &c in &copy.points {
        if let Some(near) = original.grid.nearest(placed.invert(c)) {
            pairs.push((near.at, c));
        }
    }
    pairs
}

/// `pairs` without those much farther apart under `placed` than most.
fn trimmed(placed: &Similarity, pairs: Vec<(Point, Point)>) -> Vec<(Point, Point)> {
    let gap = |(o, c): &(Point, Point)| distance(placed.apply(*o), *c);
    let mut gaps: Vec<f64> = pairs.iter().map(gap).collect();
    gaps.sort_by(f64::total_cmp);
    let Some(&median) = gaps.get(gaps.len() / 2) else {
        return pairs;
    };
    let limit = (TRIM * median).max(LEAST_TRIM);
    pairs
        .into_iter()
        .filter(|pair| gap(pair) <= limit)
        .collect()
}

/// The similarity that takes the first points of `pairs` nearest the second by least squares,
/// and the same without its turn (the centres laid on each other); `None` when the first points
/// do not spread.
pub(crate) fn solve(pairs: &[(Point, Point)]) -> Option<(Similarity, Fit)> {
    if pairs.is_empty() {
        return None;
    }
    let count = pairs.len() as f64;
    let mean = |pick: fn(&(Point, Point)) -> Point| {
        let sum = pairs
            .iter()
            .map(pick)
            .fold([0.0, 0.0], |s, p| [s[0] + p[0], s[1] + p[1]]);
        [sum[0] / count, sum[1] / count]
    };
    let (o, c) = (mean(|pair| pair.0), mean(|pair| pair.1));
    let (mut dot, mut cross, mut spread) = (0.0, 0.0, 0.0);
    for (p, q) in pairs {
        let (dp, dq) = ([p[0] - o[0], p[1] - o[1]], [q[0] - c[0], q[1] - c[1]]);
        dot += dp[0] * dq[0] + dp[1] * dq[1];
        cross += dp[0] * dq[1] - dp[1] * dq[0];
        spread += dp[0] * dp[0] + dp[1] * dp[1];
    }
    let along = dot.hypot(cross);
    if spread <= 0.0 || along <= 0.0 {
        return None;
    }
    let scale = (along / spread).clamp(SCALES[0], SCALES[1]);
    let turn = [dot / along, cross / along];
    let mut placed = Similarity {
        scale,
        turn,
        shift: [0.0, 0.0],
    };
    let turned = placed.apply(o);
    placed.shift = [c[0] - turned[0], c[1] - turned[1]];
    let fit = Fit {
        scale,
        shift: [c[0] - scale * o[0], c[1] - scale * o[1]],
    };
    Some((placed, fit))
}
