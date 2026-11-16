//! A curved object's make-up (S154f, V大's weeks 7 and 8: a curved object split into hemispheres,
//! cylinders and cones): a body turned about one axis, two to four segments along it (a cylinder,
//! a frustum, a cone at an end, a hemisphere at an end), and from 熟练 on a spout (a frustum across
//! the axis) or a handle (a squared profile pulled through) on its side. In its own terms: along
//! the axis, shares of its length; across it, diameters as shares of its box's width. Every joint
//! lies at a half, a third or a quarter of the length, a sixth from the next at least; every
//! diameter is the width, a half or a third of it, so V大's tricks find each section's square in
//! the box; the widest is the width itself: the box is tight round the body.
//!
//! A hemisphere is as long as its radius, so it sets the body's length against its width; it
//! never stands wider than the end it sits on (its ball's inner half lies in that neighbour).

use crate::maths::random::Random;

/// Where joints may lie along the axis, shares of the length; the least length of a segment.
const JOINTS: [f64; 5] = [0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 0.75];
const SHORTEST: f64 = 1.0 / 6.0;
/// The diameters a section may have, shares of the box's width.
pub(crate) const WIDTHS: [f64; 3] = [1.0 / 3.0, 0.5, 1.0];
/// The body's length over its width, at least and at most.
const PROPORTION: [f64; 2] = [0.8, 2.2];
/// A frustum's or a cone's side turned from the axis at most this much (the tangent: 45 degrees).
const STEEPEST: f64 = 1.0;
/// How often a joint keeps its diameter (else it steps to another).
const CONTINUOUS: f64 = 0.65;
/// A spout's diameter where it leaves the box's side, its tip's share of it, how far its tip
/// reaches beyond the side (shares of the width); a handle's bars' thickness and depth, and its
/// reach beyond the side.
const SPOUT_WIDE: [f64; 2] = [0.25, 1.0 / 3.0];
const TIP: f64 = 0.6;
const SPOUT_REACH: [f64; 2] = [1.0 / 3.0, 0.5];
pub(crate) const BAR: f64 = 0.1;
pub(crate) const DEEP: f64 = 0.25;
const HANDLE_REACH: [f64; 2] = [0.25, 1.0 / 3.0];
/// A spout's root across the axis at most this share of its host's radius.
const ROOT: f64 = 0.75;

/// What a segment is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SegmentKind {
    Cylinder,
    /// A cone cut short: its two ends' diameters differ, neither none.
    Frustum,
    /// A cone at an end of the body, its point there.
    Cone,
    /// A hemisphere at an end, its flat face on its neighbour, its top the body's end.
    Dome,
}

/// A segment: what it is, where it runs along the axis and its diameters at its two ends (a
/// cone's point none; a dome's both its flat face's).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub shape: SegmentKind,
    pub along: [f64; 2],
    pub wide: [f64; 2],
}

/// What a variant puts on the body's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Attached {
    Nothing,
    Spout,
    Handle,
}

/// What stands on the body's side: a spout on its side `+x`, a handle on `-x`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Attachment {
    /// A frustum across the axis from it, its middle `along`, its diameters where it leaves the
    /// box's side and at its tip, its tip `reach` beyond the side.
    Spout {
        along: f64,
        wide: [f64; 2],
        reach: f64,
    },
    /// A squared profile pulled through: two bars from the axis, the lower's foot and the upper's
    /// top at `along`, joined by an upright whose outer face lies `reach` beyond the box's side;
    /// `BAR` thick, `DEEP` through.
    Handle { along: [f64; 2], reach: f64 },
}

/// A curved object's make-up: its segments from its foot, what stands on its side, and its
/// length over its width.
#[derive(Debug, Clone, PartialEq)]
pub struct Design {
    pub segments: Vec<Segment>,
    pub attachment: Option<Attachment>,
    pub proportion: f64,
}

