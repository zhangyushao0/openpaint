//! Fill edges (S30): a fill's edge at 1x, 2x and 4x lies on the line its mask was sampled from,
//! antialiased over about one output pixel; its inside is exactly its colour; a transform's
//! lift stays hard; incremental evaluation reusing a fill's edge equals cold.

use openpaint_core::{Artwork, CanvasSize, Color, Patch, RegionMask};
use openpaint_render::{CanvasTarget, PixelRect, RegionEvaluator, RgbaImage, changed_regions};

use crate::fixture::{SIZE, blank, cold, core, pixel, plain, request, sample, with_stroke};
use crate::scale::whole;

fn canvas() -> CanvasSize {
    CanvasSize::new(SIZE[0], SIZE[1]).expect("size")
}

/// The canvas pixels whose centres `inside` holds.
fn region(inside: impl Fn(f32, f32) -> bool) -> RegionMask {
    let pixels: Vec<bool> = (0..SIZE[0] * SIZE[1])
        .map(|at| inside((at % SIZE[0]) as f32 + 0.5, (at / SIZE[0]) as f32 + 0.5))
        .collect();
    RegionMask::from_pixels(canvas(), &pixels).expect("mask")
}

/// Black filling `mask` on white.
fn black(mask: RegionMask) -> Artwork {
    blank()
        .fill_region(mask, Color::BLACK)
        .expect("fill")
        .artwork
}

/// The fill's coverage of output pixel `[x, y]`: black over white.
fn coverage(image: &RgbaImage, [x, y]: [u32; 2]) -> f32 {
    1.0 - f32::from(pixel(image, [x, y])[0]) / 255.0
}

/// Per output column, where the coverage crosses one half (canvas pixels) and how many pixels
/// are partly covered: the largest and the mean distance of the crossing from `line` (its y at
/// a canvas x), and the most partial pixels in a column.
fn crossings(image: &RgbaImage, scale: u32, line: impl Fn(f32) -> f32) -> (f32, f32, usize) {
    let [width, height] = image.size();
    let (mut largest, mut total, mut count, mut partial) = (0.0f32, 0.0, 0, 0);
    // Away from the canvas's sides, where the mask continues as its edge pixels.
    for x in 24 * scale..width - 24 * scale {
        let column: Vec<f32> = (0..height).map(|y| coverage(image, [x, y])).collect();
        partial = partial.max(column.iter().filter(|c| **c > 0.02 && **c < 0.98).count());
        let Some(k) = (0..height as usize - 1).find(|&k| column[k] < 0.5 && column[k + 1] >= 0.5)
        else {
            continue;
        };
        let t = (0.5 - column[k]) / (column[k + 1] - column[k]);
        let crossing = (k as f32 + 0.5 + t) / scale as f32;
        let off = (crossing - line((x as f32 + 0.5) / scale as f32)).abs();
        largest = largest.max(off);
        total += off;
        count += 1;
    }
    (largest, total / count.max(1) as f32, partial)
}

#[test]
fn a_fill_edge_lies_on_the_line_its_mask_samples_at_every_scale() {
    let core = core();
    let mut failed = Vec::new();
    for slope in [0.0f32, 0.11, 0.37, -0.23] {
        let line = move |x: f32| 80.0 + slope * (x - 160.3);
        let artwork = black(region(|x, y| y > line(x)));
        for scale in [1, 2, 4] {
            let image = whole(&core, &artwork, scale);
            let (largest, mean, partial) = crossings(&image, scale, line);
            eprintln!(
                "slope {slope} at {scale}x: crossing off the line by at most {largest:.3} \
                 (mean {mean:.3}) canvas px, {partial} partial pixels in a column"
            );
            // A hard staircase is off by up to 0.5 px (mean 0.25).
            if largest >= 0.25 || mean >= 0.1 || partial > 3 {
                failed.push((slope, scale));
            }
        }
    }
    assert!(failed.is_empty(), "off the line: {failed:?}");
}

