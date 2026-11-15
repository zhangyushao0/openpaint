//! The two arrangements of four to six lines read as one wrapped shape (S141, V大's W2: scattered
//! parts closed into one outer shape before it is divided; the empty shapes between the lines
//! checked against the original's), built of the smaller ones' pieces (`layouts.rs`) and held to
//! the same rules.
//!
//! - Enclosed: a closed shape of three or four lines with one or two inside it, each from its
//!   outline: the first across it from side to side; three times in five a second, across it
//!   beside the first or leaving a side inwards; the inside cut into two or three shapes.
//! - Cluster: an open chain of two or three lines and, on its members in turn away from its
//!   corners, a line leaving one, a line crossing one and a line running near one: all three with
//!   a chain of two (the near one on the leaving one's member, on its other side), all three or
//!   two of them with a chain of three, as often.

use std::f64::consts::{FRAC_PI_2, PI};

use super::layouts::{bend, chain, corner, member, pick, ring, step};
use super::{Draft, Touch};
use crate::maths::path::{Point, along};
use crate::maths::random::Random;

pub(super) fn enclosed(random: &mut Random) -> Draft {
    let count = pick(random, &[(3, 0.5), (4, 0.5)]);
    let corners = ring(count, random);
    let mut lines = Vec::with_capacity(count + 2);
    let mut touches = Vec::with_capacity(count + 4);
    for index in 0..count {
        let ends = (corners[index], corners[(index + 1) % count]);
        let tier = bend(random, 0.45, ends);
        lines.push(member(ends.0, ends.1, tier, random));
        touches.push((index, (index + 1) % count, corner(ends.1)));
    }
    // The sides the first line across runs between, from early along the one to late along the
    // other; a second across takes its shares the other way round, nearer the corners between
    // them, so the two never cross.
    let from = (random.unit() * count as f64) as usize % count;
    let to = (from + 1 + (random.unit() * (count - 1) as f64) as usize) % count;
    let (near, far) = ((0.25, 0.42), (0.58, 0.75));
    let mut inside = vec![(near, far)];
    let second = random.unit();
    if second < 0.3 {
        inside.push((far, near));
    }
    for (on_from, on_to) in inside {
        let shares = (
            random.range(on_from.0, on_from.1),
            random.range(on_to.0, on_to.1),
        );
        if let (Some((a, _)), Some((b, _))) =
            (along(&lines[from], shares.0), along(&lines[to], shares.1))
        {
            let tier = bend(random, 0.35, (a, b));
            touches.push((from, lines.len(), corner(a)));
            touches.push((to, lines.len(), corner(b)));
            lines.push(member(a, b, tier, random));
        }
    }
    if (0.3..0.6).contains(&second) {
        let side = (random.unit() * count as f64) as usize % count;
        if let Some((a, heading)) = along(&lines[side], random.range(0.3, 0.7)) {
            // Inwards: the ring's corners run round anticlockwise, its inside on their left.
            let b = step(
                a,
                heading + random.range(0.6, PI - 0.6),
                random.range(0.4, 0.65),
            );
            let tier = bend(random, 0.35, (a, b));
            touches.push((side, lines.len(), corner(a)));
            lines.push(member(a, b, tier, random));
        }
    }
    Draft { lines, touches }
}

pub(super) fn cluster(random: &mut Random) -> Draft {
    let count = pick(random, &[(2, 0.5), (3, 0.5)]);
    let corners = chain(count, random);
    let mut lines = Vec::with_capacity(6);
    let mut touches = Vec::with_capacity(6);
    for index in 0..count {
        let ends = (corners[index], corners[index + 1]);
        let tier = bend(random, 0.5, ends);
        lines.push(member(ends.0, ends.1, tier, random));
        if index > 0 {
            touches.push((index - 1, index, corner(ends.0)));
        }
    }
    let first = (random.unit() * count as f64) as usize % count;
    let skipped = (count == 3 && random.unit() < 0.5).then(|| (random.unit() * 3.0) as usize);
    // The side of its member the leaving line took.
    let mut left = 1.0;
    for (k, on) in (0..3).map(|k| (k, (first + k) % count)) {
        if skipped == Some(k) {
            continue;
        }
        let (line, touch) = match k {
            0 => {
                let at = away_from_corners(&lines[on], (on, count), random);
                left = random.sign();
                (leaving(at, left, random), corner(at.0))
            }
            1 => {
                let at = away_from_corners(&lines[on], (on, count), random);
                (crossing(at, random), Touch::Cross)
            }
            _ => {
                let side = if on == first { -left } else { random.sign() };
                (beside(&lines[on], side, random), Touch::Near)
            }
        };
        touches.push((on, lines.len(), touch));
        lines.push(line);
    }
    Draft { lines, touches }
}

/// A point of `member`, the `on`th of a chain of `count`, away from the chain's corners (the
/// first member's start, the last's end, a middle one's middle), and its heading there.
fn away_from_corners(
    member: &[Point],
    (on, count): (usize, usize),
    random: &mut Random,
) -> (Point, f64) {
    let share = match on {
        0 => random.range(0.2, 0.45),
        _ if on + 1 == count => random.range(0.55, 0.8),
        _ => random.range(0.35, 0.65),
    };
    along(member, share).unwrap_or(([0.0, 0.0], 0.0))
}

/// A line leaving the point `at` of a line heading `heading` there to its `side` (1 its left, -1
/// its right) at 35 to 95 degrees.
fn leaving((at, heading): (Point, f64), side: f64, random: &mut Random) -> Vec<Point> {
    let to = step(
        at,
        heading + side * random.range(0.6, 1.65),
        random.range(0.45, 0.8),
    );
    let tier = bend(random, 0.4, (at, to));
    member(at, to, tier, random)
}

