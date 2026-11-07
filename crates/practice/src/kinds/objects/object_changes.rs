//! The changes an object's design makes of its box (S94, `object_design.rs`), each laid from the
//! seed's sequence at halves, thirds and quarters: a block taken out (a step, a notch, a slot), a
//! cut (a slope, a wedge, a corner), the top drawn in (方台), a post standing on a top, a smaller
//! change at a corner beside the first; and the room each leaves on the top.

use crate::object_design::{Block, Change, Cut, FRACTIONS, Local, SAME, THINNEST, Turned, pick};
use crate::random::Random;

/// A round part's width at least, a share of the box.
const ROUND_WIDTH: f64 = 0.25;

/// Where a smaller change may lie on the top: a stretch along `a` and one along `c`.
pub(crate) type Room = [[f64; 2]; 2];

/// A smaller change at the corner of `room` the box's own corner is: a cut through a point on
/// each of its three edges, or (unless `cut`) a notch; a third or a half along each way, a sixth
/// short of the room's far end at least. None when the room holds no corner of the box.
pub(crate) fn at_corner(random: &mut Random, room: Room, cut: bool) -> Option<Change> {
    let end = |[from, to]: [f64; 2]| {
        if from.abs() <= SAME {
            Some((0.0, to))
        } else if (to - 1.0).abs() <= SAME {
            Some((1.0, 1.0 - from))
        } else {
            None
        }
    };
    let [(ka, ra), (kc, rc)] = [end(room[0])?, end(room[1])?];
    let width = |random: &mut Random, reach: f64| {
        let held: Vec<f64> = [1.0 / 3.0, 0.5]
            .into_iter()
            .filter(|w| *w <= reach - THINNEST + SAME)
            .collect();
        (!held.is_empty()).then(|| pick(random, &held))
    };
    let (wa, wc) = (width(random, ra)?, width(random, rc)?);
    let along = |k: f64, w: f64| if k > 0.5 { 1.0 - w } else { w };
    let (a, c) = (along(ka, wa), along(kc, wc));
    if cut {
        let down = pick(random, &[0.5, 2.0 / 3.0]);
        return Some(Change {
            cuts: vec![Cut {
                through: [[a, 1.0, kc], [ka, 1.0, c], [ka, down, kc]],
                off: [ka, 1.0, kc],
            }],
            ..Change::default()
        });
    }
    let floor = pick(random, &[1.0 / 3.0, 0.5]);
    let span = |k: f64, x: f64| if k > 0.5 { [x, 1.0] } else { [0.0, x] };
    let ([a0, a1], [c0, c1]) = (span(ka, a), span(kc, c));
    Some(Change {
        out: vec![Block {
            low: [a0, floor, c0],
            high: [a1, 1.0, c1],
        }],
        ..Change::default()
    })
}

/// `change` with the room on the top beside it: away from it along the way it reaches least
/// into (either, when alike).
pub(crate) fn beside(change: Change, random: &mut Random) -> (Change, Room) {
    let held: Vec<Local> = (change.out.iter())
        .flat_map(|b| [b.low, b.high])
        .chain(change.cuts.iter().flat_map(|cut| cut.through))
        .collect();
    let reach = |k: usize| {
        held.iter()
            .fold((1.0_f64, 0.0_f64), |(l, h), p| (l.min(p[k]), h.max(p[k])))
    };
    let (spans, side) = ([reach(0), reach(2)], random.unit() < 0.5);
    let long = |(low, high): (f64, f64)| high - low;
    let axis = match long(spans[0]).total_cmp(&long(spans[1])) {
        std::cmp::Ordering::Less => 0,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => usize::from(side),
    };
    let (low, high) = spans[axis];
    let mut room = [[0.0, 1.0], [0.0, 1.0]];
    room[axis] = if low > 1.0 - high {
        [0.0, low]
    } else {
        [high, 1.0]
    };
    (change, room)
}

