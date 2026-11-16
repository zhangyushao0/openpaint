//! Two forms laid to pass into each other (S156k), in the scene's own coordinates: the main form
//! from the corner 0 of its block, the other entering it from a side the eye sees (the width's and
//! the depth's near ends, the top: the scene lies under the eye's level or across it), as far in
//! and out as the variant asks, clear of the main form's edges and ends.

use crate::kinds::intersections::forms::{Form, others};
use crate::kinds::intersections::meeting::Meeting;
use crate::kinds::objects::object_design::{Block, Local};
use crate::maths::random::Random;

/// A main box's sides; another box's, a share of the main one's; how much of that lies out.
const SIDES: [f64; 2] = [0.8, 1.3];
const SHARE: [f64; 2] = [0.4, 0.6];
const OUT: [f64; 2] = [0.35, 0.6];
const CORNER_SHARE: [f64; 2] = [0.55, 0.75];
const CORNER_OUT: [f64; 2] = [0.25, 0.4];
/// Where a part entering a lying cylinder from its near side is centred, up from its axis, and
/// across a standing one, either way: on the band of its side the eye sees best (shares of its
/// radius).
const UP: [f64; 2] = [0.0, 0.3];
const ASIDE: f64 = 0.15;
/// How often a part enters from the side the eye sees.
const SEEN_SIDE: f64 = 0.8;
/// A main cylinder's radius and length.
const RADIUS: [f64; 2] = [0.32, 0.45];
const LENGTH: [f64; 2] = [1.3, 1.8];

/// The two forms `meeting` lays from `random`, the main one first.
pub(crate) fn formed(meeting: Meeting, random: &mut Random) -> [Form; 2] {
    match meeting {
        Meeting::Edge | Meeting::Corner => boxes(meeting == Meeting::Corner, random),
        Meeting::CylinderIntoBox => plugged(random),
        Meeting::BoxIntoCylinder | Meeting::Cylinders | Meeting::Sphere => {
            into_cylinder(meeting, random)
        }
    }
}

/// Whether an axis's high end is the one the eye sees: the top's; the width's and depth's near
/// ends are their low ones.
const fn seen_high(axis: usize) -> bool {
    axis == 1
}

/// The end of `axis` a part enters from: the one the eye sees, mostly.
fn end(axis: usize, random: &mut Random) -> bool {
    let seen = random.unit() < SEEN_SIDE;
    seen == seen_high(axis)
}

/// A main box, and another across one of its edges (two axes straddling its ends, one within) or
/// round one of its corners (all three straddling).
fn boxes(corner: bool, random: &mut Random) -> [Form; 2] {
    let size: Local = [0; 3].map(|_| random.range(SIDES[0], SIDES[1]));
    let within = if corner {
        None
    } else {
        Some((random.next_u64() % 3) as usize)
    };
    // Round a corner, the far top one, larger: its near faces meet the main box's top in sight
    // (round the near one, the line where they meet lies behind it).
    let (share, out) = if corner {
        (CORNER_SHARE, CORNER_OUT)
    } else {
        (SHARE, OUT)
    };
    let mut low = [0.0; 3];
    let mut high = [0.0; 3];
    for k in 0..3 {
        let long = size[k] * random.range(share[0], share[1]);
        let at_high = corner || end(k, random);
        if Some(k) == within {
            let margin = 0.15 * size[k];
            low[k] = random.range(margin, size[k] - margin - long);
        } else if at_high {
            low[k] = size[k] - long * (1.0 - random.range(out[0], out[1]));
        } else {
            low[k] = -long * random.range(out[0], out[1]);
        }
        high[k] = low[k] + long;
    }
    [
        Form::Box(Block {
            low: [0.0; 3],
            high: size,
        }),
        Form::Box(Block { low, high }),
    ]
}

/// A main box, and a cylinder through one of its faces, ending inside it.
fn plugged(random: &mut Random) -> [Form; 2] {
    let size: Local = [0; 3].map(|_| random.range(0.9, SIDES[1]));
    let axis = [0, 2, 1][(random.next_u64() % 3) as usize];
    let [j, m] = others(axis);
    let radius = random.range(0.2, 0.28) * size[j].min(size[m]);
    let mut low = [0.0; 3];
    let mut high = [0.0; 3];
    for k in [j, m] {
        let margin = radius + 0.15 * size[k];
        let middle = random.range(margin, size[k] - margin);
        [low[k], high[k]] = [middle - radius, middle + radius];
    }
    let (out, inside) = (random.range(0.5, 0.8), random.range(0.3, 0.55));
    if end(axis, random) {
        [low[axis], high[axis]] = [size[axis] * (1.0 - inside), size[axis] * (1.0 + out)];
    } else {
        [low[axis], high[axis]] = [-size[axis] * out, size[axis] * inside];
    }
    [
        Form::Box(Block {
            low: [0.0; 3],
            high: size,
        }),
        Form::Cylinder(Block { low, high }, axis),
    ]
}

