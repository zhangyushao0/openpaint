//! Estimates on the flat (S90, V大's week 1 二维抽象网格): quick exercises with exact answers, an
//! empty frame beside the original's, the same for the same seed. A point placed where it lies in
//! its frame; a proportion, a segment divided at a point, the same division marked on a segment
//! of another length and direction (turned at most 60 degrees, so its ends answer the original's
//! as they point); an angle, a line in its frame, drawn through a given point at the same angle.
//! The eye judges against the frame and long lines, never a grid drawn: Compare shows the exact
//! answer and the way to see it (`FlatCheck` reads the copy; the screen draws the halves and
//! thirds and the lines run on to the frame's edges).
//!
//! Frame units (a frame's side is 1, y down); angles in degrees, turned clockwise as the screen
//! shows them.

use crate::geometry::edges::Edge;
use crate::geometry::marks::{last_edges, last_mark};
use crate::geometry::off::PEN_WIDTH;
use crate::geometry::off::{Heads, Off, Straight, ringed};
use crate::marks::compared::{Outcome, Verdict};
use crate::maths::path::{Point, distance};
use crate::maths::random::Random;
use crate::records::metric::{Metric, Metrics};
use crate::sessions::plan::Family;

/// How far inside its frame a given segment keeps.
const INSIDE: f64 = 0.08;
/// The shortest line read as an angle's copy: a mark is no line.
const LINE: f64 = 0.1;
/// A near estimate's mark lies within this of the answer (three pen widths, 15 pixels of a
/// 400-pixel frame), an angle's line passes as near its dot.
pub const NEAR_OFF: f64 = 3.0 * PEN_WIDTH;
/// A near angle's line turns at most this far from the original's (degrees: twice `ON_ANGLE`,
/// where an edge stops heading for its point).
pub const NEAR_ANGLE: f64 = 4.0;

/// What an estimate asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Estimate {
    Point,
    Proportion,
    Angle,
}

impl Estimate {
    pub const ALL: [Self; 3] = [Self::Point, Self::Proportion, Self::Angle];

    #[must_use]
    pub const fn family(self) -> Family {
        match self {
            Self::Point => Family::Point,
            Self::Proportion => Family::Proportion,
            Self::Angle => Family::Angle,
        }
    }
}

/// An estimate's givens, from which its exact answer follows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Flat {
    /// A point in the original's frame, to be placed at the same place in the copy's.
    Point { at: Point },
    /// The original's segment (its ends) divided at `share` of the way from its first end, and
    /// the copy's segment, of another length and direction, to divide alike.
    Proportion {
        original: [Point; 2],
        share: f64,
        copy: [Point; 2],
    },
    /// The original's line (its ends) and the point in the copy's frame its copy runs through.
    Angle {
        original: [Point; 2],
        through: Point,
    },
}

/// An estimate's exact answer in the copy's frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlatAnswer {
    Point(Point),
    /// A line through the given point as long as the original's, inside the frame.
    Line([Point; 2]),
}

impl Flat {
    /// The estimate `estimate` seeded `seed`.
    #[must_use]
    pub fn generate(estimate: Estimate, seed: u64) -> Self {
        let mut random = Random::new(seed);
        match estimate {
            Estimate::Point => Self::Point {
                at: [random.range(0.1, 0.9), random.range(0.1, 0.9)],
            },
            Estimate::Proportion => {
                // The copy shorter or longer by 15 % at least, both well inside the frame.
                let (length, ratio) = if random.unit() < 0.5 {
                    (random.range(0.5, 0.8), random.range(0.6, 0.85))
                } else {
                    (random.range(0.4, 0.62), random.range(1.15, 1.35))
                };
                let angle = random.range(0.0, 360.0);
                let original = placed(&mut random, length, angle);
                let share = random.range(0.15, 0.85);
                let turn = random.sign() * random.range(25.0, 60.0);
                let copy = placed(&mut random, length * ratio, angle + turn);
                Self::Proportion {
                    original,
                    share,
                    copy,
                }
            }
            Estimate::Angle => {
                // Off level and upright by 6 degrees at least: those the eye has for free.
                let angle = random.range(6.0, 84.0) + if random.unit() < 0.5 { 0.0 } else { 90.0 };
                let length = random.range(0.5, 0.75);
                let original = placed(&mut random, length, angle);
                let through = [random.range(0.3, 0.7), random.range(0.3, 0.7)];
                Self::Angle { original, through }
            }
        }
    }