/// A block taken out of the box's top below `top`: a step along an edge, a notch at a corner or a
/// slot through the middle, its floor at a fraction.
pub(crate) fn block_out(random: &mut Random, top: f64) -> Change {
    // Below a whole box's top, a quarter at least either side; below a base's, a third.
    let least = if top < 1.0 { 1.0 / 3.0 } else { 0.25 };
    let floor = inside(random, (0.0, top), least);
    let along = if random.unit() < 0.5 { 0 } else { 2 };
    let other = 2 - along;
    let roll = random.unit();
    let mut low = [0.0, floor, 0.0];
    let mut high = [1.0, top, 1.0];
    let [from, to] = if roll < 0.75 {
        at_an_end(random)
    } else {
        if random.unit() < 0.5 {
            [0.25, 0.75]
        } else {
            [1.0 / 3.0, 2.0 / 3.0]
        }
    };
    (low[other], high[other]) = (from, to);
    if roll < 0.35 {
        // A notch: out of a corner, short along the other way too.
        [low[along], high[along]] = at_an_end(random);
    }
    Change {
        out: vec![Block { low, high }],
        ..Change::default()
    }
}

/// A cut of the box's top below `top`: a slope along one of its top edges, from a line on the
/// top down to a line on the side (a wedge where it runs from edge to edge), or a corner cut off
/// through a point on each of its three edges.
pub(crate) fn cut(random: &mut Random, top: f64) -> Change {
    let flip = |end: f64, by: f64| if end > 0.5 { 1.0 - by } else { by };
    let corner = [pick(random, &[0.0, 1.0]), top, pick(random, &[0.0, 1.0])];
    if random.unit() < 0.4 {
        let [a, c] = [0, 2].map(|k| flip(corner[k], pick(random, &[1.0 / 3.0, 0.5, 2.0 / 3.0])));
        // A quarter to a half of the box down from the top.
        let below: Vec<f64> = (FRACTIONS.iter().copied())
            .filter(|f| (0.25 - SAME..=0.5 + SAME).contains(&(top - f)))
            .collect();
        let down = pick(random, &below);
        let through = [
            [a, top, corner[2]],
            [corner[0], top, c],
            [corner[0], down, corner[2]],
        ];
        return Change {
            cuts: vec![Cut {
                through,
                off: corner,
            }],
            ..Change::default()
        };
    }
    // A slope along the edge across `side` (a or c) at `corner`'s end of it.
    let side = if random.unit() < 0.5 { 0 } else { 2 };
    let into = flip(
        corner[side],
        pick(random, &[1.0 / 3.0, 0.5, 2.0 / 3.0, 1.0]),
    );
    let down = pick(random, &[0.0, 0.25, 1.0 / 3.0, 0.5]).min(top - THINNEST);
    let along = 2 - side;
    let point = |end: f64, at: f64, height: f64| {
        let mut p = [0.0; 3];
        (p[along], p[side], p[1]) = (end, at, height);
        p
    };
    let off = point(0.5, corner[side], top);
    Change {
        cuts: vec![Cut {
            through: [
                point(0.0, into, top),
                point(1.0, into, top),
                point(0.0, corner[side], down),
            ],
            off,
        }],
        ..Change::default()
    }
}