/// A design of `segments` (at least and at most) with `attached` on its side, laid from `random`;
/// none when it breaks a rule (the caller lays another).
pub(crate) fn designed(
    segments: [u8; 2],
    attached: Attached,
    random: &mut Random,
) -> Option<Design> {
    let [least, most] = segments.map(usize::from);
    let count = least + (random.next_u64() % (most - least + 1) as u64) as usize;
    let joints = joints(count, random);
    let shapes = shapes(count, random);
    let wide = widths(&shapes, random);
    let segments: Vec<Segment> = (0..count)
        .map(|k| Segment {
            shape: shapes[k],
            along: [joints[k], joints[k + 1]],
            wide: wide[k],
        })
        .collect();
    let tight = segments.iter().any(|s| s.wide.contains(&1.0));
    if !tight || !segments.windows(2).all(|pair| joined(&pair[0], &pair[1])) {
        return None;
    }
    let dome = segments.iter().find(|s| s.shape == SegmentKind::Dome);
    let proportion = match dome {
        Some(dome) => dome.wide[0] / (2.0 * (dome.along[1] - dome.along[0])),
        None => random.range(PROPORTION[0], PROPORTION[1]),
    };
    // No side flatter than `STEEPEST` to the axis: a plate, not a segment the eye reads.
    let steep = |s: &Segment| {
        (s.wide[1] - s.wide[0]).abs() / (2.0 * (s.along[1] - s.along[0]) * proportion)
    };
    let flat = (segments.iter()).any(|s| s.shape != SegmentKind::Dome && steep(s) > STEEPEST);
    if flat || !(PROPORTION[0] - 1e-9..=PROPORTION[1] + 1e-9).contains(&proportion) {
        return None;
    }
    let mut design = Design {
        segments,
        attachment: None,
        proportion,
    };
    design.attachment = match attached {
        Attached::Nothing => None,
        Attached::Spout => Some(spout(&design, random)?),
        Attached::Handle => Some(handle(&design, random)?),
    };
    Some(design)
}

/// The ends and joints of `count` segments: 0, joints a sixth apart at least, 1.
fn joints(count: usize, random: &mut Random) -> Vec<f64> {
    let mut held: Vec<Vec<f64>> = Vec::new();
    for mask in 0_u32..(1 << JOINTS.len()) {
        if mask.count_ones() as usize + 1 != count {
            continue;
        }
        let mut all = vec![0.0];
        all.extend(
            (0..JOINTS.len())
                .filter(|k| mask & (1 << k) != 0)
                .map(|k| JOINTS[k]),
        );
        all.push(1.0);
        if all
            .windows(2)
            .all(|pair| pair[1] - pair[0] >= SHORTEST - 1e-9)
        {
            held.push(all);
        }
    }
    held.swap_remove((random.next_u64() % held.len() as u64) as usize)
}

/// Each of `count` segments' shape: a dome or a cone at an end now and then (one dome at most),
/// cylinders and frustums between.
fn shapes(count: usize, random: &mut Random) -> Vec<SegmentKind> {
    let plain = |random: &mut Random| {
        if random.unit() < 0.5 {
            SegmentKind::Cylinder
        } else {
            SegmentKind::Frustum
        }
    };
    let mut shapes: Vec<SegmentKind> = (0..count).map(|_| plain(random)).collect();
    let foot = random.unit();
    if foot < 0.15 {
        shapes[0] = SegmentKind::Dome;
    } else if foot < 0.25 {
        shapes[0] = SegmentKind::Cone;
    }
    let top = random.unit();
    if top < 0.3 && shapes[0] != SegmentKind::Dome {
        shapes[count - 1] = SegmentKind::Dome;
    } else if top < 0.45 {
        shapes[count - 1] = SegmentKind::Cone;
    }
    shapes
}

/// Each segment's diameters, walked from the foot: a joint keeps its diameter or steps to another;
/// a dome never wider than the end it sits on.
fn widths(shapes: &[SegmentKind], random: &mut Random) -> Vec<[f64; 2]> {
    let pick =
        |random: &mut Random, from: &[f64]| from[(random.next_u64() % from.len() as u64) as usize];
    let last = shapes.len() - 1;
    let mut out: Vec<[f64; 2]> = Vec::new();
    // The previous end's diameter, and the least the next start may be (above a dome).
    let mut before: Option<f64> = None;
    let mut least = 0.0;
    for (k, shape) in shapes.iter().enumerate() {
        let wider: Vec<f64> = WIDTHS.into_iter().filter(|w| *w >= least).collect();
        let start = match before {
            Some(w) if random.unit() < CONTINUOUS => w,
            _ => pick(random, &wider),
        };
        least = 0.0;
        let wide = match (shape, k) {
            (SegmentKind::Cylinder, _) => [start, start],
            (SegmentKind::Frustum, _) => {
                let other: Vec<f64> = WIDTHS.into_iter().filter(|w| *w != start).collect();
                [start, pick(random, &other)]
            }
            (SegmentKind::Cone, 0) => [0.0, start],
            (SegmentKind::Cone, _) => [start, 0.0],
            (SegmentKind::Dome, 0) => {
                least = start;
                [start, start]
            }
            (SegmentKind::Dome, _) => {
                let under = before.unwrap_or(1.0);
                let narrower: Vec<f64> = WIDTHS.into_iter().filter(|w| *w <= under).collect();
                let dome = if random.unit() < CONTINUOUS {
                    under
                } else {
                    pick(random, &narrower)
                };
                [dome, dome]
            }
        };
        out.push(wide);
        before = (k < last).then_some(wide[1]);
    }
    out
}

