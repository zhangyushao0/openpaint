//! Where rays meet an object's parts (S94, `object_body.rs`): a convex hull entered through its
//! planes, a cylinder or a cone through its ends' slab and its side, a ball (S153), a capsule
//! (S156k: a contour exercise's tube is a chain of them); and how far a
//! point lies past a round part's surfaces. A ray starts anywhere (S153: a shadow ray leaves a
//! surface towards the light): the eye's start at the origin, and a ray from `origin` is the same
//! test with each plane's offset and each round part's centre moved by it.

use crate::kinds::objects::object_body::{
    BALL_SURFACES, Ball, Capsule, Hit, Hull, ROUND_SURFACES, Round,
};
use crate::maths::vector::{Vector, add, dot, norm, sub, times, unit};

/// How far `p` lies outside `round` (negative inside).
pub(crate) fn round_off(round: &Round, p: Vector) -> f64 {
    round_values(round, p)
        .into_iter()
        .fold(f64::NEG_INFINITY, f64::max)
}

/// How far `p` lies past `round`'s base, past its top and past its side (negative inside each;
/// the side's measured square to the axis: enough to tell in from out).
pub(crate) fn round_values(round: &Round, p: Vector) -> [f64; 3] {
    let from = sub(p, round.base.centre());
    let along = dot(from, round.axis);
    let radial = norm(sub(from, times(round.axis, along)));
    let reach = if round.cone {
        round.radius() * (1.0 - along / round.length)
    } else {
        round.radius()
    };
    [-along, along - round.length, radial - reach]
}

/// Where the ray from `origin` along `d` enters `hull`.
pub(crate) fn hull_hit(hull: &Hull, origin: Vector, d: Vector) -> Option<Hit> {
    let (mut enter, mut leave, mut by) = (f64::NEG_INFINITY, f64::INFINITY, 0);
    for (k, plane) in hull.planes.iter().enumerate() {
        let offset = plane.offset - dot(plane.normal, origin);
        let towards = dot(plane.normal, d);
        if towards.abs() < 1e-15 {
            if offset < 0.0 {
                return None;
            }
            continue;
        }
        let t = offset / towards;
        if towards < 0.0 {
            if t > enter {
                (enter, by) = (t, k);
            }
        } else {
            leave = leave.min(t);
        }
    }
    (enter <= leave && enter > 0.0).then(|| Hit {
        t: enter,
        normal: hull.planes[by].normal,
        round: false,
        surface: hull.surfaces[by],
    })
}

/// Where the ray from `origin` along `d` enters `round`, the object's round part `index`: through
/// its base's or top's slab and its side.
pub(crate) fn round_hit(round: &Round, (origin, d): (Vector, Vector), index: usize) -> Option<Hit> {
    let centre = sub(round.base.centre(), origin);
    let w = times(centre, -1.0);
    let (wa, da) = (dot(w, round.axis), dot(d, round.axis));
    // The slab between the base (0) and the top (`length`) along the axis.
    let (low, high) = if da.abs() < 1e-15 {
        if (0.0..=round.length).contains(&wa) {
            (f64::NEG_INFINITY, f64::INFINITY)
        } else {
            return None;
        }
    } else {
        let [t0, t1] = [-wa / da, (round.length - wa) / da];
        (t0.min(t1), t0.max(t1))
    };
    let (wp, dp) = (sub(w, times(round.axis, wa)), sub(d, times(round.axis, da)));
    let r = round.radius();
    // Inside the side where |wp + t dp|² <= reach(t)², reach = r (1 - s / length) for a cone.
    let k = if round.cone { r / round.length } else { 0.0 };
    let (g0, g1) = (r - k * wa, -k * da);
    let quadratic = [
        dot(dp, dp) - g1 * g1,
        2.0 * dot(wp, dp) - 2.0 * g0 * g1,
        dot(wp, wp) - g0 * g0,
    ];
    let inside = |t: f64| {
        let [a, b, c] = quadratic;
        a * t * t + b * t + c <= 0.0 && g0 + g1 * t >= 0.0
    };
    let (enter, leave) = interval(quadratic, (low, high), inside)?;
    (enter > 0.0 && enter <= leave).then(|| {
        let at = add(origin, times(d, enter));
        let on_end = (enter - low).abs() <= 1e-12 * enter.abs().max(1.0);
        let normal = if on_end {
            times(round.axis, if da > 0.0 { -1.0 } else { 1.0 })
        } else {
            side_normal(round, at)
        };
        Hit {
            t: enter,
            normal,
            round: true,
            surface: ROUND_SURFACES + (index * 4 + usize::from(on_end)) as u32,
        }
    })
}

