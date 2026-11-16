//! An organic arrow's ribbon (S156l, Drawabox's arrows): a flat ribbon of one width flowing back
//! through space along a smooth path (`geometry/space_path`), seen by a level eye at the world's
//! origin (y up, z away, its focal length 1). The path leaves the near ground below the eye and
//! weaves side to side as it goes back, its weave fixed in the world, so in the picture the ribbon
//! narrows and its bends come closer far away. It stands like a fence: its width lies along the
//! path's normal started from up and carried along, turned over by half turns at 稳 (its back
//! showing). A bend is where the path, seen, turns back across the picture.

use std::f64::consts::PI;

use crate::geometry::space_path::{Sample, SpacePath, Turn};
use crate::maths::path::Point;
use crate::maths::random::Random;
use crate::maths::vector::{Vector, add, times};

/// The samples along the ribbon: its edges, its depth and its folds are found at these.
pub(crate) const SAMPLES: usize = 721;
/// Up: the ribbon stands like a fence.
const UP: Vector = [0.0, 1.0, 0.0];
/// The near end's depth.
const NEAR: [f64; 2] = [2.6, 3.0];
/// How far the weave drifts sideways from end to end, either way (world).
const DRIFT: f64 = 0.35;
/// How much each weave's reach and each point's depth step vary.
const REACH: [f64; 2] = [0.8, 1.15];
const STEP: f64 = 0.06;
/// How far the path rises or falls from one point to the next (world).
const RISE: f64 = 0.08;
/// A half turn's span (a share of the length), the room kept either side of it within its
/// stretch between two bends, and where along the ribbon a half turn may lie (the far end's are
/// too small to read).
const SPAN: f64 = 0.09;
const ROOM: f64 = 0.035;
const TURNED: [f64; 2] = [0.12, 0.72];

/// An organic arrow's variant: a level's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arrow {
    /// A gentle weave going back, no stretch crossing another, no turn; the far width given
    /// (入门).
    Gentle,
    /// A tighter weave going farther back, its far stretches crossing behind its near ones (熟练).
    Crossing,
    /// Crossings and one or two half turns, the ribbon's back showing (稳).
    Turning,
}

impl Arrow {
    /// Every variant, each stored by its place: a harder one goes at the end.
    pub const ALL: [Self; 3] = [Self::Gentle, Self::Crossing, Self::Turning];

    /// How its weave is laid.
    const fn weave(self) -> Weave {
        match self {
            Self::Gentle => Weave {
                points: 5,
                back: [3.6, 4.4],
                reach: [0.55, 0.8],
                half: [0.07, 0.09],
                below: [1.1, 1.25],
                turns: 0,
            },
            Self::Crossing => Weave {
                points: 7,
                back: [3.6, 4.4],
                reach: [0.6, 0.85],
                half: [0.14, 0.18],
                below: [0.95, 1.05],
                turns: 0,
            },
            Self::Turning => Weave {
                points: 5,
                back: [3.2, 4.0],
                reach: [0.6, 0.85],
                half: [0.085, 0.105],
                below: [1.05, 1.15],
                turns: 2,
            },
        }
    }
}

/// How a level's weave is laid: the points the path runs through, how far back it goes (the far
/// end's depth over the near's), how far it weaves either side (world), the ribbon's half width
/// and how far below the eye it flies (world), and at most how many half turns it makes.
struct Weave {
    points: usize,
    back: [f64; 2],
    reach: [f64; 2],
    half: [f64; 2],
    below: [f64; 2],
    turns: usize,
}

/// A ribbon in the world: its path, `SAMPLES` samples along it with their frames (the width along
/// each normal), its half width, its half turns and where it bends (sample indices).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Ribbon {
    pub(crate) path: SpacePath,
    pub(crate) samples: Vec<Sample>,
    pub(crate) half: f64,
    pub(crate) turns: Vec<Turn>,
    pub(crate) bends: Vec<usize>,
}

impl Ribbon {
    /// One ribbon of `arrow` laid from `random`, its half turns as many as its stretches between
    /// bends give room for (none, where `arrow` asks some, breaks its rules); none were its path to
    /// hold no point.
    pub(crate) fn laid(arrow: Arrow, random: &mut Random) -> Option<Self> {
        let weave = arrow.weave();
        let half = random.range(weave.half[0], weave.half[1]);
        let path = SpacePath::through(&points(&weave, random))?;
        let plain = path.samples(SAMPLES, UP, &[]);
        let bends = bends(&plain);
        let turns = turns(weave.turns, &bends, random);
        let samples = if turns.is_empty() {
            plain
        } else {
            path.samples(SAMPLES, UP, &turns)
        };
        Some(Self {
            path,
            samples,
            half,
            turns,
            bends,
        })
    }

    /// The point `u` across the ribbon from its middle at sample `i` (world): `half` on its first
    /// edge, minus `half` on its second.
    pub(crate) fn across(&self, i: usize, u: f64) -> Vector {
        let sample = &self.samples[i];
        add(sample.at, times(sample.normal, u))
    }

