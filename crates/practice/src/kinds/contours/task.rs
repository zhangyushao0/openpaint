//! A contour exercise's task (S156k): one sausage lying on and above the ground, seen from a
//! little above as the solids' boxes are, its spine (`SpacePath`) through the points a turtle
//! walks (`walk.rs`). Its sections at even steps of the spine as the eye sees it, so a part
//! turning towards the eye holds as many as one running across; the asked ones ticked on its
//! outline; at 入门 the near end's given as the example.
//!
//! The rules hold or the sausage is laid again: it never meets itself (no two points of its spine
//! three radii and more apart along it nearer than 2.2 radii) and bends no tighter than 2.2 radii;
//! every section's ellipse between 0.12 and 0.9 round and 0.06 of the frame wide at least, each
//! ticked where the eye sees it, neighbours a tube's width apart; at 熟练 its sections opening as
//! it turns towards the eye; at 稳 the tube passing in front of itself; all of it in the frame.

use crate::geometry::perspective_eye::projected;
use crate::geometry::space_path::SpacePath;
use crate::kinds::contours::outline::outline;
use crate::kinds::contours::sausage::{SEGMENTS, Sausage};
use crate::kinds::contours::sections::Contour;
use crate::kinds::contours::walk::walked;
use crate::kinds::solids::solid_answer::View;
use crate::maths::path::{Point, distance};
use crate::maths::random::Random;
use crate::maths::vector::{Vector, unit};

/// Sausages laid before the generator keeps a plain one.
const TRIES: usize = 600;
/// The tube's radius (world units).
const RADIUS: [f64; 2] = [0.2, 0.24];
/// The longer side of what it covers in the frame; paper kept round it.
const SIDE: [f64; 2] = [0.72, 0.82];
const MARGIN: f64 = 0.04;
/// The rules: the tightest bend, how near two parts come, how far apart along it they are two
/// (radii); a section's roundness, its least width; neighbours' centres apart, a share of their
/// semi-major axes; at 熟练 how much more open the roundest is than the flattest.
const TIGHTEST: f64 = 2.2;
const NEAREST: f64 = 2.2;
const PARTS: f64 = 3.0;
const ROUND: [f64; 2] = [0.12, 0.9];
const WIDE: f64 = 0.06;
const APART: f64 = 1.0;
const OPENING: f64 = 0.3;
/// At 稳, how far the near part passes over the far one at the most, radii of the nearer.
const OVERLAP: [f64; 2] = [0.5, 1.2];
/// How far in from either end the first and last sections lie, a share of the spine.
const ENDS: f64 = 0.08;

/// A contour exercise's variant: how the sausage bends and what the level gives and asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bend {
    /// One gentle bend going back in depth; the spine printed, four ticks, the near end's contour
    /// given; the ellipses drawn through (入门).
    Gentle,
    /// One strong bend, a part turning towards the eye; the spine printed, five ticks (熟练).
    Strong,
    /// An S-bend passing in front of itself; no spine; five or six contour curves, the half that
    /// faces the eye, and one round each end (稳).
    Crossing,
}

impl Bend {
    /// Every variant, each stored by its place: a harder one goes at the end.
    pub const ALL: [Self; 3] = [Self::Gentle, Self::Strong, Self::Crossing];

    /// Whether its ellipses are drawn through, whole (else only the half facing the eye).
    #[must_use]
    pub const fn through(self) -> bool {
        !matches!(self, Self::Crossing)
    }
}

/// A contour exercise, generated the same for the same variant and seed.
#[derive(Debug, Clone, PartialEq)]
pub struct ContourTask {
    pub bend: Bend,
    /// The spine in the frame.
    pub spine: Vec<Point>,
    /// The tube's outline, traced from its picture.
    pub outline: Vec<Vec<Point>>,
    /// The near end's contour given as the example (入门); none else.
    pub example: Option<Contour>,
    /// The contours asked, from one end to the other, then round the ends (稳).
    pub contours: Vec<Contour>,
    pub(crate) sausage: Sausage,
    pub(crate) view: View,
    /// Where the light comes from (one long, towards it).
    pub(crate) light: Vector,
    /// The least and greatest frame point the tube reaches.
    pub(crate) extent: [Point; 2],
}