    #[must_use]
    pub const fn estimate(&self) -> Estimate {
        match self {
            Self::Point { .. } => Estimate::Point,
            Self::Proportion { .. } => Estimate::Proportion,
            Self::Angle { .. } => Estimate::Angle,
        }
    }

    /// The exact answer in the copy's frame.
    #[must_use]
    pub fn answer(&self) -> FlatAnswer {
        match *self {
            Self::Point { at } => FlatAnswer::Point(at),
            Self::Proportion { share, copy, .. } => FlatAnswer::Point(along(copy, share)),
            Self::Angle { original, through } => {
                let d = unit(original);
                let half = distance(original[0], original[1]) / 2.0;
                let end = |sign: f64| {
                    // As far as the frame allows on either side.
                    let room = (0..2)
                        .map(|k| {
                            let step = sign * d[k];
                            if step > 1e-12 {
                                (1.0 - through[k]) / step
                            } else if step < -1e-12 {
                                -through[k] / step
                            } else {
                                f64::INFINITY
                            }
                        })
                        .fold(half, f64::min);
                    [
                        through[0] + sign * room * d[0],
                        through[1] + sign * room * d[1],
                    ]
                };
                FlatAnswer::Line([end(-1.0), end(1.0)])
            }
        }
    }
}

/// An estimate's copy as Compare read it: the last mark (a point or a proportion) or the last
/// straight line drawn (an angle), what of it is off (S98: the answer's point ringed when the mark
/// lies more than a pen's width off it, along a proportion's segment; an angle's line run on
/// across the frame when it turns from the answer, the dot ringed when it misses it), and how far
/// off it is (none when nothing was read).
#[derive(Debug, Clone, PartialEq)]
pub struct FlatCheck {
    pub mark: Option<Point>,
    pub line: Option<Edge>,
    pub off: Vec<Off>,
    /// A point's error across and down (frame units), a proportion's along its segment (a share
    /// of it), an angle's in degrees: each signed, the copy's less the answer's.
    pub metrics: Metrics,
}

impl FlatCheck {
    /// `flat`'s copy drawn as `lines` (Draw and Correct, oldest first) read and measured.
    #[must_use]
    pub fn of(flat: &Flat, lines: &[&[Point]]) -> Self {
        match *flat {
            Flat::Point { at } => {
                let mark = last_mark(lines);
                let metrics = mark.map_or_else(Vec::new, |m| {
                    vec![
                        (Metric::PointAcross, m[0] - at[0]),
                        (Metric::PointDown, m[1] - at[1]),
                    ]
                });
                Self {
                    mark,
                    line: None,
                    off: mark.and_then(|m| ringed(at, m)).into_iter().collect(),
                    metrics,
                }
            }
            Flat::Proportion { share, copy, .. } => {
                let mark = last_mark(lines);
                let error = mark.map(|m| share_along(copy, m) - share);
                let long = distance(copy[0], copy[1]);
                let off = error.filter(|off| off.abs() * long > PEN_WIDTH);
                Self {
                    mark,
                    line: None,
                    off: off
                        .map(|_| Off::Ring(along(copy, share)))
                        .into_iter()
                        .collect(),
                    metrics: (error.into_iter())
                        .map(|off| (Metric::ProportionOff, off))
                        .collect(),
                }
            }
            Flat::Angle { original, through } => {
                let line = last_edges(lines, 1, |edge| edge.length() >= LINE).pop();
                let metrics = line.map_or_else(Vec::new, |edge| {
                    let off = heading(edge.ends) - heading(original);
                    vec![(Metric::AngleOff, wrapped(off))]
                });
                // The line through the dot at the original's angle.
                let d = unit(original);
                let answer = Straight {
                    line: [through, [through[0] + d[0], through[1] + d[1]]],
                    heads: Some(Heads::Across),
                    fixed: vec![through],
                };
                Self {
                    mark: None,
                    off: line.map(|edge| answer.off(&edge)).unwrap_or_default(),
                    line,
                    metrics,
                }
            }
        }
    }

