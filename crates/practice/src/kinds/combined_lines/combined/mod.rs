//! Combined lines to copy (V大's W1 combined lines): lines in one frame, to be seen and copied as
//! one shape. Near-straight lines and curves mix, in four arrangements of two to four lines:
//! joined end to end (an open chain of corners, or a closed shape), branching off a line,
//! crossing, or running near one another without touching; and in two of four to six (S141,
//! V大's W2 wrapping and negative shapes, `wrapped.rs`): a closed shape with lines inside it, and
//! one big cluster mixing joins, branches, crossings and near runs.
//!
//! A group is made as the method copies it: its key points first (the ends, the corners, where a
//! line leaves or crosses another; `layouts.rs`), then each line between its two points with a
//! bend of its own (a single line's curvature, turned and sized so its ends are those points).
//! The whole group is then sized and placed in the frame as a single line is and held to its
//! arrangement's rules in frame units: every line long enough to read; lines meeting only where
//! they are meant to and opening clearly from there; a crossing once, away from the ends; lines
//! apart with a gap the eye sees; near lines near. A group breaking a rule is made again from
//! where the sequence stands, so the same seed gives the same group.

use crate::kinds::lines::line::Placing;
use crate::maths::path::{Point, apart, crossings, distance, length, resample};
use crate::maths::random::Random;

mod layouts;
mod wrapped;

/// Groups made at most for one seed before the last is taken (every seed tested keeps the rules
/// well within).
const TRIES: usize = 64;
/// The rules, in frame sides: the shortest line, the least gap between lines apart, how close
/// near lines come at least and at most, how far round a meeting or a crossing lines may run
/// close, the least gap beyond it.
const SHORTEST: f64 = 0.16;
const GAP: f64 = 0.05;
const NEAR: [f64; 2] = [0.04, 0.2];
const AROUND: f64 = 0.045;
const OPENING: f64 = 0.012;
/// The least angle, in degrees, between two lines where they meet at a corner or cross, and
/// from a straight run on: a corner the eye finds, a crossing that is plainly one.
const ANGLE: f64 = 30.0;
/// The spacing the rules look at the lines with, in frame sides.
const LOOKED_AT: f64 = 0.004;

/// How the lines of a group lie together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Combination {
    /// End to end: an open chain of corners, or a closed shape.
    Joined,
    /// Lines leaving a line from its middle, or two from its end.
    Branched,
    /// Lines crossing one another.
    Crossing,
    /// Lines running near one another, never touching.
    Near,
    /// A closed shape of three or four lines, one or two lines inside it from its outline (S141).
    Enclosed,
    /// Five or six lines in one group: a chain, a branch, a crossing, a near run (S141).
    Cluster,
}

impl Combination {
    pub const ALL: [Self; 6] = [
        Self::Joined,
        Self::Branched,
        Self::Crossing,
        Self::Near,
        Self::Enclosed,
        Self::Cluster,
    ];
}

/// A group of lines to copy, in frame units (the frame from `[0, 0]` to `[1, 1]`, y down).
#[derive(Debug, Clone, PartialEq)]
pub struct CombinedLines {
    /// Each line densely, as a single line is.
    pub lines: Vec<Vec<Point>>,
}

impl CombinedLines {
    /// The group of `combination` that `seed` makes.
    #[must_use]
    pub fn generate(combination: Combination, seed: u64) -> Self {
        Self {
            lines: made(combination, seed).0.lines,
        }
    }
}

/// How two lines of a group are meant to lie together.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Touch {
    /// Apart, with a gap the eye sees.
    Apart,
    /// Apart, and near.
    Near,
    /// Meeting at the point `at`, and only there; at a corner the eye finds (`corner`), or
    /// either way (a fork's arm going on from its stem).
    Meet { at: Point, corner: bool },
    /// Crossing once, plainly.
    Cross,
}

/// A group laid out before it is placed: its lines, and how pairs of them touch (a pair not
/// named lies apart).
struct Draft {
    lines: Vec<Vec<Point>>,
    touches: Vec<(usize, usize, Touch)>,
}

