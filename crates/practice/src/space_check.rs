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

use crate::edges::Edge;
use crate::marks::{last_edges, last_mark};
use crate::metric::{Metric, Metrics};
use crate::off::{Heads, Off, Straight, once, ringed};
use crate::path::{Point, meet};
use crate::perspective_check::line_angle;
use crate::space::{SpaceAnswer, SpaceTask, Trick};

/// A line is read as an upright within this of the vertical, any other as a halving line
/// (degrees).
const UPRIGHT: f64 = 15.0;
/// The error kept at most: a whole face.
const WHOLE: f64 = 1.0;

/// A point in space's copy as Compare read it: the last mark or the lines read, what of them is
/// off, and how far off (a share of the face; none when nothing was read).
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceCheck {
    pub mark: Option<Point>,
    pub lines: Vec<Edge>,
    pub off: Vec<Off>,
    pub metrics: Metrics,
}

impl SpaceCheck {
    /// `task`'s copy drawn as `lines` (Draw and Correct, oldest first) read and measured.
    #[must_use]
    pub fn of(task: &SpaceTask, lines: &[&[Point]]) -> Self {
        let target = task.target();
        let (mark, read, off) = match task.trick {
            Trick::Centre | Trick::Mirror => {
                let mark = last_mark(lines);
                let off = mark.map(|m| {
                    let [u, v] = task.on_face(m);
                    (u - target[0]).hypot(v - target[1])
                });
                (mark, Vec::new(), off)
            }
            Trick::Halves => {
                let read = last_edges(lines, 1, |e| !upright(e));
                let off = read.first().and_then(|edge| {
                    let heights = [0.0, 1.0].map(|u| {
                        let edge_at = [task.at([u, 0.0]), task.at([u, 1.0])];
                        meet(edge.ends, edge_at).map(|p| (task.on_face(p)[1] - target[0]).abs())
                    });
                    let [near, far] = heights;
                    Some(near?.max(far?))
                });
                (None, read, off)
            }
            Trick::Thirds | Trick::Step => {
                let count = target.len();
                let read = last_edges(lines, count, upright);
                let off = depths_off(task, &read, &target);
                (None, read, off)
            }
        };
        let metrics = off.map_or_else(Vec::new, |off| vec![(Metric::SpaceOff, off.min(WHOLE))]);
        Self {
            off: marked(task, mark, &read),
            mark,
            lines: read,
            metrics,
        }
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

/// How far the uprights `read` stand off the depths `target` (the face's `u`), where each meets
/// the face's middle line: in order when as many were read, else the one read against the nearer.
fn depths_off(task: &SpaceTask, read: &[Edge], target: &[f64]) -> Option<f64> {
    let middle = [task.at([0.0, 0.5]), task.at([1.0, 0.5])];
    let mut depths: Vec<f64> = (read.iter())
        .filter_map(|edge| meet(edge.ends, middle).map(|p| task.on_face(p)[0]))
        .collect();
    depths.sort_by(f64::total_cmp);
    let off = |depth: f64, goal: f64| (depth - goal).abs();
    match depths.as_slice() {
        [] => None,
        [one] if target.len() > 1 => target.iter().map(|goal| off(*one, *goal)).reduce(f64::min),
        all => (all.iter().zip(target))
            .map(|(depth, goal)| off(*depth, *goal))
            .reduce(f64::max),
    }
}
