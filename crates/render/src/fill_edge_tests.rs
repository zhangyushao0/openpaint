//! The edge field against brute force, the smoothing on lines it was sampled from, thin lines,
//! corners, and the words the evaluator reads.

use openpaint_core::{CanvasSize, RegionMask};

use super::*;

const SIDE: u32 = 24;

fn mask(inside: impl Fn(i32, i32) -> bool) -> RegionMask {
    let canvas = CanvasSize::new(SIDE, SIDE).expect("size");
    let pixels: Vec<bool> = (0..SIDE * SIDE)
        .map(|at| inside((at % SIDE) as i32, (at / SIDE) as i32))
        .collect();
    RegionMask::from_pixels(canvas, &pixels).expect("mask")
}

/// A mask's plane, looked up by canvas pixel.
struct Probe {
    plane: Plane,
    origin: [i32; 2],
}

impl Probe {
    fn of(mask: &RegionMask) -> Self {
        let [x0, y0] = mask.origin();
        Self {
            plane: Plane::of(mask, [SIDE, SIDE]),
            origin: [x0 as i32 - MARGIN, y0 as i32 - MARGIN],
        }
    }

    fn at(&self, x: i32, y: i32) -> [i32; 2] {
        [x - self.origin[0], y - self.origin[1]]
    }

    fn field(&self, x: i32, y: i32) -> f32 {
        self.plane.field(self.at(x, y))
    }

    fn smoothed(&self, x: i32, y: i32) -> f32 {
        self.plane.smoothed(self.at(x, y))
    }

    fn on_edge(&self, x: i32, y: i32) -> bool {
        self.field(x, y).abs() < FAR
    }
}

/// The value an encoded word holds, as `fill_edge.wgsl` reads it.
fn decode(word: u32) -> f32 {
    ((word & 0xffff) as i32 - ZERO) as f32 / STEPS
}

/// Brute force: the distance to the nearest centre of the other side (past the canvas, its
/// edge pixels'), less half a pixel, signed, at most `FAR`.
fn brute(inside: &impl Fn(i32, i32) -> bool, x: i32, y: i32) -> f32 {
    let at = |x: i32, y: i32| inside(x.clamp(0, SIDE as i32 - 1), y.clamp(0, SIDE as i32 - 1));
    let own = at(x, y);
    let mut nearest = f32::INFINITY;
    for v in -12..SIDE as i32 + 12 {
        for u in -12..SIDE as i32 + 12 {
            if at(u, v) != own {
                let d = (((u - x) * (u - x) + (v - y) * (v - y)) as f32).sqrt();
                nearest = nearest.min(d);
            }
        }
    }
    let value = (nearest - 0.5).min(FAR);
    if own { value } else { -value }
}

/// Deterministic blobs: a disc with a noisy rim.
fn noise(seed: u32) -> impl Fn(i32, i32) -> bool {
    move |x, y| {
        let mut h =
            (x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663) ^ seed;
        h ^= h >> 13;
        h = h.wrapping_mul(0x5bd1_e995);
        h ^= h >> 15;
        let (dx, dy) = (x - 12, y - 11);
        dx * dx + dy * dy < 40 + (h % 30) as i32
    }
}

/// The canvas pixels of `mask`'s rectangle widened by `by` (at most `MARGIN - 1`).
fn around(mask: &RegionMask, by: i32) -> Vec<(i32, i32)> {
    let ([x0, y0], [w, h]) = (mask.origin(), mask.size());
    let (x0, y0) = (x0 as i32, y0 as i32);
    (y0 - by..y0 + h as i32 + by)
        .flat_map(|y| (x0 - by..x0 + w as i32 + by).map(move |x| (x, y)))
        .collect()
}

#[test]
fn the_field_is_the_exact_distance_up_to_far() {
    for seed in 0..8 {
        let inside = noise(seed);
        let region = mask(&inside);
        let probe = Probe::of(&region);
        for (x, y) in around(&region, MARGIN - 1) {
            let expected = brute(&inside, x, y);
            let actual = probe.field(x, y);
            assert!(
                (actual - expected).abs() < 1e-6,
                "seed {seed} at {x}, {y}: {actual} for {expected}"
            );
        }
    }
}

#[test]
fn a_straight_edge_and_a_corner_keep_their_field() {
    let edge = Probe::of(&mask(|x, _| x < 10));
    let corner = Probe::of(&mask(|x, y| x < 10 && y < 12));
    for y in 0..SIDE as i32 {
        assert_eq!(edge.smoothed(9, y), 0.5);
        assert_eq!(edge.smoothed(10, y), -0.5);
    }
    // Along both sides of the corner, up to it.
    for k in 0..10 {
        assert_eq!(corner.smoothed(9, k), 0.5, "the upright side at {k}");
        assert_eq!(corner.smoothed(k, 11), 0.5, "the lower side at {k}");
    }
}