/// A line crossing a line heading `heading` at `at` at 35 to 90 degrees, reaching a little past
/// it either way.
fn crossing((at, heading): (Point, f64), random: &mut Random) -> Vec<Point> {
    let leaning = heading + random.sign() * random.range(0.6, FRAC_PI_2);
    let start = step(at, leaning + PI, random.range(0.25, 0.45));
    let end = step(at, leaning, random.range(0.25, 0.45));
    let tier = bend(random, 0.4, (start, end));
    member(start, end, tier, random)
}

/// `line`'s middle part moved to its `side` (1 its left, -1 its right): a line alike running near
/// it.
fn beside(line: &[Point], side: f64, random: &mut Random) -> Vec<Point> {
    let count = line.len() as f64;
    let (from, to) = (
        (count * random.range(0.08, 0.2)) as usize,
        (count * random.range(0.8, 0.92)) as usize,
    );
    let (Some(first), Some(last)) = (line.first(), line.last()) else {
        return Vec::new();
    };
    let away = (last[1] - first[1]).atan2(last[0] - first[0]) + side * FRAC_PI_2;
    let gap = random.range(0.2, 0.3);
    let shift = [gap * away.cos(), gap * away.sin()];
    (line.get(from..to).unwrap_or_default().iter())
        .map(|p| [p[0] + shift[0], p[1] + shift[1]])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::{Combination, Draft, Touch, made};
    use crate::maths::path::{Point, distance, nearest};

    /// Whether `p` is an end of `line`.
    fn is_end(line: &[Point], p: Point) -> bool {
        [line.first(), line.last()]
            .into_iter()
            .flatten()
            .any(|end| distance(*end, p) < 1e-9)
    }

    /// Whether `touch` between the lines `a` and `b` of `group` joins them end to end.
    fn joins(group: &Draft, (a, b): (usize, usize), touch: Touch) -> bool {
        let lines = &group.lines;
        matches!(touch, Touch::Meet { at, .. } if is_end(&lines[a], at) && is_end(&lines[b], at))
    }

    /// Whether `p` lies inside the closed polyline `outline` (a ray to its right crossing it an odd
    /// number of times).
    fn inside(outline: &[Point], p: Point) -> bool {
        let mut inside = false;
        for edge in outline.windows(2) {
            let ([x0, y0], [x1, y1]) = (edge[0], edge[1]);
            if (y0 > p[1]) != (y1 > p[1]) && p[0] < x0 + (p[1] - y0) * (x1 - x0) / (y1 - y0) {
                inside = !inside;
            }
        }
        inside
    }

    #[test]
    fn an_enclosed_group_s_inner_lines_start_on_its_outline_and_stay_inside_it() {
        // How many groups had one line across, and two.
        let mut across = [0; 3];
        for seed in 0..300 {
            let (group, kept) = made(Combination::Enclosed, seed);
            assert!(kept.is_some(), "{seed}");
            // The outline: the lines joined end to end with two others, in their order.
            let joined = |line: usize| {
                (group.touches.iter())
                    .filter(|(a, b, t)| (*a == line || *b == line) && joins(&group, (*a, *b), *t))
                    .count()
            };
            let ring: Vec<usize> = (0..group.lines.len()).filter(|l| joined(*l) == 2).collect();
            assert!((3..=4).contains(&ring.len()), "{seed}: {ring:?}");
            let sides: Vec<&[Point]> = ring.iter().map(|l| group.lines[*l].as_slice()).collect();
            let outline: Vec<Point> = sides.iter().flat_map(|side| side.iter().copied()).collect();
            assert!(
                distance(outline[0], outline[outline.len() - 1]) < 1e-9,
                "{seed}: closed"
            );
            let on = |p: Point| nearest(&sides, p).is_some_and(|(_, off)| off < 1e-9);
            let mut chords = 0;
            for line in (0..group.lines.len()).filter(|l| !ring.contains(l)) {
                let line = &group.lines[line];
                assert!(on(line[0]), "{seed}: an inner line starts on the outline");
                let off = line.iter().filter(|p| !on(**p));
                assert!(off.clone().all(|p| inside(&outline, *p)), "{seed}: inside");
                chords += usize::from(on(line[line.len() - 1]));
            }
            across[chords] += 1;
        }
        // The inside cut into two or three shapes: one line across it or two, never none.
        assert!(
            across[0] == 0 && across[1] > 0 && across[2] > 0,
            "{across:?}"
        );
    }

    #[test]
    fn a_cluster_holds_five_or_six_lines_and_three_or_four_kinds_of_meeting() {
        let mut every_kind = 0;
        for seed in 0..300 {
            let (group, kept) = made(Combination::Cluster, seed);
            assert!(kept.is_some(), "{seed}");
            let count = group.lines.len();
            assert!((5..=6).contains(&count), "{seed}");
            // Joined end to end, leaving a line's middle, crossing, running near.
            let mut kinds = [false; 4];
            for &(a, b, touch) in &group.touches {
                let kind = match touch {
                    Touch::Meet { .. } if joins(&group, (a, b), touch) => 0,
                    Touch::Meet { .. } => 1,
                    Touch::Cross => 2,
                    Touch::Near => 3,
                    Touch::Apart => continue,
                };
                kinds[kind] = true;
            }
            let held = kinds.iter().filter(|kind| **kind).count();
            assert!(held >= 3, "{seed}: {kinds:?}");
            every_kind += usize::from(held == 4);
            // One group: every line meets another or runs near it.
            let touched =
                |line: usize| (group.touches.iter()).any(|(a, b, _)| *a == line || *b == line);
            assert!((0..count).all(touched), "{seed}");
        }
        assert!(every_kind > 100, "{every_kind} of 300 hold all four");
    }
}
