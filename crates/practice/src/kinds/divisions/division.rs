//! Picture division (V大's 画面分割, S95): a whole picture's most obvious divisions, found by feel
//! (squinting, never by analysis) and drawn beside it in three levels: the main lines (red), few,
//! at most five, as long as they run, best right through the picture, small turns ignored; then
//! from them the second level (green), a part's frame placed by its distances to the lines set,
//! more and shorter; then the third (blue), more and shorter still. People's divisions differ a
//! little, none is right or wrong, so there is no answer to judge against: Compare lays the
//! picture under the lines (the overlay is the check) and this only counts and measures what was
//! drawn, per level, as V大's rules speak of it (how many main lines, how long they run).
//!
//! A level's strokes whose ends meet within two pen widths heading on are one line: V大 draws a
//! long line from short ones.

use crate::compositions::levels::{Level, LevelRow};
use crate::maths::path::{Point, distance, length};
use crate::records::metric::{Metric, Metrics};

/// The most main lines V大 asks for.
pub const MOST_MAIN: usize = 5;
/// Two pen widths, in frame units (the pens are 5 canvas pixels on a 400-pixel frame): strokes of
/// a level whose ends meet this near, heading on, are one line; a mark shorter is no line.
pub const JOIN_REACH: f64 = 0.025;
/// The most a line turns where two strokes join and still heads on, degrees: a bent line's two
/// halves drawn apart join, a frame's corner never.
pub const HEADING_ON: f64 = 35.0;
/// How far back from a stroke's end its heading is read, frame units.
const HEADING_BACK: f64 = 0.03;

/// One line of a division: its level and its points (frame units), the strokes joined.
#[derive(Debug, Clone, PartialEq)]
pub struct DivisionLine {
    pub level: Level,
    pub points: Vec<Point>,
}

/// A division as drawn: its lines, each level's row and the numbers a first Compare keeps.
#[derive(Debug, Clone, PartialEq)]
pub struct DivisionCheck {
    pub lines: Vec<DivisionLine>,
    /// Main, second, third.
    pub rows: [LevelRow; 3],
    pub metrics: Metrics,
}

impl DivisionCheck {
    /// `pieces` (strokes in frame units, each with its level) joined into lines and measured
    /// against the picture's longer side `side` (frame units).
    #[must_use]
    pub fn of(pieces: &[(Level, Vec<Point>)], side: f64) -> Self {
        let mut lines = Vec::new();
        let mut rows = [LevelRow::default(); 3];
        let mut metrics = Metrics::new();
        for level in Level::ALL {
            let of: Vec<&[Point]> = (pieces.iter())
                .filter(|(held, _)| *held == level)
                .map(|(_, points)| points.as_slice())
                .collect();
            let joined = joined(&of);
            let total: f64 = joined.iter().map(|line| length(line)).sum();
            let count = joined.len();
            let mean = (count > 0).then(|| total / count as f64 / side.max(f64::MIN_POSITIVE));
            rows[level.index()] = LevelRow {
                lines: count,
                length: mean,
            };
            metrics.push((Metric::DivisionLines(level), count as f64));
            if let Some(mean) = mean {
                metrics.push((Metric::DivisionLength(level), mean));
            }
            lines.extend(
                joined
                    .into_iter()
                    .map(|points| DivisionLine { level, points }),
            );
        }
        Self {
            lines,
            rows,
            metrics,
        }
    }

    /// Whether the main lines are more than V大's five.
    #[must_use]
    pub const fn too_many_main(&self) -> bool {
        self.rows[0].lines > MOST_MAIN
    }
}

/// One level's strokes as lines: a mark shorter than `JOIN_REACH` left out, and two whose ends
/// meet within `JOIN_REACH` heading on (turning `HEADING_ON` degrees at most) one line, the
/// nearest ends first, as often as any meet.
#[must_use]
pub fn joined(pieces: &[&[Point]]) -> Vec<Vec<Point>> {
    let mut lines: Vec<Vec<Point>> = (pieces.iter())
        .filter(|piece| length(piece) >= JOIN_REACH)
        .map(|piece| piece.to_vec())
        .collect();
    while let Some((i, j, i_last, j_last)) = nearest_join(&lines) {
        let mut second = lines.remove(j);
        if j_last {
            second.reverse();
        }
        let first = &mut lines[i];
        if !i_last {
            first.reverse();
        }
        first.extend(second);
    }
    lines
}

/// The two lines whose ends meet nearest heading on: their places (the first before the second)
/// and whether each meets by its last point.
fn nearest_join(lines: &[Vec<Point>]) -> Option<(usize, usize, bool, bool)> {
    let mut best: Option<(f64, (usize, usize, bool, bool))> = None;
    for i in 0..lines.len() {
        for j in (i + 1)..lines.len() {
            for (i_last, j_last) in [(true, false), (true, true), (false, false), (false, true)] {
                let (a, heading_a) = end(&lines[i], i_last);
                let (b, heading_b) = end(&lines[j], j_last);
                let apart = distance(a, b);
                // The second goes on where the first heads: its way in is the first's way out.
                let on = heading_a[0] * -heading_b[0] + heading_a[1] * -heading_b[1];
                let heading = on >= HEADING_ON.to_radians().cos();
                if apart <= JOIN_REACH && heading && best.is_none_or(|(d, _)| apart < d) {
                    best = Some((apart, (i, j, i_last, j_last)));
                }
            }
        }
    }
    best.map(|(_, join)| join)
}

/// `line`'s last point (or its first) and the unit direction it heads out of the line there, read
/// `HEADING_BACK` back along it (or over the whole line where it is shorter).
fn end(line: &[Point], last: bool) -> (Point, Point) {
    let ordered: Vec<Point> = if last {
        line.iter().rev().copied().collect()
    } else {
        line.to_vec()
    };
    let tip = ordered[0];
    let mut back = tip;
    let mut walked = 0.0;
    for pair in ordered.windows(2) {
        walked += distance(pair[0], pair[1]);
        back = pair[1];
        if walked >= HEADING_BACK {
            break;
        }
    }
    let d = [tip[0] - back[0], tip[1] - back[1]];
    let long = d[0].hypot(d[1]).max(f64::MIN_POSITIVE);
    (tip, [d[0] / long, d[1] / long])
}
