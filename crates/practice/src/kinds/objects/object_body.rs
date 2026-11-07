//! An object in space (S94, `object.rs`): its design (`object_design.rs`) built in the world the
//! eye sees, as convex hulls (the box's blocks left once the changes took theirs out, each cut by
//! the changes' planes) and round parts (a cylinder or a cone in its block). What the eye's rays
//! meet first (`Body::hit`), and whether a point lies in it, serve the exact visible edges
//! (`object_answer.rs`) and the picture's depth test (`object_picture.rs`).
//!
//! The world: y up, z away from the eye, which looks level from the origin.

use crate::object_design::{Block, Design, Local};
use crate::object_hull::{hull, minus, round};
use crate::object_rays::{hull_hit, round_hit, round_off, round_values};
use crate::solid_answer::Square;
use crate::vector::{Vector, add, dot, norm, sub, times};

/// Within this of a surface (world units) a point lies on it.
pub(crate) const TOUCH: f64 = 1e-7;
/// A round part's surfaces are numbered from here, a hull's below.
pub(crate) const ROUND_SURFACES: u32 = 1_000;

/// What lies at a point of a surface (`Body::touching`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Touch {
    /// Inside a part: no surface of the object there.
    Inside,
    /// The parts touching it, each with how many of its surfaces pass through it.
    Parts(Vec<usize>),
}

/// A plane: the points `normal · p <= offset` kept, `normal` one long.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Plane {
    pub(crate) normal: Vector,
    pub(crate) offset: f64,
}

/// A convex piece: the points every plane keeps; its corners (in the box's own coordinates) and
/// its edges, each by the two planes it lies on.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Hull {
    pub(crate) planes: Vec<Plane>,
    pub(crate) corners: Vec<Local>,
    pub(crate) edges: Vec<[Local; 2]>,
    /// Each plane's surface: one for planes in one place, whichever hull's (`Body::of`).
    pub(crate) surfaces: Vec<u32>,
}

/// A cylinder, or a cone narrowing from its base to a point: its base's square (the circle in it),
/// its axis one long from the base, and how long it is along it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Round {
    pub(crate) base: Square,
    pub(crate) axis: Vector,
    pub(crate) length: f64,
    pub(crate) cone: bool,
}

impl Round {
    pub(crate) fn radius(&self) -> f64 {
        norm(self.base.u) / 2.0
    }

    /// Its other end's square (a cylinder's top; a cone's point's, of no size).
    pub(crate) fn top(&self) -> Square {
        let shift = times(self.axis, self.length);
        let corner = add(self.base.corner, shift);
        if self.cone {
            let centre = add(self.base.centre(), shift);
            Square {
                corner: centre,
                u: [0.0; 3],
                v: [0.0; 3],
            }
        } else {
            Square {
                corner,
                ..self.base
            }
        }
    }
}

/// How the box lies in the world: its corner 0 and its width, height and depth as vectors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Placed {
    pub(crate) origin: Vector,
    pub(crate) axes: [Vector; 3],
}

impl Placed {
    /// Where the box's own point `p` lies in the world.
    pub(crate) fn world(&self, p: Local) -> Vector {
        (0..3).fold(self.origin, |at, k| add(at, times(self.axes[k], p[k])))
    }

    /// The box's own coordinates of the world point `w`.
    pub(crate) fn local(&self, w: Vector) -> Local {
        let from = sub(w, self.origin);
        std::array::from_fn(|k| dot(from, self.axes[k]) / dot(self.axes[k], self.axes[k]))
    }

    /// The world plane of the box's own `normal · p <= offset`.
    pub(crate) fn plane(&self, (normal, offset): (Local, f64)) -> Plane {
        let n = (0..3).fold([0.0; 3], |sum, k| {
            add(
                sum,
                times(self.axes[k], normal[k] / dot(self.axes[k], self.axes[k])),
            )
        });
        let long = norm(n).max(f64::MIN_POSITIVE);
        Plane {
            normal: times(n, 1.0 / long),
            offset: (offset + dot(n, self.origin)) / long,
        }
    }
}

