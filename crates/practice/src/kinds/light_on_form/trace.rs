//! Where light on form's families meet, as the eye sees it (S153): the terminator on the form and
//! the cast shadow's outline on the ground, traced from the model. The frame is looked at on a grid
//! (`GRID` steps across); where two neighbouring points of the form, or two of the ground, lie in
//! different families, the point between where they part is found to the last bits by halving; the
//! points each square of the grid holds are joined (marching squares), and joined pieces run on
//! into lines. Where the form meets the ground nothing is traced: the form's outline is printed.

use std::collections::HashMap;

use crate::kinds::light_on_form::answer::{Mass, Shading};
use crate::kinds::light_on_form::scene::LightTask;
use crate::maths::path::Point;

/// Steps of the grid across the frame, and halvings that find where two families part.
const GRID: usize = 160;
const HALVINGS: usize = 24;

/// Where the families meet: on the form (the terminator) and on the ground (the cast shadow's
/// outline), each a list of lines in frame units.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Parting {
    pub terminator: Vec<Vec<Point>>,
    pub cast: Vec<Vec<Point>>,
}

/// A grid point as the model paints it: whether it lies on the form, and whether in the light.
type Looked = Option<(bool, bool)>;

impl LightTask {
    /// Where the families meet, traced from the model.
    #[must_use]
    pub fn parting(&self) -> Parting {
        let side = GRID + 1;
        let node = |i: usize, j: usize| [i as f64 / GRID as f64, j as f64 / GRID as f64];
        let looked: Vec<Looked> = (0..side * side)
            .map(|at| self.shading(node(at % side, at / side)).map(looked))
            .collect();
        let at = |i: usize, j: usize| looked[j * side + i];
        // The crossing on each grid edge where two points of one kind part: by the edge's id.
        let mut crossings: HashMap<usize, (Point, bool)> = HashMap::new();
        for j in 0..side {
            for i in 0..side {
                let here = at(i, j);
                for (across, [ni, nj]) in [(true, [i + 1, j]), (false, [i, j + 1])] {
                    if ni >= side || nj >= side {
                        continue;
                    }
                    let (Some((form, lit)), Some((other_form, other_lit))) = (here, at(ni, nj))
                    else {
                        continue;
                    };
                    if form == other_form && lit != other_lit {
                        let found = self.parted(node(i, j), node(ni, nj), lit);
                        crossings.insert(edge(side, [i, j], across), (found, form));
                    }
                }
            }
        }
        let pieces = squares(side, &crossings);
        let mut parting = Parting::default();
        for (line, form) in joined(&pieces, &crossings) {
            if form {
                parting.terminator.push(line);
            } else {
                parting.cast.push(line);
            }
        }
        parting
    }

    /// Where the family changes between `a` (lit when `lit`) and `b`, found by halving.
    fn parted(&self, mut a: Point, mut b: Point, lit: bool) -> Point {
        for _ in 0..HALVINGS {
            let middle = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            let there = self.shading(middle).map(|s| s.family == Mass::Light);
            if there == Some(lit) {
                a = middle;
            } else {
                b = middle;
            }
        }
        [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
    }
}

/// A shading as a grid point holds it.
const fn looked(shading: Shading) -> (bool, bool) {
    (shading.on_form, matches!(shading.family, Mass::Light))
}

/// The id of the grid edge from `[i, j]` across (to `[i + 1, j]`) or down.
const fn edge(side: usize, [i, j]: [usize; 2], across: bool) -> usize {
    2 * (j * side + i) + if across { 0 } else { 1 }
}

/// The pieces each square of the grid holds: its crossings of the form, and those of the ground,
/// joined two by two (round the square: top, right, bottom, left), an odd one left out.
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
            for form in [true, false] {
                let held: Vec<usize> = (round.into_iter())
                    .filter(|id| crossings.get(id).is_some_and(|(_, on)| *on == form))
                    .collect();
                pieces.extend(held.as_chunks::<2>().0);
            }
        }
    }
    pieces
}

/// `pieces` (pairs of crossings) run on into lines through their crossings' points, each with
/// whether it lies on the form.
fn joined(
    pieces: &[[usize; 2]],
    crossings: &HashMap<usize, (Point, bool)>,
) -> Vec<(Vec<Point>, bool)> {
    let mut by_crossing: HashMap<usize, Vec<usize>> = HashMap::new();
    for (k, [a, b]) in pieces.iter().enumerate() {
        by_crossing.entry(*a).or_default().push(k);
        by_crossing.entry(*b).or_default().push(k);
    }
    let mut used = vec![false; pieces.len()];
    let mut lines = Vec::new();
    // Ends first (a crossing one piece holds), so an open line is walked from its end.
    let mut starts: Vec<usize> = (0..pieces.len()).collect();
    starts.sort_by_key(|k| {
        let [a, b] = pieces[*k];
        let ends = [a, b]
            .iter()
            .filter(|id| by_crossing[id].len() == 1)
            .count();
        std::cmp::Reverse(ends)
    });
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
        let form = crossings[&ids[0]].1;
        lines.push((ids.iter().map(|id| crossings[id].0).collect(), form));
    }
    lines
}
