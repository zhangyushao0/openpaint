//! The drawn kinds' demonstrations (`demo.rs`): a line (its ends and where it bulges most ticked,
//! two passes in the air, one quick stroke), combined lines (the key points, the longest line
//! first, each other from where it meets it) and a single shape (its extremes ticked, blocked in
//! with long straight lines, a negative shape looked at, the outline in ink), each then compared.

use super::{Demo, Drawn, Layout, Marker, On, Script, clearest, off_middle, tick, wobbled};
use crate::colour::Rgb8;
use crate::combined::{Combination, CombinedLines};
use crate::line::{LineExercise, LineTier};
use crate::path::{Point, crossings, distance, length, nearest};
use crate::plan::Family;
use crate::random::Random;

/// The examples' seeds: never an exercise's but by a chance of one in 2^64.
const LINE_SEED: u64 = 0xDE30_5EED_0001;
const GROUP_SEED: u64 = 0xDE30_5EED_0002;
const SHAPE_SEED: u64 = 0xDE30_5EED_0003;
/// A tick's half length; key points nearer than this are one.
const TICK: f64 = 0.035;
const SAME: f64 = 0.03;
/// The vase's colour in the original's frame.
const CLAY: Rgb8 = [176, 118, 88];

/// A single line.
pub(super) fn line() -> Demo {
    // A long arc near the frame's middle, plainly bent.
    let original = clearest(
        (LINE_SEED, 32),
        |seed| LineExercise::generate(LineTier::Arc, seed).points,
        |line| {
            let chord = line
                .first()
                .zip(line.last())
                .map_or(0.0, |(a, b)| distance(*a, *b));
            let bent = depth(line).min(0.12);
            chord.min(0.7) + 2.0 * bent - off_middle(line)
        },
    );
    let mut script = Script::new();
    script.given(On::Original, Marker::Given, Drawn::Line(original.clone()));
    script.step();
    let n = original.len();
    let bulge = bulges(&original).into_iter().map(|i| (i, i + 1));
    for (at, towards) in [(0, 1), (n - 1, n - 2)].into_iter().chain(bulge) {
        let mark = tick(original[at], original[towards.min(n - 1)], TICK);
        script.draw(On::Copy, Marker::Pencil, Drawn::Line(mark));
    }
    script.step();
    for phase in [0.4, 2.1] {
        let pass = wobbled(&original, 0.012, phase);
        script.draw(On::Copy, Marker::Air, Drawn::Line(pass));
    }
    script.step();
    let copy = wobbled(&original, 0.03, 0.5);
    script.draw(On::Copy, Marker::Ink, Drawn::Line(copy));
    let red = vec![(On::Copy, Marker::Answer, Drawn::Line(original))];
    script.compare(Family::Line, Layout::Pair, red)
}

/// How far `line` lies from the chord between its ends at most.
fn depth(line: &[Point]) -> f64 {
    let (Some(&a), Some(&b)) = (line.first(), line.last()) else {
        return 0.0;
    };
    let chord = [b[0] - a[0], b[1] - a[1]];
    let long = chord[0].hypot(chord[1]).max(f64::MIN_POSITIVE);
    (line.iter())
        .map(|p| (((p[0] - a[0]) * chord[1] - (p[1] - a[1]) * chord[0]) / long).abs())
        .fold(0.0, f64::max)
}

/// Where `line` lies farthest from the chord between its ends, on either side, when it does so
/// plainly (its most bulging points, V大's 最鼓处).
fn bulges(line: &[Point]) -> Vec<usize> {
    let (Some(&a), Some(&b)) = (line.first(), line.last()) else {
        return Vec::new();
    };
    let chord = [b[0] - a[0], b[1] - a[1]];
    let long = chord[0].hypot(chord[1]).max(f64::MIN_POSITIVE);
    let side = |p: Point| ((p[0] - a[0]) * chord[1] - (p[1] - a[1]) * chord[0]) / long;
    let mut found = Vec::new();
    for sign in [1.0, -1.0] {
        let far =
            (0..line.len())
                .map(|i| (i, sign * side(line[i])))
                .fold(
                    (0, 0.0),
                    |best, here| if here.1 > best.1 { here } else { best },
                );
        if far.1 > 0.04 && far.0 + 1 < line.len() {
            found.push(far.0);
        }
    }
    found
}