impl ContourTask {
    /// The task `bend` and `seed` make.
    #[must_use]
    pub fn generate(bend: Bend, seed: u64) -> Self {
        let mut random = Random::new(seed);
        (0..TRIES)
            .find_map(|_| laid(bend, &mut random, true))
            .unwrap_or_else(|| {
                let mut random = Random::new(0x0156_C047);
                laid(bend, &mut random, false).unwrap_or_else(|| unreachable!("laid without rules"))
            })
    }
}

/// One sausage laid from `random`; none when it breaks a rule (and `rules`).
fn laid(bend: Bend, random: &mut Random, rules: bool) -> Option<ContourTask> {
    let radius = random.range(RADIUS[0], RADIUS[1]);
    let points = walked(bend, radius, random);
    let sausage = Sausage::along(&SpacePath::through(&points)?, radius);
    let shaped = sausage.tightest() >= TIGHTEST * radius
        && sausage.closest_apart(PARTS * radius) >= NEAREST * radius;
    if rules && !shaped {
        return None;
    }
    let view = framed(&sausage, random);
    let asked = match bend {
        Bend::Gentle => 4,
        Bend::Strong => 5,
        Bend::Crossing => 5 + usize::from(random.unit() < 0.5),
    };
    let given = usize::from(bend == Bend::Gentle);
    let count = asked + given;
    let mut contours: Vec<Contour> = (placed(&sausage, view, count).into_iter())
        .map(|k| Contour::across(&sausage, k, view))
        .collect();
    if rules && !sectioned(bend, &contours, &sausage, view) {
        return None;
    }
    let example = (given == 1).then(|| contours.remove(0));
    if bend == Bend::Crossing {
        contours.extend([false, true].map(|last| Contour::round_end(&sausage, last, view)));
    }
    let reach = reached(&sausage, view);
    let outline = outline(&sausage, view, reach);
    let points = outline.iter().flatten();
    let extent = [0, 1].map(|end| {
        let pick = |k: usize| {
            let values = points.clone().map(|p| p[k]);
            if end == 0 {
                values.fold(1.0, f64::min)
            } else {
                values.fold(0.0, f64::max)
            }
        };
        [pick(0), pick(1)]
    });
    let inside = |v: f64| (MARGIN..=1.0 - MARGIN).contains(&v);
    let framed = extent.iter().all(|p| inside(p[0]) && inside(p[1]));
    if rules && !framed {
        return None;
    }
    let from_left = random.unit() < 0.5;
    Some(ContourTask {
        bend,
        spine: sausage.spine.iter().map(|p| view.at(*p)).collect(),
        outline,
        example,
        contours,
        light: unit([if from_left { -0.8 } else { 0.8 }, 1.5, -0.7]),
        sausage,
        view,
        extent,
    })
}

/// The capsules `count` sections lie in the middle of: at even steps of the spine as `view` shows
/// it, from `ENDS` of it in from either end.
fn placed(sausage: &Sausage, view: View, count: usize) -> Vec<usize> {
    // How far along the spine's image the middle of each capsule lies.
    let seen: Vec<Point> = sausage.spine.iter().map(|p| view.at(*p)).collect();
    let mut walked = 0.0;
    let middles: Vec<f64> = (seen.windows(2))
        .map(|pair| {
            let long = distance(pair[0], pair[1]);
            walked += long;
            walked - long / 2.0
        })
        .collect();
    (0..count)
        .map(|i| {
            let share = ENDS + (1.0 - 2.0 * ENDS) * i as f64 / (count - 1).max(1) as f64;
            let at = (middles.partition_point(|m| *m < share * walked)).min(SEGMENTS - 1);
            let before = at.saturating_sub(1);
            let nearer = (share * walked - middles[before]).abs() < (middles[at] - share * walked);
            if nearer { before } else { at }
        })
        .collect()
}

