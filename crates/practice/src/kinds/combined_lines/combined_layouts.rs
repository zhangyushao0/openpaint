//! The four arrangements of combined lines (`combined.rs`), laid out in working units where a
//! group is one to two across: the key points first, then each line between two of them.
//!
//! - Joined: two to four lines end to end, an open chain turning 40 to 137 degrees at each corner
//!   (as often on alike as on alternating sides), or, with three or four, a closed shape round a
//!   middle.
//! - Branched: a line with one to three lines leaving it from its middle part at 35 to 95 degrees,
//!   or two leaving its end as a fork.
//! - Crossing: a line crossed by one or two others at 35 to 90 degrees, away from its ends.
//! - Near: two to four lines alike in shape side by side, or fanning out from near one another.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use super::{Combination, Draft, Touch};
use crate::line::{Curvature, LineTier, integrate};
use crate::path::{Point, along, crossings};
use crate::random::Random;

pub(super) fn draft(combination: Combination, random: &mut Random) -> Draft {
    match combination {
        Combination::Joined => joined(random),
        Combination::Branched => branched(random),
        Combination::Crossing => crossing(random),
        Combination::Near => near(random),
    }
}

/// One of `chances`' values, each as likely as its chance (the chances add up to 1).
fn pick(random: &mut Random, chances: &[(usize, f64)]) -> usize {
    let mut left = random.unit();
    for &(value, chance) in chances {
        if left < chance {
            return value;
        }
        left -= chance;
    }
    chances.last().map_or(2, |(value, _)| *value)
}

/// `by` from `from` towards `angle`.
fn step(from: Point, angle: f64, by: f64) -> Point {
    [from[0] + by * angle.cos(), from[1] + by * angle.sin()]
}

fn turned(p: Point, angle: f64) -> Point {
    let (sin, cos) = angle.sin_cos();
    [p[0] * cos - p[1] * sin, p[0] * sin + p[1] * cos]
}

/// The shortest chord, in working units, a line may bend in an S along: a shorter S reads as a
/// squiggle.
const LONG: f64 = 0.8;

/// The bend of a line from `a` to `b`: near-straight, or with the chance `curved` an arc (seven
/// times in ten, always on a short line) or an S.
fn bend(random: &mut Random, curved: f64, (a, b): (Point, Point)) -> LineTier {
    let long = (b[0] - a[0]).hypot(b[1] - a[1]) >= LONG;
    if random.unit() >= curved {
        LineTier::Straight
    } else if random.unit() < 0.7 || !long {
        LineTier::Arc
    } else {
        LineTier::Wave
    }
}

/// Meeting at `at` at a corner the eye finds.
const fn corner(at: Point) -> Touch {
    Touch::Meet { at, corner: true }
}

/// A line from `a` to `b` bending as a single line of `tier` does: its curvature integrated,
/// then turned and sized so its ends are `a` and `b`.
fn member(a: Point, b: Point, tier: LineTier, random: &mut Random) -> Vec<Point> {
    let path = integrate(&Curvature::of(tier, random), 0.0);
    let (Some(&first), Some(&last)) = (path.first(), path.last()) else {
        return vec![a, b];
    };
    let chord = [last[0] - first[0], last[1] - first[1]];
    let wanted = [b[0] - a[0], b[1] - a[1]];
    let norm = (chord[0] * chord[0] + chord[1] * chord[1]).max(1e-12);
    // The turn and size taking `chord` to `wanted`: their quotient as complex numbers.
    let re = (wanted[0] * chord[0] + wanted[1] * chord[1]) / norm;
    let im = (wanted[1] * chord[0] - wanted[0] * chord[1]) / norm;
    let mut line: Vec<Point> = (path.iter())
        .map(|p| {
            let d = [p[0] - first[0], p[1] - first[1]];
            [a[0] + re * d[0] - im * d[1], a[1] + im * d[0] + re * d[1]]
        })
        .collect();
    if let Some(end) = line.last_mut() {
        *end = b;
    }
    line
}

fn joined(random: &mut Random) -> Draft {
    let count = pick(random, &[(2, 0.3), (3, 0.45), (4, 0.25)]);
    let closed = count >= 3 && random.unit() < 0.35;
    let corners = if closed {
        ring(count, random)
    } else {
        chain(count, random)
    };
    let mut lines = Vec::with_capacity(count);
    let mut touches = Vec::with_capacity(count);
    for index in 0..count {
        let ends = (corners[index], corners[(index + 1) % corners.len()]);
        let tier = bend(random, if closed { 0.45 } else { 0.5 }, ends);
        lines.push(member(ends.0, ends.1, tier, random));
        if index + 1 < count {
            touches.push((index, index + 1, corner(ends.1)));
        } else if closed {
            touches.push((index, 0, corner(ends.1)));
        }
    }
    Draft { lines, touches }
}

/// An open chain's `count + 1` corners.
fn chain(count: usize, random: &mut Random) -> Vec<Point> {
    let mut angle = random.range(0.0, TAU);
    let mut side = random.sign();
    let mut corners = vec![[0.0, 0.0]];
    for index in 0..count {
        if index > 0 {
            if random.unit() < 0.5 {
                side = -side;
            }
            angle += side * random.range(0.7, 2.4);
        }
        corners.push(step(corners[index], angle, random.range(0.5, 1.0)));
    }
    corners
}

/// A closed shape's `count` corners, round a middle.
fn ring(count: usize, random: &mut Random) -> Vec<Point> {
    let start = random.range(0.0, TAU);
    let share = TAU / count as f64;
    (0..count)
        .map(|k| {
            let angle = start + share * (k as f64 + random.range(-0.3, 0.3));
            step([0.0, 0.0], angle, random.range(0.55, 1.0))
        })
        .collect()
}

