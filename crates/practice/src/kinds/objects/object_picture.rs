//! An object's picture (S94, `object.rs`): the object as the eye sees it on paper-white, made from
//! the same parts the answer is (`object_body.rs`) by a depth test, the nearest surface on each
//! ray shown: lit from one side above, the tops lightest, the sides two values, a round part a
//! little darker and shaded by its turn; no lines, no hidden parts, no horizon. Each pixel is
//! its middle's surface, and where a neighbour's differs, the mean of 4 x 4 samples across it
//! (the edges antialiased, the paper's share left clear): the lit picture, drawn over any surface
//! (`lit_picture`, S153: the light's answer too).
//!
//! Every visible edge must show as a step of shade the eye sees (`legible`): across it, a pen's
//! width either side, the shades differ by `SHADE_STEP` at least, or the object is laid again.

use crate::kinds::objects::object::ObjectTask;
use crate::kinds::objects::object_body::ROUND_SURFACES;
use crate::maths::path::{Point, distance};
use crate::maths::vector::dot;

/// The light: what every surface shows, what the light adds facing it; a round part's share
/// (another material, a little darker); the clay's tint.
const AMBIENT: f64 = 0.16;
const DIFFUSE: f64 = 0.72;
const ROUND: f64 = 0.8;
const TINT: [f64; 3] = [1.0, 0.965, 0.92];
/// The paper's shade.
pub const PAPER_SHADE: f64 = 1.0;
/// The least step of shade across a visible edge, and how far either side it is looked at (a
/// pen's width of a 400-pixel frame); how far from an edge's ends, and how often along it.
pub const SHADE_STEP: f64 = 0.1;
const PROBE: f64 = 2.5 / 400.0;
const CLEAR: f64 = 0.02;
const EVERY: f64 = 0.02;
/// Samples across a pixel at an edge, each way.
const SAMPLES: usize = 4;

impl ObjectTask {
    /// The shade the eye sees at the frame point `p` (0 black, 1 white); none where it sees the
    /// paper.
    #[must_use]
    pub fn shade_at(&self, p: Point) -> Option<f64> {
        self.surface_at(p).map(|(_, shade)| shade)
    }

    /// Which surface the eye sees at the frame point `p` (a face, wherever its blocks meet flat; a
    /// round part's side or end), as the depth test finds it; none for the paper.
    #[must_use]
    pub fn surface(&self, p: Point) -> Option<(u32, bool)> {
        self.surface_at(p)
            .map(|(surface, _)| (surface, surface >= ROUND_SURFACES))
    }

    /// The surface at the frame point `p` and its shade.
    fn surface_at(&self, p: Point) -> Option<(u32, f64)> {
        let x = (p[0] - self.view.offset[0]) / self.view.scale;
        let y = (p[1] - self.view.offset[1]) / self.view.scale;
        let hit = self.body.hit([x, -y, 1.0])?;
        let lit = AMBIENT + DIFFUSE * dot(hit.normal, self.light).max(0.0);
        let share = if hit.round { ROUND } else { 1.0 };
        Some((hit.surface, lit * share))
    }

    /// The picture `side` pixels square, one to its frame: RGBA, the colour not multiplied by the
    /// alpha (the paper left clear).
    #[must_use]
    pub fn picture(&self, side: u32) -> Vec<u8> {
        // Only within the box's picture: beyond it lies paper.
        let low = [0, 1].map(|k| (self.seen.corners.iter()).fold(1.0_f64, |m, p| m.min(p[k])));
        let high = [0, 1].map(|k| (self.seen.corners.iter()).fold(0.0_f64, |m, p| m.max(p[k])));
        lit_picture((side, [low, high], TINT), |p| self.surface_at(p))
    }

    /// Whether every visible edge shows as a step of `SHADE_STEP` at least, looked at every `EVERY`
    /// along it (`CLEAR` from its ends) a probe's length either side.
    pub(crate) fn legible(&self) -> bool {
        let shade = |p: Point| self.shade_at(p).unwrap_or(PAPER_SHADE);
        let steps = |a: Point, b: Point| {
            let long = distance(a, b);
            if long <= 2.0 * CLEAR {
                return true;
            }
            let normal = [-(b[1] - a[1]) / long, (b[0] - a[0]) / long];
            let count = ((long - 2.0 * CLEAR) / EVERY).ceil().max(1.0) as usize;
            (0..=count).all(|k| {
                let s = (CLEAR + (long - 2.0 * CLEAR) * k as f64 / count as f64) / long;
                let p = [a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s];
                let side = |by: f64| shade([p[0] + normal[0] * by, p[1] + normal[1] * by]);
                (side(PROBE) - side(-PROBE)).abs() >= SHADE_STEP
            })
        };
        let straight = self.answer.sides.iter().all(|[a, b]| steps(*a, *b));
        let round = (self.answer.outlines.iter().flatten())
            .filter(|stretch| !stretch.hidden)
            .all(|stretch| {
                let total = crate::maths::path::length(&stretch.points);
                let mut walked = 0.0;
                (stretch.points.windows(2)).enumerate().all(|(k, pair)| {
                    let (a, b) = (pair[0], pair[1]);
                    let long = distance(a, b);
                    walked += long;
                    // Every fourth piece, clear of the ends, where the outlines touch the curve.
                    let clear = walked > CLEAR && walked < total - CLEAR;
                    if !clear || k % 4 != 0 || long <= 0.0 {
                        return true;
                    }
                    let middle = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
                    let normal = [-(b[1] - a[1]) / long, (b[0] - a[0]) / long];
                    let side =
                        |by: f64| shade([middle[0] + normal[0] * by, middle[1] + normal[1] * by]);
                    (side(PROBE) - side(-PROBE)).abs() >= SHADE_STEP
                })
            });
        straight && round
    }
}

