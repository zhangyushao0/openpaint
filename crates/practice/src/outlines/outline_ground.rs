//! The ground a picture's object stands on, read from the picture's edge (`outline.rs`): the
//! picture reduced to a working grid, then either a clear ground (much of its edge transparent:
//! the object is what is opaque) or a plain one, a colour that changes smoothly across the
//! picture (a photographer's sweep, a vignette, a flat fill). A plain ground is read along each of
//! the four sides (Oklab, a strip `RING` deep), each side's profile freed of where the object
//! touches it (what strays from the side's running median, or far from its trend over the whole
//! side) and smoothed, and laid across the picture as a Coons patch: exact along the sides, and exact
//! inside for any ground that is a sum of a change across and a change down (a sweep, a
//! vignette). An edge that strays too often, or whose pixels scatter round their profile, is no
//! plain ground: a busy photo yields no outline.
//!
//! Each grid pixel's difference from the ground is then how far its colour lies from the
//! ground's there (Oklab, darker differences weighed less, so a soft shadow on the floor counts
//! for less than the object), or its opacity over a clear ground.

use crate::maths::colour::oklab;
use crate::outlines::outline::OutlineRefusal;
use crate::outlines::outline_sides::profile;

/// The grid's long side, in grid pixels.
pub(crate) const WORK_SIDE: usize = 512;
/// The strip along each side the ground is read from, in grid pixels.
const RING: usize = 3;
/// An edge at least this transparent is a clear ground.
const CLEAR_RING: f64 = 0.5;
/// A plain ground: at least this share of the edge is ground, and its pixels' noise round the
/// profile (a robust standard deviation, Oklab) stays under `MOST_NOISE`.
const FITTING_RING: f64 = 0.7;
const MOST_NOISE: f64 = 0.03;
/// Darker than the ground weighs this much of its lightness difference (soft shadows).
const SHADOW_WEIGHT: f64 = 0.55;
/// An object pixel differs at least `LOW` noises from the ground (and `LEAST_LOW`), and the
/// object holds a pixel of at least `HIGH` noises (and `LEAST_HIGH`).
const LOW: f64 = 4.0;
const LEAST_LOW: f64 = 0.04;
const HIGH: f64 = 8.0;
const LEAST_HIGH: f64 = 0.075;

/// The picture reduced to the working grid: straight sRGB (0 to 1) and alpha per grid pixel,
/// rows top first.
pub(crate) struct Grid {
    pub(crate) width: usize,
    pub(crate) height: usize,
    /// Picture pixels per grid pixel, across and down.
    pub(crate) scale: [f64; 2],
    pub(crate) pixels: Vec<[f32; 4]>,
}

/// How the object stands out from its ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Ground {
    /// Much of the picture's edge is transparent.
    Clear,
    /// A smooth colour, with its noise (Oklab).
    Plain { noise: f64 },
}

/// Each grid pixel's difference from the ground, and the two levels the object is found at: a
/// pixel surely of the object differs at least `high`; under `low` a pocket the object encloses
/// is ground. On a plain ground also each pixel's Oklab colour (empty over a clear ground).
pub(crate) struct Difference {
    pub(crate) ground: Ground,
    pub(crate) values: Vec<f32>,
    pub(crate) low: f32,
    pub(crate) high: f32,
    pub(crate) lab: Vec<[f64; 3]>,
}

/// `rgba` (8-bit sRGB with straight alpha, `size` pixels) reduced to the working grid by box
/// means over premultiplied colour; `None` for no pixels or a buffer not of `size`.
pub(crate) fn grid(size: [u32; 2], rgba: &[u8]) -> Option<Grid> {
    let [width, height] = size.map(|side| side as usize);
    if width == 0 || height == 0 || rgba.len() != width * height * 4 {
        return None;
    }
    let reduce = (width.max(height) as f64 / WORK_SIDE as f64).max(1.0);
    let cells = |side: usize| ((side as f64 / reduce).round() as usize).clamp(1, side);
    let [across, down] = [cells(width), cells(height)];
    let mut sums = vec![[0.0f64; 5]; across * down];
    let columns: Vec<usize> = (0..width).map(|x| x * across / width).collect();
    for y in 0..height {
        let row = y * down / height * across;
        let row_pixels = rgba[y * width * 4..(y + 1) * width * 4].as_chunks::<4>().0;
        for (x, pixel) in row_pixels.iter().enumerate() {
            let alpha = f64::from(pixel[3]) / 255.0;
            let sum = &mut sums[row + columns[x]];
            for (channel, value) in sum.iter_mut().zip(pixel).take(3) {
                *channel += alpha * f64::from(*value) / 255.0;
            }
            sum[3] += alpha;
            sum[4] += 1.0;
        }
    }
    let pixels = sums
        .iter()
        .map(|[r, g, b, a, count]| {
            let straight = |c: f64| if *a > 0.0 { (c / a) as f32 } else { 0.0 };
            [
                straight(*r),
                straight(*g),
                straight(*b),
                (a / count.max(1.0)) as f32,
            ]
        })
        .collect();
    Some(Grid {
        width: across,
        height: down,
        scale: [width as f64 / across as f64, height as f64 / down as f64],
        pixels,
    })
}

