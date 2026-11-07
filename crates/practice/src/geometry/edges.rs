//! Straight edges found in a hand's lines (S84): what a perspective check extends. Each line is
//! read at even steps and split where it turns (Douglas–Peucker: where it leaves the chord between
//! its ends by more than `SPLIT`), neighbouring runs that turn but a little joined again while
//! together still straight, and each run fitted by a straight line (total least squares) and kept
//! only if it is straight along its whole length: a curve is no edge. Strokes along one edge are
//! one edge (S97): drawn in pieces, fitted to all their points; drawn again over it (a redraw, the
//! two overlapping over half the shorter), the later stroke's line, as a correction counts, over
//! the reach of both. A hand redraws an edge a few pixels and a degree or two off, each stroke a
//! degree or two off its own aim besides (the author's redrawn edges read as two while the
//! shorter's ends had to lie within 4 pixels of the longer's line): two lines are one edge when
//! within `DOUBLE_ANGLE` of each other and the shorter's middle near the longer's line, a tenth of
//! its length (at most two pen widths), under a face's least width (`perspective.rs`'s `FACE`, a
//! small free box's faces a fifth of its edges). An edge's ends are where its first and last
//! points fall on its line, so an overshoot past a corner stays part of the edge it continues.
//! Edges shorter than `SHORTEST` are left out, and nothing is guessed: a run that is no straight
//! line shows nothing.
//!
//! Frame units (a frame's side is 1). How reliable it is on hand-drawn boxes: `tests/edges.rs`.

use crate::maths::path::{Point, distance, length, resample};

/// The even step lines are read at: 1.6 pixels of a 400-pixel frame.
const SPACING: f64 = 0.004;
/// A line is split where it leaves the chord between its ends by more than this.
const SPLIT: f64 = 0.008;
/// Runs meeting at less than this turn (degrees) join, if together still straight.
const JOIN_TURN: f64 = 12.0;
/// A run between two turns each less than this (degrees) lies on a curve: going round a box, at
/// least every other corner turns more, and a straight line's pieces have joined.
const SMOOTH: f64 = 60.0;
/// A run is straight when no point leaves its fitted line by more than this, or this share of
/// its length, whichever is more: a hand's tremor and bow, never a curve's bend.
const STRAIGHT: f64 = 0.006;
const STRAIGHT_SHARE: f64 = 0.03;
/// Two edges are one drawn twice or in pieces when they lie within this angle (degrees: a redraw's
/// 3 and each stroke's own 2), the shorter's middle within `DOUBLE_APART_SHARE` of the longer's
/// length of its line (kept within `DOUBLE_APART`, 3 to 10 pixels; a small box's faces are some
/// fifth of its edges' length wide, a large one's 20 pixels at least), and they overlap or leave
/// at most `DOUBLE_GAP` between them along it; a redraw when they overlap over `REDRAW_SHARE` of
/// the shorter's length.
const DOUBLE_ANGLE: f64 = 7.0;
const DOUBLE_APART: [f64; 2] = [0.008, 0.025];
const DOUBLE_APART_SHARE: f64 = 0.1;
const DOUBLE_GAP: f64 = 0.03;
const REDRAW_SHARE: f64 = 0.5;
/// A correction redraws an edge it runs within this angle (degrees) of and this near.
const REDRAWN_ANGLE: f64 = 10.0;
const REDRAWN_APART: f64 = 0.03;
/// The shortest edge kept: 20 pixels of a 400-pixel frame.
pub const SHORTEST: f64 = 0.05;
/// The shortest run looked at.
const SHORTEST_RUN: f64 = 0.02;

/// A straight edge found in the copy's lines: its two ends, in the order it was drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub ends: [Point; 2],
}

impl Edge {
    #[must_use]
    pub fn length(&self) -> f64 {
        distance(self.ends[0], self.ends[1])
    }

