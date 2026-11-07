//! Polylines in square cells, for the fits' nearest-point lookups (`align.rs`, `align_shape.rs`):
//! each segment (a lone point too) is kept in every cell its box touches, so a lookup looks only
//! at the rings of cells round its point, until a ring lies farther than the nearest found, however
//! long the lines are and however densely the pen sampled them. The nearest point is the exact
//! one `path::nearest` finds over every segment.

use crate::maths::path::{Point, distance, nearest_on_segment};

/// A cell's side, in frame sides.
const CELL: f64 = 0.02;
/// Cells along a side at most.
const MOST_CELLS: usize = 4096;

/// The point of the lines nearest a point asked: where, how far, and which line holds it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Near {
    pub(crate) at: Point,
    pub(crate) distance: f64,
    pub(crate) line: usize,
}

/// A segment of a line (a lone point's ends are the point twice).
#[derive(Debug, Clone, Copy)]
struct Segment {
    a: Point,
    b: Point,
    line: usize,
}

/// The nearest found so far, by its squared distance.
#[derive(Debug, Clone, Copy)]
struct Best {
    at: Point,
    squared: f64,
    line: usize,
}

/// Lines (polylines; a lone point counts) indexed by cell.
pub(crate) struct Grid {
    segments: Vec<Segment>,
    low: Point,
    across: usize,
    down: usize,
    /// Each cell's segments, as indices into `segments`: cell `k`'s are
    /// `held[starts[k]..starts[k + 1]]`.
    starts: Vec<usize>,
    held: Vec<usize>,
}

impl Grid {
    pub(crate) fn of(lines: &[&[Point]]) -> Self {
        let segments: Vec<Segment> = (lines.iter().enumerate())
            .flat_map(|(line, points)| {
                let last = points.len().saturating_sub(1);
                let starts = if points.len() == 1 { 0..1 } else { 0..last };
                starts.map(move |k| Segment {
                    a: points[k],
                    b: points[(k + 1).min(last)],
                    line,
                })
            })
            .collect();
        let corners = segments.iter().flat_map(|s| [s.a, s.b]);
        let (low, high) = bounds(&corners.collect::<Vec<Point>>()).unwrap_or_default();
        let cells_of = |extent: f64| ((extent / CELL).floor() as usize + 1).min(MOST_CELLS);
        let (across, down) = (cells_of(high[0] - low[0]), cells_of(high[1] - low[1]));
        let mut grid = Self {
            segments,
            low,
            across,
            down,
            starts: Vec::new(),
            held: Vec::new(),
        };
        // Each segment's cells, counted, then laid out cell by cell.
        let mut placed: Vec<(usize, usize)> = Vec::new();
        for (index, s) in grid.segments.iter().enumerate() {
            let (x0, y0) = grid.cell([s.a[0].min(s.b[0]), s.a[1].min(s.b[1])]);
            let (x1, y1) = grid.cell([s.a[0].max(s.b[0]), s.a[1].max(s.b[1])]);
            for y in y0..=y1 {
                placed.extend((x0..=x1).map(|x| (y * across + x, index)));
            }
        }
        placed.sort_unstable();
        grid.starts = vec![0; across * down + 1];
        for &(cell, _) in &placed {
            grid.starts[cell + 1] += 1;
        }
        for cell in 0..across * down {
            grid.starts[cell + 1] += grid.starts[cell];
        }
        grid.held = placed.into_iter().map(|(_, index)| index).collect();
        grid
    }