#[test]
fn a_disc_fill_keeps_its_radius_at_every_scale() {
    let core = core();
    let (centre, radius) = ([160.3f32, 100.6f32], 41.3f32);
    let inside = |x: f32, y: f32| (x - centre[0]).hypot(y - centre[1]) < radius;
    let artwork = black(region(inside));
    for scale in [1u32, 2, 4] {
        let image = whole(&core, &artwork, scale);
        let s = scale as f32;
        // The covered area against the disc's.
        let area: f32 = (0..image.size()[1])
            .flat_map(|y| (0..image.size()[0]).map(move |x| [x, y]))
            .map(|at| coverage(&image, at))
            .sum::<f32>()
            / (s * s);
        let disc = std::f32::consts::PI * radius * radius;
        eprintln!("disc at {scale}x: area {area:.1} for {disc:.1}");
        assert!((area - disc).abs() / disc < 0.01, "{scale}x: area {area}");
        // Along the row through the centre, the half crossings at the disc's sides.
        let row = (centre[1] * s) as u32;
        let covered: Vec<f32> = (0..image.size()[0])
            .map(|x| coverage(&image, [x, row]))
            .collect();
        let left = covered.iter().position(|c| *c >= 0.5).expect("left side") as f32;
        let right = covered.iter().rposition(|c| *c >= 0.5).expect("right side") as f32;
        let y = (row as f32 + 0.5) / s - centre[1];
        let half = (radius * radius - y * y).sqrt();
        assert!(((left + 0.5) / s - (centre[0] - half)).abs() < 0.5 / s + 0.25);
        assert!(((right + 0.5) / s - (centre[0] + half)).abs() < 0.5 / s + 0.25);
    }
}

#[test]
fn a_fills_inside_is_exactly_its_colour_and_the_outside_untouched() {
    let core = core();
    let mask = region(|x, y| (x - 100.0).hypot(y - 90.0) < 50.0 && x > 70.0);
    let artwork = black(mask.clone());
    let image = cold(&core, &artwork);
    for y in 0..SIZE[1] {
        for x in 0..SIZE[0] {
            let around = |dx: i32, dy: i32| {
                let (u, v) = (x as i32 + dx, y as i32 + dy);
                u >= 0 && v >= 0 && mask.contains([u as u32, v as u32])
            };
            let near: Vec<bool> = (-1..=1)
                .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                .map(|(dx, dy)| around(dx, dy))
                .collect();
            if near.iter().all(|inside| *inside) {
                assert_eq!(pixel(&image, [x, y]), [0, 0, 0, 255], "inside at {x}, {y}");
            } else if near.iter().all(|inside| !*inside) {
                assert_eq!(pixel(&image, [x, y]), [255; 4], "outside at {x}, {y}");
            }
        }
    }
}

#[test]
fn a_fill_reaching_the_canvas_edge_keeps_its_corners() {
    let core = core();
    let image = cold(&core, &black(RegionMask::whole(canvas())));
    for corner in [
        [0, 0],
        [SIZE[0] - 1, 0],
        [0, SIZE[1] - 1],
        [SIZE[0] - 1, SIZE[1] - 1],
    ] {
        assert_eq!(pixel(&image, corner), [0, 0, 0, 255], "at {corner:?}");
    }
}

#[test]
fn a_transforms_lift_stays_hard() {
    let core = core();
    let painted = black(region(|_, _| true));
    let lift = region(|x, y| (x - 160.0).hypot(y - 100.0) < 30.0);
    let patch = Patch::new(canvas(), [0, 0], [1, 1], vec![0; 4]).expect("patch");
    let placement = openpaint_core::Placement::moved([0.0, 0.0]).expect("in place");
    let lifted = painted
        .transform_region(Some(lift.clone()), patch, placement)
        .expect("lifted")
        .artwork;
    let image = cold(&core, &lifted);
    for y in 0..SIZE[1] {
        for x in 0..SIZE[0] {
            let expected = if lift.contains([x, y]) {
                [255; 4]
            } else {
                [0, 0, 0, 255]
            };
            assert_eq!(pixel(&image, [x, y]), expected, "at {x}, {y}");
        }
    }
}

#[test]
fn incremental_evaluation_reusing_fill_edges_equals_cold() {
    let core = core();
    let red = Color::rgb(0.9, 0.1, 0.1).expect("red");
    let one = blank()
        .fill_region(region(|x, y| (x - 90.0).hypot(y - 90.0) < 40.0), red)
        .expect("fill")
        .artwork;
    let two = one
        .fill_region(region(|x, y| y > 0.3 * x + 60.0), Color::BLACK)
        .expect("fill")
        .artwork;
    let line = (
        plain(6.0, 1.0, Color::BLACK),
        vec![sample(20.0, 30.0, 1.0, 0), sample(300.0, 180.0, 1.0, 8)],
    );
    let three = with_stroke(&two, line);
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut shown: Option<&Artwork> = None;
    for artwork in [&one, &two, &three, &two, &one, &three] {
        let mut evaluator = RegionEvaluator::new(&core, &mut target);
        let regions = shown.map_or_else(
            || vec![PixelRect::full(SIZE)],
            |shown| changed_regions(shown, artwork),
        );
        evaluator
            .evaluate(&request(artwork), &regions)
            .expect("evaluate");
        assert_eq!(target.read(&core).expect("read"), cold(&core, artwork));
        shown = Some(artwork);
    }
}
