//! The object on a plain ground (`outline.rs`), found where the picture's own edges say it ends:
//! a marker-controlled watershed (Meyer's flooding) over the colour gradient. The ground's markers
//! are what the picture's edge reaches through smooth pixels (so a halo of light behind the
//! object or a soft shadow is ground however far its colour drifts), pockets of clear ground,
//! and smooth pockets the object encloses whose colour is the ground's round it (`outline_mesh.rs`:
//! what shows through a handle's loop); the object's are the pixels that differ from that ground
//! beyond doubt, are no shadow (darker than the ground, about as grey, not much darker) and lie
//! in parts with an edge round them. Both grow in order of the gradient, the lowest first, and
//! meet on the strongest edges between them, so a white body on a pale ground keeps its weak but
//! real edge.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::outlines::outline_ground::shade_distance;
use crate::outlines::outline_mask::parts;
use crate::outlines::outline_mesh::mesh;

/// A pixel is surely the ground's when it differs from the ground by under `CLEAR` noises (and
/// from `LEAST_CLEAR` to `MOST_CLEAR`), in a pocket of at least `LEAST_POCKET` of the picture.
const CLEAR: f64 = 3.0;
const LEAST_CLEAR: f64 = 0.008;
const MOST_CLEAR: f64 = 0.02;
const LEAST_POCKET: f64 = 0.001_5;
/// A shadow: darker than the ground by at most `SHADOW_DEPTH` (Oklab lightness), its colour
/// within `SHADOW_TINT` of the ground's.
const SHADOW_DEPTH: f64 = 0.32;
const SHADOW_TINT: f64 = 0.025;
/// Markers of the object: parts at least this share of the picture, at least `EDGED` of whose
/// rim lies on an edge (a gradient of at least `EDGE`): a patch of light or shade on the ground
/// that only differs from the ground's model fades in and has no edge.
const LEAST_MARKER: f64 = 0.000_3;
const EDGED: f64 = 0.3;
const EDGE: f32 = 0.06;
/// The smooth ground: what the picture's edge reaches through pixels whose gradient stays under
/// `SMOOTH` times the edge's median gradient (and at least `LEAST_SMOOTH`); a halo of light
/// behind the object or a pool on the floor is ground however far its colour drifts.
const SMOOTH: f32 = 4.0;
const LEAST_SMOOTH: f32 = 0.02;

/// The labels the flood gives.
const GROUND: u8 = 1;
const OBJECT: u8 = 2;

/// What the flood reads: each pixel's Oklab colour, its difference from the ground read from the
/// picture's edge, the ground's noise and the level the object surely differs at.
pub(crate) struct Flooded<'a> {
    pub(crate) size: [usize; 2],
    pub(crate) lab: &'a [[f64; 3]],
    pub(crate) values: &'a [f32],
    pub(crate) noise: f64,
    pub(crate) high: f32,
}

/// The object's pixels: true where the flood gave them to the object.
pub(crate) fn object(flooded: &Flooded<'_>) -> Vec<bool> {
    let gradient = gradient(flooded.size, flooded.lab);
    let mut labels = markers(flooded, &gradient);
    flood(flooded.size, &gradient, &mut labels);
    labels.iter().map(|label| *label == OBJECT).collect()
}

