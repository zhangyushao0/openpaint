//! A fill's smoothed edge (S30, D56): values derived from its mask when a scene first draws it,
//! kept with the scene, never stored. The evaluator (`shaders/fill_edge.wgsl`) reads them with
//! the mask's bits and turns them into each output pixel's coverage at every scale.
//!
//! The mask's signed distance field holds at each pixel centre the distance to the nearest
//! centre of the other side less half a pixel, positive inside: its zero lies on the pixel
//! edges between the two sides, and bilinear between centres it cuts each step of a staircase
//! at 45 degrees. Only the centres with one of the other side among their 8 neighbours (the
//! edge's) shape the edge, where the exact field is 1/2 or sqrt(2) - 1/2; further away only
//! its side counts (`FAR`). Where the edge points around an edge centre lie along a line (a
//! staircase does, a corner or a tight curve does not), the field there is averaged along that
//! line, so a staircase becomes the line it was sampled from; the average moves at most
//! `TOLERANCE` from the field itself, which keeps thin lines, thin gaps and every centre on its
//! side. Past the canvas the mask continues as its edge pixels, so a fill reaching the
//! canvas's edge keeps its corners there.

use openpaint_core::RegionMask;

/// How far the field is averaged along the edge each way, in canvas pixels.
const REACH: i32 = 5;
/// The radius around an edge centre whose edge points give the edge's line.
const FIT: i32 = 6;
/// Edge points further from their line than this (root mean square, canvas pixels) are no line:
/// a staircase's lie within half a pixel of it.
const STRAIGHT: f32 = 0.45;
/// Around the edge centres the plane holds every bit the fit and the average read.
const MARGIN: i32 = FIT + 2;
const _: () = assert!(REACH + 3 <= MARGIN, "the average reads within the margin");
/// How far the smoothed edge may move from the field's own zero, in canvas pixels.
pub(crate) const TOLERANCE: f32 = 0.5;
/// The field where no centre of the other side is among the 8 neighbours: at least 2 - 1/2.
pub(crate) const FAR: f32 = 1.5;
const DIAGONAL: f32 = std::f32::consts::SQRT_2 - 0.5;
/// Encoded values per canvas pixel, about the middle of 16 bits.
pub(crate) const STEPS: f32 = 8192.0;
const ZERO: i32 = 32_768;

/// The edge values of `mask` on a canvas of `canvas` pixels, as words after the mask's own:
/// per row of the mask's rectangle widened by one pixel each way (`height + 2` rows), where
/// its values start, then where they end; then one word per edge centre in the row, in
/// column order: its column in the widened rectangle times 2^16 plus its value encoded.
pub(crate) fn edge_words(mask: &RegionMask, canvas: [u32; 2]) -> Vec<u32> {
    let plane = Plane::of(mask, canvas);
    let ([x0, y0], [width, height]) = (mask.origin(), mask.size());
    let rows = height as usize + 2;
    let mut table = Vec::with_capacity(rows + 1);
    let mut values = Vec::new();
    // The widened rectangle's columns on the canvas, in plane columns.
    let left = (MARGIN - 1).max(MARGIN - x0 as i32);
    let right = (MARGIN + width as i32 + 1).min(MARGIN - x0 as i32 + canvas[0] as i32);
    for row in 0..rows {
        table.push(values.len() as u32);
        let y = y0 as i32 - 1 + row as i32;
        if y < 0 || y >= canvas[1] as i32 {
            continue;
        }
        let j = MARGIN - 1 + row as i32;
        for i in plane.edge_columns(j, left..right) {
            let column = (i - (MARGIN - 1)) as u32;
            values.push((column << 16) | encode(plane.smoothed([i, j])));
        }
    }
    table.push(values.len() as u32);
    table.extend(values);
    table
}

/// `value` in `STEPS` per pixel about `ZERO`, rounded (`fill_edge.wgsl` reads it back).
fn encode(value: f32) -> u32 {
    ((value * STEPS).round() as i32 + ZERO).clamp(0, 65_535) as u32
}