    /// How far `p` lies from cell `(x, y)`, squared (the last cells reaching on without end: they
    /// hold what lies beyond `MOST_CELLS`).
    fn apart(&self, p: Point, (x, y): (usize, usize)) -> f64 {
        let gap = |v: f64, low: f64, (at, cells): (usize, usize)| {
            let from = low + at as f64 * CELL;
            let to = if at + 1 == cells {
                f64::INFINITY
            } else {
                from + CELL
            };
            (from - v).max(v - to).max(0.0)
        };
        let dx = gap(p[0], self.low[0], (x, self.across));
        let dy = gap(p[1], self.low[1], (y, self.down));
        dx * dx + dy * dy
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

    /// The point of the lines nearest `p`; `None` when they hold no point. Rings of cells round
    /// `p`'s are looked at until one lies farther than the nearest found (a point in a cell `r`
    /// rings away lies at least `r - 1` cells from `p`, inside the grid or clamped onto it), and
    /// of a ring only the cells nearer than it; a point far from every line, whose rings would
    /// pass more cells than there are segments, looks at each segment once instead.
    pub(crate) fn nearest(&self, p: Point) -> Option<Near> {
        let (cx, cy) = self.cell(p);
        let [cx, cy] = [cx, cy].map(|v| v as isize);
        let [across, down] = [self.across, self.down].map(|v| v as isize);
        let mut best: Option<Best> = None;
        let mut looked = 0;
        for ring in 0..across.max(down) {
            let within = (ring - 1).max(0) as f64 * CELL;
            if best.is_some_and(|best| best.squared < within * within) {
                break;
            }
            if looked > self.segments.len() {
                best = None;
                for index in 0..self.segments.len() {
                    self.closer(index, p, &mut best);
                }
                break;
            }
            for y in (cy - ring).max(0)..=(cy + ring).min(down - 1) {
                // A ring's top and bottom rows whole, of the rows between their two ends.
                let step = if (y - cy).abs() == ring { 1 } else { 2 * ring };
                let mut x = cx - ring;
                while x <= cx + ring {
                    if (0..across).contains(&x) {
                        looked += 1;
                        self.look(p, (x as usize, y as usize), &mut best);
                    }
                    x += step;
                }
            }
        }
        best.map(|best| Near {
            at: best.at,
            distance: distance(best.at, p),
            line: best.line,
        })
    }

    /// The segments of cell `(x, y)` nearer `p` than `best`, into it, unless the cell lies
    /// farther.
    fn look(&self, p: Point, (x, y): (usize, usize), best: &mut Option<Best>) {
        if best.is_some_and(|best| self.apart(p, (x, y)) >= best.squared) {
            return;
        }
        let cell = y * self.across + x;
        for &index in &self.held[self.starts[cell]..self.starts[cell + 1]] {
            self.closer(index, p, best);
        }
    }

    /// Segment `index`, into `best` where its point nearest `p` is nearer.
    fn closer(&self, index: usize, p: Point, best: &mut Option<Best>) {
        let Segment { a, b, line } = self.segments[index];
        let at = nearest_on_segment(a, b, p);
        let squared = (at[0] - p[0]).powi(2) + (at[1] - p[1]).powi(2);
        if best.is_none_or(|best| squared < best.squared) {
            *best = Some(Best { at, squared, line });
        }
    }
}

/// The least and greatest corner of `points`; `None` for none.
pub(crate) fn bounds(points: &[Point]) -> Option<(Point, Point)> {
    let first = *points.first()?;
    Some(points.iter().fold((first, first), |(low, high), p| {
        (
            [low[0].min(p[0]), low[1].min(p[1])],
            [high[0].max(p[0]), high[1].max(p[1])],
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maths::path::nearest;

    #[test]
    fn the_nearest_point_is_the_one_every_segment_gives() {
        // A wave of short segments, a long diagonal, a lone point and an empty line.
        let wave: Vec<Point> = (0..=200)
            .map(|k| {
                let x = f64::from(k) / 200.0;
                [x, 0.5 + 0.2 * (x * 9.0).sin()]
            })
            .collect();
        let diagonal: &[Point] = &[[0.1, 0.9], [0.9, 0.1]];
        let lone: &[Point] = &[[0.5, 0.05]];
        let lines: Vec<&[Point]> = vec![&wave, diagonal, lone, &[]];
        let grid = Grid::of(&lines);
        for k in 0..400 {
            let t = f64::from(k);
            // Inside the lines' box and well outside it.
            let p = [1.6 * (t * 0.37).sin() + 0.5, 1.6 * (t * 0.61).cos() + 0.5];
            let (at, d) = nearest(&lines, p).expect("a point");
            let near = grid.nearest(p).expect("a point");
            assert!(
                (near.distance - d).abs() < 1e-12,
                "{p:?}: {near:?} against {at:?}"
            );
        }
        let on_lone = grid.nearest([0.5, 0.0]).expect("a point");
        assert_eq!((on_lone.at, on_lone.line), ([0.5, 0.05], 2));
        assert!(Grid::of(&[]).nearest([0.0, 0.0]).is_none());
    }
}