/// The top drawn in towards the middle (方台) below `top`: the top's sides moved in on two to
/// four sides, each side a cut through its bottom edge, and the room on the top drawn in; none
/// when the draw leaves under two moved.
pub(crate) fn taper(random: &mut Random, top: f64) -> Option<(Change, Room)> {
    let [a0, a1] = stretch(random, (0.0, 1.0), 1.0 / 3.0, true)?;
    let [c0, c1] = stretch(random, (0.0, 1.0), 1.0 / 3.0, true)?;
    let insets = [a0, 1.0 - a1, c0, 1.0 - c1];
    if insets.iter().filter(|inset| **inset > SAME).count() < 2 {
        return None;
    }
    let mut cuts = Vec::new();
    for (side, inset) in insets.into_iter().enumerate() {
        if inset <= SAME {
            continue;
        }
        let (axis, end, line) = match side {
            0 => (0, 0.0, a0),
            1 => (0, 1.0, a1),
            2 => (2, 0.0, c0),
            _ => (2, 1.0, c1),
        };
        let point = |at: f64, height: f64, across: f64| {
            let mut p = [0.0, height, 0.0];
            (p[axis], p[2 - axis]) = (at, across);
            p
        };
        cuts.push(Cut {
            through: [
                point(end, 0.0, 0.0),
                point(end, 0.0, 1.0),
                point(line, top, 0.0),
            ],
            off: point(end, top, 0.5),
        });
    }
    let change = Change {
        cuts,
        ..Change::default()
    };
    Some((change, [[a0, a1], [c0, c1]]))
}

/// A post on the top at `base`, a cylinder or a cone standing, in `room`, the part of the box
/// above `base` taken out round it; the box's `size` made so its ends are square. None when no
/// footprint fits. (A cylinder lying shows its end against its side by no step of shade one light
/// gives everywhere: its side turns through the end's shade.)
pub(crate) fn on_top(
    random: &mut Random,
    room: Room,
    base: f64,
    size: &mut [f64; 3],
) -> Option<Change> {
    let a = stretch(random, (room[0][0], room[0][1]), ROUND_WIDTH, false)?;
    let c = stretch(random, (room[1][0], room[1][1]), ROUND_WIDTH, false)?;
    let block = Block {
        low: [a[0], base, c[0]],
        high: [a[1], 1.0, c[1]],
    };
    let cone = random.unit() < 0.5;
    // The ends square: its width and depth alike in the world.
    size[2] = size[0] * (a[1] - a[0]) / (c[1] - c[0]);
    let axis = 1;
    let slab = Block {
        low: [0.0, base, 0.0],
        high: [1.0; 3],
    };
    Some(Change {
        out: vec![slab],
        turned: Some(Turned { block, axis, cone }),
        ..Change::default()
    })
}

/// A fraction inside `(from, to)`, `least` or more from both.
fn inside(random: &mut Random, (from, to): (f64, f64), least: f64) -> f64 {
    let held: Vec<f64> = (FRACTIONS.iter().copied())
        .filter(|f| *f - from >= least - SAME && to - *f >= least - SAME)
        .collect();
    if held.is_empty() {
        (from + to) / 2.0
    } else {
        pick(random, &held)
    }
}

/// A stretch from one end of an edge: from 0 or to 1, a third to two thirds long.
fn at_an_end(random: &mut Random) -> [f64; 2] {
    let long = pick(random, &[1.0 / 3.0, 0.5, 2.0 / 3.0]);
    if random.unit() < 0.5 {
        [0.0, long]
    } else {
        [1.0 - long, 1.0]
    }
}

/// A stretch of fractions within `[from, to]` at least `least` long, each end a sixth inside or,
/// when `flush`, on `from` or `to`; none when none fits.
fn stretch(
    random: &mut Random,
    (from, to): (f64, f64),
    least: f64,
    flush: bool,
) -> Option<[f64; 2]> {
    let on = |gap: f64| (flush && gap.abs() <= SAME) || gap >= THINNEST - SAME;
    let ends: Vec<f64> = (FRACTIONS.iter().copied())
        .filter(|f| *f >= from - SAME && *f <= to + SAME)
        .filter(|f| on(*f - from) && on(to - *f))
        .collect();
    let pairs: Vec<[f64; 2]> = (0..ends.len())
        .flat_map(|i| ((i + 1)..ends.len()).map(move |j| (i, j)))
        .map(|(i, j)| [ends[i], ends[j]])
        .filter(|[low, high]| high - low >= least - SAME)
        .collect();
    (!pairs.is_empty()).then(|| pairs[(random.unit() * pairs.len() as f64) as usize % pairs.len()])
}