/// Whether `below` and `above` meet at a joint the eye can tell or that the course names: a step,
/// or a turn of the surface; never one cylinder cut in two or one frustum's slope run on.
fn joined(below: &Segment, above: &Segment) -> bool {
    let slope = |s: &Segment| (s.wide[1] - s.wide[0]) / (s.along[1] - s.along[0]);
    let same = (below.wide[1] - above.wide[0]).abs() <= 1e-9;
    let straight = |s: &Segment| {
        matches!(
            s.shape,
            SegmentKind::Cylinder | SegmentKind::Frustum | SegmentKind::Cone
        )
    };
    !(same && straight(below) && straight(above) && (slope(below) - slope(above)).abs() <= 1e-9)
}

/// A spout on a cylinder or frustum at least half the width, at its middle; its root inside it.
fn spout(design: &Design, random: &mut Random) -> Option<Attachment> {
    let hosts: Vec<&Segment> = (design.segments.iter())
        .filter(|s| matches!(s.shape, SegmentKind::Cylinder | SegmentKind::Frustum))
        .filter(|s| s.wide[0].min(s.wide[1]) >= 0.5)
        .collect();
    let host = *hosts.get((random.next_u64() % hosts.len().max(1) as u64) as usize)?;
    let face = SPOUT_WIDE[(random.next_u64() % 2) as usize];
    let reach = SPOUT_REACH[(random.next_u64() % 2) as usize];
    let along = (host.along[0] + host.along[1]) / 2.0;
    let attachment = Attachment::Spout {
        along,
        wide: [face, face * TIP],
        reach,
    };
    // Its root across the axis, and where it spans the host along it (shares of the width).
    let root = spout_root(face, reach);
    let narrowest = host.wide[0].min(host.wide[1]) / 2.0;
    let span = root / design.proportion;
    let inside = along - span >= host.along[0] + 0.02 && along + span <= host.along[1] - 0.02;
    (root <= ROOT * narrowest && inside).then_some(attachment)
}

/// A spout's radius on the axis (shares of the width) for its diameter `face` where it leaves the
/// box's side and its tip `reach` beyond it: its side run on straight to the axis.
pub(crate) fn spout_root(face: f64, reach: f64) -> f64 {
    let [at_face, at_tip] = [face / 2.0, face * TIP / 2.0];
    at_face + (at_face - at_tip) * 0.5 / reach
}

/// A handle on the widest cylinder or frustum a third long at least: its bars at a quarter, a
/// third or a half, a third apart at least, inside it.
fn handle(design: &Design, random: &mut Random) -> Option<Attachment> {
    let host = (design.segments.iter())
        .filter(|s| matches!(s.shape, SegmentKind::Cylinder | SegmentKind::Frustum))
        .filter(|s| s.along[1] - s.along[0] >= 1.0 / 3.0 - 1e-9)
        .max_by(|a, b| {
            a.wide[0]
                .min(a.wide[1])
                .total_cmp(&b.wide[0].min(b.wide[1]))
        })?;
    let margin = BAR / design.proportion;
    let places: Vec<f64> = (JOINTS.into_iter())
        .filter(|p| *p >= host.along[0] + margin && *p <= host.along[1] - margin)
        .collect();
    let pairs: Vec<[f64; 2]> = (places.iter())
        .flat_map(|low| places.iter().map(move |high| [*low, *high]))
        .filter(|[low, high]| high - low >= 1.0 / 3.0 - 1e-9)
        .collect();
    let along = *pairs.get((random.next_u64() % pairs.len().max(1) as u64) as usize)?;
    let reach = HANDLE_REACH[(random.next_u64() % 2) as usize];
    Some(Attachment::Handle { along, reach })
}
