//! An object in space (S94, `object.rs`): its design (`object_design.rs`) built in the world the
//! eye sees, as convex hulls (the box's blocks left once the changes took theirs out, each cut by
//! the changes' planes), round parts (a cylinder or a cone in its block) and balls (S153). What a
//! ray meets first (`Body::cast`, from any point; `Body::hit`, the eye's), and whether a point lies
//! in it, serve the exact visible edges (`object_answer.rs`) and the picture's depth test
//! (`object_picture.rs`); standing on the ground (`Scene`, S153), whether a point is lit.
//!
//! The world: y up, z away from the eye, which looks level from the origin.

use crate::kinds::objects::object_design::{Block, Design, Local};
use crate::kinds::objects::object_hull::{hull, minus, round};
use crate::kinds::objects::object_rays::{
    ball_hit, ground_hit, hull_hit, round_hit, round_off, round_values,
};
use crate::kinds::solids::solid_answer::Square;
use crate::maths::vector::{Vector, add, dot, norm, sub, times};

/// Within this of a surface (world units) a point lies on it.
pub(crate) const TOUCH: f64 = 1e-7;
/// A round part's surfaces are numbered from here, a hull's below, a ball's from `BALL_SURFACES`;
/// the ground is `GROUND` (S153).
pub(crate) const ROUND_SURFACES: u32 = 1_000;
pub(crate) const BALL_SURFACES: u32 = 2_000;
pub(crate) const GROUND: u32 = 3_000;
/// How far a shadow ray starts off its surface (world units): clear of the surface it leaves,
/// far nearer than any part's thickness.
const LIFT: f64 = 1e-9;

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

/// A ball (S153): its centre and radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ball {
    pub(crate) centre: Vector,
    pub(crate) radius: f64,
}

/// A capsule (S156k): the points within `radius` of the segment between its ends, a cylinder
/// and the two balls that close it; a chain of them is a tube swept along a path (contours).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Capsule {
    pub(crate) ends: [Vector; 2],
    pub(crate) radius: f64,
}

impl Capsule {
    /// The point of its segment nearest `p`.
    pub(crate) fn nearest(&self, p: Vector) -> Vector {
        let [a, b] = self.ends;
        let axis = sub(b, a);
        let long = dot(axis, axis);
        let along = if long > 0.0 {
            (dot(sub(p, a), axis) / long).clamp(0.0, 1.0)
        } else {
            0.0
        };
        add(a, times(axis, along))
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
    pub(crate) balls: Vec<Ball>,
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
            balls: Vec::new(),
        }
    }

    /// What the ray from the eye along `direction` meets first; `t` 1 at `direction` itself.
    pub(crate) fn hit(&self, direction: Vector) -> Option<Hit> {
        self.cast(([0.0; 3], direction))
    }

    /// What the ray from `origin` along `d` meets first; `t` 1 at `origin + d` (S153).
    pub(crate) fn cast(&self, (origin, d): (Vector, Vector)) -> Option<Hit> {
        let hulls = (self.hulls.iter()).filter_map(|hull| hull_hit(hull, origin, d));
        let rounds = (self.rounds.iter().enumerate())
            .filter_map(|(index, round)| round_hit(round, (origin, d), index));
        let balls = (self.balls.iter().enumerate())
            .filter_map(|(index, ball)| ball_hit(ball, (origin, d), index));
        (hulls.chain(rounds).chain(balls)).min_by(|a, b| a.t.total_cmp(&b.t))
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
        for ball in &self.balls {
            let value = norm(sub(p, ball.centre)) - ball.radius;
            if take(fold((f64::NEG_INFINITY, 0), value)) {
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
        let in_ball = |ball: &Ball| norm(sub(p, ball.centre)) - ball.radius <= margin;
        self.hulls.iter().any(in_hull)
            || self.rounds.iter().enumerate().any(in_round)
            || self.balls.iter().any(in_ball)
    }
}

/// An object standing on the ground (S153): what a ray meets first, the ground a surface of the
/// scene (`GROUND`, its normal up), and whether a point is lit by a light.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Scene {
    pub(crate) body: Body,
    /// The ground's height (world y).
    pub(crate) ground: f64,
}

impl Scene {
    /// What the ray from `origin` along `d` meets first, the object or the ground.
    pub(crate) fn cast(&self, (origin, d): (Vector, Vector)) -> Option<Hit> {
        let body = self.body.cast((origin, d));
        let ground = ground_hit(self.ground, (origin, d)).map(|t| Hit {
            t,
            normal: [0.0, 1.0, 0.0],
            round: false,
            surface: GROUND,
        });
        match (body, ground) {
            (Some(body), Some(ground)) if ground.t < body.t => Some(ground),
            (Some(body), _) => Some(body),
            (None, ground) => ground,
        }
    }

    /// Whether the point `p` of a surface facing `normal` is lit by the light lying `towards` it
    /// (one long) `reach` away (`f64::INFINITY` for the sun): it faces the light and the ray
    /// leaving it towards the light meets nothing of the object on its way.
    pub(crate) fn lit(
        &self,
        (p, normal): (Vector, Vector),
        (towards, reach): (Vector, f64),
    ) -> bool {
        if dot(normal, towards) <= 0.0 {
            return false;
        }
        let from = add(p, times(normal, LIFT));
        self.body
            .cast((from, towards))
            .is_none_or(|hit| hit.t >= reach)
    }
}
