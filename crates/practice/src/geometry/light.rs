//! The light (S153), shared by the kinds that paint or build light and shadow: a sun, a direction,
//! or a lamp, a point above the ground (its foot the point of the ground under it). A picture gives
//! it as a perspective drawing does: the sun by a pin, a printed upright on the ground, its top and
//! its shadow's tip, from which its direction is exact (`sun_read`); a lamp by its point and its
//! foot. A point's shadow falls on a plane (the ground, a box's face) where the way from the light
//! through it meets the plane beyond it. A sun not parallel to the picture has two vanishing
//! points: the light's, where the rays through the corners run from (the sun's own where it lies
//! beyond the picture, the anti-sun's below the horizon where it lies behind the eye), and its
//! shadows' on the horizon right under it, where the ground lines run from.
//!
//! The world: y up, z away from the eye at the origin, which looks level; the image is the eye's
//! (`perspective_eye.rs`: x right, y down, focal length 1).

use crate::maths::path::Point;
use crate::maths::vector::{Vector, add, dot, norm, sub, times, unit};

/// A plane: the points `normal · p = offset`, `normal` one long.
pub(crate) type Plane = (Vector, f64);

/// The ground's plane at the height `ground`.
pub(crate) const fn ground_plane(ground: f64) -> Plane {
    ([0.0, 1.0, 0.0], ground)
}

/// Where the light comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Light {
    /// The sun: the way towards it, one long.
    Sun(Vector),
    /// A lamp: its point.
    Lamp(Vector),
}

impl Light {
    /// The sun `elevation` (radians) above the ground, its way along the ground turned `azimuth`
    /// (radians) from the way back to the eye (0: from behind the eye, a quarter: from the right,
    /// a half: from beyond the picture).
    pub(crate) fn sun(elevation: f64, azimuth: f64) -> Self {
        let (up, along) = elevation.sin_cos();
        let (right, back) = azimuth.sin_cos();
        Self::Sun([along * right, up, -along * back])
    }

    /// From `at`, the way to the light (one long) and how far it lies (`f64::INFINITY` for the
    /// sun).
    pub(crate) fn from(&self, at: Vector) -> (Vector, f64) {
        match *self {
            Self::Sun(way) => (way, f64::INFINITY),
            Self::Lamp(point) => {
                let to = sub(point, at);
                (unit(to), norm(to))
            }
        }
    }

    /// Where the shadow of `p` falls on `plane`: where the way from the light through `p` meets it
    /// beyond `p`; none where it never does (the way along the plane, or away from it).
    pub(crate) fn shadow_on(&self, p: Vector, (normal, offset): Plane) -> Option<Vector> {
        let (to, _) = self.from(p);
        let away = times(to, -1.0);
        let towards = dot(normal, away);
        if towards.abs() < 1e-15 {
            return None;
        }
        let t = (offset - dot(normal, p)) / towards;
        (t >= -1e-12).then(|| add(p, times(away, t.max(0.0))))
    }

    /// Where the shadow of `p` falls on the ground at the height `ground`.
    pub(crate) fn shadow(&self, p: Vector, ground: f64) -> Option<Vector> {
        self.shadow_on(p, ground_plane(ground))
    }

    /// A lamp's foot on the ground at the height `ground`; none for the sun.
    pub(crate) fn foot(&self, ground: f64) -> Option<Vector> {
        match *self {
            Self::Sun(_) => None,
            Self::Lamp([x, _, z]) => Some([x, ground, z]),
        }
    }

    /// A sun's vanishing points in the image (image units, y down): the light's (the sun's own
    /// beyond the picture, the anti-sun's behind the eye) and its shadows' on the horizon under
    /// it; none for a lamp or a sun parallel to the picture, whose rays stay parallel.
    pub(crate) fn vanishing(&self) -> Option<[Point; 2]> {
        let Self::Sun([x, y, z]) = *self else {
            return None;
        };
        (z.abs() > 1e-9).then(|| [[x / z, -y / z], [x / z, 0.0]])
    }
}

/// The shadow a box (its eight `corners`, `PerspectiveBox`'s order: bit 1 up) standing on the
/// ground at `ground` throws on it under `light`: the outline round its base corners and its top
/// corners' shadows, each with the corner it came from (its index), in order round; none where a
/// top corner throws none.
pub(crate) fn box_shadow(
    light: &Light,
    corners: &[Vector; 8],
    ground: f64,
) -> Option<Vec<(Vector, usize)>> {
    let mut points: Vec<(Vector, usize)> = Vec::new();
    for (index, corner) in corners.iter().enumerate() {
        let point = if index & 2 == 0 {
            *corner
        } else {
            light.shadow(*corner, ground)?
        };
        points.push((point, index));
    }
    Some(hull(&points))
}

