//! A shadow climbing a side (S153, 稳): where the nearer box's shadow meets the farther box's side
//! facing the lamp, the ground's shadow ends there and the side holds the rest: the nearer box's
//! corners seen from the lamp on the side's plane, cut to the side. Each corner there is found as
//! a perspective drawing finds it: the ground line from the lamp's foot through the corner under
//! the top corner meets the side, and the shadow lies straight above, on the ray.

use super::answer::convex;
use crate::geometry::light::Light;
use crate::maths::path::Point;
use crate::maths::vector::{Vector, add, cross, dot, norm, sub, times, unit};

/// A climbing shadow keeps this clear of its side's other edges (world units), and reaches this
/// high up it at least.
const CLIMB_CLEAR: f64 = 0.06;
const CLIMB_LEAST: f64 = 0.15;

/// A shadow climbing a side: its outline's pieces but the one along the ground, its corners there
/// (each a top corner, the corner under it, where the ground line through that meets the side, and
/// the shadow straight above, in the world), and whether it keeps clear of the side's other edges
/// and climbs.
pub(super) struct Climb {
    pub(super) pieces: Vec<[Vector; 2]>,
    pub(super) corners: Vec<[Vector; 4]>,
    pub(super) kept: bool,
}

/// The nearer box's shadow on the farther's side facing the lamp; none where no side faces the lamp
/// with the nearer box before it, or the light is the sun.
pub(super) fn climbed(corners: &[[Vector; 8]], light: &Light) -> Option<Climb> {
    let Light::Lamp(lamp) = *light else {
        return None;
    };
    let (near, far) = (&corners[0], &corners[1]);
    let middle = times(far.iter().fold([0.0; 3], |sum, c| add(sum, *c)), 1.0 / 8.0);
    // The far box's uprights' faces, each its corners: bottom, along, then up.
    let faces = [[0, 1, 2], [4, 5, 6], [0, 4, 2], [1, 5, 3]];
    faces.into_iter().find_map(|[o, along, up]| {
        let p0 = far[o];
        let (u, v) = (sub(far[along], p0), sub(far[up], p0));
        let mut n = unit(cross(u, v));
        if dot(n, sub(middle, p0)) > 0.0 {
            n = times(n, -1.0);
        }
        let height = |p: Vector| dot(n, sub(p, p0));
        let before = (near.iter()).all(|c| height(*c) > 0.0 && height(*c) < height(lamp));
        (height(lamp) > 0.0 && before).then(|| on_side(near, (lamp, p0), (u, v)))
    })
}

/// The shadow `near`'s corners throw from `lamp` on the side from `p0` along `u` and up `v`.
fn on_side(near: &[Vector; 8], (lamp, p0): (Vector, Vector), (u, v): (Vector, Vector)) -> Climb {
    let (width, height) = (norm(u), norm(v));
    let (across, up) = (unit(u), unit(v));
    let n = unit(cross(u, v));
    let flat: Vec<Point> = (near.iter())
        .map(|c| {
            let to = sub(*c, lamp);
            let t = dot(n, sub(p0, lamp)) / dot(n, to);
            let x = sub(add(lamp, times(to, t)), p0);
            [dot(x, across), dot(x, up)]
        })
        .collect();
    let outline = convex(flat.clone());
    let clear = (outline.iter()).filter(|p| p[1] > 0.0).all(|p| {
        p[0] >= CLIMB_CLEAR && p[0] <= width - CLIMB_CLEAR && p[1] <= height - CLIMB_CLEAR
    });
    let cut = clipped(&outline, [width, height]);
    let climbs = cut.iter().any(|p| p[1] >= CLIMB_LEAST);
    let world = |p: Point| add(p0, add(times(across, p[0]), times(up, p[1])));
    let pieces = (0..cut.len())
        .map(|k| [cut[k], cut[(k + 1) % cut.len()]])
        .filter(|[a, b]| !(a[1].abs() <= 1e-9 && b[1].abs() <= 1e-9))
        .map(|[a, b]| [world(a), world(b)])
        .collect();
    // The top corners whose shadows lie on the side, as the outline holds them.
    let on_outline =
        |p: &Point| (outline.iter()).any(|q| (q[0] - p[0]).hypot(q[1] - p[1]) <= 1e-12);
    let corners = (0..8)
        .filter(|index| index & 2 != 0 && flat[*index][1] > 0.0 && on_outline(&flat[*index]))
        .map(|index| {
            let [s, h] = flat[index];
            [
                near[index],
                near[index & !2],
                world([s, 0.0]),
                world([s, h]),
            ]
        })
        .collect();
    Climb {
        pieces,
        corners,
        kept: clear && climbs,
    }
}

/// The convex `outline` cut to the rectangle from the origin to `[width, height]`.
fn clipped(outline: &[Point], [width, height]: [f64; 2]) -> Vec<Point> {
    let edges: [(usize, f64, bool); 4] = [
        (0, 0.0, true),
        (0, width, false),
        (1, 0.0, true),
        (1, height, false),
    ];
    let mut kept = outline.to_vec();
    for (axis, at, above) in edges {
        let inside = |p: &Point| if above { p[axis] >= at } else { p[axis] <= at };
        let mut next = Vec::new();
        for k in 0..kept.len() {
            let (a, b) = (kept[k], kept[(k + 1) % kept.len()]);
            if inside(&a) {
                next.push(a);
            }
            if inside(&a) != inside(&b) {
                let t = (at - a[axis]) / (b[axis] - a[axis]);
                next.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
            }
        }
        kept = next;
    }
    kept
}