    #[must_use]
    pub fn middle(&self) -> Point {
        let [a, b] = self.ends;
        [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
    }

    /// The unit direction from its first end to its last.
    #[must_use]
    pub fn direction(&self) -> Point {
        let [a, b] = self.ends;
        let long = self.length().max(f64::MIN_POSITIVE);
        [(b[0] - a[0]) / long, (b[1] - a[1]) / long]
    }
}

/// A run of points, the straight line fitted to it and when its latest stroke was drawn (runs
/// are numbered in the order drawn).
#[derive(Debug, Clone)]
struct Run {
    points: Vec<Point>,
    line: Fitted,
    order: usize,
}

impl Run {
    /// Its line's two ends.
    fn ends(&self) -> [Point; 2] {
        self.line.extent.map(|t| self.line.at(t))
    }
}

/// A fitted line: a point on it (the points' centre), its unit direction (along the points'
/// order), the points' extent along it and how far the farthest leaves it.
#[derive(Debug, Clone, Copy)]
struct Fitted {
    centre: Point,
    along: Point,
    extent: [f64; 2],
    off: f64,
}

impl Fitted {
    fn length(&self) -> f64 {
        self.extent[1] - self.extent[0]
    }

    fn straight(&self) -> bool {
        self.off <= STRAIGHT.max(STRAIGHT_SHARE * self.length())
    }

    fn at(&self, t: f64) -> Point {
        [
            self.centre[0] + t * self.along[0],
            self.centre[1] + t * self.along[1],
        ]
    }