fn branched(random: &mut Random) -> Draft {
    let end = step([0.0, 0.0], random.range(0.0, TAU), 1.0);
    let tier = bend(random, 0.6, ([0.0, 0.0], end));
    let main = member([0.0, 0.0], end, tier, random);
    if random.unit() < 0.3 {
        return fork(main, random);
    }
    let count = pick(random, &[(1, 0.55), (2, 0.35), (3, 0.1)]);
    // Each leaves within its own slot of the line's middle part.
    let slot = 0.64 / count as f64;
    let mut lines = vec![main];
    let mut touches = Vec::with_capacity(count);
    for k in 0..count {
        let share = 0.18 + slot * (k as f64 + 0.5 + random.range(-0.3, 0.3));
        if let Some((from, heading)) = along(&lines[0], share) {
            let leaves = heading + random.sign() * random.range(0.6, 1.65);
            let to = step(from, leaves, random.range(0.35, 0.75));
            let tier = bend(random, 0.4, (from, to));
            touches.push((0, lines.len(), corner(from)));
            lines.push(member(from, to, tier, random));
        }
    }
    Draft { lines, touches }
}

/// `main` with two lines leaving its end, one to either side.
fn fork(main: Vec<Point>, random: &mut Random) -> Draft {
    let Some((end, heading)) = along(&main, 1.0) else {
        return Draft {
            lines: vec![main],
            touches: Vec::new(),
        };
    };
    let mut lines = vec![main];
    for side in [1.0, -1.0] {
        let to = step(
            end,
            heading + side * random.range(0.3, 0.9),
            random.range(0.45, 0.85),
        );
        let tier = bend(random, 0.4, (end, to));
        lines.push(member(end, to, tier, random));
    }
    // An arm may go on nearly straight from the stem: the other arm shows the fork.
    let on = Touch::Meet {
        at: end,
        corner: false,
    };
    let touches = vec![(0, 1, on), (0, 2, on), (1, 2, corner(end))];
    Draft { lines, touches }
}

fn crossing(random: &mut Random) -> Draft {
    let count = pick(random, &[(2, 0.65), (3, 0.35)]);
    let angle = random.range(0.0, TAU);
    let half = random.range(0.4, 0.6);
    let ends = [
        step([0.0, 0.0], angle + PI, half),
        step([0.0, 0.0], angle, half),
    ];
    let tier = bend(random, 0.5, (ends[0], ends[1]));
    let first = member(ends[0], ends[1], tier, random);
    let at = random.range(0.3, 0.7);
    let (second, second_chord) = across(ends, at, random);
    let mut lines = vec![first, second];
    let mut touches = vec![(0, 1, Touch::Cross)];
    if count == 3 {
        let other = if at < 0.5 {
            random.range(at + 0.25, 0.88)
        } else {
            random.range(0.12, at - 0.25)
        };
        let (third, third_chord) = across(ends, other, random);
        let meet = !crossings(&second_chord, &third_chord).is_empty();
        touches.push((0, 2, Touch::Cross));
        touches.push((1, 2, if meet { Touch::Cross } else { Touch::Apart }));
        lines.push(third);
    }
    Draft { lines, touches }
}

/// A line crossing the chord `ends` the share `at` along it, at 35 to 90 degrees to it, and its
/// own chord.
fn across(ends: [Point; 2], at: f64, random: &mut Random) -> (Vec<Point>, [Point; 2]) {
    let [a, b] = ends;
    let crossed = [a[0] + (b[0] - a[0]) * at, a[1] + (b[1] - a[1]) * at];
    let heading = (b[1] - a[1]).atan2(b[0] - a[0]);
    let leaning = heading + random.sign() * random.range(0.6, FRAC_PI_2);
    let start = step(crossed, leaning + PI, random.range(0.3, 0.6));
    let end = step(crossed, leaning, random.range(0.3, 0.6));
    let tier = bend(random, 0.4, (start, end));
    (member(start, end, tier, random), [start, end])
}

fn near(random: &mut Random) -> Draft {
    let count = pick(random, &[(2, 0.45), (3, 0.4), (4, 0.15)]);
    let tier = bend(random, 0.65, ([0.0, 0.0], [1.0, 0.0]));
    // The shape they share: a line of unit length from the origin, heading along x.
    let shape = integrate(&Curvature::of(tier, random), 0.0);
    let chord = shape.last().map_or(0.0, |end| end[1].atan2(end[0]));
    let across = chord + FRAC_PI_2;
    let fan = random.unit() < 0.4;
    let gap = random.range(0.14, 0.24);
    let spread = random.range(0.07, 0.16);
    let angle = random.range(0.0, TAU);
    let lines = (0..count)
        .map(|k| {
            let k = k as f64;
            let stretch = random.range(0.78, 1.08);
            let slide = random.range(-0.12, 0.12);
            // Side by side, or fanning out from near the shape's start.
            let (offset, turn) = if fan {
                (k * gap * 0.5, k * spread)
            } else {
                (k * gap, 0.0)
            };
            let moved = [
                slide * chord.cos() + offset * across.cos(),
                slide * chord.sin() + offset * across.sin(),
            ];
            (shape.iter())
                .map(|p| {
                    let q = turned([p[0] * stretch, p[1] * stretch], turn);
                    turned([q[0] + moved[0], q[1] + moved[1]], angle)
                })
                .collect()
        })
        .collect();
    let touches = (1..count).map(|k| (k - 1, k, Touch::Near)).collect();
    Draft { lines, touches }
}