/// The group of `combination` that `seed` makes, placed, and the try that kept the rules (none:
/// the last try, taken as it is).
fn made(combination: Combination, seed: u64) -> (Draft, Option<usize>) {
    let mut random = Random::new(seed);
    let mut last = Draft {
        lines: Vec::new(),
        touches: Vec::new(),
    };
    for attempt in 0..TRIES {
        let draft = layouts::draft(combination, &mut random);
        // Groups of many lines a little larger, their inner shapes room to read.
        let size = match combination {
            Combination::Enclosed | Combination::Cluster => random.range(0.64, 0.76),
            _ => random.range(0.56, 0.76),
        };
        let drawn: Vec<&[Point]> = draft.lines.iter().map(Vec::as_slice).collect();
        let placing = Placing::of(&drawn, size, &mut random);
        let lines: Vec<Vec<Point>> = (draft.lines.iter())
            .map(|line| line.iter().map(|p| placing.apply(*p)).collect())
            .collect();
        let touches: Vec<(usize, usize, Touch)> = (draft.touches.iter())
            .map(|&(a, b, touch)| match touch {
                Touch::Meet { at, corner } => (
                    a,
                    b,
                    Touch::Meet {
                        at: placing.apply(at),
                        corner,
                    },
                ),
                other => (a, b, other),
            })
            .collect();
        let kept = keeps_rules(&lines, &touches);
        last = Draft { lines, touches };
        if kept {
            return (last, Some(attempt));
        }
    }
    (last, None)
}

/// Whether `lines` keep their arrangement's rules, `touches` saying how pairs touch.
fn keeps_rules(lines: &[Vec<Point>], touches: &[(usize, usize, Touch)]) -> bool {
    if lines.iter().any(|line| length(line) < SHORTEST) {
        return false;
    }
    let looked: Vec<Vec<Point>> = lines.iter().map(|l| resample(l, LOOKED_AT)).collect();
    for a in 0..looked.len() {
        for b in a + 1..looked.len() {
            let touch = touches
                .iter()
                .find(|(i, j, _)| (*i, *j) == (a, b) || (*i, *j) == (b, a))
                .map_or(Touch::Apart, |(_, _, touch)| *touch);
            if !keeps(&looked[a], &looked[b], touch) {
                return false;
            }
        }
    }
    true
}

/// Whether the lines `a` and `b` lie together as `touch` says.
fn keeps(a: &[Point], b: &[Point], touch: Touch) -> bool {
    match touch {
        Touch::Apart => apart(a, b) >= GAP,
        Touch::Near => (NEAR[0]..=NEAR[1]).contains(&apart(a, b)),
        Touch::Meet { at, corner } => {
            let (a, b) = (away(a, at), away(b, at));
            let open = (a.iter()).all(|a| b.iter().all(|b| apart(a, b) >= OPENING));
            let turns = |piece: &Vec<Point>| leaving(piece, at);
            let mut angles = (a.iter().filter_map(turns))
                .flat_map(|u| b.iter().filter_map(turns).map(move |v| between(u, v)));
            open && (!corner || angles.all(plain))
        }
        Touch::Cross => match crossings(a, b)[..] {
            [at] => {
                let clear = [a, b]
                    .iter()
                    .flat_map(|line| [line.first(), line.last()])
                    .flatten()
                    .all(|end| distance(*end, at) >= AROUND);
                let directions = (heading(a, at), heading(b, at));
                let plainly = match directions {
                    (Some(u), Some(v)) => plain(between(u, v)),
                    _ => false,
                };
                clear && plainly
            }
            _ => false,
        },
    }
}

/// Whether an angle between two lines (degrees) is plainly neither a straight run on nor a
/// fold back.
fn plain(degrees: f64) -> bool {
    (ANGLE..=180.0 - ANGLE).contains(&degrees)
}

/// The angle between the directions `u` and `v`, in degrees.
fn between(u: Point, v: Point) -> f64 {
    let cos = (u[0] * v[0] + u[1] * v[1]) / (u[0].hypot(u[1]) * v[0].hypot(v[1])).max(1e-12);
    cos.clamp(-1.0, 1.0).acos().to_degrees()
}

