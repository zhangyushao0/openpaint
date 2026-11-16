//! What an object is made of (S94, V大's 几何体概括单体): the box that holds it, in the box's own
//! coordinates (`a` along its width towards the right vanishing point, `b` up, `c` along its depth
//! towards the left one, each from 0 to 1), and one or two changes of it, the biggest first, as
//! V大 makes them: a block taken out of an edge, a corner or the middle of the top (a step, a notch,
//! a slot); a cut through points on its edges (a slope, a wedge, a corner); its top drawn in
//! towards the middle (方台, four cuts); a cylinder or a cone in a part of it, standing or lying on
//! a top (a post, a roller; S93's square-ended solids, the part above its support taken out).
//! Every coordinate is a half, a third or a quarter of the box's edges (`FRACTIONS`), so V大's
//! tricks find every corner, and two along an axis lie at least a sixth apart (`THINNEST`): no
//! part, gap or step thinner.

use crate::kinds::objects::object_changes::{at_corner, beside, block_out, cut, on_top, taper};
use crate::maths::random::Random;

/// A point in the box's own coordinates.
pub(crate) type Local = [f64; 3];

/// The shares of the box's edges every coordinate is one of.
pub(crate) const FRACTIONS: [f64; 7] = [0.0, 0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 0.75, 1.0];
/// The least distance between two coordinates along an axis.
pub(crate) const THINNEST: f64 = 1.0 / 6.0;
/// Within this two coordinates are one.
pub(crate) const SAME: f64 = 1e-9;
/// How often an object has a second change.
pub(crate) const SECOND: f64 = 0.5;
/// The box's sides before a round part sets one (world units, as `space.rs`'s boxes), and how far
/// apart its longest and shortest may be.
const SIDE: [f64; 2] = [0.7, 1.4];
const PROPORTION: f64 = 2.4;

/// How the box becomes the object: its first, biggest change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Carve {
    /// A block taken out of an edge, a corner or the middle of the top.
    Block,
    /// A cut through points on its edges.
    Cut,
    /// Its top drawn in towards the middle (方台).
    Taper,
    /// A cylinder or a cone in a part of it.
    Round,
}

impl Carve {
    pub const ALL: [Self; 4] = [Self::Block, Self::Cut, Self::Taper, Self::Round];
}

/// A block of the box: from `low` to `high` along each axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Block {
    pub(crate) low: Local,
    pub(crate) high: Local,
}

impl Block {
    pub(crate) const WHOLE: Self = Self {
        low: [0.0; 3],
        high: [1.0; 3],
    };

    /// Whether `p` lies in it or on it.
    pub(crate) fn holds(&self, p: Local) -> bool {
        (0..3).all(|k| (self.low[k] - SAME..=self.high[k] + SAME).contains(&p[k]))
    }
}

/// A cut: the plane through three points, the side holding `off` taken away.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Cut {
    pub(crate) through: [Local; 3],
    pub(crate) off: Local,
}

impl Cut {
    /// The plane `normal · p <= offset` it keeps.
    pub(crate) fn kept(&self) -> (Local, f64) {
        let [p, q, r] = self.through;
        let (u, v) = (minus(q, p), minus(r, p));
        let normal = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let offset = dot(normal, p);
        if dot(normal, self.off) > offset {
            (normal, offset)
        } else {
            (normal.map(|v| -v), -offset)
        }
    }

    /// Whether `p` lies on its plane.
    pub(crate) fn passes(&self, p: Local) -> bool {
        let (normal, offset) = self.kept();
        (dot(normal, p) - offset).abs() <= SAME * dot(normal, normal).sqrt().max(1.0)
    }
}

/// A cylinder along `axis` in `block`, its ends the block's square faces across it; or a cone on
/// the face at the axis's low end, its point at the high end's centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Turned {
    pub(crate) block: Block,
    pub(crate) axis: usize,
    pub(crate) cone: bool,
}

/// One change of the box: blocks taken out, cuts, a round part.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Change {
    pub(crate) out: Vec<Block>,
    pub(crate) cuts: Vec<Cut>,
    pub(crate) turned: Option<Turned>,
}

/// An object's make-up: its changes, the biggest first, and the box's width, height and depth.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Design {
    pub(crate) changes: Vec<Change>,
    pub(crate) size: [f64; 3],
}

impl Design {
    /// Whether a round part stands on it (a post: seen from above, its ends no slivers).
    pub(crate) fn posted(&self) -> bool {
        self.changes.iter().any(|change| change.turned.is_some())
    }

    /// The same object turned about its upright by `quarters` quarter turns (S113), as the box's
    /// own coordinates of another corner at the front: a quarter takes `(a, c)` to `(1 - c, a)`,
    /// its width and depth swapped; never mirrored. A round part keeps its axis's place (a post
    /// stands; a lying one never comes, `object_changes::on_top`).
    pub(crate) fn turned(&self, quarters: u8) -> Self {
        (0..quarters % 4).fold(self.clone(), |design, _| design.quarter())
    }