/// The signed distance of centre `[x, y]` from the line `y = slope x + offset` (inside below).
fn from_line(slope: f32, offset: f32, x: i32, y: i32) -> f32 {
    (y as f32 - slope * x as f32 - offset) / (1.0 + slope * slope).sqrt()
}

#[test]
fn a_staircase_comes_back_to_the_line_it_samples() {
    for (slope, offset) in [(0.125, 6.3), (0.2, 4.1), (0.5, 2.7), (-0.3, 15.2)] {
        let inside = |x: i32, y: i32| (y as f32) > slope * x as f32 + offset;
        let region = mask(inside);
        let probe = Probe::of(&region);
        let (mut raw, mut smooth) = (0.0f32, 0.0f32);
        // Away from the canvas's sides, where the line meets the replicated edge.
        for (x, y) in around(&region, 1) {
            if !(6..SIDE as i32 - 6).contains(&x) || probe.field(x, y).abs() > 0.6 {
                continue;
            }
            let true_distance = from_line(slope, offset, x, y);
            raw = raw.max((probe.field(x, y) - true_distance).abs());
            smooth = smooth.max((probe.smoothed(x, y) - true_distance).abs());
        }
        eprintln!("slope {slope}: field off the line by {raw:.3}, smoothed {smooth:.3}");
        assert!(
            smooth < 0.2,
            "slope {slope}: smoothed {smooth} from the line"
        );
        assert!(smooth < raw, "slope {slope}: smoothing brings it closer");
    }
}

#[test]
fn a_one_pixel_line_and_a_one_pixel_gap_keep_their_width() {
    let line = Probe::of(&mask(|_, y| y == 11));
    let gap = Probe::of(&mask(|_, y| y != 11));
    for x in 4..SIDE as i32 - 4 {
        assert_eq!(line.smoothed(x, 11), 0.5, "the line at {x}");
        assert_eq!(gap.smoothed(x, 11), -0.5, "the gap at {x}");
    }
}

#[test]
fn smoothing_moves_no_centre_across_and_at_most_the_tolerance() {
    for seed in 0..8 {
        let region = mask(noise(seed));
        let probe = Probe::of(&region);
        for (x, y) in around(&region, 1) {
            if !probe.on_edge(x, y) {
                continue;
            }
            let (own, smoothed) = (probe.field(x, y), probe.smoothed(x, y));
            assert!((smoothed - own).abs() <= TOLERANCE + 1e-6);
            assert!(smoothed * own >= 0.0, "{x}, {y} stays on its side");
        }
    }
}

/// `cargo test --release -p openpaint-render --lib fill_edge -- --ignored --nocapture`
#[test]
#[ignore = "a measurement"]
fn deriving_a_full_canvas_fill() {
    let [w, h] = [2732u32, 2048];
    let canvas = CanvasSize::new(w, h).expect("size");
    let holes = |x: u32, y: u32| {
        let (dx, dy) = ((x % 240) as i32 - 120, (y % 240) as i32 - 120);
        dx * dx + dy * dy > 90 * 90
    };
    for (name, inside) in [
        ("whole canvas", &(|_, _| true) as &dyn Fn(u32, u32) -> bool),
        ("canvas round 108 holes", &holes),
    ] {
        let pixels: Vec<bool> = (0..w * h).map(|at| inside(at % w, at / w)).collect();
        let mask = RegionMask::from_pixels(canvas, &pixels).expect("mask");
        let started = std::time::Instant::now();
        let words = edge_words(&mask, [w, h]);
        eprintln!(
            "{name}: {} edge centres in {:.1} ms",
            words.len() - (h as usize + 3),
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
}

#[test]
fn the_words_list_every_edge_centre_of_the_widened_rectangle_by_row() {
    let inside = |x: i32, y: i32| (5..9).contains(&x) && (3..6).contains(&y);
    let mask = mask(inside);
    let words = edge_words(&mask, [SIDE, SIDE]);
    let rows = mask.size()[1] as usize + 2;
    let values = &words[rows + 1..];
    assert_eq!(words[rows] as usize, values.len());
    let probe = Probe::of(&mask);
    for row in 0..rows {
        let listed: Vec<u32> = values[words[row] as usize..words[row + 1] as usize]
            .iter()
            .map(|word| word >> 16)
            .collect();
        let y = 3 - 1 + row as i32;
        let expected: Vec<u32> = (0..mask.size()[0] + 2)
            .filter(|column| probe.on_edge(5 - 1 + *column as i32, y))
            .collect();
        assert_eq!(listed, expected, "row {row}");
    }
    assert!((decode(values[0]) - probe.smoothed(4, 2)).abs() <= 0.5 / STEPS);
}
