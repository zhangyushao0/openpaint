//! Ghosted lines' variants and their tasks (S156, after Drawabox's lesson 1): a printed line with
//! its start dotted, drawn over eight times (superimposed lines); pairs of points, a straight
//! stroke between each (ghosted lines); a plane's four corners in perspective (ghosted planes,
//! `plane.rs`). The answer is the exact lines between the given points, each from where it starts
//! to where it ends.
//!
//! A task is laid again from its seed's sequence until it keeps every rule (`keeps`): every given
//! point inside the frame by `INSIDE`; a superimposed line `LONG` long; five pairs each `PAIRED`
//! long and `APART` from every other, so each stroke has room; a plane as `plane.rs` rules it.

use super::plane::Plane;
use crate::maths::path::{Point, apart, distance};
use crate::maths::random::Random;

/// Every given point inside the frame by this (frame units).
const INSIDE: f64 = 0.08;
/// A superimposed line's length, at least and at most.
const LONG: [f64; 2] = [0.45, 0.75];
/// The pairs a frame holds, each pair's length at least and at most, and how far apart each line
/// lies from every other.
const PAIRS: usize = 5;
const PAIRED: [f64; 2] = [0.2, 0.62];
const APART: f64 = 0.07;
/// Tries at a pair before the frame is laid again, and tries at a task before the last is kept.
const PAIR_TRIES: usize = 60;
const TRIES: usize = 200;

/// What a ghosted-lines exercise gives, one a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ghosted {
    /// A printed line, its start dotted: eight strokes over it (叠线, 入门).
    Superimposed,
    /// Five pairs of points at any angle and length: a stroke between each pair (鬼影线, 熟练).
    Pairs,
    /// A plane's four corners in perspective: its edges, its diagonals and the two lines through
    /// its centre toward its edges' vanishing points (鬼影面, 稳).
    Plane,
}

impl Ghosted {
    pub const ALL: [Self; 3] = [Self::Superimposed, Self::Pairs, Self::Plane];
}

/// A ghosted-lines exercise: the lines to draw, each from where it starts to where it ends (frame
/// units), which are its answer; the plane whose corners are given, at 稳.
#[derive(Debug, Clone, PartialEq)]
pub struct GhostedTask {
    pub ghosted: Ghosted,
    pub lines: Vec<[Point; 2]>,
    pub plane: Option<Plane>,
}

impl GhostedTask {
    /// The task `ghosted` lays from `seed`, the same for the same two.
    #[must_use]
    pub fn generate(ghosted: Ghosted, seed: u64) -> Self {
        let mut random = Random::new(seed);
        let mut last = None;
        for _ in 0..TRIES {
            let task = laid(ghosted, &mut random);
            if task.keeps() {
                return task;
            }
            last = Some(task);
        }
        last.unwrap_or_else(|| laid(ghosted, &mut Random::new(seed)))
    }

    /// The points it gives: a superimposed line's start, each pair's two ends, a plane's corners.
    #[must_use]
    pub fn points(&self) -> Vec<Point> {
        match (self.ghosted, &self.plane) {
            (Ghosted::Superimposed, _) => self.lines.iter().map(|[start, _]| *start).collect(),
            (_, Some(plane)) => plane.corners.to_vec(),
            _ => self.lines.iter().flatten().copied().collect(),
        }
    }

    /// Whether it keeps every rule of its variant.
    #[must_use]
    pub fn keeps(&self) -> bool {
        let inside = |p: &Point| p.iter().all(|v| (INSIDE..=1.0 - INSIDE).contains(v));
        let given = self.points().iter().all(inside);
        let long = |[low, high]: [f64; 2]| {
            (self.lines.iter()).all(|[a, b]| (low..=high).contains(&distance(*a, *b)))
        };
        given
            && match (self.ghosted, &self.plane) {
                (Ghosted::Superimposed, None) => self.lines.len() == 1 && long(LONG),
                (Ghosted::Pairs, None) => {
                    let room = (0..self.lines.len()).all(|k| {
                        self.lines[..k]
                            .iter()
                            .all(|other| fits(self.lines[k], other))
                    });
                    self.lines.len() == PAIRS && long(PAIRED) && room
                }
                (Ghosted::Plane, Some(plane)) => plane.keeps() && self.lines == plane.lines(),
                _ => false,
            }
    }
}

/// Whether `line` lies `APART` from `other`.
fn fits(line: [Point; 2], other: &[Point; 2]) -> bool {
    apart(&line, other) >= APART
}

/// A task of `ghosted` laid from `random`, its rules not yet asked.
fn laid(ghosted: Ghosted, random: &mut Random) -> GhostedTask {
    let (lines, plane) = match ghosted {
        Ghosted::Superimposed => (vec![segment(random, LONG)], None),
        Ghosted::Pairs => (pairs(random), None),
        Ghosted::Plane => {
            let plane = Plane::laid(random);
            (plane.lines(), Some(plane))
        }
    };
    GhostedTask {
        ghosted,
        lines,
        plane,
    }
}

/// A straight segment `length` long at any angle, its middle where both ends may lie inside.
fn segment(random: &mut Random, [low, high]: [f64; 2]) -> [Point; 2] {
    let long = random.range(low, high);
    let turn = random.range(0.0, std::f64::consts::TAU);
    let [dx, dy] = [turn.cos() * long / 2.0, turn.sin() * long / 2.0];
    let room = |d: f64| INSIDE + d.abs();
    let middle = [
        random.range(room(dx), 1.0 - room(dx)),
        random.range(room(dy), 1.0 - room(dy)),
    ];
    [
        [middle[0] - dx, middle[1] - dy],
        [middle[0] + dx, middle[1] + dy],
    ]
}

/// Up to `PAIRS` pairs, each laid where it keeps `APART` from those before it.
fn pairs(random: &mut Random) -> Vec<[Point; 2]> {
    let mut lines: Vec<[Point; 2]> = Vec::new();
    for _ in 0..PAIR_TRIES {
        if lines.len() == PAIRS {
            break;
        }
        let line = segment(random, PAIRED);
        if lines.iter().all(|other| fits(line, other)) {
            lines.push(line);
        }
    }
    lines
}