/// The mask's bits over its rectangle widened by `MARGIN` (canvas pixel `[x0, y0]` of the
/// mask's origin at plane column and row `MARGIN`), past the canvas its edge pixels': rows of
/// 64-bit words, bit `i % 64` of word `i / 64` for column `i`.
struct Plane {
    rows: Vec<u64>,
    /// Words per row.
    stride: usize,
}

impl Plane {
    fn of(mask: &RegionMask, canvas: [u32; 2]) -> Self {
        let ([x0, y0], [w, h]) = (mask.origin(), mask.size());
        let (width, height) = (
            w as usize + 2 * MARGIN as usize,
            h as usize + 2 * MARGIN as usize,
        );
        let stride = width.div_ceil(64);
        let mut plane = Self {
            rows: vec![0; stride * height],
            stride,
        };
        let margin = MARGIN as usize;
        let source = mask.stride() as usize;
        for y in 0..h as usize {
            let row = &mut plane.rows[(margin + y) * stride..(margin + y + 1) * stride];
            for (k, &word) in mask.words()[y * source..(y + 1) * source]
                .iter()
                .enumerate()
            {
                // Only the rectangle's own columns.
                let kept = (w as usize - 32 * k).min(32);
                let word = u64::from(word) & (u64::MAX >> (64 - kept));
                let at = margin + 32 * k;
                row[at / 64] |= word << (at % 64);
                if let Some(next) = row.get_mut(at / 64 + 1).filter(|_| at % 64 > 32) {
                    *next |= word >> (64 - at % 64);
                }
            }
            // Past the canvas's left and right edges, its edge pixels.
            let first = (row[margin / 64] >> (margin % 64)) & 1;
            let end = margin + w as usize - 1;
            let last = (row[end / 64] >> (end % 64)) & 1;
            for i in 0..margin {
                if x0 == 0 {
                    row[i / 64] |= first << (i % 64);
                }
                if x0 + w == canvas[0] {
                    let i = end + 1 + i;
                    row[i / 64] |= last << (i % 64);
                }
            }
        }
        // Past the canvas's top and bottom, its edge rows.
        for j in 0..margin {
            if y0 == 0 {
                plane
                    .rows
                    .copy_within(margin * stride..(margin + 1) * stride, j * stride);
            }
            if y0 + h == canvas[1] {
                let last = margin + h as usize - 1;
                let to = (last + 1 + j) * stride;
                plane
                    .rows
                    .copy_within(last * stride..(last + 1) * stride, to);
            }
        }
        plane
    }

    fn bit(&self, [x, y]: [i32; 2]) -> bool {
        let x = x as usize;
        (self.rows[y as usize * self.stride + x / 64] >> (x % 64)) & 1 == 1
    }

    /// The columns of row `j` within `columns` that lie on the edge, in order: where the 3 x 3
    /// around holds bits of both sides, found a word at a time.
    fn edge_columns(&self, j: i32, columns: std::ops::Range<i32>) -> Vec<i32> {
        let row = |j: i32| &self.rows[j as usize * self.stride..(j as usize + 1) * self.stride];
        let (above, own, below) = (row(j - 1), row(j), row(j + 1));
        let mut found = Vec::new();
        for k in columns.start as usize / 64..=(columns.end as usize - 1) / 64 {
            let (mut any, mut all) = (0u64, u64::MAX);
            for line in [above, own, below] {
                let word = line[k];
                let left = (word << 1) | k.checked_sub(1).map_or(0, |p| line[p] >> 63);
                let right = (word >> 1) | line.get(k + 1).map_or(0, |n| n << 63);
                any |= word | left | right;
                all &= word & left & right;
            }
            let mut edge = any & !all;
            while edge != 0 {
                let i = (k * 64) as i32 + edge.trailing_zeros() as i32;
                if columns.contains(&i) {
                    found.push(i);
                }
                edge &= edge - 1;
            }
        }
        found
    }

