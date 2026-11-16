//! Where two forms meet, exact (S156k), and how it is found (Drawabox's form intersections, V大's
//! weeks 7 and 8: where the parts pass into the main body, marked before they are built):
//!
//! - Two boxes: where a face of one crosses a face of the other, a straight piece between the
//!   points where an edge of either pierces the other's face; found by carrying each edge into the
//!   other's face towards its vanishing point.
//! - A cylinder through a box's face: its circle in the face, the exact ellipse in its square; found
//!   by the square on the face, its diagonals and middles.
//! - A box into a cylinder's side: lines along the side where the box's faces along it cut it, arcs
//!   of its sections where the faces across it do; found by the sections at the box's faces and the
//!   lines along the side, carried over its length.
//! - Two cylinders, or a ball sunk into a cylinder: round one cylinder, its side's line at each turn
//!   carried to the other surface (`track.rs`); found by a few sections across both forms, where
//!   each section's lines cross.
//!
//! The scene's own coordinates: world lengths along the box's width, height and depth (`Placed`).

use crate::kinds::contours::sections::Circle;
use crate::kinds::intersections::forms::{Form, others, touches};
use crate::kinds::intersections::track::{Side, Track};
use crate::kinds::objects::object_body::Placed;
use crate::kinds::objects::object_design::{Block, Local};
use crate::maths::vector::{Vector, norm, sub, times};

/// The sections across both forms' share of the entering form's radius either side of its middle.
const ACROSS: f64 = 0.6;

/// Where two forms meet, in the world: the line in tracks, each pierce point with the form it
/// pierces (0 the main one, 1 the other), and the way to find it: lines carried along faces and
/// sides, sections' squares and circles, the points the sections find.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Met {
    pub(crate) tracks: Vec<Track>,
    pub(crate) pierce: Vec<(Vector, usize)>,
    pub(crate) lines: Vec<[Vector; 2]>,
    pub(crate) squares: Vec<Circle>,
    pub(crate) rings: Vec<Circle>,
    pub(crate) found: Vec<Vector>,
}

/// Where `forms` meet, laid in the world by `placed`.
pub(crate) fn met(forms: &[Form; 2], placed: Placed) -> Met {
    match *forms {
        [Form::Box(a), Form::Box(b)] => boxes(a, b, placed),
        [Form::Box(a), Form::Cylinder(b, axis)] => plugged(a, (b, axis), placed),
        [Form::Cylinder(..), Form::Box(b)] => notched(&forms[0], b, placed),
        [Form::Cylinder(..), Form::Cylinder(..)] => teed(forms, placed),
        [Form::Cylinder(..), Form::Sphere(centre, radius)] => {
            sunk(&forms[0], (centre, radius), placed)
        }
        _ => Met::default(),
    }
}

/// Whether `v` lies strictly between `low` and `high`.
fn between(v: f64, low: f64, high: f64) -> bool {
    v > low + 1e-9 && v < high - 1e-9
}

/// The local point with `values` on the axes `axes`.
fn local(pairs: [(usize, f64); 3]) -> Local {
    let mut p = [0.0; 3];
    for (axis, value) in pairs {
        p[axis] = value;
    }
    p
}

/// Two boxes: each face of `a` against each face of `b` across another axis.
fn boxes(a: Block, b: Block, placed: Placed) -> Met {
    let mut met = Met::default();
    for k in 0..3 {
        for at in [a.low[k], a.high[k]]
            .into_iter()
            .filter(|v| between(*v, b.low[k], b.high[k]))
        {
            for j in others(k) {
                let faces = [b.low[j], b.high[j]].into_iter();
                for on in faces.filter(|v| between(*v, a.low[j], a.high[j])) {
                    let m = 3 - k - j;
                    let [from, to] = [a.low[m].max(b.low[m]), a.high[m].min(b.high[m])];
                    if to <= from {
                        continue;
                    }
                    let point = |t: f64| placed.world(local([(k, at), (j, on), (m, t)]));
                    met.tracks.push(Track::Line([point(from), point(to)]));
                    // An end at `a`'s edge pierces `b`'s face; else `b`'s edge pierces `a`'s.
                    for t in [from, to] {
                        let pierced = usize::from(t == a.low[m] || t == a.high[m]);
                        let held = |(p, _): &(Vector, usize)| norm(sub(*p, point(t))) < 1e-9;
                        if !met.pierce.iter().any(held) {
                            met.pierce.push((point(t), pierced));
                        }
                    }
                    let far = [a.low[m].min(b.low[m]), a.high[m].max(b.high[m])];
                    met.lines.push(far.map(point));
                }
            }
        }
    }
    met
}

