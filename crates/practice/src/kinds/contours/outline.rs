//! A sausage's outline (S156k), traced once from its picture: printed grey in the copy's frame, a
//! given, not an answer. The frame is looked at on a grid (`GRID` steps across); where two
//! neighbouring points differ (the paper and the tube, or two places of the tube whose depths jump,
//! a nearer part or a bend's near side hiding what lies behind it), the point between where they
//! part is found by halving; the points each square of the grid holds are joined two by two, each
//! kind of edge apart (marching squares), and the pieces run on into lines.

use std::collections::HashMap;

use crate::kinds::contours::sausage::Sausage;
use crate::kinds::solids::solid_answer::View;
use crate::maths::path::Point;
use crate::maths::vector::{Vector, norm, sub, times};

/// Steps of the grid across the frame, and halvings that find where two points part.
const GRID: usize = 200;
const HALVINGS: usize = 30;
/// Neighbouring points of the grid whose surfaces lie this many radii apart are two places, one
/// hiding the other (a grid step on one surface, grazing it at its edge, moves half a radius).
const JUMP: f64 = 1.0;

/// What the eye sees at a grid point: the paper, or the point of the tube it meets first.
type Looked = Option<Vector>;

/// The sausage seen through a view, as the trace looks at it.
struct Seen<'a> {
    sausage: &'a Sausage,
    view: View,
}

impl Seen<'_> {
    fn looked(&self, p: Point) -> Looked {
        let x = (p[0] - self.view.offset[0]) / self.view.scale;
        let y = (p[1] - self.view.offset[1]) / self.view.scale;
        let d = [x, -y, 1.0];
        (self.sausage.cast(([0.0; 3], d))).map(|(hit, _)| times(d, hit.t))
    }

    /// Whether `a` and `b` lie on two sides of an outline: against the paper (true), or across a
    /// jump in depth (false).
    fn parted(&self, a: Looked, b: Looked) -> Option<bool> {
        match (a, b) {
            (None, None) => None,
            (Some(p), Some(q)) => (norm(sub(p, q)) > JUMP * self.sausage.radius).then_some(false),
            _ => Some(true),
        }
    }

    /// Where `a` (seen as `at_a`) parts from `b` (seen as `at_b`), found by halving: a point is
    /// on `a`'s side where it sees the paper as `a` does, or a surface nearer `a`'s than `b`'s.
    fn parting(&self, (mut a, at_a): (Point, Looked), (mut b, at_b): (Point, Looked)) -> Point {
        let alike = |looked: Looked| match (looked, at_a, at_b) {
            (Some(m), Some(p), Some(q)) => norm(sub(m, p)) <= norm(sub(m, q)),
            (looked, at_a, _) => looked.is_some() == at_a.is_some(),
        };
        for _ in 0..HALVINGS {
            let middle = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            if alike(self.looked(middle)) {
                a = middle;
            } else {
                b = middle;
            }
        }
        [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
    }
}

/// `sausage`'s outline through `view` within `[low, high]` (frame units, what it may reach):
/// lines in frame units, its edge against the paper and where it hides itself.
pub(crate) fn outline(sausage: &Sausage, view: View, [low, high]: [Point; 2]) -> Vec<Vec<Point>> {
    let seen = Seen { sausage, view };
    let side = GRID + 1;
    let node = |i: usize, j: usize| [i as f64 / GRID as f64, j as f64 / GRID as f64];
    let within = |p: Point| (0..2).all(|k| (low[k]..=high[k]).contains(&p[k]));
    let looked: Vec<Looked> = (0..side * side)
        .map(|at| {
            let p = node(at % side, at / side);
            if within(p) { seen.looked(p) } else { None }
        })
        .collect();
    let at = |i: usize, j: usize| looked[j * side + i];
    // The crossing on each grid edge where two points part, by the edge's id, with its kind.
    let mut crossings: HashMap<usize, (Point, bool)> = HashMap::new();
    for j in 0..side {
        for i in 0..side {
            for (across, [ni, nj]) in [(true, [i + 1, j]), (false, [i, j + 1])] {
                if ni >= side || nj >= side {
                    continue;
                }
                if let Some(paper) = seen.parted(at(i, j), at(ni, nj)) {
                    let ends = ((node(i, j), at(i, j)), (node(ni, nj), at(ni, nj)));
                    let found = seen.parting(ends.0, ends.1);
                    crossings.insert(edge(side, [i, j], across), (found, paper));
                }
            }
        }
    }
    joined(&squares(side, &crossings), &crossings)
}

/// The id of the grid edge from `[i, j]` across (to `[i + 1, j]`) or down.
const fn edge(side: usize, [i, j]: [usize; 2], across: bool) -> usize {
    2 * (j * side + i) + if across { 0 } else { 1 }
}

/// The pieces each square of the grid holds: its crossings of each kind joined two by two, round
/// the square (top, right, bottom, left), an odd one left out.
fn squares(side: usize, crossings: &HashMap<usize, (Point, bool)>) -> Vec<[usize; 2]> {
    let mut pieces = Vec::new();
    for j in 0..side - 1 {
        for i in 0..side - 1 {
            let round = [
                edge(side, [i, j], true),
                edge(side, [i + 1, j], false),
                edge(side, [i, j + 1], true),
                edge(side, [i, j], false),
            ];
            for paper in [true, false] {
                let held: Vec<usize> = (round.into_iter())
                    .filter(|id| crossings.get(id).is_some_and(|(_, kind)| *kind == paper))
                    .collect();
                pieces.extend(held.as_chunks::<2>().0);
            }
        }
    }
    pieces
}

/// `pieces` (pairs of crossings) run on into lines through their crossings' points, an open line
/// from its end.
fn joined(pieces: &[[usize; 2]], crossings: &HashMap<usize, (Point, bool)>) -> Vec<Vec<Point>> {
    let mut by_crossing: HashMap<usize, Vec<usize>> = HashMap::new();
    for (k, [a, b]) in pieces.iter().enumerate() {
        by_crossing.entry(*a).or_default().push(k);
        by_crossing.entry(*b).or_default().push(k);
    }
    let ends = |k: &usize| {
        (pieces[*k].iter())
            .filter(|id| by_crossing[*id].len() == 1)
            .count()
    };
    let mut starts: Vec<usize> = (0..pieces.len()).collect();
    starts.sort_by_key(|k| std::cmp::Reverse(ends(k)));
    let mut used = vec![false; pieces.len()];
    let mut lines = Vec::new();
    for start in starts {
        if used[start] {
            continue;
        }
        used[start] = true;
        let [a, b] = pieces[start];
        let first = if by_crossing[&b].len() == 1 { b } else { a };
        let mut ids = vec![first, if first == a { b } else { a }];
        loop {
            let last = ids[ids.len() - 1];
            let next = (by_crossing[&last].iter()).find(|k| !used[**k]).copied();
            let Some(next) = next else { break };
            used[next] = true;
            let [p, q] = pieces[next];
            ids.push(if p == last { q } else { p });
        }
        lines.push(ids.iter().map(|id| crossings[id].0).collect());
    }
    lines
}