    /// The point `u` across the ribbon at the place `at` between samples (a sample's index and a
    /// share of the way to the next), its middle and normal taken straight between them.
    pub(crate) fn between(&self, at: f64, u: f64) -> Vector {
        let last = SAMPLES - 1;
        let i = (at.floor().max(0.0) as usize).min(last - 1);
        let share = (at - i as f64).clamp(0.0, 1.0);
        let [a, b] = [i, i + 1].map(|k| self.across(k, u));
        add(times(a, 1.0 - share), times(b, share))
    }

    /// Whether sample `i` lies within a half turn's span.
    pub(crate) fn turning(&self, i: usize) -> bool {
        let s = share(i);
        (self.turns.iter()).any(|turn| (s - turn.at).abs() <= turn.span / 2.0)
    }
}

/// Sample `i`'s share of the length.
pub(crate) fn share(i: usize) -> f64 {
    i as f64 / (SAMPLES - 1) as f64
}

/// Where the level eye sees the world point `p`: right and down over its depth.
pub(crate) fn image(p: Vector) -> Point {
    [p[0] / p[2], -p[1] / p[2]]
}

/// How the eye's picture lies in the frame: made `scale` frame units an image unit and moved by
/// `offset` (a picture made larger and moved is the same view through a longer lens).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct View {
    pub(crate) scale: f64,
    pub(crate) offset: Point,
}

impl View {
    /// The view laying the images `points` in the frame's middle, their longer side `fill` long.
    pub(crate) fn fitted(points: &[Point], fill: f64) -> Self {
        let low = [0, 1].map(|k| points.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min));
        let high = [0, 1].map(|k| {
            points
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        let scale = fill / (high[0] - low[0]).max(high[1] - low[1]).max(1e-9);
        let offset = [0, 1].map(|k| 0.5 - scale * (low[k] + high[k]) / 2.0);
        Self { scale, offset }
    }

    /// Where the world point `p` lies in the frame.
    pub(crate) fn frame(&self, p: Vector) -> Point {
        let [x, y] = image(p);
        [
            self.offset[0] + self.scale * x,
            self.offset[1] + self.scale * y,
        ]
    }
}

/// The path's points: from the near end below the eye back to the far end, the depth stepping by
/// a like share each time, weaving to either side by turns, drifting and rising a little.
fn points(weave: &Weave, random: &mut Random) -> Vec<Vector> {
    let near = random.range(NEAR[0], NEAR[1]);
    let back = random.range(weave.back[0], weave.back[1]);
    let reach = random.range(weave.reach[0], weave.reach[1]);
    let below = random.range(weave.below[0], weave.below[1]);
    let side = random.sign();
    let drift = [random.range(-DRIFT, DRIFT), random.range(-DRIFT, DRIFT)];
    let last = (weave.points - 1) as f64;
    (0..weave.points)
        .map(|k| {
            let along = k as f64 / last;
            let inner = k > 0 && k + 1 < weave.points;
            let stepped = if inner {
                along + random.range(-STEP, STEP) / last
            } else {
                along
            };
            let depth = near * back.powf(stepped);
            let way = if k % 2 == 0 { side } else { -side };
            let x = way * reach * random.range(REACH[0], REACH[1]);
            let drifted = drift[0] + (drift[1] - drift[0]) * along;
            [x + drifted, -below + random.range(-RISE, RISE), depth]
        })
        .collect()
}

/// Where `samples`' middle, seen, turns back across the picture: the samples where its image's
/// sideways step changes sign.
fn bends(samples: &[Sample]) -> Vec<usize> {
    let across: Vec<f64> = samples.iter().map(|sample| image(sample.at)[0]).collect();
    (1..across.len() - 1)
        .filter(|&i| {
            let (before, after) = (across[i] - across[i - 1], across[i + 1] - across[i]);
            before * after < 0.0 || (before != 0.0 && after == 0.0)
        })
        .collect()
}

/// One or `most` half turns (none for none), each in the later half of a stretch between two bends
/// with room either side (a stretch's start lies behind the stretch before it, near the bend), in
/// `TURNED`, a stretch each, laid from `random`; fewer where the stretches give less room.
fn turns(most: usize, bends: &[usize], random: &mut Random) -> Vec<Turn> {
    if most == 0 {
        return Vec::new();
    }
    let ends: Vec<f64> = std::iter::once(0.0)
        .chain(bends.iter().map(|&i| share(i)))
        .chain([1.0])
        .collect();
    let mut rooms: Vec<[f64; 2]> = (ends.windows(2))
        .map(|w| [w[0] + ROOM + SPAN / 2.0, w[1] - ROOM - SPAN / 2.0])
        .map(|[from, to]| [from.max(TURNED[0]), to.min(TURNED[1])])
        .filter(|[from, to]| from <= to)
        .collect();
    let count = if most > 1 && random.unit() < 0.5 {
        most
    } else {
        1
    };
    let mut laid = Vec::with_capacity(count);
    for _ in 0..count {
        if rooms.is_empty() {
            break;
        }
        let pick = (random.next_u64() % rooms.len() as u64) as usize;
        let [from, to] = rooms.remove(pick);
        laid.push(Turn {
            at: random.range(from, to),
            span: SPAN,
            by: random.sign() * PI,
        });
    }
    laid.sort_by(|a, b| a.at.total_cmp(&b.at));
    laid
}