/// A cylinder (`block` along `axis`) through a face of the box `a`: its circle there.
fn plugged(a: Block, (block, axis): (Block, usize), placed: Placed) -> Met {
    let face = if block.low[axis] < a.low[axis] {
        a.low[axis]
    } else {
        a.high[axis]
    };
    let [j, m] = others(axis);
    let radius = (block.high[j] - block.low[j]) / 2.0;
    let middle = |k: usize| (block.low[k] + block.high[k]) / 2.0;
    let circle = Circle {
        centre: placed.world(local([(axis, face), (j, middle(j)), (m, middle(m))])),
        u: times(placed.axes[j], radius),
        v: times(placed.axes[m], radius),
    };
    let outer = if face == a.low[axis] {
        block.low[axis]
    } else {
        block.high[axis]
    };
    let mut met = Met {
        tracks: vec![Track::whole(circle)],
        squares: vec![circle],
        ..Met::default()
    };
    if let Some(turns) = touches(&circle) {
        met.pierce = turns.map(|t| (circle.at(t), 0)).to_vec();
    }
    let axis_at = |v: f64| placed.world(local([(axis, v), (j, middle(j)), (m, middle(m))]));
    met.lines.push([axis_at(outer), axis_at(face)]);
    met
}

/// The way a form `b` enters the cylinder `a` (radius, axis's point, axis): across which of its
/// other axes, and from which side (+1 its far side, -1 its near one).
fn entering(a: &Form, b: Block) -> Option<((f64, Local, usize), usize, f64)> {
    let (radius, at, axis) = a.round()?;
    let beyond = |k: usize| [at[k] - radius - b.low[k], b.high[k] - at[k] - radius];
    let j = (others(axis).into_iter()).max_by(|p, q| {
        beyond(*p)[0]
            .max(beyond(*p)[1])
            .total_cmp(&beyond(*q)[0].max(beyond(*q)[1]))
    })?;
    let side = if beyond(j)[1] > beyond(j)[0] {
        1.0
    } else {
        -1.0
    };
    Some(((radius, at, axis), j, side))
}

/// A box `b` into the cylinder `a`'s side: lines along it, arcs of its sections.
fn notched(a: &Form, b: Block, placed: Placed) -> Met {
    let Some(((radius, at, axis), j, side)) = entering(a, b) else {
        return Met::default();
    };
    let m = 3 - axis - j;
    let mut met = Met::default();
    let out = |v: f64| at[j] + side * (radius * radius - (v - at[m]).powi(2)).max(0.0).sqrt();
    let point = |s: f64, v: f64| placed.world(local([(axis, s), (j, out(v)), (m, v)]));
    let length = match a {
        Form::Cylinder(block, _) => [block.low[axis], block.high[axis]],
        _ => [0.0; 2],
    };
    for v in [b.low[m], b.high[m]] {
        met.tracks.push(Track::Line(
            [b.low[axis], b.high[axis]].map(|s| point(s, v)),
        ));
        met.lines.push(length.map(|s| point(s, v)));
    }
    let turn = |v: f64| ((v - at[m]) / radius).clamp(-1.0, 1.0).asin();
    for s in [b.low[axis], b.high[axis]] {
        let section = Circle {
            centre: placed.world(local([(axis, s), (j, at[j]), (m, at[m])])),
            u: times(placed.axes[j], side * radius),
            v: times(placed.axes[m], radius),
        };
        met.tracks
            .push(Track::Arc(section, [turn(b.low[m]), turn(b.high[m])]));
        met.rings.push(section);
        met.pierce
            .extend([b.low[m], b.high[m]].map(|v| (point(s, v), 0)));
    }
    met
}

