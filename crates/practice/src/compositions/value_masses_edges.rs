//! An answer's mass edges (`value_masses.rs`), as Compare draws them red over the copy: wherever
//! two neighbouring cells hold different values, the side between them; the sides joined end to
//! end into lines, each running from where three masses meet (or the picture's edge) to the next,
//! or round a closed loop; then thinned within `THIN` cells (the cells' stair becomes the slant it
//! stands for, well inside a pen's width) and its corners rounded (Chaikin, `ROUNDS` times), so the
//! see-through red drawn along it never doubles over itself. In the picture's shares of its width
//! and height (0 to 1).

use crate::compositions::value_masses::ValueMasses;
use crate::maths::path::Point;
use crate::outlines::outline_trace::{off_segment, thinned};

/// How far a thinned edge may leave the cells' sides, in cells.
const THIN: f64 = 1.5;
/// Times a line's corners are cut.
const ROUNDS: usize = 3;

impl ValueMasses {
    /// The edges between its masses, in shares of the picture.
    #[must_use]
    pub fn edges(&self) -> Vec<Vec<Point>> {
        let [w, h] = self.size.map(|side| side as usize);
        let graph = Sides::of(self, w, h);
        let share = |corner: usize| {
            let (x, y) = (corner % (w + 1), corner / (w + 1));
            [x as f64 / w as f64, y as f64 / h as f64]
        };
        let cell = 1.0 / w.max(h) as f64;
        (graph.lines().into_iter())
            .map(|(corners, closed)| {
                let points: Vec<Point> = corners.into_iter().map(share).collect();
                let thin = if closed {
                    thinned(&points[..points.len() - 1], THIN * cell)
                } else {
                    thinned_open(&points, THIN * cell)
                };
                rounded(thin, closed)
            })
            .collect()
    }
}

/// The cells' sides between different values, as a graph on the cells' corners (corner `x, y` is
/// `y (w + 1) + x`).
struct Sides {
    /// Each side's two corners.
    ends: Vec<[usize; 2]>,
    /// Each corner's sides.
    at: Vec<Vec<usize>>,
}

impl Sides {
    fn of(masses: &ValueMasses, w: usize, h: usize) -> Self {
        let corner = |x: usize, y: usize| y * (w + 1) + x;
        let mut sides = Self {
            ends: Vec::new(),
            at: vec![Vec::new(); (w + 1) * (h + 1)],
        };
        for y in 0..h {
            for x in 0..w {
                let here = masses.value([x, y]);
                if x + 1 < w && masses.value([x + 1, y]) != here {
                    sides.add([corner(x + 1, y), corner(x + 1, y + 1)]);
                }
                if y + 1 < h && masses.value([x, y + 1]) != here {
                    sides.add([corner(x, y + 1), corner(x + 1, y + 1)]);
                }
            }
        }
        sides
    }

    fn add(&mut self, [a, b]: [usize; 2]) {
        let side = self.ends.len();
        self.ends.push([a, b]);
        self.at[a].push(side);
        self.at[b].push(side);
    }

    /// The sides joined into lines of corners, and whether each closes on itself: from every
    /// corner where other than two sides meet, along each side not walked yet, to the next such
    /// corner; then round the loops left.
    fn lines(&self) -> Vec<(Vec<usize>, bool)> {
        let mut walked = vec![false; self.ends.len()];
        let mut lines = Vec::new();
        let ends = (0..self.at.len()).filter(|c| !self.at[*c].is_empty() && self.at[*c].len() != 2);
        for corner in ends {
            for &side in &self.at[corner] {
                if !walked[side] {
                    lines.push((self.walk(corner, side, &mut walked), false));
                }
            }
        }
        for side in 0..self.ends.len() {
            if !walked[side] {
                let start = self.ends[side][0];
                lines.push((self.walk(start, side, &mut walked), true));
            }
        }
        lines
    }

    /// The corners from `corner` along `side` and on through corners where two sides meet, until
    /// another corner or back where it began.
    fn walk(&self, corner: usize, side: usize, walked: &mut [bool]) -> Vec<usize> {
        let mut line = vec![corner];
        let (mut here, mut along) = (corner, side);
        loop {
            walked[along] = true;
            let [a, b] = self.ends[along];
            here = if a == here { b } else { a };
            line.push(here);
            let next = (self.at[here].len() == 2)
                .then(|| self.at[here].iter().copied().find(|s| !walked[*s]))
                .flatten();
            match next {
                Some(next) => along = next,
                None => return line,
            }
        }
    }
}

/// An open line thinned to the points it needs within `tolerance`, its ends kept (Douglas and
/// Peucker).
fn thinned_open(line: &[Point], tolerance: f64) -> Vec<Point> {
    if line.len() < 3 {
        return line.to_vec();
    }
    let mut keep = vec![false; line.len()];
    let last = line.len() - 1;
    (keep[0], keep[last]) = (true, true);
    let mut spans = vec![(0, last)];
    while let Some((from, to)) = spans.pop() {
        let worst = (from + 1..to)
            .map(|i| (i, off_segment(line[i], line[from], line[to])))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((worst, _)) = worst.filter(|(_, gap)| *gap > tolerance) {
            keep[worst] = true;
            spans.push((from, worst));
            spans.push((worst, to));
        }
    }
    (line.iter().zip(keep))
        .filter_map(|(p, kept)| kept.then_some(*p))
        .collect()
}

/// `line` with its corners cut `ROUNDS` times (a quarter of each segment from either end), an open
/// one's ends kept; a closed one comes back closed (its first point again at its end).
fn rounded(mut line: Vec<Point>, closed: bool) -> Vec<Point> {
    let mix = |a: Point, b: Point, t: f64| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
    for _ in 0..ROUNDS {
        if line.len() < 3 {
            break;
        }
        let count = line.len();
        let segments = if closed { count } else { count - 1 };
        let mut cut = Vec::with_capacity(2 * segments + 2);
        if !closed {
            cut.push(line[0]);
        }
        for k in 0..segments {
            let (a, b) = (line[k], line[(k + 1) % count]);
            cut.push(mix(a, b, 0.25));
            cut.push(mix(a, b, 0.75));
        }
        if !closed {
            cut.push(line[count - 1]);
        }
        line = cut;
    }
    if closed && let Some(first) = line.first().copied() {
        line.push(first);
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_halves_meet_along_one_straight_edge_and_an_island_is_a_loop() {
        // Dark above, light below; a middle square in the light.
        let (w, h) = (40, 30);
        let values: Vec<u8> = (0..w * h)
            .map(|at| {
                let (x, y) = (at % w, at / w);
                if (20..30).contains(&x) && (20..26).contains(&y) {
                    1
                } else if y < 12 {
                    0
                } else {
                    2
                }
            })
            .collect();
        let masses = ValueMasses::checked([40, 30], values).expect("a map");
        let edges = masses.edges();
        assert_eq!(edges.len(), 2, "{edges:?}");
        let across = edges.iter().find(|e| e[0] != *e.last().unwrap_or(&e[0]));
        let across = across.expect("the edge across");
        assert!(across.iter().all(|p| (p[1] - 12.0 / 30.0).abs() < 1e-9));
        let ends = [across[0][0], across[across.len() - 1][0]];
        assert!(ends.contains(&0.0) && ends.contains(&1.0));
        let island = edges
            .iter()
            .find(|e| e[0] == *e.last().unwrap_or(&[9.0, 9.0]));
        let island = island.expect("the island's loop");
        assert!(
            (island.iter()).all(|p| (0.49..=0.76).contains(&p[0]) && (0.66..=0.87).contains(&p[1]))
        );
    }
}
