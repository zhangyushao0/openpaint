//! Two forms passing into each other (S156k), in one box's space: both lie along its three
//! directions, so they share its vanishing points (Drawabox's forms in one space). A form is a box,
//! a cylinder filling a block of square ends, or a sphere, in the scene's own coordinates (world
//! lengths along the box's width, height and depth from its corner 0, `Placed`); built as unit 4's
//! objects' parts (`object_body.rs`) for the depth test: what a ray meets first, its hull's, round
//! part's or ball's hit.
//!
//! Each form's lines as it is drawn through: a box's twelve edges, a cylinder's two rims and the
//! two lines along its side where planes through the eye touch it, a sphere's outline (the circle
//! where the eye's lines of sight touch it).

use crate::kinds::contours::sections::Circle;
use crate::kinds::intersections::track::Track;
use crate::kinds::objects::object_body::{Ball, Body, Placed};
use crate::kinds::objects::object_design::{Block, Local, Turned};
use crate::kinds::objects::object_hull::{hull, round};
use crate::maths::vector::{Vector, dot, norm, sub, times};

/// A form in the scene's own coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Form {
    Box(Block),
    /// A cylinder along `axis` (0 width, 1 height, 2 depth) filling `block`, whose two other
    /// sides are alike.
    Cylinder(Block, usize),
    Sphere(Local, f64),
}

/// The other two of the three axes, in order.
pub(crate) const fn others(axis: usize) -> [usize; 2] {
    match axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    }
}

impl Form {
    /// How far the point `p` (the scene's coordinates) lies outside it, negative inside: exact on
    /// its surface, enough to tell in from out elsewhere.
    pub(crate) fn off(&self, p: Local) -> f64 {
        match *self {
            Self::Box(block) => (0..3)
                .map(|k| (block.low[k] - p[k]).max(p[k] - block.high[k]))
                .fold(f64::NEG_INFINITY, f64::max),
            Self::Cylinder(block, axis) => {
                let [j, m] = others(axis);
                let radius = (block.high[j] - block.low[j]) / 2.0;
                let middle = [j, m].map(|k| (block.low[k] + block.high[k]) / 2.0);
                let radial = (p[j] - middle[0]).hypot(p[m] - middle[1]);
                (block.low[axis] - p[axis])
                    .max(p[axis] - block.high[axis])
                    .max(radial - radius)
            }
            Self::Sphere(centre, radius) => norm(sub(p, centre)) - radius,
        }
    }

    /// A cylinder's radius, its axis's line (a point on it and the axis), its length's ends.
    pub(crate) fn round(&self) -> Option<(f64, Local, usize)> {
        let Self::Cylinder(block, axis) = *self else {
            return None;
        };
        let [j, m] = others(axis);
        let mut at = [0.0; 3];
        at[j] = (block.low[j] + block.high[j]) / 2.0;
        at[m] = (block.low[m] + block.high[m]) / 2.0;
        at[axis] = block.low[axis];
        Some(((block.high[j] - block.low[j]) / 2.0, at, axis))
    }

    /// It built into `body` in the world `placed` lays the scene in; `surface` its first surface.
    pub(crate) fn built(&self, body: &mut Body, placed: Placed, surface: u32) {
        match *self {
            Self::Box(block) => {
                if let Some(mut hull) = hull(&block, &[], placed) {
                    hull.surfaces = (0..hull.planes.len() as u32).map(|k| surface + k).collect();
                    body.hulls.push(hull);
                }
            }
            Self::Cylinder(block, axis) => {
                let turned = Turned {
                    block,
                    axis,
                    cone: false,
                };
                body.rounds.push(round(&turned, placed));
            }
            Self::Sphere(centre, radius) => body.balls.push(Ball {
                centre: placed.world(centre),
                radius,
            }),
        }
    }

    /// Its lines as it is drawn through, in the world.
    pub(crate) fn lines(&self, placed: Placed) -> Vec<Track> {
        match *self {
            Self::Box(block) => (crate::geometry::perspective::EDGES.iter())
                .map(|(a, b, _)| Track::Line([*a, *b].map(|i| placed.world(corner(block, i)))))
                .collect(),
            Self::Cylinder(..) => {
                let Some(circles) = self.rims(placed) else {
                    return Vec::new();
                };
                let mut lines: Vec<Track> = circles.iter().map(|c| Track::whole(*c)).collect();
                lines.extend(outlines(&circles).into_iter().map(Track::Line));
                lines
            }
            Self::Sphere(centre, radius) => {
                vec![Track::whole(silhouette(placed.world(centre), radius))]
            }
        }
    }

    /// A cylinder's two rims in the world, its base's first.
    pub(crate) fn rims(&self, placed: Placed) -> Option<[Circle; 2]> {
        let (radius, base, axis) = self.round()?;
        let Self::Cylinder(block, _) = *self else {
            return None;
        };
        let [j, m] = others(axis);
        let [u, v] = [j, m].map(|k| times(placed.axes[k], radius));
        let mut top = base;
        top[axis] = block.high[axis];
        Some([base, top].map(|at| Circle {
            centre: placed.world(at),
            u,
            v,
        }))
    }

    /// The points its picture is the convex hull of, in the world.
    pub(crate) fn hull_points(&self, placed: Placed) -> Vec<Vector> {
        let tau = std::f64::consts::TAU;
        let round = |circle: Circle| (0..64).map(move |k| circle.at(k as f64 / 64.0 * tau));
        match *self {
            Self::Box(block) => (0..8).map(|i| placed.world(corner(block, i))).collect(),
            Self::Cylinder(..) => (self.rims(placed).into_iter().flatten())
                .flat_map(round)
                .collect(),
            Self::Sphere(centre, radius) => {
                round(silhouette(placed.world(centre), radius)).collect()
            }
        }
    }
}

/// Corner `i` of `block`, bit 0 its width's far end, bit 1 its top, bit 2 its depth's far end.
pub(crate) fn corner(block: Block, i: usize) -> Local {
    std::array::from_fn(|k| {
        if i & (1 << k) == 0 {
            block.low[k]
        } else {
            block.high[k]
        }
    })
}

/// The two lines along a cylinder between its rims `[base, top]` where planes through the eye
/// touch its side: a rim's outward way `n` holds `n · c + r = 0` (`c` its centre, `r` its radius).
pub(crate) fn outlines([base, top]: &[Circle; 2]) -> Vec<[Vector; 2]> {
    touches(base)
        .map(|turns| turns.map(|t| [base.at(t), top.at(t)]).to_vec())
        .unwrap_or_default()
}

/// The turns of `circle` (of a cylinder's rim) where planes through the eye touch the side.
pub(crate) fn touches(circle: &Circle) -> Option<[f64; 2]> {
    let r = norm(circle.u);
    let (a, b) = (
        dot(circle.u, circle.centre) / r,
        dot(circle.v, circle.centre) / r,
    );
    let reach = a.hypot(b);
    (reach > r).then(|| {
        let (phi, spread) = (b.atan2(a), (-r / reach).acos());
        [phi - spread, phi + spread]
    })
}

/// A ball's outline: the circle where the eye's lines of sight touch it.
pub(crate) fn silhouette(centre: Vector, radius: f64) -> Circle {
    let far = norm(centre).max(f64::MIN_POSITIVE);
    let share = (radius / far).powi(2);
    let r = radius * (1.0 - share).max(0.0).sqrt();
    let [u, v] = crate::maths::vector::across(times(centre, 1.0 / far));
    Circle {
        centre: times(centre, 1.0 - share),
        u: times(u, r),
        v: times(v, r),
    }
}