/// The markers: the ground along the picture's edge and in clear pockets, the object where it
/// surely differs, is no shadow and has an edge; 0 elsewhere.
fn markers(flooded: &Flooded<'_>, gradient: &[f32]) -> Vec<u8> {
    let [width, height] = flooded.size;
    let count = width * height;
    let mut labels = vec![0u8; count];
    let clear_level = (CLEAR * flooded.noise).clamp(LEAST_CLEAR, MOST_CLEAR) as f32;
    let clear: Vec<bool> = flooded.values.iter().map(|v| *v < clear_level).collect();
    for pocket in parts(flooded.size, &clear) {
        if pocket.len() as f64 >= LEAST_POCKET * count as f64 {
            for i in pocket {
                labels[i] = GROUND;
            }
        }
    }
    let (smooth, level) = smooth_ground(flooded, gradient);
    for (label, smooth) in labels.iter_mut().zip(&smooth) {
        if *smooth {
            *label = GROUND;
        }
    }
    // The ground as the smooth ground shows it round the object, and each pixel's difference
    // from it.
    let near = mesh(flooded.size, flooded.lab, &smooth);
    let values: Vec<f32> = (0..count)
        .map(|i| shade_distance(flooded.lab[i], near[i]) as f32)
        .collect();
    // Smooth pockets the object encloses whose colour is the ground's: what shows through it.
    let still: Vec<bool> = (0..count)
        .map(|i| !smooth[i] && gradient[i] < level)
        .collect();
    for pocket in parts(flooded.size, &still) {
        let mean = pocket.iter().map(|&i| values[i]).sum::<f32>() / pocket.len() as f32;
        if pocket.len() as f64 >= LEAST_POCKET * count as f64 && mean < clear_level {
            for i in pocket {
                labels[i] = GROUND;
            }
        }
    }
    let sure: Vec<bool> = (0..count)
        .map(|i| values[i] >= flooded.high && !smooth[i] && !shadow(flooded.lab[i], near[i]))
        .collect();
    for part in parts(flooded.size, &sure) {
        let large = part.len() as f64 >= LEAST_MARKER * count as f64;
        if large && edged(&part, &sure, flooded.size, gradient) {
            for i in part {
                labels[i] = OBJECT;
            }
        }
    }
    for (i, label) in labels.iter_mut().enumerate() {
        let (x, y) = (i % width, i / width);
        if (x == 0 || y == 0 || x + 1 == width || y + 1 == height) && *label != OBJECT {
            *label = GROUND;
        }
    }
    labels
}

/// The ground the picture's edge reaches through smooth pixels (4-connected), from the edge's
/// pixels that are the ground's (not the object's where it touches the edge), and the gradient
/// under which a pixel is smooth.
fn smooth_ground(flooded: &Flooded<'_>, gradient: &[f32]) -> (Vec<bool>, f32) {
    let [width, height] = flooded.size;
    let on_edge = |i: usize| {
        let (x, y) = (i % width, i / width);
        x == 0 || y == 0 || x + 1 == width || y + 1 == height
    };
    let mut ring: Vec<f32> = (0..gradient.len())
        .filter(|&i| on_edge(i))
        .map(|i| gradient[i])
        .collect();
    ring.sort_by(f32::total_cmp);
    let level = (SMOOTH * ring[ring.len() / 2]).max(LEAST_SMOOTH);
    let mut reached = vec![false; gradient.len()];
    let mut stack: Vec<usize> = (0..gradient.len())
        .filter(|&i| on_edge(i) && gradient[i] < level && flooded.values[i] < flooded.high)
        .collect();
    for &i in &stack {
        reached[i] = true;
    }
    while let Some(i) = stack.pop() {
        let (x, y) = (i % width, i / width);
        let neighbours = [
            (x > 0).then(|| i - 1),
            (x + 1 < width).then(|| i + 1),
            (y > 0).then(|| i - width),
            (y + 1 < height).then(|| i + width),
        ];
        for n in neighbours.into_iter().flatten() {
            if !reached[n] && gradient[n] < level {
                reached[n] = true;
                stack.push(n);
            }
        }
    }
    (reached, level)
}

/// Whether at least `EDGED` of the rim of `part` (its pixels beside one not in `inside`) lies on
/// an edge.
fn edged(part: &[usize], inside: &[bool], [width, height]: [usize; 2], gradient: &[f32]) -> bool {
    let rim: Vec<usize> = part
        .iter()
        .copied()
        .filter(|&i| {
            let (x, y) = (i % width, i / width);
            let out = |n: usize| !inside[n];
            (x > 0 && out(i - 1))
                || (x + 1 < width && out(i + 1))
                || (y > 0 && out(i - width))
                || (y + 1 < height && out(i + width))
        })
        .collect();
    let on_edge = rim.iter().filter(|&&i| gradient[i] >= EDGE).count();
    on_edge as f64 >= EDGED * rim.len().max(1) as f64
}