/// The view that frames `sausage`: the longer side of what it may cover `SIDE` long, near the
/// frame's middle.
fn framed(sausage: &Sausage, random: &mut Random) -> View {
    let [low, high] = covered(sausage);
    let long = (high[0] - low[0]).max(high[1] - low[1]).max(1e-9);
    let scale = random.range(SIDE[0], SIDE[1]) / long;
    let shift = [random.range(-0.03, 0.03), random.range(-0.03, 0.03)];
    let offset = [0, 1].map(|k| 0.5 + shift[k] - (low[k] + high[k]) / 2.0 * scale);
    View { scale, offset }
}

/// What the tube may cover on the eye's image: each spine point's ball's image, a little round.
fn covered(sausage: &Sausage) -> [Point; 2] {
    let mut low = [f64::INFINITY; 2];
    let mut high = [f64::NEG_INFINITY; 2];
    for p in &sausage.spine {
        let [x, y] = projected(*p);
        let reach = 1.2 * sausage.radius / p[2].max(1e-9);
        for k in 0..2 {
            low[k] = low[k].min([x, y][k] - reach);
            high[k] = high[k].max([x, y][k] + reach);
        }
    }
    [low, high]
}

/// What the tube may reach in the frame through `view`.
fn reached(sausage: &Sausage, view: View) -> [Point; 2] {
    let [low, high] = covered(sausage);
    let at = |p: Point| [0, 1].map(|k| p[k] * view.scale + view.offset[k]);
    let [low, high] = [at(low), at(high)];
    [low.map(|v| v - 0.01), high.map(|v| v + 0.01)]
}

/// Whether `contours` (the given one first at 入门) keep the sections' rules.
fn sectioned(bend: Bend, contours: &[Contour], sausage: &Sausage, view: View) -> bool {
    let shapes = contours.iter().all(|c| {
        let exact = c.guide.exact;
        (ROUND[0]..=ROUND[1]).contains(&exact.roundness()) && 2.0 * exact.axes[0] >= WIDE
    });
    let ticked = contours.iter().all(|c| !c.ticks.is_empty());
    let apart = contours.windows(2).all(|pair| {
        let [a, b] = [pair[0].guide.exact, pair[1].guide.exact];
        distance(a.centre, b.centre) >= APART * (a.axes[0] + b.axes[0])
    });
    let roundness = contours.iter().map(|c| c.guide.exact.roundness());
    let opening = roundness.clone().fold(0.0, f64::max) - roundness.fold(1.0, f64::min);
    let level = match bend {
        Bend::Gentle => sausage.spine[SEGMENTS][2] > sausage.spine[0][2] + sausage.radius,
        Bend::Strong => opening >= OPENING,
        Bend::Crossing => in_front(sausage, view),
    };
    shapes && ticked && apart && level
}

/// Whether the tube passes in front of itself legibly: two of its parts apart along it, their
/// balls' images overlapping by `OVERLAP` radii at the most, half a radius at least: the far part
/// hidden in part, never most of its width.
fn in_front(sausage: &Sausage, view: View) -> bool {
    let step = sausage.length() / SEGMENTS as f64;
    let gap = (PARTS * 2.0 * sausage.radius / step).ceil() as usize;
    let seen: Vec<(Point, f64)> = (sausage.spine.iter())
        .map(|p| (view.at(*p), sausage.radius / p[2] * view.scale))
        .collect();
    let mut most = f64::NEG_INFINITY;
    for i in 0..seen.len() {
        for j in (i + gap)..seen.len() {
            let ((a, ra), (b, rb)) = (seen[i], seen[j]);
            most = most.max((ra + rb - distance(a, b)) / ra.min(rb));
        }
    }
    (OVERLAP[0]..=OVERLAP[1]).contains(&most)
}
