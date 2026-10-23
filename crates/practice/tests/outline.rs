//! The silhouette outline (S80) on pictures made here, each with its true silhouette: a studio
//! photo (a sweep, a halo behind the object, noise, a soft shadow under it, a handle's loop), flat
//! grounds (a transparent cut-out with separate parts, a figure on flat white) and grounds that
//! must yield none (a busy photo, an object cut by the edge, nothing at all). A derived outline,
//! filled, covers the true silhouette within a few percent; a refused one is no outline.

use openpaint_practice::{Outline, OutlineRefusal, Random, outline};

/// A picture being made: 8-bit sRGB with straight alpha.
struct Canvas {
    size: [u32; 2],
    rgba: Vec<u8>,
}

impl Canvas {
    fn new(size: [u32; 2], mut ground: impl FnMut(f64, f64) -> [f64; 4]) -> Self {
        let [w, h] = size;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let pixel = ground(f64::from(x) + 0.5, f64::from(y) + 0.5);
                rgba.extend(pixel.map(|c| c.clamp(0.0, 255.0).round() as u8));
            }
        }
        Self { size, rgba }
    }

    /// Lays `colour` where `inside` holds, by its 2 x 2 coverage of each pixel (antialiased).
    fn lay(
        &mut self,
        inside: &dyn Fn(f64, f64) -> bool,
        colour: &mut dyn FnMut(f64, f64) -> [f64; 4],
    ) {
        let [w, h] = self.size.map(f64::from);
        self.lay_in([0.0, 0.0, w, h], inside, colour);
    }

    /// As `lay`, looking only within `[left, top, right, bottom]`.
    fn lay_in(
        &mut self,
        [left, top, right, bottom]: [f64; 4],
        inside: &dyn Fn(f64, f64) -> bool,
        colour: &mut dyn FnMut(f64, f64) -> [f64; 4],
    ) {
        let [w, h] = self.size;
        let span = |low: f64, high: f64, side: u32| {
            (low.floor().max(0.0) as u32)..(high.ceil().max(0.0) as u32).min(side)
        };
        for y in span(top, bottom, h) {
            for x in span(left, right, w) {
                let (fx, fy) = (f64::from(x), f64::from(y));
                let covered = [0.25, 0.75]
                    .iter()
                    .flat_map(|dy| [0.25, 0.75].map(|dx| inside(fx + dx, fy + dy)))
                    .filter(|c| *c)
                    .count() as f64
                    / 4.0;
                if covered > 0.0 {
                    let at = ((y * w + x) * 4) as usize;
                    let over = colour(fx + 0.5, fy + 0.5);
                    for c in 0..4 {
                        let under = f64::from(self.rgba[at + c]);
                        let value = under + covered * over[3] / 255.0 * (over[c] - under);
                        self.rgba[at + c] = value.clamp(0.0, 255.0).round() as u8;
                    }
                    if over[3] >= 255.0 {
                        let alpha = f64::from(self.rgba[at + 3]).max(covered * 255.0);
                        self.rgba[at + 3] = alpha.round() as u8;
                    }
                }
            }
        }
    }
}

fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64) -> impl Fn(f64, f64) -> bool {
    move |x, y| ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0
}

/// Whether `(x, y)` lies inside the outline, its contours filled even-odd.
fn filled(outline: &Outline, x: f64, y: f64) -> bool {
    let mut inside = false;
    for contour in &outline.contours {
        let count = contour.len();
        for i in 0..count {
            let (a, b) = (contour[i], contour[(i + 1) % count]);
            if (a[1] > y) != (b[1] > y) && x < a[0] + (y - a[1]) / (b[1] - a[1]) * (b[0] - a[0]) {
                inside = !inside;
            }
        }
    }
    inside
}

/// How much of the true silhouette and the filled outline they share, of all either covers
/// (intersection over union), looked at every 2 pixels.
fn overlap(outline: &Outline, size: [u32; 2], truth: &dyn Fn(f64, f64) -> bool) -> f64 {
    let (mut both, mut either) = (0usize, 0usize);
    for y in (0..size[1]).step_by(2) {
        for x in (0..size[0]).step_by(2) {
            let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let (a, b) = (filled(outline, px, py), truth(px, py));
            both += usize::from(a && b);
            either += usize::from(a || b);
        }
    }
    both as f64 / either.max(1) as f64
}

/// A studio ground: light at the top, darker down the sweep, darker to the sides, a halo of light
/// behind the middle, a little noise.
fn studio(size: [u32; 2], seed: u64) -> Canvas {
    let mut random = Random::new(seed);
    let [w, h] = size.map(f64::from);
    Canvas::new(size, move |x, y| {
        let (u, v) = (x / w - 0.5, y / h - 0.5);
        let halo = 22.0 * (-(u * u + v * v) / 0.05).exp();
        let base = 200.0 - 60.0 * (v + 0.5) - 40.0 * u * u + halo + random.range(-3.0, 3.0);
        [base, base, base + 4.0, 255.0]
    })
}

