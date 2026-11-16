//! Only what is off (S97, S98) where the app reads one stroke where one is asked (an estimate's
//! mark or line, a point in space's mark, line or uprights; D123): Compare lays the exact answer
//! over the copy and marks only the parts of it the copy's ink misses by more than the Draw pen's
//! width (`PEN_WIDTH`, 5 pixels of a 400-pixel frame); ink within it gets nothing.
//!
//! - A point the ink marks more than a pen's width off is ringed (an estimate's, a centre's, a
//!   mirror's).
//! - A straight line is run on to where it heads, as Drawabox's rough-perspective check does:
//!   past its vanishing point (ringed), across the frame (an angle copied), or a leaning upright
//!   given its true vertical. One that heads right but lies off its place has the points that fix
//!   it ringed: where V大's construction finds it (a halving line's centre, a third's point on the
//!   diagonal, the next face's foot).
//!
//! Frame units.

use crate::geometry::edges::Edge;
use crate::geometry::heading::{ON_ANGLE, line_angle, plumb, reach, sub, toward};
use crate::maths::path::{Point, across_frame, distance};
use crate::sheets::layout::FRAME;

/// The Draw pen's width (the app's `INK_DIAMETER`, 5 canvas pixels) on a frame of `FRAME`.
pub const PEN_WIDTH: f64 = 5.0 / FRAME;

/// A mark of what is off, red over the copy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Off {
    /// A point or a middle the ink misses.
    Ring(Point),
    /// An edge of the ink run on dashed from its end past where it should head (that point,
    /// ringed, when there is one).
    RunOn {
        line: [Point; 2],
        past: Option<Point>,
    },
    /// An upright that leans: the true vertical through it.
    Plumb([Point; 2]),
}

/// Where a straight part of an answer heads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Heads {
    /// A point its line runs through beyond it: a vanishing point.
    Point(Point),
    /// Upright: a level eye's verticals stay vertical.
    Upright,
    /// On along itself to the frame's edges: an angle copied.
    Across,
}

/// A straight part of an answer: its line, where it heads (none where no hand aims) and the
/// points that fix it.
#[derive(Debug, Clone, PartialEq)]
pub struct Straight {
    pub line: [Point; 2],
    pub heads: Option<Heads>,
    pub fixed: Vec<Point>,
}

impl Straight {
    /// What of `ink`, an edge read for it, is off: nothing within a pen's width of its line; else
    /// the ink run on where it turns from where the part heads by more than `ON_ANGLE`; else the
    /// points that fix the part which the ink's line misses by more than a pen's width.
    #[must_use]
    pub fn off(&self, ink: &Edge) -> Vec<Off> {
        if ink
            .ends
            .iter()
            .all(|p| from_line(self.line, *p) <= PEN_WIDTH)
        {
            return Vec::new();
        }
        if let Some(mark) = self.headed(ink) {
            return vec![mark];
        }
        (self.fixed.iter())
            .filter(|p| from_line(ink.ends, **p) > PEN_WIDTH)
            .map(|p| Off::Ring(*p))
            .collect()
    }

    /// How far the middle of `ink` lies from its line: which part an edge read is for.
    #[must_use]
    pub fn apart(&self, ink: &Edge) -> f64 {
        from_line(self.line, ink.middle())
    }

    /// `ink` run on to where the part heads, when it turns from it by more than `ON_ANGLE`.
    fn headed(&self, ink: &Edge) -> Option<Off> {
        let d = ink.direction();
        match self.heads? {
            Heads::Point(p) => (line_angle(d, sub(p, ink.middle())) > ON_ANGLE)
                .then(|| toward(ink, p, reach(&[p])))
                .flatten()
                .map(|line| Off::RunOn {
                    line,
                    past: Some(p),
                }),
            Heads::Upright => {
                (line_angle(d, [0.0, 1.0]) > ON_ANGLE).then(|| Off::Plumb(plumb(ink)))
            }
            Heads::Across => (line_angle(d, sub(self.line[1], self.line[0])) > ON_ANGLE)
                .then(|| across_frame(ink.middle(), d))
                .flatten()
                .map(|line| Off::RunOn { line, past: None }),
        }
    }
}

/// `answer` ringed when `mark` lies more than a pen's width off it.
#[must_use]
pub fn ringed(answer: Point, mark: Point) -> Option<Off> {
    (distance(answer, mark) > PEN_WIDTH).then_some(Off::Ring(answer))
}

/// `marks` each once.
#[must_use]
pub fn once(marks: Vec<Off>) -> Vec<Off> {
    let mut out: Vec<Off> = Vec::new();
    for mark in marks {
        let same = |kept: &Off| match (kept, &mark) {
            (Off::Ring(a), Off::Ring(b)) => distance(*a, *b) <= 1e-9,
            (kept, mark) => kept == mark,
        };
        if !out.iter().any(same) {
            out.push(mark);
        }
    }
    out
}

/// How far `p` lies from the line through `line`'s two points.
fn from_line([a, b]: [Point; 2], p: Point) -> f64 {
    let long = distance(a, b);
    if long <= 0.0 {
        return distance(a, p);
    }
    ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])).abs() / long
}
