//! What a hand marked in a frame (S90): the last mark, a dot or a small cross, and the last
//! straight lines drawn, read from the copy's lines in the order they were drawn. The last one
//! counts, so a mark made again, or corrected in blue after Compare, is read in place of the one
//! before; construction lines (Sketch's) never reach here, and a long line drawn with Draw is no
//! mark.
//!
//! Frame units (a frame's side is 1).

use crate::geometry::edges::{Edge, edges};
use crate::maths::path::{Point, crossings};

/// A mark is no wider and no taller than this: 40 pixels of a 400-pixel frame.
pub const MARK: f64 = 0.1;

/// The point the last mark among `lines` (oldest first) stands for: the last line no larger
/// than `MARK`; where the small line drawn before it crosses it (a cross in two strokes), their
/// crossing, else the middle of its extent (a dot, a tick, a small ring). None without a mark.
#[must_use]
pub fn last_mark(lines: &[&[Point]]) -> Option<Point> {
    let small: Vec<&[Point]> = (lines.iter().copied())
        .filter(|line| extent(line).is_some_and(|[w, h]| w <= MARK && h <= MARK))
        .collect();
    let (last, before) = small.split_last()?;
    if let Some(crossed) = before
        .last()
        .and_then(|other| crossings(other, last).first().copied())
    {
        return Some(crossed);
    }
    middle(last)
}

/// The last `count` straight edges among `lines` (oldest first) that `accept` takes, the latest
/// first: each line's own edges (`edges.rs`, longest first), the lines read from the last.
#[must_use]
pub fn last_edges(lines: &[&[Point]], count: usize, accept: impl Fn(&Edge) -> bool) -> Vec<Edge> {
    let mut found = Vec::new();
    for line in lines.iter().rev() {
        for edge in edges(&[line]) {
            if found.len() < count && accept(&edge) {
                found.push(edge);
            }
        }
    }
    found
}

/// The width and height `line` spans; none for no point.
fn extent(line: &[Point]) -> Option<[f64; 2]> {
    let (low, high) = bounds(line)?;
    Some([high[0] - low[0], high[1] - low[1]])
}

/// The middle of what `line` spans.
fn middle(line: &[Point]) -> Option<Point> {
    let (low, high) = bounds(line)?;
    Some([(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0])
}

fn bounds(line: &[Point]) -> Option<(Point, Point)> {
    let first = *line.first()?;
    Some(line.iter().fold((first, first), |(low, high), p| {
        (
            [low[0].min(p[0]), low[1].min(p[1])],
            [high[0].max(p[0]), high[1].max(p[1])],
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_mark_counts_a_cross_at_its_crossing() {
        let long: &[Point] = &[[0.1, 0.1], [0.8, 0.8]];
        let dot: &[Point] = &[[0.30, 0.40], [0.31, 0.41]];
        let later: &[Point] = &[[0.60, 0.20], [0.62, 0.22]];
        let mark = last_mark(&[dot, long, later]).expect("a mark");
        assert!((mark[0] - 0.61).abs() < 1e-12 && (mark[1] - 0.21).abs() < 1e-12);
        let across: &[Point] = &[[0.46, 0.5], [0.54, 0.5]];
        let down: &[Point] = &[[0.52, 0.46], [0.52, 0.54]];
        let cross = last_mark(&[dot, across, down]).expect("a cross");
        assert!((cross[0] - 0.52).abs() < 1e-12 && (cross[1] - 0.5).abs() < 1e-12);
        assert_eq!(last_mark(&[long]), None, "a long line is no mark");
        assert_eq!(last_mark(&[]), None);
    }

    #[test]
    fn the_last_edges_come_latest_first() {
        let first: &[Point] = &[[0.1, 0.2], [0.1, 0.8]];
        let second: &[Point] = &[[0.2, 0.2], [0.8, 0.25]];
        let third: &[Point] = &[[0.5, 0.2], [0.5, 0.8]];
        let upright = |edge: &Edge| edge.direction()[0].abs() < 0.2;
        let found = last_edges(&[first, second, third], 2, upright);
        assert_eq!(found.len(), 2);
        assert!(
            (found[0].middle()[0] - 0.5).abs() < 1e-9,
            "the latest first"
        );
        assert!((found[1].middle()[0] - 0.1).abs() < 1e-9);
    }
}