/// A dark bronze teapot (a body, a spout, a handle's loop) with a soft shadow on the floor
/// under it, and its true silhouette.
fn teapot(canvas: &mut Canvas) -> impl Fn(f64, f64) -> bool + use<> {
    let shadow = ellipse(470.0, 560.0, 250.0, 34.0);
    canvas.lay(&shadow, &mut |x, _| {
        let fade = (1.0 - ((x - 470.0) / 250.0).abs()).clamp(0.0, 1.0);
        [60.0, 60.0, 62.0, 110.0 * fade]
    });
    let body = ellipse(450.0, 400.0, 200.0, 150.0);
    let spout = |x: f64, y: f64| {
        (230.0..330.0).contains(&x) && (y - (300.0 + 0.6 * (330.0 - x))).abs() < 18.0
    };
    let handle_outer = ellipse(650.0, 380.0, 95.0, 95.0);
    let handle_hole = ellipse(668.0, 380.0, 55.0, 60.0);
    let handle = move |x: f64, y: f64| x > 620.0 && handle_outer(x, y) && !handle_hole(x, y);
    let truth = move |x: f64, y: f64| body(x, y) || spout(x, y) || handle(x, y);
    let mut grain = Random::new(5);
    canvas.lay(&truth, &mut |x, y| {
        let shade = 40.0 * ((x - 450.0) / 300.0) + 25.0 * ((y - 400.0) / 150.0);
        let base = 95.0 - shade + grain.range(-8.0, 8.0);
        [base, base * 0.8, base * 0.55, 255.0]
    });
    truth
}

#[test]
fn a_studio_photo_gives_the_object_its_handle_s_loop_and_no_shadow() {
    let size = [900, 700];
    let mut canvas = studio(size, 11);
    let truth = teapot(&mut canvas);
    let outline = outline(size, &canvas.rgba).expect("an outline");
    let shared = overlap(&outline, size, &truth);
    assert!(shared >= 0.96, "the silhouette shares {shared:.3}");
    let hole = outline.contours.iter().any(|contour| {
        let n = contour.len() as f64;
        let [x, y] = contour
            .iter()
            .fold([0.0, 0.0], |s, p| [s[0] + p[0] / n, s[1] + p[1] / n]);
        (x - 668.0).abs() < 15.0 && (y - 380.0).abs() < 15.0
    });
    assert!(
        hole,
        "the handle's loop is a hole: {} contours",
        outline.contours.len()
    );
    assert!(
        !filled(&outline, 470.0, 585.0),
        "the shadow under it is ground"
    );
    assert!(
        outline
            .contours
            .iter()
            .flatten()
            .all(|p| (0.0..=900.0).contains(&p[0]) && (0.0..=700.0).contains(&p[1]))
    );
}

#[test]
fn a_transparent_cut_out_keeps_its_separate_parts() {
    let size = [600, 600];
    let mut canvas = Canvas::new(size, |_, _| [0.0, 0.0, 0.0, 0.0]);
    let body = ellipse(300.0, 360.0, 150.0, 190.0);
    let head = ellipse(300.0, 140.0, 90.0, 85.0);
    let star = ellipse(520.0, 80.0, 40.0, 40.0);
    let truth = move |x: f64, y: f64| body(x, y) || head(x, y) || star(x, y);
    canvas.lay(&truth, &mut |_, y| [240.0, 180.0 - y / 6.0, 60.0, 255.0]);
    let outline = outline(size, &canvas.rgba).expect("an outline");
    let shared = overlap(&outline, size, &truth);
    assert!(shared >= 0.98, "the silhouette shares {shared:.3}");
    assert!(filled(&outline, 520.0, 80.0), "the separate star is a part");
}

#[test]
fn a_figure_on_flat_white_is_found() {
    let size = [800, 600];
    let mut canvas = Canvas::new(size, |_, _| [255.0, 255.0, 255.0, 255.0]);
    let fish = ellipse(400.0, 300.0, 300.0, 110.0);
    let tail = |x: f64, y: f64| x > 650.0 && x < 760.0 && (y - 300.0).abs() < (x - 650.0) * 0.9;
    let truth = move |x: f64, y: f64| fish(x, y) || tail(x, y);
    canvas.lay(&truth, &mut |x, _| [90.0 + x / 8.0, 120.0, 150.0, 255.0]);
    let outline = outline(size, &canvas.rgba).expect("an outline");
    let shared = overlap(&outline, size, &truth);
    assert!(shared >= 0.97, "the silhouette shares {shared:.3}");
}

#[test]
fn grounds_that_are_no_ground_give_no_outline() {
    let size = [640, 480];
    // A busy photo: textured things of every colour over each other everywhere.
    let mut random = Random::new(3);
    let mut grain = Random::new(8);
    let mut busy = Canvas::new(size, |_, _| [128.0, 128.0, 128.0, 255.0]);
    for _ in 0..160 {
        let (cx, cy) = (random.range(0.0, 640.0), random.range(0.0, 480.0));
        let (rx, ry) = (random.range(10.0, 90.0), random.range(10.0, 90.0));
        let colour = [0; 3].map(|_| random.range(0.0, 255.0));
        let area = [cx - rx, cy - ry, cx + rx, cy + ry];
        busy.lay_in(area, &ellipse(cx, cy, rx, ry), &mut |_, _| {
            let texture = grain.range(-30.0, 30.0);
            [
                colour[0] + texture,
                colour[1] + texture,
                colour[2] + texture,
                255.0,
            ]
        });
    }
    assert_eq!(
        outline(size, &busy.rgba).err(),
        Some(OutlineRefusal::Ground)
    );
    // An object running off the picture's right edge.
    let mut cut = studio(size, 4);
    cut.lay(&ellipse(560.0, 240.0, 120.0, 120.0), &mut |_, _| {
        [70.0, 50.0, 40.0, 255.0]
    });
    assert_eq!(outline(size, &cut.rgba).err(), Some(OutlineRefusal::Cut));
    // Nothing at all.
    let empty = studio(size, 9);
    assert_eq!(
        outline(size, &empty.rgba).err(),
        Some(OutlineRefusal::Nothing)
    );
    // Pixels that are not a picture of the size given.
    assert_eq!(
        outline([4, 4], &[0; 12]).err(),
        Some(OutlineRefusal::Pixels)
    );
}