/// The ground of `grid` and every pixel's difference from it.
///
/// # Errors
/// `Ground` when the edge is neither clear nor a plain ground.
pub(crate) fn difference(grid: &Grid) -> Result<Difference, OutlineRefusal> {
    let [width, height] = [grid.width, grid.height];
    let ring = ring(grid);
    if clear(grid) {
        return Ok(Difference {
            ground: Ground::Clear,
            values: grid.pixels.iter().map(|p| p[3]).collect(),
            low: 0.5,
            high: 0.5,
            lab: Vec::new(),
        });
    }
    let lab: Vec<[f64; 3]> = grid.pixels.iter().map(|p| lab_on_white(*p)).collect();
    let patch = Patch::read([width, height], &lab)?;
    let ground: Vec<[f64; 3]> = (0..lab.len())
        .map(|i| patch.at(i % width, i / width))
        .collect();
    let mut gaps: Vec<f64> = ring.iter().map(|&i| distance(lab[i], ground[i])).collect();
    gaps.sort_by(f64::total_cmp);
    let noise = 1.4826 * gaps[gaps.len() / 2];
    if patch.fitting < FITTING_RING || noise > MOST_NOISE {
        return Err(OutlineRefusal::Ground);
    }
    let values = (lab.iter().zip(&ground))
        .map(|(lab, ground)| shade_distance(*lab, *ground) as f32)
        .collect();
    Ok(Difference {
        ground: Ground::Plain { noise },
        values,
        low: (LOW * noise).max(LEAST_LOW) as f32,
        high: (HIGH * noise).max(LEAST_HIGH) as f32,
        lab,
    })
}

/// The grid pixels of the strip `RING` deep along the picture's edge.
fn ring(grid: &Grid) -> Vec<usize> {
    let [width, height] = [grid.width, grid.height];
    let in_ring = |i: usize| {
        let (x, y) = (i % width, i / width);
        x < RING || y < RING || x + RING >= width || y + RING >= height
    };
    (0..grid.pixels.len()).filter(|&i| in_ring(i)).collect()
}

/// Whether the picture's ground is clear: at least `CLEAR_RING` of its edge transparent.
pub(crate) fn clear(grid: &Grid) -> bool {
    let ring = ring(grid);
    let clear = ring.iter().filter(|&&i| grid.pixels[i][3] < 0.5).count();
    clear as f64 >= CLEAR_RING * ring.len() as f64
}

/// A pixel in Oklab, laid on white where it is see-through.
fn lab_on_white([r, g, b, a]: [f32; 4]) -> [f64; 3] {
    let on_white = |c: f32| ((c * a + 1.0 - a).clamp(0.0, 1.0) * 255.0).round() as u8;
    oklab([on_white(r), on_white(g), on_white(b)])
}

/// How far `lab` lies from the ground's `ground`, a darker lightness weighed `SHADOW_WEIGHT`.
pub(crate) fn shade_distance(lab: [f64; 3], ground: [f64; 3]) -> f64 {
    let lighter = lab[0] - ground[0];
    let weight = if lighter < 0.0 { SHADOW_WEIGHT } else { 1.0 };
    (weight * lighter).hypot((lab[1] - ground[1]).hypot(lab[2] - ground[2]))
}

pub(crate) fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).hypot((a[1] - b[1]).hypot(a[2] - b[2]))
}

/// The ground laid across the picture from its four sides' profiles.
struct Patch {
    top: Vec<[f64; 3]>,
    bottom: Vec<[f64; 3]>,
    left: Vec<[f64; 3]>,
    right: Vec<[f64; 3]>,
    /// The share of the edge's places that were ground.
    fitting: f64,
}

impl Patch {
    /// The four sides of the picture `lab` (`size` grid pixels) read as profiles.
    fn read([width, height]: [usize; 2], lab: &[[f64; 3]]) -> Result<Self, OutlineRefusal> {
        if width < 4 * RING || height < 4 * RING {
            return Err(OutlineRefusal::Ground);
        }
        let strip = |at: &dyn Fn(usize, usize) -> usize, length: usize| -> Vec<[f64; 3]> {
            (0..length)
                .map(|along| {
                    let mut sum = [0.0; 3];
                    for depth in 0..RING {
                        let value = lab[at(along, depth)];
                        for (s, v) in sum.iter_mut().zip(value) {
                            *s += v / RING as f64;
                        }
                    }
                    sum
                })
                .collect()
        };
        let sides = [
            strip(&|x, d| d * width + x, width),
            strip(&|x, d| (height - 1 - d) * width + x, width),
            strip(&|y, d| y * width + d, height),
            strip(&|y, d| y * width + width - 1 - d, height),
        ];
        let mut ground = 0;
        let [top, bottom, left, right] = sides.map(|side| {
            let (profile, kept) = profile(&side);
            ground += kept;
            profile
        });
        Ok(Self {
            top,
            bottom,
            left,
            right,
            fitting: ground as f64 / (2 * (width + height)) as f64,
        })
    }

    /// The ground at grid pixel `(x, y)`.
    fn at(&self, x: usize, y: usize) -> [f64; 3] {
        let (w, h) = (self.top.len(), self.left.len());
        let u = x as f64 / (w - 1) as f64;
        let v = y as f64 / (h - 1) as f64;
        let corner = |a: [f64; 3], b: [f64; 3], c: usize| (a[c] + b[c]) / 2.0;
        let mut out = [0.0; 3];
        for (c, value) in out.iter_mut().enumerate() {
            let [tl, tr] = [
                corner(self.top[0], self.left[0], c),
                corner(self.top[w - 1], self.right[0], c),
            ];
            let [bl, br] = [
                corner(self.bottom[0], self.left[h - 1], c),
                corner(self.bottom[w - 1], self.right[h - 1], c),
            ];
            *value = (1.0 - v) * self.top[x][c]
                + v * self.bottom[x][c]
                + (1.0 - u) * self.left[y][c]
                + u * self.right[y][c]
                - ((1.0 - u) * (1.0 - v) * tl
                    + u * (1.0 - v) * tr
                    + (1.0 - u) * v * bl
                    + u * v * br);
        }
        out
    }
}
