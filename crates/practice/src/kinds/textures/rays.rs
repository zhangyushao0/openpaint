//! Texture's strip seen and lit (S154j): the eye looks at the ground from the front and above,
//! orthographically (`view`: no perspective, a strip of a surface), and meets a bump (a ball, a
//! lying cylinder, a plate: the objects' exact hits, `Body::cast`), the ground, or a groove's
//! trough (a half cylinder cut into the ground, its ends upright). A point is in the light family
//! where it faces the sun and the ray from it toward the sun leaves the strip unhit: out of its
//! trough through the opening, past every bump (light on form's rule at a surface's scale).
//!
//! The strip's world: y up, the ground at 0; one world unit is the frame's width.

use crate::kinds::objects::object_body::{GROUND, Scene};
use crate::maths::path::Point;
use crate::maths::vector::{Vector, add, cross, dot, sub, times, unit};

/// How far behind the strip the eye's rays start (world units); how far a ray leaving a surface
/// starts off it.
const FAR: f64 = 10.0;
const LIFT: f64 = 1e-9;
/// A groove's surfaces are numbered from here, two each (its side, its ends).
pub(crate) const GROOVE_SURFACES: u32 = 4_000;

/// A groove: a half cylinder cut into the ground along `along` (one long, level) from `start` for
/// `length`, `radius` wide either side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Groove {
    pub(crate) start: Vector,
    pub(crate) along: Vector,
    pub(crate) length: f64,
    pub(crate) radius: f64,
}

impl Groove {
    /// `p` across the groove: how far along it, how far to its side (level), how far up.
    pub(crate) fn local(&self, p: Vector) -> [f64; 3] {
        let side = cross([0.0, 1.0, 0.0], self.along);
        let from = sub(p, self.start);
        [dot(from, self.along), dot(from, side), from[1]]
    }

    /// Whether the ground's point `p` lies over its opening.
    fn over(&self, p: Vector) -> bool {
        let [along, side, _] = self.local(p);
        (0.0..=self.length).contains(&along) && side.abs() < self.radius
    }

    /// Where the ray from `p` (inside the trough) along `d` leaves it through its side or its
    /// ends, how far along the ray, and the trough's normal there (toward its inside).
    fn exit(&self, p: Vector, d: Vector) -> Option<(f64, Vector)> {
        let [a0, s0, y0] = self.local(p);
        let side = cross([0.0, 1.0, 0.0], self.along);
        let (da, ds, dy) = (dot(d, self.along), dot(d, side), d[1]);
        let mut best: Option<(f64, Vector)> = None;
        let mut keep = |t: f64, normal: Vector| {
            if t > LIFT && best.is_none_or(|(held, _)| t < held) {
                best = Some((t, normal));
            }
        };
        // The side: s² + y² = r², the far root from inside.
        let [a, b, c] = [
            ds * ds + dy * dy,
            2.0 * (s0 * ds + y0 * dy),
            s0 * s0 + y0 * y0 - self.radius * self.radius,
        ];
        let disc = b * b - 4.0 * a * c;
        if a > 1e-15 && disc >= 0.0 {
            let t = (-b + disc.sqrt()) / (2.0 * a);
            let [s, y] = [s0 + t * ds, y0 + t * dy];
            let inward = unit(add(times(side, -s), [0.0, -y, 0.0]));
            keep(t, inward);
        }
        // The ends, upright at 0 and `length` along it.
        if da.abs() > 1e-15 {
            for (end, inward) in [(0.0, 1.0), (self.length, -1.0)] {
                keep((end - a0) / da, times(self.along, inward));
            }
        }
        best
    }
}

/// The strip: its bumps on the ground (`Scene`), its grooves, the way to the sun (one long) and
/// how far the eye looks down (radians).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Strip {
    pub(crate) scene: Scene,
    pub(crate) grooves: Vec<Groove>,
    pub(crate) sun: Vector,
    pub(crate) tilt: f64,
}

/// What the eye meets: the point, its surface's outward normal, which surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Seen {
    pub(crate) at: Vector,
    pub(crate) normal: Vector,
    pub(crate) surface: u32,
}

impl Strip {
    /// The eye's way, and the way up the frame.
    fn view(&self) -> (Vector, Vector) {
        let (sin, cos) = self.tilt.sin_cos();
        ([0.0, -sin, cos], [0.0, cos, sin])
    }

    /// Where the eye sees the frame point `p` (frame units, the frame one wide and half as high)
    /// on the ground, and its ray there.
    pub(crate) fn ray(&self, [u, v]: Point) -> (Vector, Vector) {
        let (d, up) = self.view();
        let on = add([u - 0.5, 0.0, 0.0], times(up, 0.25 - v));
        (sub(on, times(d, FAR)), d)
    }

    /// The frame point (frame units) where the eye sees the world point `p`.
    #[cfg(test)]
    pub(crate) fn frame_point(&self, p: Vector) -> Point {
        let (_, up) = self.view();
        [0.5 + p[0], 0.25 - dot(p, up)]
    }

    /// What the eye sees at the frame point `p`.
    pub(crate) fn seen(&self, p: Point) -> Option<Seen> {
        let (origin, d) = self.ray(p);
        let hit = self.scene.cast((origin, d))?;
        let at = add(origin, times(d, hit.t));
        if hit.surface == GROUND
            && let Some((k, groove)) = (self.grooves.iter().enumerate()).find(|(_, g)| g.over(at))
            && let Some((t, normal)) = groove.exit(at, d)
        {
            let surface = GROOVE_SURFACES + 2 * k as u32 + u32::from(normal[1].abs() < 1e-9);
            return Some(Seen {
                at: add(at, times(d, t)),
                normal,
                surface,
            });
        }
        Some(Seen {
            at,
            normal: hit.normal,
            surface: hit.surface,
        })
    }

    /// Whether `seen` lies in the light family: facing the sun, its ray toward the sun out of its
    /// trough through the opening and past every bump.
    pub(crate) fn lit(&self, seen: &Seen) -> bool {
        let sun = self.sun;
        if dot(seen.normal, sun) <= 0.0 {
            return false;
        }
        let from = add(seen.at, times(seen.normal, LIFT));
        if seen.surface >= GROOVE_SURFACES {
            let groove = self.grooves[((seen.surface - GROOVE_SURFACES) / 2) as usize];
            let opening = -from[1] / sun[1].max(f64::MIN_POSITIVE);
            if groove.exit(from, sun).is_some_and(|(t, _)| t < opening) {
                return false;
            }
        }
        self.scene.lit((seen.at, seen.normal), (sun, f64::INFINITY))
    }
}
