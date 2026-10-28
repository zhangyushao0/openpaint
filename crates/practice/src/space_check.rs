//! Compare for points in space (S90): the copy read as the trick asks (the last mark for a centre
//! or a mirror, the last line along the face for its halves, the last uprights for its thirds or
//! the next face; `marks.rs`) and carried back onto the face (`SpaceTask::on_face`), so its error
//! is a share of the face in space, near and far alike: a mark's distance from the answer in the
//! face's own coordinates, a halving line's height off the middle where it meets the face's near
//! and far edges, an upright's depth off its third or off the next face's.

use crate::edges::Edge;
use crate::marks::{last_edges, last_mark};
use crate::metric::{Metric, Metrics};
use crate::path::{Point, meet};
use crate::perspective_check::line_angle;
use crate::space::{SpaceTask, Trick};

/// A line is read as an upright within this of the vertical, any other as a halving line
/// (degrees).
const UPRIGHT: f64 = 15.0;
/// The error kept at most: a whole face.
const WHOLE: f64 = 1.0;

/// A point in space's copy as Compare read it: the last mark or the lines read, and how far off
/// (a share of the face; none when nothing was read).
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceCheck {
    pub mark: Option<Point>,
    pub lines: Vec<Edge>,
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
            mark,
            lines: read,
            metrics,
        }
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
