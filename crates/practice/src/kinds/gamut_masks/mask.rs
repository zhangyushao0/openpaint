//! Gurney's gamut mask (S153, unit 6; James Gurney, *Color and Light*): a shape laid on the hue
//! and chroma wheel (Oklab's `a`, `b` plane, grey in its middle), only the colours inside it used.
//! A triangle's corners are the subjective primaries a painter premixes and paints from; a long
//! diamond across the wheel holds two opposite hues and the greys between; a small triangle off
//! the middle nears monochrome. It trains saturation as a hard limit, V大's least instinctive
//! control 〔p8:476-700〕. A mask is kept in its kind's variant (its shape, the hue its apex points
//! to), so a record's copy is judged against the mask it was painted to whatever the pack becomes.
//!
//! The rule is exact: a colour lies inside the shape, within `INSIDE` (a just-noticeable step), or
//! it does not. A shape here is any convex polygon of the plane, counter-clockwise, so the screen
//! dims the colour wheel and veils the card's disc by the same tests (`distance`, `span`).

use crate::colour_areas::main_hue::{HUE_STEP, turn};
use crate::maths::colour::{Rgb8, in_gamut, most_chroma};
use crate::maths::path::Point;
use crate::maths::random::Random;

/// How far outside the mask a colour may lie and still count inside (Oklab `a`, `b`): a
/// just-noticeable step.
pub const INSIDE: f64 = 0.02;
/// The triangle's apex, a pure hue at the wheel's rim as Gurney's; its base past grey on the other
/// side and its half width, as shares of the apex's chroma: the greys inside it, its base corners
/// muted.
const APEX: f64 = 0.17;
const BASE: f64 = 0.3;
const BASE_HALF: f64 = 0.35;
/// The long diamond's ends (Oklab chroma) and its half width across the grey middle: narrow.
const DIAMOND_END: f64 = 0.15;
const DIAMOND_HALF: f64 = 0.035;
/// The small triangle's apex, its base along the same hue (past grey by more than `INSIDE`
/// reaches, so no grey lies inside) and its half width there.
const SMALL_APEX: f64 = 0.13;
const SMALL_BASE: f64 = 0.045;
const SMALL_HALF: f64 = 0.04;
/// The lightnesses a corner's swatch may be shown at: its hue's most colourful between them, the
/// nearest `MIDDLE` where several are as colourful.
const SHOWN: (f64, f64) = (0.45, 0.85);
const MIDDLE: f64 = 0.65;
/// Hues on the circle, a step of `HUE_STEP` each.
const STEPS: u8 = 36;

/// A mask's shape, harder by level: Gurney's triangle taking in the greys, his long diamond
/// across the wheel, a small triangle off the middle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaskShape {
    Triangle,
    Diamond,
    Small,
}

impl MaskShape {
    pub const ALL: [Self; 3] = [Self::Triangle, Self::Diamond, Self::Small];

    fn index(self) -> u8 {
        let at = Self::ALL
            .iter()
            .position(|shape| *shape == self)
            .unwrap_or(0);
        u8::try_from(at).unwrap_or(0)
    }

    /// How far a mask of it pointing to `hue` keeps from the crop's `main` hue (degrees): its
    /// apex's turn off it; a diamond's nearer end's.
    fn off(self, hue: u16, main: Option<f64>) -> f64 {
        let Some(main) = main else {
            return 180.0;
        };
        let apex = turn(f64::from(hue), main).abs();
        match self {
            Self::Diamond => apex.min(180.0 - apex),
            Self::Triangle | Self::Small => apex,
        }
    }
}

/// A gamut mask: its shape and the hue its apex points to (Oklab, degrees, a multiple of
/// `HUE_STEP`; a diamond's one end).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mask {
    pub shape: MaskShape,
    pub hue: u16,
}

impl Mask {
    /// A mask of `shape` picked by `random` among the hues keeping at least `least` degrees from
    /// the crop's `main` hue (any without one), among the farthest where none keeps so far.
    #[must_use]
    pub fn chosen(shape: MaskShape, main: Option<f64>, least: f64, random: &mut Random) -> Self {
        let all: Vec<u16> = (0..u16::from(STEPS)).map(|step| step * HUE_STEP).collect();
        let farthest = (all.iter()).fold(0.0, |far, hue| shape.off(*hue, main).max(far));
        let least = least.min(farthest);
        let from: Vec<u16> = (all.iter().copied())
            .filter(|hue| shape.off(*hue, main) >= least)
            .collect();
        let at = ((random.unit() * from.len() as f64) as usize).min(from.len() - 1);
        Self {
            shape,
            hue: from[at],
        }
    }

    /// As its kind's variant stores it: its shape's place times the hues a turn, then its hue's
    /// step (108 codes).
    #[must_use]
    pub fn code(self) -> u8 {
        let step = u8::try_from(self.hue / HUE_STEP).unwrap_or(0) % STEPS;
        self.shape.index() * STEPS + step
    }

    /// The mask stored as `code`; none past the last shape.
    #[must_use]
    pub fn of_code(code: u8) -> Option<Self> {
        let shape = *MaskShape::ALL.get(usize::from(code / STEPS))?;
        let hue = u16::from(code % STEPS) * HUE_STEP;
        Some(Self { shape, hue })
    }