    /// `flat`'s copy so read: incomplete while no mark or line was found; clean when nothing is
    /// marked (within a pen's width); near when a mark lies within `NEAR_OFF` of the answer (along
    /// a proportion's segment) or a line turns `NEAR_ANGLE` at most and passes within `NEAR_OFF`
    /// of its dot; else redo.
    #[must_use]
    pub fn verdict(&self, flat: &Flat) -> Verdict {
        let near = match (*flat, self.mark, self.line) {
            (Flat::Point { at }, Some(mark), _) => distance(mark, at) <= NEAR_OFF,
            (Flat::Proportion { share, copy, .. }, Some(mark), _) => {
                (share_along(copy, mark) - share).abs() * distance(copy[0], copy[1]) <= NEAR_OFF
            }
            (Flat::Angle { original, through }, _, Some(line)) => {
                let turned = wrapped(heading(line.ends) - heading(original));
                turned.abs() <= NEAR_ANGLE && from_line(line.ends, through) <= NEAR_OFF
            }
            _ => return Verdict::Incomplete,
        };
        Verdict::App(match (self.off.is_empty(), near) {
            (true, _) => Outcome::Clean,
            (false, true) => Outcome::Near,
            (false, false) => Outcome::Redo,
        })
    }
}

/// How far `p` lies from the line through `a` and `b`.
fn from_line([a, b]: [Point; 2], p: Point) -> f64 {
    let d = unit([a, b]);
    ((p[0] - a[0]) * d[1] - (p[1] - a[1]) * d[0]).abs()
}

/// A segment `length` long at `angle` degrees, its middle placed by `random` so it keeps inside
/// the frame.
fn placed(random: &mut Random, length: f64, angle: f64) -> [Point; 2] {
    let (sin, cos) = angle.to_radians().sin_cos();
    let half = [cos * length / 2.0, sin * length / 2.0];
    let middle = half.map(|h| {
        let room = INSIDE + h.abs();
        random.range(room, (1.0 - room).max(room))
    });
    [
        [middle[0] - half[0], middle[1] - half[1]],
        [middle[0] + half[0], middle[1] + half[1]],
    ]
}

/// The point `share` of the way along `segment` from its first end.
fn along([a, b]: [Point; 2], share: f64) -> Point {
    [a[0] + (b[0] - a[0]) * share, a[1] + (b[1] - a[1]) * share]
}

/// How far along `segment` from its first end `p` lies, as a share of its length.
fn share_along([a, b]: [Point; 2], p: Point) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let squared = (d[0] * d[0] + d[1] * d[1]).max(f64::MIN_POSITIVE);
    ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / squared
}

/// `segment`'s unit direction.
fn unit([a, b]: [Point; 2]) -> Point {
    let long = distance(a, b).max(f64::MIN_POSITIVE);
    [(b[0] - a[0]) / long, (b[1] - a[1]) / long]
}

/// The line's angle, degrees, turned clockwise from level as the screen shows it.
fn heading([a, b]: [Point; 2]) -> f64 {
    (b[1] - a[1]).atan2(b[0] - a[0]).to_degrees()
}

/// An angle between two lines' headings, brought into -90 to 90 degrees.
fn wrapped(angle: f64) -> f64 {
    let turned = angle.rem_euclid(180.0);
    if turned > 90.0 {
        turned - 180.0
    } else {
        turned
    }
}
