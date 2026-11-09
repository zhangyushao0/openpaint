//! Combined lines' demonstration (`demos/demo.rs`): the key points, the longest line first, each
//! other from where it meets it, then compared.

use crate::demos::demo::{
    Demo, DemoLayout, Drawn, Marker, On, Script, clearest, off_middle, wobbled,
};
use crate::kinds::combined_lines::combined::{Combination, CombinedLines};
use crate::maths::path::{Point, crossings, distance, length, nearest};
use crate::sessions::plan::Family;

/// The example's seed: never an exercise's but by a chance of one in 2^64.
const GROUP_SEED: u64 = 0xDE30_5EED_0002;
/// Key points nearer than this are one.
const SAME: f64 = 0.03;

/// Combined lines.
pub(crate) fn combined() -> Demo {
    // Three lines or more, the group near the frame's middle.
    let lines = clearest(
        (GROUP_SEED, 16),
        |seed| CombinedLines::generate(Combination::Branched, seed).lines,
        |lines| {
            let all: Vec<Point> = lines.iter().flatten().copied().collect();
            (lines.len().min(3) as f64) - off_middle(&all)
        },
    );
    let mut script = Script::new();
    for line in &lines {
        script.given(On::Original, Marker::Given, Drawn::Line(line.clone()));
    }
    script.step();
    for point in key_points(&lines) {
        script.draw(On::Copy, Marker::Pencil, Drawn::Cross(point));
    }
    let mut order: Vec<usize> = (0..lines.len()).collect();
    order.sort_by(|a, b| length(&lines[*b]).total_cmp(&length(&lines[*a])));
    let longest = &lines[order[0]];
    script.step();
    script.draw(
        On::Copy,
        Marker::Ink,
        Drawn::Line(wobbled(longest, 0.015, 0.2)),
    );
    script.step();
    for (turn, index) in order.iter().enumerate().skip(1) {
        // From where it meets the longest: its nearer end first.
        let mut line = lines[*index].clone();
        let gap = |p: Point| nearest(&[longest.as_slice()], p).map_or(1.0, |(_, d)| d);
        if let (Some(first), Some(last)) = (line.first(), line.last())
            && gap(*last) < gap(*first)
        {
            line.reverse();
        }
        let copy = wobbled(&line, 0.015, 0.9 * turn as f64);
        script.draw(On::Copy, Marker::Ink, Drawn::Line(copy));
    }
    let red = lines
        .into_iter()
        .map(|line| (On::Copy, Marker::Answer, Drawn::Line(line)))
        .collect();
    script.compare(Family::Combined, DemoLayout::Pair, red)
}

/// A group's key points: every line's ends and where lines cross, each place once.
fn key_points(lines: &[Vec<Point>]) -> Vec<Point> {
    let mut points: Vec<Point> = Vec::new();
    let mut add = |p: Point| {
        if points.iter().all(|q| distance(*q, p) > SAME) {
            points.push(p);
        }
    };
    for (index, line) in lines.iter().enumerate() {
        if let (Some(first), Some(last)) = (line.first(), line.last()) {
            add(*first);
            add(*last);
        }
        for other in &lines[index + 1..] {
            crossings(line, other).into_iter().for_each(&mut add);
        }
    }
    points
}
