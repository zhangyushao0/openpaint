//! `cargo shape-assets`: the brush shape textures (S45, DECISIONS D72), generated from seeded
//! value noise, never downloaded: `crates/render/assets/shape-<name>.bin`, 256 x 256 bytes, row
//! major. Each byte is the signed distance from the texel's centre to the shape's rim in texels
//! (positive inside), `128 + 16 d` rounded and clamped to `0..=255` (so -8 to +7.94 texels in
//! sixteenths): the renderer reads the rim's distance where a texel covers at least an output
//! pixel (a sharp edge at any zoom) and the coverage `clamp(0.5 + d, 0, 1)`, box-filtered into
//! mip levels, where it covers less.
//!
//! Each shape is a field `f` over the square `[-1, 1]^2`, inside where `f > 0`, and the distance
//! is `f / |grad f|`. Every value is f64 arithmetic without transcendental functions, so the
//! files are the same bytes on every machine.

use std::path::Path;

/// Texels per side of the committed textures.
pub const SIZE: usize = 256;

/// The textures, in the order the renderer binds them.
pub const SHAPES: [(&str, Field); 4] = [
    ("chalk", chalk),
    ("grainy", grainy),
    ("splatter", splatter),
    ("rough-square", rough_square),
];

/// A shape's field at a point of the square.
pub type Field = fn(f64, f64) -> f64;

/// Writes every texture into `crates/render/assets/` under `root`, then lists them.
pub fn generate(root: &Path) -> Result<(), String> {
    let assets = root.join("crates/render/assets");
    for (name, field) in SHAPES {
        let path = assets.join(format!("shape-{name}.bin"));
        let bytes = texture(field, SIZE);
        std::fs::write(&path, &bytes).map_err(|error| format!("{}: {error}", path.display()))?;
        let inside = bytes.iter().filter(|byte| **byte >= 128).count();
        println!(
            "  {} ({} bytes, {:.1} % inside)",
            path.display(),
            bytes.len(),
            100.0 * inside as f64 / bytes.len() as f64
        );
    }
    Ok(())
}

/// `field` sampled at the centres of `size` x `size` texels, as encoded distances.
pub fn texture(field: Field, size: usize) -> Vec<u8> {
    let texel = 2.0 / size as f64;
    let step = 0.25 * texel;
    let mut bytes = Vec::with_capacity(size * size);
    for row in 0..size {
        for column in 0..size {
            let x = (column as f64 + 0.5) * texel - 1.0;
            let y = (row as f64 + 0.5) * texel - 1.0;
            let value = field(x, y);
            let dx = (field(x + step, y) - field(x - step, y)) / (2.0 * step);
            let dy = (field(x, y + step) - field(x, y - step)) / (2.0 * step);
            let slope = (dx * dx + dy * dy).sqrt().max(1e-9);
            // In texels: the square's units are half a side.
            let distance = value / slope / texel;
            bytes.push((128.0 + 16.0 * distance).round().clamp(0.0, 255.0) as u8);
        }
    }
    bytes
}

/// A chalk disc: a slightly lumpy round body whose rim is eaten into by fine noise, leaving a
/// broken edge with specks and small holes near it.
fn chalk(x: f64, y: f64) -> f64 {
    let r = (x * x + y * y).sqrt();
    let body = 0.80 - r + 0.12 * (fbm(x, y, 2.5, 2, 11) - 0.5);
    let erosion = smoothstep(0.3, 0.85, r) * 0.34 * (fbm(x, y, 7.0, 2, 12) - 0.4);
    body - erosion - 0.05 * (fbm(x, y, 20.0, 1, 13) - 0.5)
}

/// A grainy disc: round, full of small holes.
fn grainy(x: f64, y: f64) -> f64 {
    let r = (x * x + y * y).sqrt();
    let body = 0.86 - r + 0.06 * (fbm(x, y, 3.0, 2, 21) - 0.5);
    let grain = 0.5 * (fbm(x, y, 11.0, 2, 22) - 0.36);
    body.min(grain)
}

/// A splatter: a middle blob and smaller ones flung out all around it, their edges a little
/// uneven.
fn splatter(x: f64, y: f64) -> f64 {
    // The eight directions of a compass, so no angle needs a sine.
    const D: f64 = std::f64::consts::FRAC_1_SQRT_2;
    const AROUND: [[f64; 2]; 8] = [
        [1.0, 0.0],
        [D, D],
        [0.0, 1.0],
        [-D, D],
        [-1.0, 0.0],
        [-D, -D],
        [0.0, -1.0],
        [D, -D],
    ];
    let mut best = 0.42 - (x * x + y * y).sqrt();
    for k in 0..13u32 {
        let [u, v, w] = [0, 1, 2].map(|c| unit(pcg(31 + 3 * k + c)));
        let [dx, dy] = AROUND[(k as usize * 3 + (v * 2.0) as usize) % 8];
        let far = 0.42 + 0.45 * u;
        // A little off the compass line, either side.
        let side = 0.25 * (v - 0.5);
        let (cx, cy) = (far * (dx - side * dy), far * (dy + side * dx));
        let radius = (0.04 + 0.13 * w) * (1.25 - far);
        let (ex, ey) = (x - cx, y - cy);
        best = best.max(radius - (ex * ex + ey * ey).sqrt());
    }
    best + 0.05 * (fbm(x, y, 7.0, 2, 32) - 0.5)
}