/// Where the ray from `origin` along `d` enters `ball`, the object's ball `index` (S153): the
/// nearer root of `|origin + t d - centre|² = r²`, in front of the start.
pub(crate) fn ball_hit(ball: &Ball, (origin, d): (Vector, Vector), index: usize) -> Option<Hit> {
    let w = sub(origin, ball.centre);
    let [a, b, c] = [
        dot(d, d),
        2.0 * dot(w, d),
        dot(w, w) - ball.radius * ball.radius,
    ];
    let disc = b * b - 4.0 * a * c;
    if a <= 0.0 || disc < 0.0 {
        return None;
    }
    let enter = (-b - disc.sqrt()) / (2.0 * a);
    (enter > 0.0).then(|| Hit {
        t: enter,
        normal: unit(add(w, times(d, enter))),
        round: true,
        surface: BALL_SURFACES + index as u32,
    })
}

/// Where the ray from `origin` along `d` enters `capsule` (S156k), its surface `surface`: the first
/// of its side's entry within its length and its end balls' (it is their union, and convex); the
/// outward normal there from the nearest point of its segment.
pub(crate) fn capsule_hit(
    capsule: &Capsule,
    (origin, d): (Vector, Vector),
    surface: u32,
) -> Option<Hit> {
    let [a, b] = capsule.ends;
    let r = capsule.radius;
    let axis = sub(b, a);
    let long = norm(axis);
    let side = if long > 0.0 {
        let e = times(axis, 1.0 / long);
        let w = sub(origin, a);
        let (we, de) = (dot(w, e), dot(d, e));
        let (wp, dp) = (sub(w, times(e, we)), sub(d, times(e, de)));
        let [qa, qb, qc] = [dot(dp, dp), 2.0 * dot(wp, dp), dot(wp, wp) - r * r];
        let disc = qb * qb - 4.0 * qa * qc;
        let t = (-qb - disc.max(0.0).sqrt()) / (2.0 * qa);
        let on = qa > 1e-15 && disc >= 0.0 && t > 0.0;
        (on && (0.0..=long).contains(&(we + t * de))).then_some(t)
    } else {
        None
    };
    let ball = |centre: Vector| ball_hit(&Ball { centre, radius: r }, (origin, d), 0);
    let ends = [a, b]
        .into_iter()
        .filter_map(|end| ball(end).map(|hit| hit.t));
    let t = side.into_iter().chain(ends).min_by(f64::total_cmp)?;
    let at = add(origin, times(d, t));
    Some(Hit {
        t,
        normal: unit(sub(at, capsule.nearest(at))),
        round: true,
        surface,
    })
}

/// Where the ray from `origin` along `d` meets the ground at the height `ground` from above (S153);
/// none for a ray along it, away from it or starting under it.
pub(crate) fn ground_hit(ground: f64, (origin, d): (Vector, Vector)) -> Option<f64> {
    let t = (ground - origin[1]) / d[1];
    (d[1] < 0.0 && origin[1] >= ground && t > 0.0).then_some(t)
}

/// Where in `[low, high]` the ray is `inside`: the breaks the quadratic's roots make, each piece
/// tried at its middle (the part is convex, so the pieces inside run together).
fn interval(
    [a, b, c]: [f64; 3],
    (low, high): (f64, f64),
    inside: impl Fn(f64) -> bool,
) -> Option<(f64, f64)> {
    let mut breaks = vec![low, high];
    if a.abs() > 1e-15 {
        let disc = b * b - 4.0 * a * c;
        if disc >= 0.0 {
            let root = disc.sqrt();
            breaks.extend([(-b - root) / (2.0 * a), (-b + root) / (2.0 * a)]);
        }
    } else if b.abs() > 1e-15 {
        breaks.push(-c / b);
    }
    breaks.retain(|t| t.is_finite() && *t >= low && *t <= high);
    breaks.sort_by(f64::total_cmp);
    let mut held: Option<(f64, f64)> = None;
    for pair in breaks.windows(2) {
        if pair[1] > pair[0] && inside((pair[0] + pair[1]) / 2.0) {
            held = Some(held.map_or((pair[0], pair[1]), |(from, _)| (from, pair[1])));
        }
    }
    held
}

/// The outward normal of `round`'s side at `at`.
fn side_normal(round: &Round, at: Vector) -> Vector {
    let from = sub(at, round.base.centre());
    let out = unit(sub(from, times(round.axis, dot(from, round.axis))));
    if round.cone {
        unit(add(
            times(out, round.length),
            times(round.axis, round.radius()),
        ))
    } else {
        out
    }
}