    fn quarter(&self) -> Self {
        let turn = |p: Local| [1.0 - p[2], p[1], p[0]];
        let block = |b: &Block| {
            let (low, high) = (turn(b.low), turn(b.high));
            Block {
                low: std::array::from_fn(|k| low[k].min(high[k])),
                high: std::array::from_fn(|k| low[k].max(high[k])),
            }
        };
        let changes = (self.changes.iter())
            .map(|change| Change {
                out: change.out.iter().map(block).collect(),
                cuts: (change.cuts.iter())
                    .map(|cut| Cut {
                        through: cut.through.map(turn),
                        off: turn(cut.off),
                    })
                    .collect(),
                turned: change.turned.map(|turned| Turned {
                    block: block(&turned.block),
                    axis: [2, 1, 0][turned.axis.min(2)],
                    ..turned
                }),
            })
            .collect();
        let [width, height, depth] = self.size;
        Self {
            changes,
            size: [depth, height, width],
        }
    }
}

/// A design of `carve` laid from `random`, of two changes when `two` (a round part's alone);
/// none when no footprint fits or the sides a round part asks fall outside the box's
/// proportions (the caller lays another).
pub(crate) fn designed(carve: Carve, random: &mut Random, two: bool) -> Option<Design> {
    // A round part is the biggest change alone, or the smaller one on a base two thirds high
    // (the base's own change a third high at least, so no edge of it falls under 40 pixels); the
    // smaller change of a block or a cut is otherwise a cut or a notch at a corner beside it.
    let two = two && carve != Carve::Round;
    let post = carve == Carve::Round || (two && (carve == Carve::Taper || random.unit() < 0.5));
    let base = match (carve, post) {
        (Carve::Round, _) => pick(random, &[1.0 / 3.0, 0.5]),
        (_, true) => 2.0 / 3.0,
        (_, false) => 1.0,
    };
    let whole = [[0.0, 1.0], [0.0, 1.0]];
    let carved = match carve {
        Carve::Block => Some(beside(block_out(random, base), random)),
        Carve::Cut => Some(beside(cut(random, base), random)),
        Carve::Taper => Some(taper(random, base)?),
        Carve::Round => None,
    };
    let mut size = [0; 3].map(|_| random.range(SIDE[0], SIDE[1]));
    let room = carved.as_ref().map_or(whole, |(_, room)| *room);
    let round = if post {
        Some(on_top(random, room, base, &mut size)?)
    } else if two {
        Some(at_corner(random, room, carve == Carve::Block)?)
    } else {
        None
    };
    let longest = size.iter().copied().fold(0.0, f64::max);
    let shortest = size.iter().copied().fold(f64::INFINITY, f64::min);
    if longest > PROPORTION * shortest {
        return None;
    }
    let mut changes: Vec<Change> = (carved.into_iter().map(|(change, _)| change))
        .chain(round)
        .collect();
    if carve == Carve::Round {
        changes.rotate_right(1);
    }
    // A smaller change at a corner turned to the eye's: the box mirrored so its corner is the
    // nearest, where it shows.
    if two && !post {
        let corner = changes.last().map(near_corner).unwrap_or([0.0; 3]);
        for axis in [0, 2].into_iter().filter(|k| corner[*k] > 0.5) {
            changes = changes
                .iter()
                .map(|change| mirrored(change, axis))
                .collect();
        }
    }
    Some(Design { changes, size })
}

/// The box's corner a corner change lies at: the point its cut takes off, or its block's corner.
fn near_corner(change: &Change) -> Local {
    if let Some(cut) = change.cuts.first() {
        return cut.off;
    }
    change.out.first().map_or([0.0; 3], |block| {
        std::array::from_fn(|k| if block.low[k] <= SAME { 0.0 } else { 1.0 })
    })
}

/// `change` mirrored across the box's middle along `axis`.
fn mirrored(change: &Change, axis: usize) -> Change {
    let flip = |mut p: Local| {
        p[axis] = 1.0 - p[axis];
        p
    };
    let block = |b: &Block| {
        let (low, high) = (flip(b.low), flip(b.high));
        Block {
            low: std::array::from_fn(|k| low[k].min(high[k])),
            high: std::array::from_fn(|k| low[k].max(high[k])),
        }
    };
    Change {
        out: change.out.iter().map(block).collect(),
        cuts: (change.cuts.iter())
            .map(|cut| Cut {
                through: cut.through.map(flip),
                off: flip(cut.off),
            })
            .collect(),
        turned: change.turned.map(|turned| Turned {
            block: block(&turned.block),
            ..turned
        }),
    }
}

/// One of `values`, chosen by `random`.
pub(crate) fn pick(random: &mut Random, values: &[f64]) -> f64 {
    values[(random.unit() * values.len() as f64) as usize % values.len()]
}

fn minus(a: Local, b: Local) -> Local {
    std::array::from_fn(|k| a[k] - b[k])
}

fn dot(a: Local, b: Local) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