    /// The signed distance field at `at`, exact up to `FAR`.
    fn field(&self, [x, y]: [i32; 2]) -> f32 {
        let inside = self.bit([x, y]);
        let other = |dx: i32, dy: i32| self.bit([x + dx, y + dy]) != inside;
        let distance = if other(1, 0) || other(-1, 0) || other(0, 1) || other(0, -1) {
            0.5
        } else if other(1, 1) || other(-1, 1) || other(1, -1) || other(-1, -1) {
            DIAGONAL
        } else {
            FAR
        };
        if inside { distance } else { -distance }
    }

    /// The field bilinear between centres at plane point `p`.
    fn between(&self, p: [f32; 2]) -> f32 {
        let (fx, fy) = (p[0].floor(), p[1].floor());
        let (tx, ty) = (p[0] - fx, p[1] - fy);
        let (x, y) = (fx as i32, fy as i32);
        let top = lerp(self.field([x, y]), self.field([x + 1, y]), tx);
        let bottom = lerp(self.field([x, y + 1]), self.field([x + 1, y + 1]), tx);
        lerp(top, bottom, ty)
    }

    /// The field at edge centre `at`, averaged along the edge's line through it (triangle
    /// weights over `REACH`), at most `TOLERANCE` from the field there; where the edge points
    /// around lie along no line, the field itself.
    fn smoothed(&self, at: [i32; 2]) -> f32 {
        let own = self.field(at);
        let Some([tx, ty]) = line(&self.edge_points(at)) else {
            return own;
        };
        let (mut sum, mut weights) = (0.0, 0.0);
        for k in -REACH..=REACH {
            let weight = (REACH + 1 - k.abs()) as f32;
            let s = k as f32;
            sum += weight * self.between([at[0] as f32 + s * tx, at[1] as f32 + s * ty]);
            weights += weight;
        }
        (sum / weights).clamp(own - TOLERANCE, own + TOLERANCE)
    }

    /// The edge points within `FIT` of `at`, relative to it: the midpoints between neighbouring
    /// centres of either side.
    fn edge_points(&self, at: [i32; 2]) -> Vec<[f32; 2]> {
        let mut points = Vec::new();
        for dy in -FIT..=FIT {
            for dx in -FIT..=FIT {
                if dx * dx + dy * dy > FIT * FIT {
                    continue;
                }
                let p = [at[0] + dx, at[1] + dy];
                let inside = self.bit(p);
                if self.bit([p[0] + 1, p[1]]) != inside {
                    points.push([dx as f32 + 0.5, dy as f32]);
                }
                if self.bit([p[0], p[1] + 1]) != inside {
                    points.push([dx as f32, dy as f32 + 0.5]);
                }
            }
        }
        points
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The unit direction of the line `points` lie along (their principal axis), or `None` for
/// fewer than three points or points further from it than `STRAIGHT`. Arithmetic and square
/// roots only, so it is the same on every platform.
fn line(points: &[[f32; 2]]) -> Option<[f32; 2]> {
    let n = points.len() as f32;
    if n < 3.0 {
        return None;
    }
    let (mx, my) = points
        .iter()
        .fold((0.0, 0.0), |(x, y), p| (x + p[0] / n, y + p[1] / n));
    let (mut a, mut b, mut c) = (0.0f32, 0.0f32, 0.0f32);
    for p in points {
        let (dx, dy) = (p[0] - mx, p[1] - my);
        a += dx * dx;
        b += dx * dy;
        c += dy * dy;
    }
    let half = (a - c) * 0.5;
    let root = (half * half + b * b).sqrt();
    // The spreads along and across the axis; across, the mean squared distance from it.
    let (along, across) = ((a + c) * 0.5 + root, (a + c) * 0.5 - root);
    if along <= 0.0 || across > STRAIGHT * STRAIGHT * n {
        return None;
    }
    // (b, along - a) and (along - c, b) are both along the axis; take the longer.
    let length = |[u, v]: [f32; 2]| u * u + v * v;
    let [u, v] = if length([b, along - a]) >= length([along - c, b]) {
        [b, along - a]
    } else {
        [along - c, b]
    };
    let norm = length([u, v]).sqrt();
    (norm > 0.0).then(|| [u / norm, v / norm])
}

#[cfg(test)]
#[path = "fill_edge_tests.rs"]
mod tests;
