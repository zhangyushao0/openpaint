//! Compare for points in space (S90): the copy read as the trick asks (the last mark for a centre
//! or a mirror, the last line along the face for its halves, the last uprights for its thirds or
//! the next face; `marks.rs`) and carried back onto the face (`SpaceTask::on_face`), so its error
//! is a share of the face in space, near and far alike: a mark's distance from the answer in the
//! face's own coordinates, a halving line's height off the middle where it meets the face's near
//! and far edges, an upright's depth off its third or off the next face's.
//!
//! What is off (S98, `off.rs`), on the picture: a mark more than a pen's width off ringed; a
//! halving line run on past the vanishing point it misses, or its centre ringed; an upright that
//! leans given its true vertical, one standing off its place ringed where V大's construction finds
//! it (a third's point on the diagonal, the next face's foot on the bottom edge run on).

use crate::geometry::edges::Edge;
use crate::geometry::heading::line_angle;
use crate::geometry::marks::{last_edges, last_mark};
use crate::geometry::off::{Heads, Off, Straight, once, ringed};
use crate::kinds::points_in_space::space::{SpaceAnswer, SpaceTask, Trick};
use crate::marks::card::{Say, screen_way};
use crate::marks::compared::{Outcome, Verdict};
use crate::maths::path::{Point, meet};
use crate::records::metric::{Metric, Metrics, value};

/// A line is read as an upright within this of the vertical, any other as a halving line
/// (degrees).
const UPRIGHT: f64 = 15.0;
/// The error kept at most: a whole face.
const WHOLE: f64 = 1.0;
/// A near point in space is off by at most this share of the face: under half the way from a
/// half to a third (0.17 apart), so a near copy is still nearer the trick asked than another.
pub const NEAR_FACE: f64 = 0.08;

/// A point in space's copy as Compare read it: the last mark or the lines read, what of them is
/// off, and how far off (a share of the face; none when nothing was read).
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceCheck {
    pub mark: Option<Point>,
    pub lines: Vec<Edge>,
    pub off: Vec<Off>,
    pub metrics: Metrics,
    /// The way the worst of it lies off the answer on the picture (frame units, y down): a
    /// mark's offset, a halving line's up or down at its worse end, an upright's left or right.
    pub toward: Option<Point>,
}

impl SpaceCheck {
    /// `task`'s copy drawn as `lines` (Draw and Correct, oldest first) read and measured.
    #[must_use]
    pub fn of(task: &SpaceTask, lines: &[&[Point]]) -> Self {
        let target = task.target();
        let (mark, read, found) = match task.trick {
            Trick::Centre | Trick::Mirror => {
                let mark = last_mark(lines);
                let found = mark.map(|m| {
                    let [u, v] = task.on_face(m);
                    let answer = task.at([target[0], target[1]]);
                    let toward = [m[0] - answer[0], m[1] - answer[1]];
                    ((u - target[0]).hypot(v - target[1]), toward)
                });
                (mark, Vec::new(), found)
            }
            Trick::Halves => {
                let read = last_edges(lines, 1, |e| !upright(e));
                let found = read
                    .first()
                    .and_then(|edge| height_off(task, edge, target[0]));
                (None, read, found)
            }
            Trick::Thirds | Trick::Step => {
                let count = target.len();
                let read = last_edges(lines, count, upright);
                let found = depths_off(task, &read, &target);
                (None, read, found)
            }
        };
        let metrics = found.map_or_else(Vec::new, |(off, _)| {
            vec![(Metric::SpaceOff, off.min(WHOLE))]
        });
        Self {
            off: marked(task, mark, &read),
            mark,
            lines: read,
            metrics,
            toward: found.map(|(_, toward)| toward),
        }
    }

    /// `task`'s copy so read: incomplete while no mark or line was found (or fewer uprights than
    /// the thirds ask); clean when nothing is marked; near within `NEAR_FACE` of the face; else
    /// redo.
    #[must_use]
    pub fn verdict(&self, task: &SpaceTask) -> Verdict {
        let uprights = matches!(task.trick, Trick::Thirds | Trick::Step);
        let Some(off) = value(&self.metrics, Metric::SpaceOff) else {
            return Verdict::Incomplete;
        };
        if uprights && self.lines.len() < task.target().len() {
            return Verdict::Incomplete;
        }
        Verdict::App(if self.off.is_empty() {
            Outcome::Clean
        } else if off <= NEAR_FACE {
            Outcome::Near
        } else {
            Outcome::Redo
        })
    }

