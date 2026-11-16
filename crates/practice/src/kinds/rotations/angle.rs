//! The small box at the asked view's angle (S113, 稳; V大's weeks 7 and 8: 画一个小方块做角度示意):
//! where only the new box's near upright is given, the angle is printed as a small box beside it.
//! It is the asked box's front corner, height and two front edges made `SMALL` of their size,
//! toward the horizon as much (so it lies on the box's side of it, its top or bottom seen as the
//! box's), its edges run to the asked box's own vanishing points and completed as any two-point
//! box; at the frame's side farther from the box, clear of the box and of the horizon's label, or
//! none (the generator lays another view).

use crate::geometry::perspective::{Direction, PerspectiveBox};
use crate::maths::path::{Point, distance, meet};

/// The small box's share of the asked box's size.
const SMALL: f64 = 0.24;
/// How far inside the frame it is placed and keeps (its edges run to the box's own vanishing
/// points, so its width is near the box's share), and how far from the box and the horizon's
/// label.
const PLACED: f64 = 0.09;
const INSIDE: f64 = 0.06;
const CLEAR: f64 = 0.04;
/// The horizon's label from the frame's left, and its height above the horizon.
const LABEL: [f64; 2] = [0.16, 0.05];

/// The small box at `seen`'s angle, beside it; none where it finds no room.
pub(crate) fn angle_box(seen: &PerspectiveBox) -> Option<PerspectiveBox> {
    let c = seen.corners;
    let [low, high] = bounds(&c);
    let y = seen.horizon + SMALL * (c[0][1] - seen.horizon);
    let left = PLACED + SMALL * (c[0][0] - low[0]);
    let right = 1.0 - PLACED - SMALL * (high[0] - c[0][0]);
    // The side with more room first.
    let sides = if low[0] > 1.0 - high[0] {
        [left, right]
    } else {
        [right, left]
    };
    sides
        .into_iter()
        .filter_map(|x| small(seen, [x, y]))
        .find(|small| clear(small, seen))
}

/// The small box whose front corner lies at `front`: none where its lines do not meet.
fn small(seen: &PerspectiveBox, front: Point) -> Option<PerspectiveBox> {
    let c = seen.corners;
    let [left, right] = [Direction::Left, Direction::Right].map(|d| seen.vanishing[d.index()]);
    let (left, right) = (left?, right?);
    let toward = |v: Point, long: f64| {
        let d = distance(front, v).max(f64::MIN_POSITIVE);
        [
            front[0] + (v[0] - front[0]) * long / d,
            front[1] + (v[1] - front[1]) * long / d,
        ]
    };
    let up = |p: Point| [p, [p[0], p[1] - 1.0]];
    let m0 = front;
    let m2 = [front[0], front[1] - SMALL * (c[0][1] - c[2][1])];
    let m1 = toward(right, SMALL * distance(c[0], c[1]));
    let m4 = toward(left, SMALL * distance(c[0], c[4]));
    let m3 = meet([m2, right], up(m1))?;
    let m6 = meet([m2, left], up(m4))?;
    let m5 = meet([m1, left], [m4, right])?;
    let m7 = meet([m3, left], [m6, right])?;
    Some(PerspectiveBox {
        corners: [m0, m1, m2, m3, m4, m5, m6, m7],
        ..*seen
    })
}

/// Whether `small` lies inside the frame, clear of `seen`'s box and of the horizon's label.
fn clear(small: &PerspectiveBox, seen: &PerspectiveBox) -> bool {
    let [low, high] = bounds(&small.corners);
    let inside = low
        .iter()
        .chain(&high)
        .all(|v| (INSIDE..=1.0 - INSIDE).contains(v));
    let apart = |[l, h]: [Point; 2]| {
        low[0] > h[0] + CLEAR
            || high[0] < l[0] - CLEAR
            || low[1] > h[1] + CLEAR
            || high[1] < l[1] - CLEAR
    };
    let label = [[0.0, seen.horizon - LABEL[1]], [LABEL[0], seen.horizon]];
    inside && apart(bounds(&seen.corners)) && apart(label)
}

/// The least and the greatest of `points` each way.
fn bounds(points: &[Point]) -> [Point; 2] {
    let low = [0, 1].map(|k| points.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min));
    let high = [0, 1].map(|k| {
        points
            .iter()
            .map(|p| p[k])
            .fold(f64::NEG_INFINITY, f64::max)
    });
    [low, high]
}