/// How a part enters the main cylinder: its radius and length; its axis, the axis the part
/// enters across and the third; from which side of it (+1 its far one); where the part is centred
/// along the third, from the axis.
#[derive(Debug, Clone, Copy)]
struct Entry {
    radius: f64,
    length: f64,
    axes: [usize; 3],
    side: f64,
    off: f64,
}

/// A main cylinder, lying or standing, and a box, a thinner cylinder or a ball entering its side.
fn into_cylinder(meeting: Meeting, random: &mut Random) -> [Form; 2] {
    let radius = random.range(RADIUS[0], RADIUS[1]);
    let length = random.range(LENGTH[0], LENGTH[1]);
    let axis = [0, 2, 1][(random.next_u64() % 3) as usize];
    let mut size = [2.0 * radius; 3];
    size[axis] = length;
    let main = Form::Cylinder(
        Block {
            low: [0.0; 3],
            high: size,
        },
        axis,
    );
    // Across which axis the other enters: a standing one's side across either of the ground's,
    // a lying one's across the ground's other; from the near side, mostly (not the top: seen from
    // a little above, the top of a lying one runs along its outline).
    let j = if axis == 1 {
        [0, 2][(random.next_u64() % 2) as usize]
    } else {
        2 - axis
    };
    let side = if end(j, random) { 1.0 } else { -1.0 };
    let m = 3 - axis - j;
    let off = if m == 1 {
        random.range(UP[0], UP[1]) * radius
    } else {
        random.range(-ASIDE, ASIDE) * radius
    };
    let entry = Entry {
        radius,
        length,
        axes: [axis, j, m],
        side,
        off,
    };
    let other = match meeting {
        Meeting::BoxIntoCylinder => notch(entry, random),
        Meeting::Cylinders => tee(entry, random),
        _ => ball(entry, random),
    };
    [main, other]
}

/// The block from `pairs` of an axis and its two ends.
fn block(pairs: [(usize, [f64; 2]); 3]) -> Block {
    let mut low = [0.0; 3];
    let mut high = [0.0; 3];
    for (axis, [a, b]) in pairs {
        [low[axis], high[axis]] = [a.min(b), a.max(b)];
    }
    Block { low, high }
}

/// A box into the main cylinder: its faces along the cylinder cut its side clear of its
/// outlines, its inner face inside it.
fn notch(entry: Entry, random: &mut Random) -> Form {
    let Entry {
        radius,
        length,
        axes: [axis, j, m],
        side,
        off,
    } = entry;
    let half = random.range(0.15, 0.24) * length;
    let along = random.range(0.15 * length + half, 0.85 * length - half);
    let wide = random.range(0.25, 0.4) * radius;
    let reach = (radius * radius - (off.abs() + wide).powi(2)).sqrt();
    let inner = radius + side * reach * random.range(0.2, 0.6);
    let outer = radius + side * radius * (1.0 + random.range(0.5, 0.9));
    Form::Box(block([
        (axis, [along - half, along + half]),
        (m, [radius + off - wide, radius + off + wide]),
        (j, [inner, outer]),
    ]))
}

/// A thinner cylinder into the main one, its inner end on the main one's axis.
fn tee(entry: Entry, random: &mut Random) -> Form {
    let Entry {
        radius,
        length,
        axes: [axis, j, m],
        side,
        off,
    } = entry;
    let thin = random.range(0.35, 0.5) * radius;
    let along = random.range(thin + 0.15 * length, length - thin - 0.15 * length);
    let outer = radius + side * radius * (1.0 + random.range(0.6, 1.0));
    Form::Cylinder(
        block([
            (axis, [along - thin, along + thin]),
            (m, [radius + off - thin, radius + off + thin]),
            (j, [radius, outer]),
        ]),
        j,
    )
}

/// A ball sunk into the main cylinder's side.
fn ball(entry: Entry, random: &mut Random) -> Form {
    let Entry {
        radius,
        length,
        axes: [axis, j, m],
        side,
        off,
    } = entry;
    let r = random.range(0.6, 0.9) * radius;
    let mut centre = [0.0; 3];
    centre[axis] = random.range(r + 0.12 * length, length - r - 0.12 * length);
    centre[m] = radius + off;
    centre[j] = radius + side * (radius + random.range(0.25, 0.6) * r);
    Form::Sphere(centre, r)
}