    /// The reason beside a result not right (S138): the mark, the halving line or the upright
    /// the way it lies off the answer on the picture; a little within `NEAR_FACE`, clearly beyond.
    #[must_use]
    pub fn reason(&self, task: &SpaceTask) -> Option<Say> {
        if !matches!(
            self.verdict(task),
            Verdict::App(Outcome::Near | Outcome::Redo)
        ) {
            return None;
        }
        let what = match task.trick {
            Trick::Centre | Trick::Mirror => "point",
            Trick::Halves => "line",
            Trick::Thirds | Trick::Step => "upright",
        };
        let off = value(&self.metrics, Metric::SpaceOff)?;
        Some(Say::reason(
            what,
            screen_way(self.toward?),
            (off, NEAR_FACE),
        ))
    }
}

/// What of `mark` or the lines `read` is off on the picture: the answer's point ringed, or each
/// line against the part of the answer nearest it.
fn marked(task: &SpaceTask, mark: Option<Point>, read: &[Edge]) -> Vec<Off> {
    if let SpaceAnswer::Point(answer) = task.answer() {
        return mark.and_then(|m| ringed(answer, m)).into_iter().collect();
    }
    let parts = straights(task);
    let off = read.iter().flat_map(|edge| {
        (parts.iter())
            .min_by(|a, b| a.apart(edge).total_cmp(&b.apart(edge)))
            .map(|part| part.off(edge))
            .unwrap_or_default()
    });
    once(off.collect())
}

/// The lines a trick asks for as straight parts: the halving line heading for the vanishing point,
/// fixed at the centre; each third's upright fixed where it crosses the diagonal (the face's own
/// `[u, u]`); the next face's far upright fixed at its foot.
fn straights(task: &SpaceTask) -> Vec<Straight> {
    let at = |u: f64, v: f64| task.at([u, v]);
    let upright = |u: f64, fixed: Point| Straight {
        line: [at(u, 0.0), at(u, 1.0)],
        heads: Some(Heads::Upright),
        fixed: vec![fixed],
    };
    match task.trick {
        Trick::Halves => vec![Straight {
            line: [at(0.0, 0.5), at(1.0, 0.5)],
            heads: Some(Heads::Point(task.vanishing)),
            fixed: vec![at(0.5, 0.5)],
        }],
        Trick::Thirds => [1.0 / 3.0, 2.0 / 3.0]
            .map(|u| upright(u, at(u, u)))
            .to_vec(),
        Trick::Step => vec![upright(2.0, at(2.0, 0.0))],
        Trick::Centre | Trick::Mirror => Vec::new(),
    }
}

/// Whether `edge` stands upright: a two-point box's verticals stay vertical on the picture.
fn upright(edge: &Edge) -> bool {
    line_angle(edge.direction(), [0.0, 1.0]) <= UPRIGHT
}

/// How far the halving line `edge` runs off the face's middle height `goal` (the face's `v`) at
/// the worse of its near and far edges, and that end's way off it on the picture (up or down).
fn height_off(task: &SpaceTask, edge: &Edge, goal: f64) -> Option<(f64, Point)> {
    let heights = [0.0, 1.0].map(|u| {
        let edge_at = [task.at([u, 0.0]), task.at([u, 1.0])];
        meet(edge.ends, edge_at).map(|p| (u, task.on_face(p)[1]))
    });
    let [near, far] = heights;
    let (u, v) = [near?, far?]
        .into_iter()
        .max_by(|a, b| (a.1 - goal).abs().total_cmp(&(b.1 - goal).abs()))?;
    let way = task.at([u, v])[1] - task.at([u, goal])[1];
    Some(((v - goal).abs(), [0.0, way]))
}

/// How far the uprights `read` stand off the depths `target` (the face's `u`), where each meets
/// the face's middle line: in order when as many were read (the worst), else the one read against
/// the nearer; and its way off on the picture (left or right).
fn depths_off(task: &SpaceTask, read: &[Edge], target: &[f64]) -> Option<(f64, Point)> {
    let middle = [task.at([0.0, 0.5]), task.at([1.0, 0.5])];
    let mut depths: Vec<f64> = (read.iter())
        .filter_map(|edge| meet(edge.ends, middle).map(|p| task.on_face(p)[0]))
        .collect();
    depths.sort_by(f64::total_cmp);
    let pairs: Vec<(f64, f64)> = match depths.as_slice() {
        [one] if target.len() > 1 => target.iter().map(|goal| (*one, *goal)).collect(),
        all => all.iter().copied().zip(target.iter().copied()).collect(),
    };
    let apart = |(depth, goal): &(f64, f64)| (depth - goal).abs();
    let worst = |a: &(f64, f64), b: &(f64, f64)| apart(a).total_cmp(&apart(b));
    let (depth, goal) = if depths.len() == 1 && target.len() > 1 {
        pairs.into_iter().min_by(worst)?
    } else {
        pairs.into_iter().max_by(worst)?
    };
    let way = task.at([depth, 0.5])[0] - task.at([goal, 0.5])[0];
    Some(((depth - goal).abs(), [way, 0.0]))
}