/// A thinner cylinder into the cylinder `forms[0]`'s side: round its outer rim, each line along
/// its side carried to the other's.
fn teed(forms: &[Form; 2], placed: Placed) -> Met {
    let [a, b] = forms;
    let (Form::Cylinder(block, _), Some((thin, _, _))) = (*b, b.round()) else {
        return Met::default();
    };
    let Some(((radius, at, axis), j, side)) = entering(a, block) else {
        return Met::default();
    };
    let m = 3 - axis - j;
    let outer = if side > 0.0 {
        block.high[j]
    } else {
        block.low[j]
    };
    let middle = |k: usize| (block.low[k] + block.high[k]) / 2.0;
    let rim = Circle {
        centre: placed.world(local([(j, outer), (axis, middle(axis)), (m, middle(m))])),
        u: times(placed.axes[axis], thin),
        v: times(placed.axes[m], thin),
    };
    let way = times(placed.axes[j], -side);
    let into = Side {
        at: placed.world(at),
        axis: placed.axes[axis],
        radius,
    };
    let track = Track::Into { rim, way, into };
    let mut met = Met {
        tracks: vec![track],
        ..Met::default()
    };
    if let Some(turns) = touches(&rim) {
        met.pierce = turns.map(|t| (track.at(t), 0)).to_vec();
    }
    // Sections across both forms, each a plane along both axes: the thin one's two side lines
    // there carried to the thick one's, the thick one's line along it there over its length.
    let out = |v: f64| at[j] + side * (radius * radius - (v - at[m]).powi(2)).max(0.0).sqrt();
    let length = match a {
        Form::Cylinder(whole, _) => [whole.low[axis], whole.high[axis]],
        _ => [0.0; 2],
    };
    for h in [-ACROSS, 0.0, ACROSS] {
        let turn = h.asin();
        for t in [turn, std::f64::consts::PI - turn] {
            met.lines.push([rim.at(t), track.at(t)]);
            met.found.push(track.at(t));
        }
        let v = middle(m) + h * thin;
        met.lines
            .push(length.map(|s| placed.world(local([(axis, s), (j, out(v)), (m, v)]))));
    }
    met
}

/// A ball (`centre`, `radius`) sunk into the cylinder `a`'s side: round the cylinder where the
/// ball meets it; sections square to the cylinder's axis cut both in circles that cross on it.
fn sunk(a: &Form, (centre, radius): (Local, f64), placed: Placed) -> Met {
    let ball = Block {
        low: centre.map(|v| v - radius),
        high: centre.map(|v| v + radius),
    };
    let Some(((round, at, axis), j, side)) = entering(a, ball) else {
        return Met::default();
    };
    let m = 3 - axis - j;
    let rim = Circle {
        centre: placed.world(at),
        u: times(placed.axes[j], side * round),
        v: times(placed.axes[m], round),
    };
    let [aside, up] = [side * (centre[j] - at[j]), centre[m] - at[m]];
    let reach = aside.hypot(up);
    let spread = ((round * round + reach * reach - radius * radius) / (2.0 * round * reach))
        .clamp(-1.0, 1.0)
        .acos();
    let track = Track::Lens {
        rim,
        way: placed.axes[axis],
        ball: (placed.world(centre), radius),
        middle: up.atan2(aside),
        spread,
    };
    let quarter = std::f64::consts::FRAC_PI_2;
    let mut met = Met {
        tracks: vec![track],
        pierce: (0..4).map(|k| (track.at(k as f64 * quarter), 0)).collect(),
        ..Met::default()
    };
    for f in [-1.0, 0.0, 1.0] {
        let s = centre[axis] + f * ACROSS * ACROSS * radius;
        let cut = (radius * radius - (s - centre[axis]).powi(2)).sqrt();
        let circle = |middle: Local, r: f64| Circle {
            centre: placed.world(local([(axis, s), (j, middle[j]), (m, middle[m])])),
            u: times(placed.axes[j], r),
            v: times(placed.axes[m], r),
        };
        met.rings.extend([circle(at, round), circle(centre, cut)]);
        let crossings = crossed([at[j], at[m], round], [centre[j], centre[m], cut]);
        met.found.extend(
            (crossings.into_iter()).map(|[x, y]| placed.world(local([(axis, s), (j, x), (m, y)]))),
        );
    }
    met
}

/// Where the circles `[x, y, r]` and `[x, y, r]` of one plane cross.
fn crossed([ax, ay, ar]: [f64; 3], [bx, by, br]: [f64; 3]) -> Vec<[f64; 2]> {
    let (dx, dy) = (bx - ax, by - ay);
    let d = dx.hypot(dy);
    if d <= 0.0 || d > ar + br || d < (ar - br).abs() {
        return Vec::new();
    }
    let along = (ar * ar - br * br + d * d) / (2.0 * d);
    let off = (ar * ar - along * along).max(0.0).sqrt();
    let (ux, uy) = (dx / d, dy / d);
    let foot = [ax + along * ux, ay + along * uy];
    vec![
        [foot[0] - off * uy, foot[1] + off * ux],
        [foot[0] + off * uy, foot[1] - off * ux],
    ]
}
