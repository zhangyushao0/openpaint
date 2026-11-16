//! Texture's field (S154j): a strip of ground covered with small forms, seen from the front and
//! above (`rays.rs`), lit by a low sun from the left or the right (20 to 35 degrees high: long
//! shadows). The level says the forms: at 入门 round pebbles of one size, each a half-buried ball;
//! at 熟练 a mix of plates lying tilted (thin boxes), ridges (half cylinders lying) and grooves (half
//! cylinders cut in), a few pebbles; at 稳 a dense mix, the half toward the light kept to its
//! biggest shadows (Drawabox: less toward the light).
//!
//! A strip is laid again from its seed's sequence until it keeps every rule (`read.rs` reads the
//! shadows): 12 to 30 of them, each two pens wide; none lying along the frame's edge over half its
//! outline.

use crate::kinds::objects::object_body::{Ball, Body, Hull, Placed, Round, Scene};
use crate::kinds::objects::object_design::Block;
use crate::kinds::objects::object_hull::hull;
use crate::kinds::solids::solid_answer::Square;
use crate::kinds::textures::rays::{Groove, Strip};
use crate::maths::random::{Random, derived};
use crate::maths::vector::{Vector, add, cross, sub, times, unit};

/// Strips laid before the generator keeps the last whatever it breaks; places tried a form.
const TRIES: u64 = 30;
const PLACES: usize = 40;
/// The sun's height and how far its way leans toward or away from the eye (degrees); how far the
/// eye looks down.
const ELEVATION: [f64; 2] = [20.0, 35.0];
const LEAN: f64 = 25.0;
const TILT: [f64; 2] = [45.0, 55.0];
/// The forms' room on the ground: kept off the strip's ends and its top and bottom (frame units),
/// and apart by this beyond their reach.
const MARGIN: f64 = 0.05;
const APART: f64 = 0.012;

/// The forms a level scatters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bumps {
    /// Round pebbles of one size (入门).
    Pebbles,
    /// Plates, ridges, grooves and a few pebbles (熟练).
    Mixed,
    /// A dense mix, the half toward the light kept to its biggest shadows (稳).
    Dense,
}

/// Texture's variant: the forms and which side the sun stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Texture {
    pub bumps: Bumps,
    pub from_right: bool,
}

impl Texture {
    /// Every variant, each stored by its place: a harder one goes at the end.
    pub const ALL: [Self; 6] = [
        Self::of(Bumps::Pebbles, false),
        Self::of(Bumps::Pebbles, true),
        Self::of(Bumps::Mixed, false),
        Self::of(Bumps::Mixed, true),
        Self::of(Bumps::Dense, false),
        Self::of(Bumps::Dense, true),
    ];

    const fn of(bumps: Bumps, from_right: bool) -> Self {
        Self { bumps, from_right }
    }
}

/// A form's footprint on the ground: a stretch (`a` to `b`, a pebble's a point) and its reach.
#[derive(Debug, Clone, Copy)]
struct Footprint {
    a: Vector,
    b: Vector,
    reach: f64,
}

impl Footprint {
    /// How far apart two footprints' stretches come, about (their points sampled).
    fn gap(&self, other: &Self) -> f64 {
        let at = |f: &Self, k: usize| add(f.a, times(sub(f.b, f.a), k as f64 / 6.0));
        let mut least = f64::INFINITY;
        for i in 0..=6 {
            for j in 0..=6 {
                let d = sub(at(self, i), at(other, j));
                least = least.min(d[0].hypot(d[2]));
            }
        }
        least - self.reach - other.reach
    }
}

/// What a strip is built of while it is laid.
#[derive(Default)]
struct Laying {
    balls: Vec<Ball>,
    rounds: Vec<Round>,
    hulls: Vec<Hull>,
    grooves: Vec<Groove>,
    taken: Vec<Footprint>,
}

/// The strip of `texture` laid from `seed`, its strip and whether its light half is to be sparse.
pub(crate) fn laid(texture: Texture, seed: u64) -> Strip {
    let mut random = Random::new(seed);
    let tilt = random.range(TILT[0], TILT[1]).to_radians();
    let (elevation, lean) = (
        random.range(ELEVATION[0], ELEVATION[1]).to_radians(),
        random.range(-LEAN, LEAN).to_radians(),
    );
    let side = if texture.from_right { 1.0 } else { -1.0 };
    let (up, level) = elevation.sin_cos();
    let sun = [side * level * lean.cos(), up, level * lean.sin()];
    let depth = 0.25 / tilt.sin();
    let mut laying = Laying::default();
    let counts = match texture.bumps {
        Bumps::Pebbles => [random.range(14.0, 20.0) as usize, 0, 0, 0],
        Bumps::Mixed => [4, 4, 4, 3],
        Bumps::Dense => [7, 7, 6, 4],
    };
    let one_size = random.range(0.042, 0.05);
    for (kind, count) in counts.into_iter().enumerate() {
        for _ in 0..count {
            let _ = (0..PLACES).find(|_| place(&mut laying, (kind, one_size), depth, &mut random));
        }
    }
    let Laying {
        balls,
        rounds,
        hulls,
        grooves,
        ..
    } = laying;
    let body = Body {
        placed: Placed {
            origin: [0.0; 3],
            axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        },
        hulls,
        rounds,
        balls,
    };
    Strip {
        scene: Scene { body, ground: 0.0 },
        grooves,
        sun: unit(sun),
        tilt,
    }
}

