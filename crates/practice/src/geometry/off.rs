//! Only what is off (S97, S98): Compare lays the exact answer over the copy and marks only the
//! parts of it the copy's ink misses by more than the Draw pen's width (`PEN_WIDTH`, 5 pixels of
//! a 400-pixel frame); ink within it gets nothing.
//!
//! - A point the ink marks more than a pen's width off is ringed (an estimate's, a centre's, a
//!   mirror's).
//! - A straight edge is run on to where it heads, as Drawabox's rough-perspective check does
//!   (S97's boxes): past its vanishing point or a cone's point (ringed), across the frame (an
//!   angle copied), or a leaning upright given its true vertical. One that heads right but lies off
//!   its place has the points that fix it ringed: where V大's construction finds it (a halving
//!   line's centre, a third's point on the diagonal, the next face's foot, a solid's side's
//!   touches, an object's corners).
//! - A curve, an ellipse in its frame (a cylinder's end, a cone's base, an object's round end in
//!   its square; a sphere's outline along its axes), has the middles it falls short of or runs past
//!   ringed, as S97's ellipse: the ellipse fitted to it, or, drawn too little round to fit, its
//!   pieces about each middle they pass.
//!
//! Frame units.

use crate::edges::Edge;
use crate::ellipse::{Ellipse, EllipseGuide};
use crate::ellipse_fit::missed;
use crate::path::{Point, across_frame, distance, length, resample};
use crate::perspective_check::{ON_ANGLE, line_angle, plumb, reach, sub, toward};
use crate::solid::PEN_WIDTH;
use crate::solid_check::ReadCurve;

/// A curve's pieces are read at this even step; a middle is judged only where they pass it within
/// `PASSES` of its turn either way, from the points within `ABOUT` of it (radians round the
/// ellipse, as on the circle it is drawn from).
const SPACING: f64 = 0.004;
const PASSES: f64 = std::f64::consts::FRAC_PI_8;
const ABOUT: f64 = std::f64::consts::FRAC_PI_4;
/// A piece of a curve shorter than this says nothing of it (8 pixels of a 400-pixel frame).
const STUB: f64 = 0.02;

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
    /// A point its line runs through beyond it: a vanishing point, a cone's point.
    Point(Point),
    /// Upright: a level eye's verticals stay vertical.
    Upright,
    /// On along itself to the frame's edges: an angle copied.
    Across,
}

/// A straight part of an answer: its line, where it heads (none where no hand aims: an object's
/// slanted edge) and the points that fix it.
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

/// The middles of `frame` the curve `read` for its ellipse falls short of or runs past by more
/// than a pen's width: by the ellipse fitted to it, else by its pieces about each middle they
/// pass, those `STUB` long at least (a stroke's stray end near it, a side's overshoot, is none).
#[must_use]
pub fn middles_missed(frame: &EllipseGuide, read: &ReadCurve) -> Vec<Off> {
    let points: Vec<Point> = (read.pieces.iter())
        .filter(|piece| length(piece) >= STUB)
        .flat_map(|piece| resample(piece, SPACING))
        .collect();
    let exact = frame.exact;
    let reach = |side: usize, normal: Point| match read.fitted {
        Some(fitted) => Some(fitted.support(normal)),
        None => {
            let middle = turn(&exact, frame.touches[side]);
            let near: Vec<(f64, Point)> = (points.iter())
                .map(|p| (wrapped(turn(&exact, *p) - middle), *p))
                .filter(|(off, _)| off.abs() <= ABOUT)
                .collect();
            let passes = near.iter().any(|(off, _)| (-PASSES..=0.0).contains(off))
                && near.iter().any(|(off, _)| (0.0..=PASSES).contains(off));
            passes.then(|| {
                (near.iter())
                    .map(|(_, p)| normal[0] * p[0] + normal[1] * p[1])
                    .fold(f64::NEG_INFINITY, f64::max)
            })
        }
    };
    (missed(frame, reach).into_iter())
        .map(|side| Off::Ring(side.middle))
        .collect()
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

/// Where `p` lies round `e`: the angle round its centre on the circle it is drawn from.
pub(crate) fn turn(e: &Ellipse, p: Point) -> f64 {
    let (sin, cos) = e.angle.sin_cos();
    let [dx, dy] = [p[0] - e.centre[0], p[1] - e.centre[1]];
    let along = (dx * cos + dy * sin) / e.axes[0].max(f64::MIN_POSITIVE);
    let across = (-dx * sin + dy * cos) / e.axes[1].max(f64::MIN_POSITIVE);
    across.atan2(along)
}

/// `angle` brought into -pi to pi.
fn wrapped(angle: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    let turned = angle.rem_euclid(tau);
    if turned > std::f64::consts::PI {
        turned - tau
    } else {
        turned
    }
}

/// How far `p` lies from the line through `line`'s two points.
fn from_line([a, b]: [Point; 2], p: Point) -> f64 {
    let long = distance(a, b);
    if long <= 0.0 {
        return distance(a, p);
    }
    ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])).abs() / long
}
