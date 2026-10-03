//! The region a fill covers, at the source's native resolution (draweave's ColorDrop, its
//! behaviour carried over and tested here).
//!
//! 1. Flood: four neighbours from the seed; a pixel joins while its colour distance from the
//!    seed's colour (not its neighbour's: no creeping) is at most `threshold * 255`. The
//!    distance is the largest per-channel difference of premultiplied 8-bit RGBA, alpha
//!    included, so transparent paper and black line art differ fully. Four neighbours never
//!    cross a diagonal gap.
//! 2. Edge ring: one ring of pixels around the flood joins where the distance keeps rising (or
//!    holds) outward, so the fill reaches under the antialiased edge of a line instead of
//!    leaving a light seam; it stops at a local peak, never floods on from there, and so
//!    never crosses a one-pixel line or opens a pinhole.
//!
//! The result is a hard-edged mask: no gap closing, no feathering.

use std::collections::VecDeque;
use std::fmt;

use openpaint_core::{CanvasSize, RegionMask};

/// Why no region came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillError {
    /// Pixels not one RGBA quadruple per canvas pixel, a seed off the canvas or a threshold
    /// outside `0..=1`.
    InvalidInput,
}

impl fmt::Display for FillError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid fill input"),
        }
    }
}

impl std::error::Error for FillError {}

/// Flood state of one pixel.
const OUTSIDE: u8 = 0;
const INSIDE: u8 = 1;
const REJECTED: u8 = 2;

/// The pixels connected to `seed` whose colours stay within `threshold` (`0..=1`) of the
/// seed's, plus the edge ring, from `pixels`: premultiplied 8-bit RGBA of a `canvas`-sized
/// picture, rows top first.
///
/// # Errors
/// `InvalidInput` as documented on it.
pub fn connected_region(
    canvas: CanvasSize,
    pixels: &[u8],
    seed: [u32; 2],
    threshold: f32,
) -> Result<RegionMask, FillError> {
    let [width, height] = [canvas.width(), canvas.height()];
    let count = width as usize * height as usize;
    if pixels.len() != count * 4
        || seed[0] >= width
        || seed[1] >= height
        || !(0.0..=1.0).contains(&threshold)
    {
        return Err(FillError::InvalidInput);
    }
    let source = Source {
        pixels,
        width: width as usize,
        height: height as usize,
        seed: seed[1] as usize * width as usize + seed[0] as usize,
    };
    let state = source.flood(threshold * 255.0);
    let inside = source.edge_ring(&state);
    RegionMask::from_pixels(canvas, &inside).map_err(|_| FillError::InvalidInput)
}

struct Source<'a> {
    pixels: &'a [u8],
    width: usize,
    height: usize,
    /// Pixel index of the seed.
    seed: usize,
}

impl Source<'_> {
    /// The largest per-channel difference between pixel `index` and the seed.
    fn distance(&self, index: usize) -> u8 {
        let pixel = &self.pixels[index * 4..index * 4 + 4];
        let seed = &self.pixels[self.seed * 4..self.seed * 4 + 4];
        pixel
            .iter()
            .zip(seed)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0)
    }

    /// Four-neighbour flood from the seed: `INSIDE` within `limit`, `REJECTED` where it was
    /// looked at and refused, `OUTSIDE` where it never reached.
    fn flood(&self, limit: f32) -> Vec<u8> {
        let (width, height) = (self.width, self.height);
        let mut state = vec![OUTSIDE; width * height];
        let mut queue = VecDeque::from([self.seed]);
        state[self.seed] = INSIDE;
        while let Some(index) = queue.pop_front() {
            let (x, y) = (index % width, index / width);
            let neighbours = [
                (x > 0).then(|| index - 1),
                (x + 1 < width).then_some(index + 1),
                (y > 0).then(|| index - width),
                (y + 1 < height).then_some(index + width),
            ];
            for next in neighbours.into_iter().flatten() {
                if state[next] != OUTSIDE {
                    continue;
                }
                if f32::from(self.distance(next)) <= limit {
                    state[next] = INSIDE;
                    queue.push_back(next);
                } else {
                    state[next] = REJECTED;
                }
            }
        }
        state
    }

    /// The flood plus its edge ring: a pixel outside it joins when, in one of the eight
    /// directions, its inner neighbour is in the flood and nearer the seed's colour than it is,
    /// and its outer neighbour is at least as far: the colour still rises towards a line.
    fn edge_ring(&self, state: &[u8]) -> Vec<bool> {
        let mut inside: Vec<bool> = state.iter().map(|&s| s == INSIDE).collect();
        let (width, height) = (self.width as isize, self.height as isize);
        let at = |x: isize, y: isize| {
            (x >= 0 && y >= 0 && x < width && y < height).then(|| (y * width + x) as usize)
        };
        for y in 0..height {
            for x in 0..width {
                let index = (y * width + x) as usize;
                if state[index] == INSIDE {
                    continue;
                }
                let distance = self.distance(index);
                if distance == 0 {
                    continue;
                }
                inside[index] = DIRECTIONS.iter().any(|&(dx, dy)| {
                    match (at(x - dx, y - dy), at(x + dx, y + dy)) {
                        (Some(inner), Some(outer)) => {
                            state[inner] == INSIDE
                                && self.distance(inner) < distance
                                && self.distance(outer) >= distance
                        }
                        _ => false,
                    }
                });
            }
        }
        inside
    }
}

/// The eight neighbour directions.
const DIRECTIONS: [(isize, isize); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];
