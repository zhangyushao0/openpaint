//! What unit 3's demonstrations share (S92, S98): a third's tick, a point moved, a small cross as
//! the hand marks it and the check reads it.

use crate::maths::path::Point;

/// A tick's half length across a frame's edge or a segment at a third.
pub(crate) const THIRD_TICK: f64 = 0.025;
/// A cross's arm as the hand marks it (frame units).
const ARM: f64 = 0.02;

/// `p` moved by `by`.
pub(crate) fn moved(p: Point, by: Point) -> Point {
    [p[0] + by[0], p[1] + by[1]]
}

/// The two strokes of a small cross at `p`, as the check reads them.
pub(crate) fn crossed(p: Point) -> [Vec<Point>; 2] {
    [
        vec![[p[0] - ARM, p[1]], [p[0] + ARM, p[1]]],
        vec![[p[0], p[1] - ARM], [p[0], p[1] + ARM]],
    ]
}

/// `strokes` as lines to read.
pub(crate) fn read(strokes: &[Vec<Point>]) -> Vec<&[Point]> {
    strokes.iter().map(Vec::as_slice).collect()
}