/// Combined lines.
pub(super) fn combined() -> Demo {
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
    script.compare(Family::Combined, Layout::Pair, red)
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

/// A single shape: a vase, its silhouette in the original's frame.
pub(super) fn shape() -> Demo {
    let vase = Vase::generate(SHAPE_SEED);
    let outline = vase.outline();
    let mut script = Script::new();
    let body = Drawn::Band {
        rows: vase.rows.clone(),
        colour: Some(CLAY),
    };
    script.given(On::Original, Marker::Given, body);
    script.step();
    // Top and bottom, then left and right, each pair looked at together.
    for pair in vase.extremes().chunks(2) {
        let ticks = pair.iter().map(|(at, towards)| {
            (
                On::Copy,
                Marker::Pencil,
                Drawn::Line(tick(*at, *towards, TICK)),
            )
        });
        script.together(ticks.collect());
    }
    script.step();
    for side in vase.blocked() {
        script.draw(On::Copy, Marker::Pencil, Drawn::Line(side));
    }
    script.step();
    let pocket = Drawn::Band {
        rows: vase.pocket(),
        colour: None,
    };
    script.draw(On::Copy, Marker::Shade, pocket);
    script.step();
    script.draw(
        On::Copy,
        Marker::Ink,
        Drawn::Line(wobbled(&outline, 0.012, 1.3)),
    );
    let faint = Drawn::Band {
        rows: vase.rows,
        colour: Some(CLAY),
    };
    let compared = vec![
        (On::Copy, Marker::Faint, faint),
        (On::Copy, Marker::Answer, Drawn::Line(outline)),
    ];
    script.compare(Family::Shape, Layout::Pair, compared)
}

/// A vase seen from the side: its rows from the lip down, each its left and right edge.
struct Vase {
    rows: Vec<[Point; 2]>,
}

/// The vase's profile, top to bottom: height and half width (a lip, a neck, a shoulder, a belly,
/// a foot).
const PROFILE: [Point; 10] = [
    [0.12, 0.15],
    [0.16, 0.16],
    [0.21, 0.08],
    [0.3, 0.065],
    [0.41, 0.17],
    [0.56, 0.26],
    [0.71, 0.22],
    [0.8, 0.12],
    [0.85, 0.09],
    [0.9, 0.14],
];
/// Its middle across the frame; the rows of its lip, from the top.
const MIDDLE: f64 = 0.5;
const LIP: usize = 8;

impl Vase {
    /// The vase `seed` makes: the profile's widths a little varied, joined smoothly.
    fn generate(seed: u64) -> Self {
        let mut random = Random::new(seed);
        let profile: Vec<Point> = (PROFILE.iter())
            .map(|[y, w]| [*y, w * random.range(0.92, 1.08)])
            .collect();
        let mut rows = Vec::new();
        for k in 0..profile.len() - 1 {
            let at = |i: usize| profile[i.clamp(0, profile.len() - 1)];
            let (p0, p1, p2, p3) = (at(k.saturating_sub(1)), at(k), at(k + 1), at(k + 2));
            for s in 0..8 {
                let t = f64::from(s) / 8.0;
                let y = p1[0] + (p2[0] - p1[0]) * t;
                let w = catmull(p0[1], p1[1], p2[1], p3[1], t).max(0.02);
                rows.push([[MIDDLE - w, y], [MIDDLE + w, y]]);
            }
        }
        let [y, w] = profile[profile.len() - 1];
        rows.push([[MIDDLE - w, y], [MIDDLE + w, y]]);
        Self { rows }
    }

    /// Its outline round: down the right side, back up the left, closed at the lip.
    fn outline(&self) -> Vec<Point> {
        let right = self.rows.iter().map(|[_, r]| *r);
        let left = self.rows.iter().rev().map(|[l, _]| *l);
        let mut points: Vec<Point> = right.chain(left).collect();
        points.push(self.rows[0][1]);
        points
    }

    /// Its top, bottom, left and right, each with a point inwards from it: a tick across the way
    /// in marks how far the silhouette reaches (level at the top and bottom, upright at the
    /// sides).
    fn extremes(&self) -> [(Point, Point); 4] {
        let widest = (0..self.rows.len())
            .max_by(|a, b| self.rows[*a][1][0].total_cmp(&self.rows[*b][1][0]))
            .unwrap_or(0);
        let [top, bottom] = [self.rows[0][0][1], self.rows[self.rows.len() - 1][0][1]];
        let [left, right] = self.rows[widest];
        let inwards = |p: Point, d: Point| (p, [p[0] + d[0], p[1] + d[1]]);
        [
            inwards([MIDDLE, top], [0.0, 0.1]),
            inwards([MIDDLE, bottom], [0.0, -0.1]),
            inwards(left, [0.1, 0.0]),
            inwards(right, [-0.1, 0.0]),
        ]
    }

    /// The silhouette blocked in with long straight lines through its turning points, the right
    /// side and the left as the hand draws them, then the lip and the foot.
    fn blocked(&self) -> Vec<Vec<Point>> {
        let turns: Vec<usize> = (0..PROFILE.len())
            .map(|k| (k * 8).min(self.rows.len() - 1))
            .collect();
        let keep = [0, 1, 3, 5, 7, 9];
        let side = |which: usize| -> Vec<Point> {
            keep.iter().map(|k| self.rows[turns[*k]][which]).collect()
        };
        let (top, bottom) = (self.rows[0], self.rows[self.rows.len() - 1]);
        vec![
            side(1),
            side(0),
            vec![top[0], top[1]],
            vec![bottom[0], bottom[1]],
        ]
    }

    /// The negative shape at the right of the neck: between the outline and the straight line
    /// down from the lip's edge to where the outline comes out to it again.
    fn pocket(&self) -> Vec<[Point; 2]> {
        let lip = (0..=LIP)
            .max_by(|a, b| self.rows[*a][1][0].total_cmp(&self.rows[*b][1][0]))
            .unwrap_or(0);
        let edge = self.rows[lip][1][0];
        self.rows[lip..]
            .iter()
            .skip_while(|[_, right]| right[0] >= edge - 1e-9)
            .take_while(|[_, right]| right[0] < edge)
            .map(|[_, right]| [*right, [edge, right[1]]])
            .collect()
    }
}

/// Catmull-Rom between `b` and `c` at `t`.
fn catmull(a: f64, b: f64, c: f64, d: f64, t: f64) -> f64 {
    let t2 = t * t;
    0.5 * (2.0 * b
        + (c - a) * t
        + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
        + (3.0 * b - a - 3.0 * c + d) * t2 * t)
}