/// A rough square: a solid body whose sides across the frame's y axis fray into streaks along
/// its x axis (the stroke's direction: bristle marks running along a painted band) and whose
/// ends are ragged row by row, as a loaded flat brush leaves them.
fn rough_square(x: f64, y: f64) -> f64 {
    let streaks = fbm(0.0, 45.0 * y, 1.0, 2, 41);
    let frayed = smoothstep(0.5, 0.85, y.abs());
    let across = 0.85 - y.abs() - 0.35 * frayed * (streaks - 0.42);
    let rows = fbm(0.0, 9.0 * y, 1.0, 2, 42);
    let along = 0.84 - x.abs() - 0.12 * (rows - 0.5) - 0.04 * (fbm(x, y, 10.0, 1, 43) - 0.5);
    across.min(along)
}

/// Fractal value noise in `0..1`: `octaves` layers from `frequency` cells per unit, each twice
/// as fine and half as strong.
fn fbm(x: f64, y: f64, frequency: f64, octaves: u32, seed: u32) -> f64 {
    let (mut sum, mut weight, mut total, mut scale) = (0.0, 1.0, 0.0, frequency);
    for octave in 0..octaves {
        sum += weight * noise(x * scale, y * scale, seed.wrapping_mul(97) + octave);
        total += weight;
        weight *= 0.5;
        scale *= 2.0;
    }
    sum / total
}

/// Value noise: a hashed value per lattice point, blended by the quintic fade.
fn noise(x: f64, y: f64, seed: u32) -> f64 {
    let (fx, fy) = (x.floor(), y.floor());
    let (tx, ty) = (fade(x - fx), fade(y - fy));
    let (ix, iy) = (fx as i64, fy as i64);
    let at = |dx: i64, dy: i64| lattice(ix + dx, iy + dy, seed);
    let top = at(0, 0) + (at(1, 0) - at(0, 0)) * tx;
    let bottom = at(0, 1) + (at(1, 1) - at(0, 1)) * tx;
    top + (bottom - top) * ty
}

fn lattice(ix: i64, iy: i64, seed: u32) -> f64 {
    // Two's complement bits of small lattice coordinates: the same on every machine.
    let (x, y) = (ix as u32, iy as u32);
    unit(pcg(x.wrapping_mul(0x9E37_79B9) ^ pcg(y ^ pcg(seed))))
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (6.0 * t - 15.0) + 10.0)
}

fn smoothstep(low: f64, high: f64, value: f64) -> f64 {
    let t = ((value - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A hash's top 24 bits as a share in `0..1`.
fn unit(hash: u32) -> f64 {
    f64::from(hash >> 8) / f64::from(1u32 << 24)
}

fn pcg(value: u32) -> u32 {
    let state = value.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shape inside its square: a tenth of it or more has paint, the border none.
    fn inside_its_square(name: &str, bytes: &[u8], size: usize) {
        let inside = bytes.iter().filter(|byte| **byte >= 128).count();
        assert!(inside > size * size / 10, "{name}: {inside} texels inside");
        let last = size - 1;
        let border = (0..size).flat_map(|k| [k, last * size + k, size * k, size * k + last]);
        assert!(
            border.into_iter().all(|at| bytes[at] < 128),
            "{name}: the border lies outside"
        );
    }

    #[test]
    fn generating_twice_gives_the_same_bytes() {
        for (name, field) in SHAPES {
            let first = texture(field, 32);
            assert_eq!(first.len(), 32 * 32);
            assert_eq!(first, texture(field, 32), "{name}");
            inside_its_square(name, &first, 32);
        }
    }

    #[test]
    fn a_disc_is_its_distance_field() {
        let disc: Field = |x, y| 0.5 - (x * x + y * y).sqrt();
        let bytes = texture(disc, 64);
        // The texel centred 3.5 texels inside the rim along the middle row.
        let column = 32 + 16 - 4;
        let byte = bytes[32 * 64 + column];
        let expected = 128.0 + 16.0 * 3.5;
        assert!(
            (f64::from(byte) - expected).abs() <= 2.0,
            "{byte} for {expected}"
        );
    }

    #[test]
    fn the_committed_textures_are_the_generated_ones() {
        let root = crate::process::root();
        for (name, field) in SHAPES {
            let path = root.join(format!("crates/render/assets/shape-{name}.bin"));
            let committed = std::fs::read(&path).expect("a generated texture");
            assert!(
                committed == texture(field, SIZE),
                "{name}: run cargo shape-assets"
            );
            inside_its_square(name, &committed, SIZE);
        }
    }
}