/// The lit picture of any surface (S153: an object's, the light's answer, later a sausage or a
/// texture strip): `side` pixels square, one to its frame, RGBA, the colour not multiplied by the
/// alpha (the paper left clear). `surface` gives a frame point's surface (an id that tells two
/// apart) and shade (0 black, 1 white), none for the paper; the shade is drawn in `tint`. Only
/// pixels within `[low, high]` (frame units, a pixel or two round) are drawn. Each pixel is its
/// middle's surface, and where a neighbour's differs, the mean of `SAMPLES` x `SAMPLES` samples
/// across it: the edges antialiased, the paper's share left clear.
pub(crate) fn lit_picture(
    (side, bounds, tint): (u32, [Point; 2], [f64; 3]),
    surface: impl Fn(Point) -> Option<(u32, f64)>,
) -> Vec<u8> {
    let coloured = |p: Point| surface(p).map(|(id, shade)| (id, [shade; 3]));
    coloured_picture((side, bounds, tint), coloured)
}

/// The lit picture in colour (S154: colour under light's render): as `lit_picture`, `surface`
/// giving each frame point's colour (linear 0 to 1 a channel, as a shade is) instead of a shade.
pub(crate) fn coloured_picture(
    (side, [low, high], tint): (u32, [Point; 2], [f64; 3]),
    surface: impl Fn(Point) -> Option<(u32, [f64; 3])>,
) -> Vec<u8> {
    let side = side.max(1) as usize;
    let pixel = 1.0 / side as f64;
    let centre = |x: usize, y: usize| [(x as f64 + 0.5) * pixel, (y as f64 + 0.5) * pixel];
    let range = |k: usize| {
        let from = ((low[k] / pixel).floor().max(0.0) as usize).saturating_sub(2);
        let to = ((high[k] / pixel).ceil().max(0.0) as usize + 2).min(side);
        from..to
    };
    let mut middles: Vec<Option<(u32, [f64; 3])>> = vec![None; side * side];
    for y in range(1) {
        for x in range(0) {
            middles[y * side + x] = surface(centre(x, y));
        }
    }
    let mut pixels = vec![0_u8; side * side * 4];
    for y in range(1) {
        for x in range(0) {
            let own = middles[y * side + x].map(|(id, _)| id);
            let other = |dx: isize, dy: isize| {
                let (nx, ny) = (x.checked_add_signed(dx), y.checked_add_signed(dy));
                let held = nx.zip(ny).filter(|(nx, ny)| *nx < side && *ny < side);
                held.and_then(|(nx, ny)| middles[ny * side + nx])
                    .map(|(id, _)| id)
                    != own
            };
            let edge = other(-1, 0) || other(1, 0) || other(0, -1) || other(0, 1);
            let (shade, cover) = if edge {
                across(&surface, centre(x, y), pixel)
            } else {
                middles[y * side + x].map_or(([0.0; 3], 0.0), |(_, shade)| (shade, 1.0))
            };
            let at = (y * side + x) * 4;
            for k in 0..3 {
                pixels[at + k] = (shade[k] * tint[k] * 255.0).round().clamp(0.0, 255.0) as u8;
            }
            pixels[at + 3] = (cover * 255.0).round() as u8;
        }
    }
    pixels
}

/// The mean colour of `surface`'s samples across the pixel `pixel` wide about `centre`, channel by
/// channel, and the share of them on a surface.
fn across(
    surface: &impl Fn(Point) -> Option<(u32, [f64; 3])>,
    centre: Point,
    pixel: f64,
) -> ([f64; 3], f64) {
    let (mut sum, mut count) = ([0.0; 3], 0usize);
    for j in 0..SAMPLES {
        for i in 0..SAMPLES {
            let at = |k: usize| ((k as f64 + 0.5) / SAMPLES as f64 - 0.5) * pixel;
            if let Some((_, shade)) = surface([centre[0] + at(i), centre[1] + at(j)]) {
                for k in 0..3 {
                    sum[k] += shade[k];
                }
                count += 1;
            }
        }
    }
    let cover = count as f64 / (SAMPLES * SAMPLES) as f64;
    let mean = sum.map(|s| if count > 0 { s / count as f64 } else { 0.0 });
    (mean, cover)
}