    /// How far along it, and how far off it, `p` lies.
    fn place(&self, p: Point) -> (f64, f64) {
        let d = [p[0] - self.centre[0], p[1] - self.centre[1]];
        let t = d[0] * self.along[0] + d[1] * self.along[1];
        let off = d[0] * self.along[1] - d[1] * self.along[0];
        (t, off.abs())
    }
}

/// The straight edges in `lines` (polylines in frame units), longest first.
#[must_use]
pub fn edges(lines: &[&[Point]]) -> Vec<Edge> {
    let mut runs: Vec<Run> = Vec::new();
    for line in lines {
        let even = resample(line, SPACING);
        if length(&even) < SHORTEST_RUN {
            continue;
        }
        runs.extend(split(&even).into_iter().filter(|run| run.line.straight()));
    }
    for (order, run) in runs.iter_mut().enumerate() {
        run.order = order;
    }
    join_doubles(&mut runs);
    let mut found: Vec<Edge> = (runs.iter())
        .map(|run| Edge { ends: run.ends() })
        .filter(|edge| edge.length() >= SHORTEST)
        .collect();
    found.sort_by(|a, b| b.length().total_cmp(&a.length()));
    found
}

/// The edges of a copy corrected after a Compare: the `corrections` and the `earlier` edges none
/// of them redraws. A correction redraws an edge when it runs within `REDRAWN_ANGLE` of it, its
/// middle within `REDRAWN_APART` of the edge's line, over at least half the shorter's length:
/// the latest try counts, not one averaged with the first.
#[must_use]
pub fn corrected(earlier: Vec<Edge>, corrections: Vec<Edge>) -> Vec<Edge> {
    let redraws = |c: &Edge, e: &Edge| {
        let (d, m) = (e.direction(), c.middle());
        let along = |p: Point| (p[0] - e.ends[0][0]) * d[0] + (p[1] - e.ends[0][1]) * d[1];
        let off = ((m[0] - e.ends[0][0]) * d[1] - (m[1] - e.ends[0][1]) * d[0]).abs();
        let (a, b) = (along(c.ends[0]), along(c.ends[1]));
        let shared = a.max(b).min(e.length()) - a.min(b).max(0.0);
        let angle = turn(c.direction(), d);
        angle.min(180.0 - angle) <= REDRAWN_ANGLE
            && off <= REDRAWN_APART
            && shared >= 0.5 * c.length().min(e.length())
    };
    let kept = (earlier.into_iter()).filter(|e| !corrections.iter().any(|c| redraws(c, e)));
    corrections.iter().copied().chain(kept).collect()
}

/// `points`'s least-squares line; none for fewer than two points or no extent.
fn fit(points: &[Point]) -> Option<Fitted> {
    let (first, last) = (*points.first()?, *points.last()?);
    let n = points.len() as f64;
    let centre = [0, 1].map(|k| points.iter().map(|p| p[k]).sum::<f64>() / n);
    let [mut xx, mut xy, mut yy] = [0.0; 3];
    for p in points {
        let [x, y] = [p[0] - centre[0], p[1] - centre[1]];
        (xx, xy, yy) = (xx + x * x, xy + x * y, yy + y * y);
    }
    if xx + yy <= 0.0 {
        return None;
    }
    let angle = 0.5 * (2.0 * xy).atan2(xx - yy);
    let mut along = [angle.cos(), angle.sin()];
    let way = [last[0] - first[0], last[1] - first[1]];
    if way[0] * along[0] + way[1] * along[1] < 0.0 {
        along = [-along[0], -along[1]];
    }
    let mut line = Fitted {
        centre,
        along,
        extent: [0.0; 2],
        off: 0.0,
    };
    let (start, _) = line.place(first);
    let (end, _) = line.place(last);
    let off = points.iter().map(|p| line.place(*p).1).fold(0.0, f64::max);
    line.extent = [start.min(end), start.max(end)];
    line.off = off;
    Some(line)
}

/// `points` (evenly spaced) split where they turn, the runs that turn but a little joined again.
fn split(points: &[Point]) -> Vec<Run> {
    let mut breaks = vec![0, points.len() - 1];
    let mut open = vec![(0, points.len() - 1)];
    while let Some((from, to)) = open.pop() {
        let chord = Edge {
            ends: [points[from], points[to]],
        };
        let farthest = (from + 1..to)
            .map(|i| (i, off_chord(&chord, points[i])))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((at, off)) = farthest
            && off > SPLIT
        {
            breaks.push(at);
            open.extend([(from, at), (at, to)]);
        }
    }
    breaks.sort_unstable();
    breaks.dedup();
    let mut runs: Vec<Vec<Point>> = (breaks.windows(2))
        .map(|pair| points[pair[0]..=pair[1]].to_vec())
        .collect();
    join_turns(&mut runs);
    let runs: Vec<Run> = (runs.into_iter())
        .filter_map(|points| {
            fit(&points).map(|line| Run {
                points,
                line,
                order: 0,
            })
        })
        .collect();
    let curve = on_curves(&runs);
    (runs.into_iter().zip(curve))
        .filter(|(run, curve)| !curve && run.line.length() >= SHORTEST_RUN)
        .map(|(run, _)| run)
        .collect()
}

/// Which of a line's runs lie on a curve: between two turns each less than `SMOOTH`; and an end
/// run turning on so into a run on a curve.
fn on_curves(runs: &[Run]) -> Vec<bool> {
    let turns: Vec<f64> = (runs.windows(2))
        .map(|pair| turn(pair[0].line.along, pair[1].line.along))
        .collect();
    let smooth = |a: f64, b: f64| a < SMOOTH && b < SMOOTH;
    let n = runs.len();
    let mut curve: Vec<bool> = (0..n)
        .map(|i| i > 0 && i + 1 < n && smooth(turns[i - 1], turns[i]))
        .collect();
    if n >= 3 {
        curve[0] = curve[1] && turns[0] < SMOOTH;
        curve[n - 1] = curve[n - 2] && turns[n - 2] < SMOOTH;
    }
    curve
}

/// How far `p` lies from the chord's line (its first end when it has no length).
fn off_chord(chord: &Edge, p: Point) -> f64 {
    let [a, b] = chord.ends;
    let long = chord.length();
    if long <= 0.0 {
        return distance(a, p);
    }
    ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])).abs() / long
}