/// The points of `points` round their outline on the ground (x and z), in order round; those
/// inside it or on its sides between two others left out.
fn hull(points: &[(Vector, usize)]) -> Vec<(Vector, usize)> {
    let mut sorted: Vec<(Vector, usize)> = points.to_vec();
    sorted.sort_by(|(a, _), (b, _)| a[0].total_cmp(&b[0]).then(a[2].total_cmp(&b[2])));
    let turn = |o: Vector, a: Vector, b: Vector| {
        (a[0] - o[0]) * (b[2] - o[2]) - (a[2] - o[2]) * (b[0] - o[0])
    };
    let mut round: Vec<(Vector, usize)> = Vec::new();
    for pass in [false, true] {
        let start = round.len();
        let walk: Vec<(Vector, usize)> = if pass {
            sorted.iter().rev().copied().collect()
        } else {
            sorted.clone()
        };
        for point in walk {
            while round.len() >= start + 2
                && turn(round[round.len() - 2].0, round[round.len() - 1].0, point.0) <= 1e-12
            {
                round.pop();
            }
            round.push(point);
        }
        round.pop();
    }
    round
}

/// A pin: a printed upright standing on the ground, its foot and its top.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Pin {
    pub(crate) foot: Vector,
    pub(crate) top: Vector,
}

impl Pin {
    /// A pin standing at `foot`, `height` tall.
    pub(crate) fn at(foot: Vector, height: f64) -> Self {
        Self {
            foot,
            top: add(foot, [0.0, height, 0.0]),
        }
    }

    /// Its shadow's tip under `light`: the top's shadow on the ground at its foot.
    pub(crate) fn tip(&self, light: &Light) -> Option<Vector> {
        light.shadow(self.top, self.foot[1])
    }
}

/// The sun a pin gives in the image of the eye whose image lies `scale` frame units a unit and
/// moved by `offset` (a frame point `p` lies at image `(p - offset) / scale`) over the ground at
/// `ground`: from the frame points of its foot, its top and its shadow's tip, the foot and the tip
/// on the ground, the top above the foot. Exact: the way from the tip to the top.
#[cfg(test)]
pub(crate) fn sun_read(
    (scale, offset): (f64, Point),
    ground: f64,
    [foot, top, tip]: [Point; 3],
) -> Option<Vector> {
    let ray = |p: Point| [(p[0] - offset[0]) / scale, -(p[1] - offset[1]) / scale, 1.0];
    let on_ground = |p: Point| {
        let d = ray(p);
        (d[1] < 0.0).then(|| times(d, ground / d[1]))
    };
    let (foot, tip) = (on_ground(foot)?, on_ground(tip)?);
    // The top stands over the foot: at its distance, as high as its image says.
    let height = ray(top)[1] * foot[2];
    let top = [foot[0], height, foot[2]];
    Some(unit(sub(top, tip)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shadow_lies_on_its_plane_on_the_way_from_the_light() {
        let ground = -2.0;
        for light in [
            Light::sun(0.7, 0.4),
            Light::sun(0.6, 2.7),
            Light::Lamp([0.5, 1.0, 3.0]),
        ] {
            let p = [0.3, -1.2, 4.0];
            let shadow = light.shadow(p, ground).expect("on the ground");
            assert!((shadow[1] - ground).abs() < 1e-12);
            let (to_light, _) = light.from(shadow);
            let (to_light_p, _) = light.from(p);
            assert!(norm(sub(to_light, to_light_p)) < 1e-12, "{light:?}");
        }
        // A point above the lamp throws no shadow down.
        assert_eq!(
            Light::Lamp([0.0, 0.0, 3.0]).shadow([0.0, 0.5, 3.0], -2.0),
            None
        );
    }

    #[test]
    fn a_sun_s_shadow_vanishing_point_lies_on_the_horizon_under_its_light_s() {
        let sun = Light::sun(0.6, 2.6);
        let [light, shadows] = sun.vanishing().expect("not parallel");
        assert!((light[0] - shadows[0]).abs() < 1e-15 && shadows[1] == 0.0);
        // In front of the eye the sun lies above the horizon; behind it the anti-sun below.
        assert!(light[1] < 0.0);
        let [anti, _] = Light::sun(0.6, 0.5).vanishing().expect("not parallel");
        assert!(anti[1] > 0.0);
        assert_eq!(
            Light::sun(0.6, std::f64::consts::FRAC_PI_2).vanishing(),
            None
        );
    }
}
