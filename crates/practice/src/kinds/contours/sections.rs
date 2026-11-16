//! A sausage's contour lines (S156k, Drawabox's contour ellipses): sections of the tube, each a
//! circle of its radius square to the spine, in the middle of one capsule (`sausage.rs`), so it
//! lies on the model exactly; at 稳 one round each end too, on its ball. Seen, the circle's
//! square's image and the exact ellipse in it (`ellipse.rs`'s `square_on`), its minor axis along
//! the spine as the eye sees it; its stretches seen (the surface faces the eye there and nothing
//! hides it) and hidden; and where its ellipse touches the tube's outline (where the surface turns
//! from the eye), a short tick across the outline, where the eye sees it.

use crate::geometry::ellipse::{Ellipse, EllipseGuide, square_on};
use crate::kinds::contours::sausage::Sausage;
use crate::kinds::contours::seen::{State, stretches};
use crate::kinds::solids::solid_answer::{Stretch, View};
use crate::maths::path::Point;
use crate::maths::vector::{Vector, add, cross, dot, sub, times, unit};

/// A tick's half across the outline (frame units); how far past a touch the face is looked at.
const TICK: f64 = 0.02;
const PAST: f64 = 0.03;
/// How far beyond the tube a touch's paper is looked at, a share of the radius.
const BEYOND: f64 = 0.15;
/// How far an end's circle lies out from the end along the spine, a share of the radius.
const CAP: f64 = 0.5;

/// A circle in space: its centre and two radii square to each other (its square's half sides).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Circle {
    pub(crate) centre: Vector,
    pub(crate) u: Vector,
    pub(crate) v: Vector,
}

impl Circle {
    pub(crate) fn at(&self, t: f64) -> Vector {
        add(
            self.centre,
            add(times(self.u, t.cos()), times(self.v, t.sin())),
        )
    }

    /// Its square's corners in order round.
    pub(crate) fn square(&self) -> [Vector; 4] {
        [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]
            .map(|[s, t]| add(self.centre, add(times(self.u, s), times(self.v, t))))
    }
}

/// A contour line: a section of the tube as the eye sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Contour {
    /// Its square seen and the exact ellipse in it, touching its sides' middles.
    pub guide: EllipseGuide,
    /// The ellipse in stretches, seen and hidden.
    pub stretches: Vec<Stretch>,
    /// A short tick across the outline at each place the ellipse touches it, where seen.
    pub ticks: Vec<[Point; 2]>,
    /// Whether it lies round an end (稳), not across the tube.
    pub cap: bool,
    pub(crate) circle: Circle,
    /// The surface's centre at its points: the spine's point (a section's), the end (a cap's).
    pub(crate) core: Vector,
}

impl Contour {
    /// The section in the middle of capsule `k` of `sausage`, seen through `view`.
    pub(crate) fn across(sausage: &Sausage, k: usize, view: View) -> Self {
        let [a, b] = [sausage.spine[k], sausage.spine[k + 1]];
        let along = unit(sub(b, a));
        let turned = add(sausage.normals[k], sausage.normals[k + 1]);
        let u = unit(sub(turned, times(along, dot(turned, along))));
        let v = cross(along, u);
        let centre = times(add(a, b), 0.5);
        let r = sausage.radius;
        let circle = Circle {
            centre,
            u: times(u, r),
            v: times(v, r),
        };
        Self::seen(sausage, (circle, centre, false), view)
    }

    /// The circle round the end of `sausage` (its last when `last`), on the end's ball.
    pub(crate) fn round_end(sausage: &Sausage, last: bool, view: View) -> Self {
        let spine = &sausage.spine;
        let (end, before) = if last {
            (spine[spine.len() - 1], spine[spine.len() - 2])
        } else {
            (spine[0], spine[1])
        };
        let out = unit(sub(end, before));
        let k = if last { sausage.normals.len() - 1 } else { 0 };
        let u = unit(sub(
            sausage.normals[k],
            times(out, dot(sausage.normals[k], out)),
        ));
        let r = sausage.radius * (1.0 - CAP * CAP).sqrt();
        let circle = Circle {
            centre: add(end, times(out, CAP * sausage.radius)),
            u: times(u, r),
            v: times(cross(out, u), r),
        };
        Self::seen(sausage, (circle, end, true), view)
    }

    /// `circle` on `sausage` round `core` seen through `view`.
    fn seen(sausage: &Sausage, (circle, core, cap): (Circle, Vector, bool), view: View) -> Self {
        let guide = square_on(circle.square().map(|p| view.at(p)));
        let state = |t: f64| {
            let p = circle.at(t);
            let faces = dot(sub(p, core), p) < 0.0;
            if faces && sausage.sees(p) {
                State::Seen
            } else {
                State::Hidden
            }
        };
        let tau = std::f64::consts::TAU;
        let stretches = stretches(&|t| circle.at(t), (tau, true), view, &state);
        let mut contour = Self {
            guide,
            stretches,
            ticks: Vec::new(),
            cap,
            circle,
            core,
        };
        if !cap {
            contour.ticks = contour.touching((sausage, view), &state);
        }
        contour
    }

    /// The ticks where its ellipse touches the outline: where the surface turns from the eye
    /// (`n · p = 0`, `n` the way out from the core), each where the face beside it is seen and so
    /// is what lies just beyond it (no nearer part of the tube passes in front there).
    fn touching(
        &self,
        (sausage, view): (&Sausage, View),
        state: &impl Fn(f64) -> State,
    ) -> Vec<[Point; 2]> {
        let Circle { centre, u, v } = self.circle;
        let r2 = dot(u, u);
        // n · p = (u · c cos t + v · c sin t) / r + r = 0.
        let (a, b) = (dot(u, centre), dot(v, centre));
        let reach = a.hypot(b);
        if r2 >= reach {
            return Vec::new();
        }
        let (phi, spread) = (b.atan2(a), (-r2 / reach).acos());
        let beyond = |t: f64| {
            let p = self.circle.at(t);
            sausage.sees(add(p, times(sub(p, centre), BEYOND)))
        };
        [(phi + spread, PAST), (phi - spread, -PAST)]
            .into_iter()
            .filter(|(t, past)| state(t + past) == State::Seen && beyond(*t))
            .map(|(t, _)| {
                let p = view.at(self.circle.at(t));
                let n = normal(&self.guide.exact, p);
                let [x, y] = [n[0] * TICK, n[1] * TICK];
                [[p[0] - x, p[1] - y], [p[0] + x, p[1] + y]]
            })
            .collect()
    }

    /// Its ellipse's points as the eye sees the circle's (frame units), seen or hidden, for a hand
    /// that draws it whole.
    pub(crate) fn whole(&self, pieces: usize) -> Vec<Point> {
        self.guide.exact.outline(pieces)
    }
}

/// The unit normal of `ellipse` at its point `p`.
pub(crate) fn normal(ellipse: &Ellipse, p: Point) -> Point {
    let (sin, cos) = ellipse.angle.sin_cos();
    let [dx, dy] = [p[0] - ellipse.centre[0], p[1] - ellipse.centre[1]];
    let (x, y) = (dx * cos + dy * sin, -dx * sin + dy * cos);
    let [a, b] = ellipse.axes;
    let (gx, gy) = (x / (a * a), y / (b * b));
    let n = [gx * cos - gy * sin, gx * sin + gy * cos];
    let long = n[0].hypot(n[1]).max(f64::MIN_POSITIVE);
    [n[0] / long, n[1] / long]
}