/// The way `piece` (a line's part beyond `AROUND` from `at`) leaves `at`: from `at` to its end
/// nearer `at`.
fn leaving(piece: &[Point], at: Point) -> Option<Point> {
    let (first, last) = (piece.first()?, piece.last()?);
    let near = if distance(*first, at) <= distance(*last, at) {
        first
    } else {
        last
    };
    Some([near[0] - at[0], near[1] - at[1]])
}

/// The direction of `line` through its point nearest `at`, over `AROUND` either side.
fn heading(line: &[Point], at: Point) -> Option<Point> {
    let nearest =
        (0..line.len()).min_by(|i, j| distance(line[*i], at).total_cmp(&distance(line[*j], at)))?;
    let reach = (AROUND / LOOKED_AT).ceil() as usize;
    let (from, to) = (
        nearest.saturating_sub(reach),
        (nearest + reach).min(line.len() - 1),
    );
    let (a, b) = (line[from], line[to]);
    (from < to).then(|| [b[0] - a[0], b[1] - a[1]])
}

/// The pieces of `line` farther than `AROUND` from `at`.
fn away(line: &[Point], at: Point) -> Vec<Vec<Point>> {
    let mut pieces = Vec::new();
    let mut piece = Vec::new();
    for &p in line {
        if distance(p, at) > AROUND {
            piece.push(p);
        } else if !piece.is_empty() {
            pieces.push(std::mem::take(&mut piece));
        }
    }
    if !piece.is_empty() {
        pieces.push(piece);
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_arrangement_keeps_its_rules_seed_after_seed() {
        for combination in Combination::ALL {
            let mut tries = 0;
            let lines = match combination {
                Combination::Enclosed => 4..=6,
                Combination::Cluster => 5..=6,
                _ => 2..=4,
            };
            for seed in 0..500 {
                let (group, kept) = made(combination, seed);
                let Some(attempt) = kept else {
                    panic!("{combination:?} {seed}: no group kept the rules");
                };
                tries += attempt + 1;
                assert!(lines.contains(&group.lines.len()), "{combination:?} {seed}");
            }
            // Made again seldom: a sheet's groups cost little to make.
            assert!(
                tries < 500 * 4,
                "{combination:?}: {tries} tries for 500 seeds"
            );
        }
    }

    #[test]
    fn the_rules_tell_apart_from_touching_and_crossing() {
        let across: Vec<Point> = vec![[0.2, 0.5], [0.8, 0.5]];
        let above: Vec<Point> = vec![[0.2, 0.4], [0.8, 0.4]];
        let close: Vec<Point> = vec![[0.2, 0.52], [0.8, 0.52]];
        let down: Vec<Point> = vec![[0.5, 0.2], [0.5, 0.8]];
        let corner: Vec<Point> = vec![[0.8, 0.5], [0.8, 0.2]];
        assert!(keeps(&across, &above, Touch::Apart) && keeps(&across, &above, Touch::Near));
        assert!(!keeps(&across, &close, Touch::Apart));
        assert!(!keeps(&across, &down, Touch::Apart));
        assert!(!keeps(&across, &above, Touch::Cross));
        let (across, corner) = (resample(&across, 0.004), resample(&corner, 0.004));
        let meet = |corner| Touch::Meet {
            at: [0.8, 0.5],
            corner,
        };
        assert!(keeps(&across, &corner, meet(true)));
        assert!(
            !keeps(&across, &corner, Touch::Cross),
            "a crossing at an end"
        );
        let folded: Vec<Point> = resample(&[[0.8, 0.5], [0.3, 0.51]], 0.004);
        assert!(!keeps(&across, &folded, meet(false)));
        // Running on nearly straight: no corner to find.
        let on: Vec<Point> = resample(&[[0.8, 0.5], [0.95, 0.53]], 0.004);
        assert!(!keeps(&across, &on, meet(true)) && keeps(&across, &on, meet(false)));
        let (down, shallow) = (
            resample(&down, 0.004),
            resample(&[[0.2, 0.45], [0.8, 0.55]], 0.004),
        );
        assert!(keeps(&across, &down, Touch::Cross));
        assert!(
            !keeps(&across, &shallow, Touch::Cross),
            "too shallow a crossing"
        );
    }
}
