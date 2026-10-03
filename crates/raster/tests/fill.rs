//! The fill region's behaviour, carried over from draweave: holes, transparency against line
//! art, diagonal gaps, soft (antialiased) edges, open gaps and pinholes.

use openpaint_core::{CanvasSize, RegionMask};
use openpaint_raster::{FillError, connected_region};

const CLEAR: [u8; 4] = [0, 0, 0, 0];
const INK: [u8; 4] = [0, 0, 0, 255];

/// A picture of `width` x `height` premultiplied pixels, all `CLEAR`.
struct Picture {
    width: u32,
    height: u32,
    pixels: Vec<[u8; 4]>,
}

impl Picture {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![CLEAR; (width * height) as usize],
        }
    }

    fn set(&mut self, x: u32, y: u32, pixel: [u8; 4]) {
        self.pixels[(y * self.width + x) as usize] = pixel;
    }

    /// A one-pixel outline of the rectangle from `min` to `max` (inclusive).
    fn outline(&mut self, min: [u32; 2], max: [u32; 2]) {
        for y in min[1]..=max[1] {
            for x in min[0]..=max[0] {
                if x == min[0] || x == max[0] || y == min[1] || y == max[1] {
                    self.set(x, y, INK);
                }
            }
        }
    }

    fn fill(&self, seed: [u32; 2], threshold: f32) -> RegionMask {
        let canvas = CanvasSize::new(self.width, self.height).unwrap();
        let bytes: Vec<u8> = self.pixels.iter().flatten().copied().collect();
        connected_region(canvas, &bytes, seed, threshold).unwrap()
    }
}

#[test]
fn one_connected_region_keeps_its_holes_and_threshold_one_takes_everything() {
    let mut picture = Picture::new(20, 12);
    picture.outline([2, 2], [17, 9]);
    picture.set(8, 5, INK);
    let region = picture.fill([4, 5], 0.15);
    assert!(region.contains([4, 5]) && region.contains([16, 8]));
    assert!(!region.contains([8, 5]), "the dot inside stays a hole");
    assert!(!region.contains([0, 5]), "outside the outline");
    assert!(!region.contains([2, 5]), "the line itself");
    assert_eq!((region.origin(), region.size()), ([3, 3], [14, 6]));
    let everything = picture.fill([4, 5], 1.0);
    assert_eq!((everything.origin(), everything.size()), ([0, 0], [20, 12]));
}

#[test]
fn the_threshold_tells_transparency_from_faint_paint_and_diagonals_are_no_path() {
    let mut picture = Picture::new(2, 2);
    picture.set(1, 0, [0, 0, 0, 40]);
    picture.set(0, 1, INK);
    let low = picture.fill([0, 0], 0.1);
    assert!(
        !low.contains([1, 1]),
        "the diagonal is no path; faint paint is a wall at 10 %"
    );
    assert!(!low.contains([1, 0]));
    let high = picture.fill([0, 0], 0.2);
    assert!(
        high.contains([1, 1]),
        "40 of 255 is within 20 %: through the faint pixel"
    );
    assert!(
        !high.contains([0, 1]),
        "black line art is never transparent paper"
    );
}

#[test]
fn a_soft_edge_is_filled_under_its_rising_side_without_crossing_the_line() {
    // Clear paper, an antialiased vertical line (alpha ramps up to the core and down), paper.
    let ramp = [0, 0, 64, 160, 255, 160, 64, 0, 0];
    let mut picture = Picture::new(ramp.len() as u32, 3);
    for y in 0..3 {
        for (x, alpha) in ramp.iter().enumerate() {
            picture.set(x as u32, y, [0, 0, 0, *alpha]);
        }
    }
    let region = picture.fill([0, 1], 0.1);
    for y in 0..3 {
        assert!(
            region.contains([1, y]) && region.contains([2, y]),
            "the ring under 64"
        );
        assert!(!region.contains([3, y]), "one ring only, not up the ramp");
        assert!(
            !region.contains([4, y]) && !region.contains([7, y]),
            "never across"
        );
    }
}

#[test]
fn an_open_gap_lets_the_fill_through_but_a_diagonal_gap_does_not() {
    let mut open = Picture::new(12, 12);
    open.outline([2, 2], [9, 9]);
    open.set(9, 5, CLEAR);
    assert!(open.fill([5, 5], 0.1).contains([0, 0]), "no gap closing");
    // A staircase wall: its pixels touch only at corners, which four neighbours never pass.
    let mut stairs = Picture::new(6, 6);
    for i in 0..6 {
        stairs.set(i, 5 - i, INK);
    }
    let region = stairs.fill([0, 0], 0.1);
    assert!(region.contains([1, 1]) && !region.contains([5, 5]));
}

#[test]
fn a_pinhole_in_the_line_art_leaks_and_one_inside_the_region_stays_open() {
    let mut picture = Picture::new(14, 9);
    picture.outline([1, 1], [12, 7]);
    picture.set(4, 4, INK);
    let sealed = picture.fill([6, 4], 0.6);
    assert!(
        !sealed.contains([4, 4]),
        "an ink pinhole inside the region is kept"
    );
    assert!(!sealed.contains([0, 0]));
    picture.set(12, 4, CLEAR);
    let leaked = picture.fill([6, 4], 0.6);
    assert!(
        leaked.contains([0, 0]) && leaked.contains([13, 8]),
        "a one-pixel break leaks"
    );
}

#[test]
fn bad_input_is_refused() {
    let canvas = CanvasSize::new(2, 2).unwrap();
    let pixels = [0u8; 16];
    let refused = Err(FillError::InvalidInput);
    assert_eq!(
        connected_region(canvas, &pixels[..12], [0, 0], 0.5),
        refused
    );
    assert_eq!(connected_region(canvas, &pixels, [2, 0], 0.5), refused);
    assert_eq!(connected_region(canvas, &pixels, [0, 0], 1.5), refused);
    assert_eq!(connected_region(canvas, &pixels, [0, 0], f32::NAN), refused);
}