/// Neighbouring runs that turn less than `JOIN_TURN` joined while together straight.
fn join_turns(runs: &mut Vec<Vec<Point>>) {
    let mut at = 0;
    while at + 1 < runs.len() {
        let (a, b) = (fit(&runs[at]), fit(&runs[at + 1]));
        let turn = match (a, b) {
            (Some(a), Some(b)) => turn(a.along, b.along),
            _ => 0.0,
        };
        let mut joined = runs[at].clone();
        joined.extend_from_slice(&runs[at + 1][1..]);
        if turn < JOIN_TURN && fit(&joined).is_some_and(|line| line.straight()) {
            runs[at] = joined;
            runs.remove(at + 1);
            at = at.saturating_sub(1);
        } else {
            at += 1;
        }
    }
}

/// The turn from direction `a` to `b`, degrees (0 to 180).
fn turn(a: Point, b: Point) -> f64 {
    let dot = a[0] * b[0] + a[1] * b[1];
    let cross = a[0] * b[1] - a[1] * b[0];
    cross.abs().atan2(dot).to_degrees()
}

/// Strokes along one edge joined into one: drawn twice, retraced or in pieces.
fn join_doubles(runs: &mut Vec<Run>) {
    'joining: loop {
        for i in 0..runs.len() {
            for j in i + 1..runs.len() {
                if let Some(run) = joined(&runs[i], &runs[j]) {
                    runs[i] = run;
                    runs.remove(j);
                    continue 'joining;
                }
            }
        }
        return;
    }
}

/// `a` and `b` as one edge, if they are strokes along one: a redraw, the later's line over the
/// reach of both; pieces, fitted together.
fn joined(a: &Run, b: &Run) -> Option<Run> {
    let (long, short) = if a.line.length() >= b.line.length() {
        (a, b)
    } else {
        (b, a)
    };
    let overlap = doubled(&long.line, &short.line)?;
    let (earlier, later) = if a.order <= b.order { (a, b) } else { (b, a) };
    let reach = [earlier.ends(), later.ends()];
    let (points, way) = if overlap >= REDRAW_SHARE * short.line.length() {
        (later.points.clone(), later.line.along)
    } else {
        let mut points = earlier.points.clone();
        points.extend_from_slice(&later.points);
        (points, long.line.along)
    };
    let mut line = fit_along(&points, way)?;
    for end in reach.iter().flatten() {
        let (t, _) = line.place(*end);
        line.extent = [line.extent[0].min(t), line.extent[1].max(t)];
    }
    Some(Run {
        points,
        line,
        order: later.order,
    })
}

/// `points`' line, oriented along `way`, its extent over every point.
fn fit_along(points: &[Point], way: Point) -> Option<Fitted> {
    let mut line = fit(points)?;
    if line.along[0] * way[0] + line.along[1] * way[1] < 0.0 {
        line.along = [-line.along[0], -line.along[1]];
    }
    let placed: Vec<(f64, f64)> = points.iter().map(|p| line.place(*p)).collect();
    let low = placed.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let high = placed.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    line.extent = [low, high];
    line.off = placed.iter().map(|p| p.1).fold(0.0, f64::max);
    Some(line)
}

/// How far the lines `long` and `short` (the shorter) overlap along `long` (less than zero: the
/// gap between them), when they are one edge drawn twice or in pieces; none when they are not.
fn doubled(long: &Fitted, short: &Fitted) -> Option<f64> {
    let angle = turn(long.along, short.along);
    if angle.min(180.0 - angle) > DOUBLE_ANGLE {
        return None;
    }
    let (_, off) = long.place(short.at((short.extent[0] + short.extent[1]) / 2.0));
    let apart = (DOUBLE_APART_SHARE * long.length()).clamp(DOUBLE_APART[0], DOUBLE_APART[1]);
    if off > apart {
        return None;
    }
    let ends = short.extent.map(|t| long.place(short.at(t)).0);
    let (low, high) = (ends[0].min(ends[1]), ends[0].max(ends[1]));
    let overlap = high.min(long.extent[1]) - low.max(long.extent[0]);
    (overlap >= -DOUBLE_GAP).then_some(overlap)
}
