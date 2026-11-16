//! Cast shadows' scene (S153, 投影): two boxes standing on the ground, two-point as the boxes kind's
//! (one yaw, their vanishing points on the horizon in or beyond the frame), the horizon printed,
//! and the light: a sun parallel to the picture (入门: every ray at the pin's slant, every ground
//! line along the pin's shadow, one box perhaps a block lying), a sun beyond the picture or behind
//! the eye (熟练: its light's point above the horizon or the anti-sun's below it, the shadows' point
//! on the horizon under it, both printed as rings, the pin beside), or a lamp (稳: its point and its
//! foot; in every second scene the nearer box's shadow climbs the farther one's side).
//!
//! A scene is laid again from its seed's sequence until it keeps every rule (`rules.rs`).

use crate::geometry::light::{Light, Pin};
use crate::geometry::perspective::PerspectiveBox;
use crate::geometry::perspective_eye::{box_corners, directions, framing, projected};
use crate::kinds::cast_shadows::answer::{Shadows, shadows};
use crate::kinds::solids::solid_answer::View;
use crate::maths::path::Point;
use crate::maths::random::Random;
use crate::maths::vector::Vector;

/// Scenes laid before the generator keeps one whatever it breaks.
const TRIES: usize = 600;
/// The boxes, their shadows and the light's givens in the frame together, and the eye above the
/// ground (world units).
const SIDE: [f64; 2] = [0.84, 0.9];
const EYE: [f64; 2] = [2.2, 3.0];
/// The room above the horizon, a share of how far below it the scene reaches.
const HORIZON_ROOM: f64 = 0.1;

/// Where the light comes from: the sun parallel to the picture (入门), beyond it or behind the eye
/// (熟练), a lamp, its nearer box's shadow climbing the farther in `Climbing` (稳).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Throw {
    Side,
    Front,
    Behind,
    Lamp,
    Climbing,
}

impl Throw {
    /// Every variant, each stored by its place: a harder one goes at the end.
    pub const ALL: [Self; 5] = [
        Self::Side,
        Self::Front,
        Self::Behind,
        Self::Lamp,
        Self::Climbing,
    ];

    /// Whether a lamp gives the light.
    pub(super) const fn lamp(self) -> bool {
        matches!(self, Self::Lamp | Self::Climbing)
    }
}

/// Boxes on the ground whose shadows are drawn, generated the same for the same variant and seed.
#[derive(Debug, Clone, PartialEq)]
pub struct ShadowTask {
    pub throw: Throw,
    /// The boxes as the eye sees them, nearer first: printed, their hidden edges dashed.
    pub boxes: Vec<PerspectiveBox>,
    pub(crate) corners: Vec<[Vector; 8]>,
    pub(crate) ground: f64,
    pub(crate) view: View,
    pub(crate) light: Light,
    /// The pin's foot, top and shadow's tip (frame units), where a sun gives the light.
    pub pinned: Option<[Point; 3]>,
    /// The light's two points (frame units): a sun's light's and shadows' vanishing points, or a
    /// lamp and its foot; none for a sun parallel to the picture.
    pub points: Option<[Point; 2]>,
    /// The shadows' outline and how each corner is found.
    pub shadows: Shadows,
}

impl ShadowTask {
    /// The task `throw` and `seed` make.
    #[must_use]
    pub fn generate(throw: Throw, seed: u64) -> Self {
        let mut random = Random::new(seed);
        let mut last = None;
        for _ in 0..TRIES {
            let task = laid(throw, &mut random);
            if task.keeps() {
                return task;
            }
            last = Some(task);
        }
        last.unwrap_or_else(|| laid(throw, &mut Random::new(seed)))
    }

    /// The horizon's height in the frame.
    #[must_use]
    pub fn horizon(&self) -> f64 {
        self.view.offset[1]
    }
}

