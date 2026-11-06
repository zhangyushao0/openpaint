//! The copies `key-shifts` judges (S107): a crop's areas painted again in given colours at a
//! session frame's size, flat as a careful hand fills them, or as a hand paints: each area's
//! colour a little off, its edges wandering, a brush's texture and grain over it, and strokes of
//! a neighbour's colour crossing an edge.

use std::f64::consts::TAU;

use openpaint_practice::{Areas, Random, Rgb8, from_oklab, oklab};

/// A copy's longer side, in pixels: a session's frame.
pub(crate) const SIDE: u32 = 400;
/// How far a hand's colour is off an area's, at most (Oklab lightness, then `a` and `b`).
const OFF_LIGHTNESS: f64 = 0.025;
const OFF_HUE: f64 = 0.012;
/// How far a hand's edge wanders, pixels; its brush's texture (Oklab lightness over a grid of
/// `TEXTURE_CELL` pixels) and grain (8-bit steps a pixel).
const WOBBLE: f64 = 5.0;
const TEXTURE: f64 = 0.02;
const TEXTURE_CELL: f64 = 16.0;
const GRAIN: f64 = 3.0;
/// Strokes of a neighbour's colour across an edge: how many, their radius and length, pixels.
const STROKES: usize = 8;
const STROKE_RADIUS: f64 = 6.0;
const STROKE_LENGTH: f64 = 36.0;

/// The copy's size for a crop of `picture` pixels: its shape, `SIDE` long.
pub(crate) fn size_of(picture: [u32; 2]) -> [u32; 2] {
    let longer = f64::from(picture[0].max(picture[1]).max(1));
    picture.map(|side| ((f64::from(side) * f64::from(SIDE) / longer).round() as u32).max(1))
}

/// `colours` as a hand picks them: each a little lighter or darker and off in hue, by `random`.
pub(crate) fn picked_by_hand(colours: &[Rgb8], random: &mut Random) -> Vec<Rgb8> {
    (colours.iter())
        .map(|colour| {
            let [l, a, b] = oklab(*colour);
            let off = |most: f64, random: &mut Random| random.range(-most, most);
            let l = l + off(OFF_LIGHTNESS, random);
            from_oklab([l, a + off(OFF_HUE, random), b + off(OFF_HUE, random)])
        })
        .collect()
}

/// `areas` each in its `colours`, flat, at `size` (8-bit RGBA).
pub(crate) fn flat(areas: &Areas, colours: &[Rgb8], [w, h]: [u32; 2]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let p = [
                (f64::from(x) + 0.5) / f64::from(w),
                (f64::from(y) + 0.5) / f64::from(h),
            ];
            let [r, g, b] = colours[areas.area_at(p)];
            pixels.extend_from_slice(&[r, g, b, 255]);
        }
    }
    pixels
}

/// `areas` each in its `colours` as a hand paints them at `size`, unevenly, seeded `seed`.
pub(crate) fn by_hand(areas: &Areas, colours: &[Rgb8], [w, h]: [u32; 2], seed: u64) -> Vec<u8> {
    let mut random = Random::new(seed);
    let phases: [f64; 4] = std::array::from_fn(|_| random.range(0.0, TAU));
    let cells = [w, h].map(|side| (f64::from(side) / TEXTURE_CELL) as usize + 2);
    let grid: Vec<f64> = (0..cells[0] * cells[1])
        .map(|_| random.range(-1.0, 1.0))
        .collect();
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (f64::from(x), f64::from(y));
            let dx = WOBBLE * ((fy / 37.0 + phases[0]).sin() + 0.5 * (fy / 11.0 + phases[1]).sin());
            let dy = WOBBLE * ((fx / 41.0 + phases[2]).sin() + 0.5 * (fx / 13.0 + phases[3]).sin());
            let p = [
                ((fx + 0.5 + dx) / f64::from(w)).clamp(0.0, 0.9999),
                ((fy + 0.5 + dy) / f64::from(h)).clamp(0.0, 0.9999),
            ];
            let [l, a, b] = oklab(colours[areas.area_at(p)]);
            let texture = TEXTURE * smooth_noise(&grid, cells[0], [fx, fy]);
            let [r, g, b] = from_oklab([l + texture, a, b]).map(|c| {
                let grain = random.range(-GRAIN, GRAIN);
                (f64::from(c) + grain).round().clamp(0.0, 254.0) as u8
            });
            pixels.extend_from_slice(&[r, g, b, 255]);
        }
    }
    strokes(&mut pixels, (areas, colours), [w, h], &mut random);
    pixels
}

/// The value noise of `grid` (`across` values a row, one every `TEXTURE_CELL` pixels) at the
/// pixel `[x, y]`, bilinear.
fn smooth_noise(grid: &[f64], across: usize, [x, y]: [f64; 2]) -> f64 {
    let [u, v] = [x / TEXTURE_CELL, y / TEXTURE_CELL];
    let [c, r] = [u.floor() as usize, v.floor() as usize];
    let [s, t] = [u.fract(), v.fract()];
    let at = |c: usize, r: usize| grid.get(r * across + c).copied().unwrap_or(0.0);
    let top = at(c, r) * (1.0 - s) + at(c + 1, r) * s;
    let bottom = at(c, r + 1) * (1.0 - s) + at(c + 1, r + 1) * s;
    top * (1.0 - t) + bottom * t
}

/// `STROKES` strokes, each of one area's colour from a point of its edge with a neighbour across
/// into the neighbour.
fn strokes(
    pixels: &mut [u8],
    (areas, colours): (&Areas, &[Rgb8]),
    [w, h]: [u32; 2],
    random: &mut Random,
) {
    if areas.neighbours.is_empty() {
        return;
    }
    for _ in 0..STROKES {
        let at = (random.unit() * areas.neighbours.len() as f64) as usize;
        let (a, b) = areas.neighbours[at.min(areas.neighbours.len() - 1)];
        let (from, into) = if random.unit() < 0.5 { (a, b) } else { (b, a) };
        let edges = areas.edges(from, Some(into));
        let points: Vec<[f64; 2]> = edges.into_iter().flatten().collect();
        let Some(start) = points.get((random.unit() * points.len() as f64) as usize) else {
            continue;
        };
        let start = [start[0] * f64::from(w), start[1] * f64::from(h)];
        let (sin, cos) = random.range(0.0, TAU).sin_cos();
        let [r, g, b] = colours[from];
        for step in 0..=24 {
            let t = f64::from(step) / 24.0 * STROKE_LENGTH;
            let centre = [start[0] + cos * t, start[1] + sin * t];
            let reach = STROKE_RADIUS.ceil() as i64;
            for oy in -reach..=reach {
                for ox in -reach..=reach {
                    let (px, py) = (centre[0] as i64 + ox, centre[1] as i64 + oy);
                    let inside = ((ox * ox + oy * oy) as f64) <= STROKE_RADIUS * STROKE_RADIUS;
                    if inside && (0..i64::from(w)).contains(&px) && (0..i64::from(h)).contains(&py)
                    {
                        let index = (py as usize * w as usize + px as usize) * 4;
                        pixels[index..index + 3].copy_from_slice(&[r, g, b]);
                    }
                }
            }
        }
    }
}