    /// Its corners in Oklab's `a`, `b` plane, counter-clockwise: the apex first.
    #[must_use]
    pub fn corners(self) -> Vec<Point> {
        let (sin, cos) = f64::from(self.hue).to_radians().sin_cos();
        let at = |along: f64, across: f64| [cos * along - sin * across, sin * along + cos * across];
        match self.shape {
            MaskShape::Triangle => {
                let (base, half) = (-BASE * APEX, BASE_HALF * APEX);
                vec![at(APEX, 0.0), at(base, half), at(base, -half)]
            }
            MaskShape::Diamond => vec![
                at(DIAMOND_END, 0.0),
                at(0.0, DIAMOND_HALF),
                at(-DIAMOND_END, 0.0),
                at(0.0, -DIAMOND_HALF),
            ],
            MaskShape::Small => vec![
                at(SMALL_APEX, 0.0),
                at(SMALL_BASE, SMALL_HALF),
                at(SMALL_BASE, -SMALL_HALF),
            ],
        }
    }

    /// Its corners' colours, the swatches beside the frame a painter premixes: each at its hue
    /// and chroma, or as much as sRGB holds there, at the lightness within `SHOWN` where sRGB
    /// holds most of it, the nearest `MIDDLE` among those holding as much (a muted corner a middle
    /// value).
    #[must_use]
    pub fn swatches(self) -> Vec<Rgb8> {
        (self.corners().into_iter())
            .map(|[a, b]| {
                let (chroma, hue) = (a.hypot(b), b.atan2(a).to_degrees());
                let mut lightnesses: Vec<f64> = (0..=40)
                    .map(|step| SHOWN.0 + (SHOWN.1 - SHOWN.0) * f64::from(step) / 40.0)
                    .collect();
                lightnesses.sort_by(|x, y| (x - MIDDLE).abs().total_cmp(&(y - MIDDLE).abs()));
                let held = |l: f64| most_chroma(l, hue, chroma);
                let most = lightnesses
                    .iter()
                    .fold(0.0, |most: f64, l| most.max(held(*l)));
                let lightness = (lightnesses.iter().copied())
                    .find(|l| held(*l) >= most - 1e-6)
                    .unwrap_or(MIDDLE);
                in_gamut([lightness, chroma, hue])
            })
            .collect()
    }

    /// Its middle (Oklab `a`, `b`): its corners' mean, inside it.
    #[must_use]
    pub fn middle(self) -> Point {
        let corners = self.corners();
        let count = corners.len().max(1) as f64;
        let [a, b] = (corners.iter()).fold([0.0; 2], |sum, p| [sum[0] + p[0], sum[1] + p[1]]);
        [a / count, b / count]
    }

    /// Whether the colour at `ab` (Oklab `a`, `b`) counts inside it.
    #[must_use]
    pub fn holds(self, ab: Point) -> bool {
        distance(&self.corners(), ab) <= INSIDE
    }
}

/// How far `p` lies outside the convex polygon `corners` (counter-clockwise): 0 inside it, else
/// its distance to the nearest edge.
#[must_use]
pub fn distance(corners: &[Point], p: Point) -> f64 {
    let q = nearest(corners, p);
    (p[0] - q[0]).hypot(p[1] - q[1])
}

/// The point of the convex polygon `corners` (counter-clockwise) nearest `p`: `p` itself inside
/// it.
#[must_use]
pub fn nearest(corners: &[Point], p: Point) -> Point {
    let edges = || (0..corners.len()).map(|k| (corners[k], corners[(k + 1) % corners.len()]));
    let inside = edges().all(|(a, b)| cross(sub(b, a), sub(p, a)) >= 0.0);
    if inside || corners.len() < 3 {
        return p;
    }
    edges()
        .map(|(a, b)| {
            let edge = sub(b, a);
            let long = dot(edge, edge).max(f64::MIN_POSITIVE);
            let t = (dot(sub(p, a), edge) / long).clamp(0.0, 1.0);
            [a[0] + t * edge[0], a[1] + t * edge[1]]
        })
        .min_by(|x, y| dot(sub(p, *x), sub(p, *x)).total_cmp(&dot(sub(p, *y), sub(p, *y))))
        .unwrap_or(p)
}

/// Where the ray from grey along the unit direction `d` lies inside the convex polygon
/// `corners`: from and to how far (from 0 where grey lies inside); none where it misses.
#[must_use]
pub fn span(corners: &[Point], d: Point) -> Option<(f64, f64)> {
    let count = corners.len();
    let mut hits: Vec<f64> = (0..count)
        .filter_map(|k| {
            let (p, q) = (corners[k], corners[(k + 1) % count]);
            let e = sub(q, p);
            let det = cross(d, e);
            if det.abs() < 1e-12 {
                return None;
            }
            // The ray `t d` meets the edge `p + s e` where `t d - s e = p`.
            let t = cross(p, e) / det;
            let s = cross(p, d) / det;
            ((-1e-9..=1.0 + 1e-9).contains(&s) && t >= 0.0).then_some(t)
        })
        .collect();
    hits.sort_by(f64::total_cmp);
    let grey = distance(corners, [0.0, 0.0]) == 0.0;
    match (grey, hits.first(), hits.last()) {
        (true, _, Some(far)) => Some((0.0, *far)),
        (false, Some(near), Some(far)) if far > near => Some((*near, *far)),
        _ => None,
    }
}

fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn cross(a: Point, b: Point) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