/// The sun `throw` takes, laid from `random`: parallel to the picture, beyond it or behind the
/// eye; none for a lamp.
fn sun(throw: Throw, random: &mut Random) -> Option<Light> {
    let degrees = |random: &mut Random, low: f64, high: f64| random.range(low, high).to_radians();
    Some(match throw {
        Throw::Side => {
            let (up, along) = degrees(random, 40.0, 55.0).sin_cos();
            Light::Sun([random.sign() * along, up, 0.0])
        }
        // Low enough that the light's point lies on the paper (`PAPER`).
        Throw::Front => {
            let elevation = degrees(random, 20.0, 30.0);
            let azimuth = std::f64::consts::PI + random.sign() * degrees(random, 8.0, 30.0);
            Light::sun(elevation, azimuth)
        }
        // Well to one side, so the shadows running away show beside the boxes, not behind them.
        Throw::Behind => {
            let elevation = degrees(random, 22.0, 35.0);
            Light::sun(elevation, random.sign() * degrees(random, 20.0, 45.0))
        }
        Throw::Lamp | Throw::Climbing => return None,
    })
}

/// The boxes' middles on the ground (x, z) and the light, laid from `random`: the nearer box at
/// `near`; under a sun parallel to the picture the farther behind it towards the light (their
/// shadows run side by side), else beside it; a lamp before them (between them, or before the
/// nearer with the farther behind it along the lamp's light where the shadow climbs).
fn placed(throw: Throw, random: &mut Random, near: [f64; 2], ground: f64) -> ([f64; 2], Light) {
    if let Some(sun) = sun(throw, random) {
        let far = match sun {
            Light::Sun([x, _, _]) if throw == Throw::Side => [
                near[0] + x.signum() * random.range(1.5, 2.3),
                near[1] + random.range(1.6, 2.2),
            ],
            _ => [
                near[0] + random.sign() * random.range(2.0, 2.8),
                near[1] + random.range(-0.5, 1.0),
            ],
        };
        return (far, sun);
    }
    if throw == Throw::Climbing {
        // Low, before the nearer box and to its side, so the farther (taller) stands beside it
        // on the picture and the nearer's shadow reaches its side.
        let lamp = [
            near[0] - random.sign() * random.range(1.2, 1.8),
            ground + random.range(1.7, 2.0),
            near[1] - random.range(0.3, 0.8),
        ];
        let way = [near[0] - lamp[0], near[1] - lamp[2]];
        let long = way[0].hypot(way[1]).max(f64::MIN_POSITIVE);
        let reach = random.range(1.7, 2.2);
        let far = [
            near[0] + way[0] / long * reach,
            near[1] + way[1] / long * reach,
        ];
        return (far, Light::Lamp(lamp));
    }
    let far = [
        near[0] + random.sign() * random.range(2.2, 3.0),
        near[1] + random.range(-0.5, 1.0),
    ];
    let lamp = [
        (near[0] + far[0]) / 2.0 + random.range(-0.3, 0.3),
        ground + random.range(2.6, 3.4),
        near[1].min(far[1]) - random.range(0.6, 1.4),
    ];
    (far, Light::Lamp(lamp))
}

/// One scene of `throw` laid from `random`, whatever rule it breaks: the world first, then the
/// view framing the boxes, their shadows, the light's givens and the horizon.
fn laid(throw: Throw, random: &mut Random) -> ShadowTask {
    let yaw = random.range(25.0, 65.0).to_radians();
    let ground = -random.range(EYE[0], EYE[1]);
    let near = [random.range(-0.5, 0.5), random.range(4.2, 5.0)];
    let sides = |random: &mut Random, high: f64| {
        [
            random.range(0.6, 1.1),
            random.range(0.6, high),
            random.range(0.6, 1.1),
        ]
    };
    let mut first = sides(random, 1.2);
    let mut second = sides(random, 1.1);
    if throw == Throw::Side && random.unit() < 0.5 {
        // A block lying.
        second = [
            random.range(0.9, 1.4),
            random.range(0.25, 0.4),
            random.range(0.6, 1.0),
        ];
    }
    if throw == Throw::Climbing {
        // The nearer small and low, the farther wide and tall: the shadow climbs the side and
        // stays on it.
        first = [
            random.range(0.5, 0.65),
            random.range(0.75, 0.9),
            random.range(0.5, 0.65),
        ];
        second = [
            random.range(1.1, 1.4),
            random.range(1.0, 1.2),
            random.range(1.1, 1.4),
        ];
    }
    let (far, light) = placed(throw, random, near, ground);
    let corners: Vec<[Vector; 8]> = [(near, first), (far, second)]
        .iter()
        .map(|([x, z], size)| box_corners([*x, ground + size[1] / 2.0, *z], yaw, *size))
        .collect();
    let pin = (!throw.lamp()).then(|| standing(random, &corners, ground));
    let view = framed(&corners, (pin, &light, ground), random);
    let boxes = corners.iter().map(|c| seen_box(c, yaw, view)).collect();
    let pinned = pin.and_then(|pin| {
        let tip = pin.tip(&light)?;
        Some([pin.foot, pin.top, tip].map(|p| view.at(p)))
    });
    let points = points(&light, ground, view);
    let shadows = shadows(&corners, (&light, ground, throw == Throw::Climbing), view);
    ShadowTask {
        throw,
        boxes,
        corners,
        ground,
        view,
        light,
        pinned,
        points,
        shadows,
    }
}

