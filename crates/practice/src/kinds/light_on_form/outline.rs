//! Light on form's lines of the form (S153): its visible edges and outlines, printed grey (an
//! object's as `object_answer.rs` finds them, a sphere's outline as `solid_answer.rs` does); where
//! it meets the ground (a box's base edges, a round part's rim standing on it or the line a lying
//! one rests on, a ball's lowest point), what its contact lies along; and the visible edges where
//! a lit face meets an unlit one, which a flat face's core lies along.

use crate::geometry::light::Light;
use crate::kinds::light_on_form::scene::LightTask;
use crate::kinds::objects::object_answer::seen;
use crate::kinds::objects::object_body::{Body, Scene};
use crate::kinds::solids::solid_answer::{Body as Solid, View, answer};
use crate::maths::path::Point;
use crate::maths::vector::{Vector, dot, norm, sub, times, unit};

/// Within this of the ground (world units) a point stands on it; within this of a plane, on it.
const ON: f64 = 1e-9;
/// A rim standing on the ground drawn in this many pieces.
const RIM: usize = 96;
/// An edge whose middle lies this share of the way short of the eye's ray's end is seen.
const SEEN: f64 = 1e-6;

/// The form's visible edges and outlines, as `task`'s view sees them (frame units).
pub(crate) fn printed(task: &LightTask) -> Vec<Vec<Point>> {
    let body = &task.scene.body;
    if let Some(ball) = body.balls.first() {
        let outline = answer(&Solid::Sphere(ball.centre, ball.radius), task.view, &[]);
        return (outline.outlines.into_iter().flatten())
            .map(|stretch| stretch.points)
            .collect();
    }
    let found = seen(body, task.view).answer;
    let sides = found.sides.iter().map(|side| side.to_vec());
    let curves = (found.outlines.into_iter().flatten())
        .filter(|stretch| !stretch.hidden)
        .map(|stretch| stretch.points);
    sides.chain(curves).collect()
}

/// Where the form of `scene` meets the ground, through `view`: lines, or a point alone.
pub(crate) fn contact(scene: &Scene, view: View) -> Vec<Vec<Point>> {
    let on = |p: Vector| (p[1] - scene.ground).abs() <= ON;
    let body = &scene.body;
    let mut lines: Vec<Vec<Point>> = Vec::new();
    for hull in &body.hulls {
        for edge in &hull.edges {
            let [a, b] = edge.map(|p| body.placed.world(p));
            if on(a) && on(b) {
                lines.push(vec![view.at(a), view.at(b)]);
            }
        }
    }
    for round in &body.rounds {
        let mut lowest: Vec<Vector> = Vec::new();
        for square in [round.base, round.top()] {
            let centre = square.centre();
            let level = sub([0.0, 1.0, 0.0], times(round.axis, round.axis[1]));
            if norm(square.u) <= 0.0 {
                // A cone's point.
                if on(centre) {
                    lowest.push(centre);
                }
            } else if norm(level) <= ON {
                // A level rim stands on the ground or lies above it.
                if on(centre) {
                    let tau = std::f64::consts::TAU;
                    let at = |k: usize| view.at(square.rim(k as f64 / RIM as f64 * tau));
                    lines.push((0..=RIM).map(at).collect());
                }
            } else {
                let low = sub(centre, times(unit(level), round.radius()));
                if on(low) {
                    lowest.push(low);
                }
            }
        }
        if let [a, b] = lowest.as_slice() {
            lines.push(vec![view.at(*a), view.at(*b)]);
        }
    }
    for ball in &body.balls {
        let low = sub(ball.centre, [0.0, ball.radius, 0.0]);
        if on(low) {
            lines.push(vec![view.at(low)]);
        }
    }
    lines
}

/// The visible edges of `scene`'s form where a face `light` lights meets one it does not, through
/// `view`: a flat face's core lies along them.
pub(crate) fn creases(scene: &Scene, light: &Light, view: View) -> Vec<[Point; 2]> {
    let body = &scene.body;
    let mut creases = Vec::new();
    for hull in &body.hulls {
        for edge in &hull.edges {
            let [a, b] = edge.map(|p| body.placed.world(p));
            let (towards, _) = light.from(a);
            let turns: Vec<f64> = (hull.planes.iter())
                .filter(|plane| {
                    let off = |p: Vector| (dot(plane.normal, p) - plane.offset).abs();
                    off(a) <= ON * 1e3 && off(b) <= ON * 1e3
                })
                .map(|plane| dot(plane.normal, towards))
                .collect();
            let lit = turns.iter().any(|turn| *turn > 0.0);
            let unlit = turns.iter().any(|turn| *turn <= 0.0);
            let middle = halfway(a, b);
            if lit && unlit && visible(body, middle) {
                creases.push([view.at(a), view.at(b)]);
            }
        }
    }
    creases
}

/// The middle of `a` and `b`.
fn halfway(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|k| (a[k] + b[k]) / 2.0)
}

/// Whether the eye sees `p` of `body`: nothing of it nearer on its ray.
fn visible(body: &Body, p: Vector) -> bool {
    body.hit(p).is_none_or(|hit| hit.t >= 1.0 - SEEN)
}
