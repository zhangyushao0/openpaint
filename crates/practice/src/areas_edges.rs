//! A colour combination's edges as lines to draw (`areas.rs`): the cell edges of the map between
//! an area and another, chained into lines where they meet end to end (a line stops where three
//! areas meet) and smoothed (each step's middle, averaged with its neighbours over a few cells),
//! so an outline or a mark follows the edge as the picture shows it rather than the map's steps.

use std::collections::HashMap;

use crate::areas::{Areas, GRID};
use crate::path::Point;

/// A corner of the map's cells: column and row, 0 to `GRID`.
type Corner = [usize; 2];

impl Areas {
    /// The edges between `area` and `other` (any other area, when none; never the frame's edge)
    /// as smoothed lines in frame units.
    #[must_use]
    pub fn edges(&self, area: usize, other: Option<usize>) -> Vec<Vec<Point>> {
        let segments = self.cell_edges(area, other);
        chained(&segments)
            .into_iter()
            .map(|corners| {
                let points: Vec<Point> = (corners.iter())
                    .map(|[c, r]| [*c as f64 / GRID as f64, *r as f64 / GRID as f64])
                    .collect();
                smoothed(&points)
            })
            .collect()
    }

    /// The map's cell edges between `area` and `other` (or any other area).
    fn cell_edges(&self, area: usize, other: Option<usize>) -> Vec<[Corner; 2]> {
        let counts = |there: usize| other.map_or(there != area, |other| there == other);
        let mut segments = Vec::new();
        for row in 0..GRID {
            for column in 0..GRID {
                if self.cell([column, row]) != area {
                    continue;
                }
                if column + 1 < GRID && counts(self.cell([column + 1, row])) {
                    segments.push([[column + 1, row], [column + 1, row + 1]]);
                }
                if row + 1 < GRID && counts(self.cell([column, row + 1])) {
                    segments.push([[column, row + 1], [column + 1, row + 1]]);
                }
                if column > 0 && counts(self.cell([column - 1, row])) {
                    segments.push([[column, row], [column, row + 1]]);
                }
                if row > 0 && counts(self.cell([column, row - 1])) {
                    segments.push([[column, row], [column + 1, row]]);
                }
            }
        }
        segments
    }
}

/// `segments` chained into lines of corners: each line runs on through corners where only it
/// passes, and stops at a corner where more lines meet or at its end.
fn chained(segments: &[[Corner; 2]]) -> Vec<Vec<Corner>> {
    let mut at: HashMap<Corner, Vec<usize>> = HashMap::new();
    for (index, [a, b]) in segments.iter().enumerate() {
        at.entry(*a).or_default().push(index);
        at.entry(*b).or_default().push(index);
    }
    let mut used = vec![false; segments.len()];
    let mut lines = Vec::new();
    for first in 0..segments.len() {
        if used[first] {
            continue;
        }
        used[first] = true;
        let [a, b] = segments[first];
        let mut line = vec![a, b];
        extend(&mut line, (segments, &at), &mut used);
        line.reverse();
        extend(&mut line, (segments, &at), &mut used);
        lines.push(line);
    }
    lines
}

/// `line` carried on from its last corner through segments not used yet, while the corner joins
/// just two.
fn extend(
    line: &mut Vec<Corner>,
    (segments, at): (&[[Corner; 2]], &HashMap<Corner, Vec<usize>>),
    used: &mut [bool],
) {
    while let Some(&end) = line.last() {
        let meeting = at.get(&end).map_or(&[][..], Vec::as_slice);
        if meeting.len() != 2 {
            return;
        }
        let Some(&next) = meeting.iter().find(|index| !used[**index]) else {
            return;
        };
        used[next] = true;
        let [a, b] = segments[next];
        line.push(if a == end { b } else { a });
    }
}

/// Steps of the map a smoothed point averages on either side.
const REACH: usize = 4;

/// The line through `corners` smoothed: the middle of each step, averaged with up to `REACH`
/// steps either side (fewer near an open line's ends, which it keeps; round a closed line).
fn smoothed(corners: &[Point]) -> Vec<Point> {
    if corners.len() < 3 {
        return corners.to_vec();
    }
    let closed = corners.first() == corners.last();
    let middles: Vec<Point> = (corners.windows(2))
        .map(|pair| {
            [
                (pair[0][0] + pair[1][0]) / 2.0,
                (pair[0][1] + pair[1][1]) / 2.0,
            ]
        })
        .collect();
    let count = middles.len();
    let averaged = |at: usize| -> Point {
        let reach = if closed {
            REACH.min(count / 2)
        } else {
            REACH.min(at).min(count - 1 - at)
        };
        let mut sum = [0.0, 0.0];
        for offset in 0..=2 * reach {
            let index = if closed {
                (at + count + offset - reach) % count
            } else {
                at + offset - reach
            };
            sum[0] += middles[index][0];
            sum[1] += middles[index][1];
        }
        let n = (2 * reach + 1) as f64;
        [sum[0] / n, sum[1] / n]
    };
    let mut line: Vec<Point> = (0..count).map(averaged).collect();
    if closed {
        line.push(line[0]);
    } else {
        line.insert(0, corners[0]);
        line.push(corners[corners.len() - 1]);
    }
    line
}