/// The view framing the boxes of `corners`, their shadows under `light` on the ground at `ground`,
/// the pin and its shadow or the lamp and its foot, and the horizon a little under the frame's top,
/// laid from `random`.
fn framed(
    corners: &[[Vector; 8]],
    (pin, light, ground): (Option<Pin>, &Light, f64),
    random: &mut Random,
) -> View {
    let mut seen: Vec<Vector> = corners.iter().flatten().copied().collect();
    for box_corners in corners {
        seen.extend(box_corners.iter().filter_map(|c| light.shadow(*c, ground)));
    }
    seen.extend(
        pin.iter()
            .flat_map(|pin| [pin.top].into_iter().chain(pin.tip(light))),
    );
    if let Light::Lamp(lamp) = *light {
        seen.extend([lamp, [lamp[0], ground, lamp[2]]]);
    }
    let mut image: Vec<Point> = seen.into_iter().map(projected).collect();
    let middle = image.iter().map(|p| p[0]).sum::<f64>() / image.len().max(1) as f64;
    let lowest = image.iter().map(|p| p[1]).fold(0.0, f64::max);
    image.push([middle, -HORIZON_ROOM * lowest]);
    let all: [Point; 8] = std::array::from_fn(|k| bounds_corner(&image, k));
    let shift = [random.range(-0.03, 0.03), random.range(-0.03, 0.03)];
    let (scale, offset) = framing(&all, 0.0, (random.range(SIDE[0], SIDE[1]), shift), None);
    View { scale, offset }
}

/// A corner of the bounds of `points` (`k` 0 to 3; the rest repeat), for framing.
fn bounds_corner(points: &[Point], k: usize) -> Point {
    let low = [0, 1].map(|i| points.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min));
    let high = [0, 1].map(|i| {
        points
            .iter()
            .map(|p| p[i])
            .fold(f64::NEG_INFINITY, f64::max)
    });
    [
        [low[0], low[1]],
        [high[0], low[1]],
        [high[0], high[1]],
        [low[0], high[1]],
    ][k % 4]
}

/// The box of `corners` turned `yaw` as `view` sees it: two-point, below the horizon (its front
/// corner the front edge's top).
fn seen_box(corners: &[Vector; 8], yaw: f64, view: View) -> PerspectiveBox {
    let [left, right, _] = directions(yaw);
    PerspectiveBox {
        corners: corners.map(|c| view.at(c)),
        horizon: view.offset[1],
        vanishing: [Some(view.at(left)), Some(view.at(right)), None],
        front: 2,
    }
}

/// A pin on the ground laid from `random` beside the boxes `corners`: to one side of them, before
/// the nearer.
fn standing(random: &mut Random, corners: &[[Vector; 8]], ground: f64) -> Pin {
    let middle = |c: &[Vector; 8]| [0, 2].map(|k| c.iter().map(|p| p[k]).sum::<f64>() / 8.0);
    let [a, b] = [middle(&corners[0]), middle(&corners[1])];
    let foot = [
        (a[0] + b[0]) / 2.0 + random.sign() * random.range(1.4, 2.4),
        ground,
        a[1].min(b[1]) - random.range(0.2, 1.2),
    ];
    Pin::at(foot, random.range(0.5, 0.8))
}

/// The light's two points in the frame: a sun's light's and shadows' vanishing points, or a lamp
/// and its foot.
fn points(light: &Light, ground: f64, view: View) -> Option<[Point; 2]> {
    let image = |p: Point| {
        [
            p[0] * view.scale + view.offset[0],
            p[1] * view.scale + view.offset[1],
        ]
    };
    match light {
        Light::Sun(_) => light.vanishing().map(|both| both.map(image)),
        Light::Lamp(point) => Some([view.at(*point), view.at(light.foot(ground)?)]),
    }
}