/// Whether `lab` looks like a shadow on the ground's `ground`.
fn shadow(lab: [f64; 3], ground: [f64; 3]) -> bool {
    let darker = ground[0] - lab[0];
    let tint = (lab[1] - ground[1]).hypot(lab[2] - ground[2]);
    (0.0..=SHADOW_DEPTH).contains(&darker) && tint <= SHADOW_TINT
}

/// The colour gradient's size at each pixel: Sobel over each Oklab channel after a binomial blur
/// ([1 4 6 4 1] across and down), the channels' squares summed.
fn gradient([width, height]: [usize; 2], lab: &[[f64; 3]]) -> Vec<f32> {
    let blurred = blur([width, height], lab);
    let at = |x: usize, y: usize| blurred[y.min(height - 1) * width + x.min(width - 1)];
    (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            let (l, r) = (x.saturating_sub(1), x + 1);
            let (u, d) = (y.saturating_sub(1), y + 1);
            let mut sum = 0.0;
            for c in 0..3 {
                let gx = (at(r, u)[c] + 2.0 * at(r, y)[c] + at(r, d)[c])
                    - (at(l, u)[c] + 2.0 * at(l, y)[c] + at(l, d)[c]);
                let gy = (at(l, d)[c] + 2.0 * at(x, d)[c] + at(r, d)[c])
                    - (at(l, u)[c] + 2.0 * at(x, u)[c] + at(r, u)[c]);
                sum += gx * gx + gy * gy;
            }
            sum.sqrt() as f32
        })
        .collect()
}

/// `lab` blurred by [1 4 6 4 1] / 16 across, then down, the edge's pixels repeated.
fn blur([width, height]: [usize; 2], lab: &[[f64; 3]]) -> Vec<[f64; 3]> {
    const WEIGHTS: [f64; 5] = [1.0 / 16.0, 4.0 / 16.0, 6.0 / 16.0, 4.0 / 16.0, 1.0 / 16.0];
    let pass = |source: &[[f64; 3]], across: bool| -> Vec<[f64; 3]> {
        (0..width * height)
            .map(|i| {
                let (x, y) = (i % width, i / width);
                let mut sum = [0.0; 3];
                for (k, weight) in WEIGHTS.iter().enumerate() {
                    let offset = k as isize - 2;
                    let (sx, sy) = if across {
                        (x.saturating_add_signed(offset).min(width - 1), y)
                    } else {
                        (x, y.saturating_add_signed(offset).min(height - 1))
                    };
                    let value = source[sy * width + sx];
                    for (s, v) in sum.iter_mut().zip(value) {
                        *s += weight * v;
                    }
                }
                sum
            })
            .collect()
    };
    pass(&pass(lab, true), false)
}

/// Meyer's flooding: every unlabelled pixel takes the label of the neighbour it is reached from,
/// the lowest gradient first (4-connected; ties in the order they were reached).
fn flood([width, height]: [usize; 2], gradient: &[f32], labels: &mut [u8]) {
    let mut queued = vec![false; labels.len()];
    let mut heap = BinaryHeap::new();
    let mut order = 0u64;
    let level = |i: usize| (f64::from(gradient[i]) * 1.0e5) as u64;
    let neighbours = |i: usize| {
        let (x, y) = (i % width, i / width);
        [
            (x > 0).then(|| i - 1),
            (x + 1 < width).then(|| i + 1),
            (y > 0).then(|| i - width),
            (y + 1 < height).then(|| i + width),
        ]
    };
    for i in 0..labels.len() {
        if labels[i] == 0 {
            continue;
        }
        for n in neighbours(i).into_iter().flatten() {
            if labels[n] == 0 && !queued[n] {
                queued[n] = true;
                heap.push(Reverse((level(n), order, n, labels[i])));
                order += 1;
            }
        }
    }
    while let Some(Reverse((_, _, i, label))) = heap.pop() {
        labels[i] = label;
        for n in neighbours(i).into_iter().flatten() {
            if labels[n] == 0 && !queued[n] {
                queued[n] = true;
                heap.push(Reverse((level(n), order, n, label)));
                order += 1;
            }
        }
    }
}
