//! How far the raster probe's image lies from the evaluator's (S57): per channel in 8-bit levels,
//! the largest and the mean difference, and where the differences above one level lie: on an
//! antialiased edge (a pixel some neighbour of which differs by `EDGE` levels or more in the
//! evaluator's image: rims, ink edges, the paper's grain) or away from one.

use crate::image::RgbaImage;

/// The step between neighbours, in 8-bit levels, from which a pixel counts as on an edge.
pub const EDGE: u8 = 8;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ImageDifference {
    pub pixels: usize,
    pub largest: [u8; 4],
    pub mean: [f64; 4],
    /// Pixels off by more than one level in some channel, and of them those away from edges.
    pub beyond_one: usize,
    pub beyond_one_inside: usize,
    /// The largest difference of a pixel away from edges, where the first such pixel lies, and
    /// the largest step to a neighbour there in the evaluator's image (a step as large as the
    /// difference is an edge of lower contrast than `EDGE`, moved).
    pub largest_inside: u8,
    pub worst_inside: [u32; 2],
    pub contrast_inside: u8,
}

/// `rastered` against `evaluated`; `None` when their sizes differ.
#[must_use]
pub fn image_difference(evaluated: &RgbaImage, rastered: &RgbaImage) -> Option<ImageDifference> {
    if evaluated.size() != rastered.size() {
        return None;
    }
    let [width, height] = evaluated.size().map(|side| side as usize);
    let (e, r) = (evaluated.pixels(), rastered.pixels());
    let mut found = ImageDifference {
        pixels: width * height,
        ..ImageDifference::default()
    };
    let mut sums = [0u64; 4];
    for y in 0..height {
        for x in 0..width {
            let at = (y * width + x) * 4;
            let mut most = 0;
            for channel in 0..4 {
                let off = e[at + channel].abs_diff(r[at + channel]);
                sums[channel] += u64::from(off);
                found.largest[channel] = found.largest[channel].max(off);
                most = most.max(off);
            }
            if most > 1 {
                found.beyond_one += 1;
                let step = contrast(e, [width, height], [x, y]);
                if step < EDGE {
                    found.beyond_one_inside += 1;
                    if most > found.largest_inside {
                        found.largest_inside = most;
                        found.worst_inside = [x as u32, y as u32];
                        found.contrast_inside = step;
                    }
                }
            }
        }
    }
    let count = found.pixels.max(1) as f64;
    found.mean = sums.map(|sum| sum as f64 / count);
    Some(found)
}

/// The largest step, in any channel, from pixel `[x, y]` to one of its eight neighbours.
fn contrast(pixels: &[u8], [width, height]: [usize; 2], [x, y]: [usize; 2]) -> u8 {
    let at = |x: usize, y: usize| (y * width + x) * 4;
    let here = at(x, y);
    let mut step = 0;
    for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
        for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
            let there = at(nx, ny);
            for c in 0..4 {
                step = step.max(pixels[here + c].abs_diff(pixels[there + c]));
            }
        }
    }
    step
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn differences_are_counted_on_and_away_from_edges() {
        // A 4 x 1 image: an edge between the second and third pixels.
        let evaluated = RgbaImage::new([4, 1], [[10; 4], [10; 4], [200; 4], [200; 4]].concat());
        let rastered = RgbaImage::new([4, 1], [[13; 4], [10; 4], [195; 4], [200; 4]].concat());
        let found = image_difference(&evaluated, &rastered).expect("same size");
        assert_eq!(found.largest, [5; 4]);
        assert_eq!((found.beyond_one, found.beyond_one_inside), (2, 1));
        assert_eq!((found.largest_inside, found.worst_inside), (3, [0, 0]));
        assert_eq!(found.contrast_inside, 0);
        assert!((found.mean[0] - 2.0).abs() < 1e-9);
        let other = RgbaImage::new([1, 1], vec![0; 4]);
        assert!(image_difference(&evaluated, &other).is_none());
    }
}