/// What a ray met first: how far along it, the outward normal there, and whether a round part.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Hit {
    pub(crate) t: f64,
    pub(crate) normal: Vector,
    pub(crate) round: bool,
    /// Which surface: a hull's plane, or a round part's side or end.
    pub(crate) surface: u32,
}

/// An object in the world.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Body {
    pub(crate) placed: Placed,
    pub(crate) hulls: Vec<Hull>,
    pub(crate) rounds: Vec<Round>,
}

impl Body {
    /// `design` built in the box `placed`.
    pub(crate) fn of(design: &Design, placed: Placed) -> Self {
        let mut blocks = vec![Block::WHOLE];
        for change in &design.changes {
            for out in &change.out {
                blocks = blocks.iter().flat_map(|b| minus(*b, out)).collect();
            }
        }
        let cuts: Vec<(Local, f64)> = (design.changes.iter())
            .flat_map(|change| change.cuts.iter().map(|cut| cut.kept()))
            .collect();
        let mut hulls: Vec<Hull> = (blocks.iter())
            .filter_map(|block| hull(block, &cuts, placed))
            .collect();
        // Planes in one place are one surface, so blocks meeting flat show no seam.
        let mut places: Vec<Plane> = Vec::new();
        for hull in &mut hulls {
            hull.surfaces = (hull.planes.iter())
                .map(|plane| {
                    let same = |held: &Plane| {
                        norm(sub(held.normal, plane.normal)) <= 1e-9
                            && (held.offset - plane.offset).abs() <= 1e-9
                    };
                    let at = places.iter().position(same).unwrap_or_else(|| {
                        places.push(*plane);
                        places.len() - 1
                    });
                    at as u32
                })
                .collect();
        }
        let rounds = (design.changes.iter())
            .filter_map(|change| change.turned)
            .map(|turned| round(&turned, placed))
            .collect();
        Self {
            placed,
            hulls,
            rounds,
        }
    }

    /// What the ray from the eye along `direction` meets first; `t` 1 at `direction` itself.
    pub(crate) fn hit(&self, direction: Vector) -> Option<Hit> {
        let hulls = self
            .hulls
            .iter()
            .filter_map(|hull| hull_hit(hull, direction));
        let rounds = (self.rounds.iter().enumerate())
            .filter_map(|(index, round)| round_hit(round, direction, index));
        hulls.chain(rounds).min_by(|a, b| a.t.total_cmp(&b.t))
    }

    /// What lies at `p`: inside a part, or the parts whose surfaces pass through it (within
    /// `TOUCH`), each with how many of its surfaces do.
    pub(crate) fn touching(&self, p: Vector) -> Touch {
        let mut parts = Vec::new();
        // A part's greatest value and how many lie on its surfaces.
        let mut take = |(most, on): (f64, usize)| -> bool {
            if (-TOUCH..=TOUCH).contains(&most) {
                parts.push(on);
            }
            most < -TOUCH
        };
        let fold =
            |(most, on): (f64, usize), v: f64| (most.max(v), on + usize::from(v.abs() <= TOUCH));
        for hull in &self.hulls {
            let values = (hull.planes.iter()).map(|plane| dot(plane.normal, p) - plane.offset);
            if take(values.fold((f64::NEG_INFINITY, 0), fold)) {
                return Touch::Inside;
            }
        }
        for round in &self.rounds {
            let values = round_values(round, p).into_iter();
            if take(values.fold((f64::NEG_INFINITY, 0), fold)) {
                return Touch::Inside;
            }
        }
        Touch::Parts(parts)
    }

    /// Whether `p` lies in a part of it, `margin` beyond its surfaces at most (inside them, when
    /// negative); `skip` a round part left out.
    pub(crate) fn holds(&self, p: Vector, margin: f64, skip: Option<usize>) -> bool {
        let in_hull = |hull: &Hull| {
            (hull.planes.iter()).all(|plane| dot(plane.normal, p) - plane.offset <= margin)
        };
        let in_round =
            |(index, round): (usize, &Round)| Some(index) != skip && round_off(round, p) <= margin;
        self.hulls.iter().any(in_hull) || self.rounds.iter().enumerate().any(in_round)
    }
}