/// The `seed`th strip of `texture` tried: the seed's own, then its sequence's.
pub(crate) fn tried(texture: Texture, seed: u64, k: u64) -> Strip {
    laid(texture, if k == 0 { seed } else { derived(seed, k) })
}

/// The strips a generator tries.
pub(crate) const fn tries() -> u64 {
    TRIES
}

/// A form of `kind` (a pebble, a plate, a ridge, a groove; `one_size` a pebble's radius) placed
/// at random within the strip `depth` deep either side, clear of those taken; whether it was.
fn place(
    laying: &mut Laying,
    (kind, one_size): (usize, f64),
    depth: f64,
    random: &mut Random,
) -> bool {
    let x = random.range(-0.5 + MARGIN, 0.5 - MARGIN);
    let z = random.range(-depth + MARGIN * 1.6, depth - MARGIN);
    let (footprint, made) = formed((kind, one_size), [x, 0.0, z], random);
    if laying.taken.iter().any(|held| held.gap(&footprint) < APART) {
        return false;
    }
    laying.taken.push(footprint);
    match made {
        Made::Ball(ball) => laying.balls.push(ball),
        Made::Hull(mut plate) => {
            let first = 6 * laying.hulls.len() as u32;
            plate.surfaces = (first..first + 6).collect();
            laying.hulls.push(plate);
        }
        Made::Round(round) => laying.rounds.push(round),
        Made::Groove(groove) => laying.grooves.push(groove),
    }
    true
}

/// A form of `kind` (`one_size` a pebble's radius) at `centre` on the ground, its turn and size
/// drawn from `random`, and its footprint.
fn formed(
    (kind, one_size): (usize, f64),
    centre: Vector,
    random: &mut Random,
) -> (Footprint, Made) {
    let turn = random.range(0.0, std::f64::consts::PI);
    let along = [turn.cos(), 0.0, turn.sin()];
    let stretch = |a, b, reach| Footprint { a, b, reach };
    match kind {
        0 => {
            let radius = one_size * random.range(0.9, 1.1);
            let sunk = radius * random.range(0.0, 0.35);
            let ball = Ball {
                centre: [centre[0], -sunk, centre[2]],
                radius,
            };
            (stretch(centre, centre, radius), Made::Ball(ball))
        }
        1 => {
            let [w, d] = [random.range(0.09, 0.12), random.range(0.065, 0.085)];
            let rise = random.range(10.0, 22.0).to_radians();
            let half = times(along, w / 2.0);
            let made = Made::Hull(plate(centre, along, [w, d], rise));
            (stretch(sub(centre, half), add(centre, half), d / 2.0), made)
        }
        _ => {
            let length = random.range(0.13, 0.2);
            let radius = random.range(0.028, 0.036);
            let half = times(along, length / 2.0);
            let start = sub(centre, half);
            let made = if kind == 2 {
                Made::Round(ridge(start, along, length, radius))
            } else {
                Made::Groove(Groove {
                    start,
                    along,
                    length,
                    radius,
                })
            };
            (stretch(start, add(centre, half), radius), made)
        }
    }
}

/// A form made.
enum Made {
    Ball(Ball),
    Hull(Hull),
    Round(Round),
    Groove(Groove),
}

/// A plate `[width, depth]` lying on the ground at `centre` along `along`, its far edge raised
/// `rise` (radians), thin.
fn plate(centre: Vector, along: Vector, [w, d]: [f64; 2], rise: f64) -> Hull {
    let across = cross(along, [0.0, 1.0, 0.0]);
    let (up, level) = rise.sin_cos();
    let a = times(along, w);
    let b = add(times(across, d * level), [0.0, d * up, 0.0]);
    let thick = times(unit(cross(b, a)), 0.016);
    let thick = if thick[1] < 0.0 {
        times(thick, -1.0)
    } else {
        thick
    };
    let origin = sub(centre, add(times(a, 0.5), times(b, 0.5)));
    let placed = Placed {
        origin: [origin[0], 0.0, origin[2]],
        axes: [a, b, thick],
    };
    hull(&Block::WHOLE, &[], placed).unwrap_or(Hull {
        planes: Vec::new(),
        corners: Vec::new(),
        edges: Vec::new(),
        surfaces: Vec::new(),
    })
}

/// A ridge: a cylinder lying on the ground from `start` along `along`, half sunk in it.
fn ridge(start: Vector, along: Vector, length: f64, radius: f64) -> Round {
    let across = cross(along, [0.0, 1.0, 0.0]);
    let [u, v] = [times(across, 2.0 * radius), [0.0, 2.0 * radius, 0.0]];
    Round {
        base: Square {
            corner: sub(start, times(add(u, v), 0.5)),
            u,
            v,
        },
        axis: along,
        length,
        cone: false,
    }
}
