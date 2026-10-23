//! Value scales: the same scale per seed, its even steps even in lightness (Oklab `L`) with the
//! hue held, and a copy's cells read from its pixels and checked against the even scale on known
//! cases: a cell of another hue at the same value of the wheel reads as lighter as the eye sees.

use openpaint_practice::{Metric, Rgb8, ScaleTone, ValueScale, from_hsv, hsv, oklab, value};

const PAPER: Rgb8 = [255, 255, 255];
const SIDE: u32 = 400;

#[test]
fn the_same_seed_gives_the_same_scale_between_a_far_dark_and_light() {
    for tone in ScaleTone::ALL {
        let mut counts = [0; 8];
        for seed in 0..300 {
            let scale = ValueScale::generate(tone, seed);
            assert_eq!(scale, ValueScale::generate(tone, seed));
            assert!((5..=7).contains(&scale.cells), "{tone:?} {seed}");
            counts[scale.cells] += 1;
            assert!(scale.light - scale.dark >= 0.48, "{tone:?} {seed}");
            let [dark, light] = scale.ends();
            match tone {
                ScaleTone::Grey => assert!(dark[0] == dark[1] && dark[1] == dark[2]),
                ScaleTone::Hue => assert!((0.15..=0.6).contains(&hsv(light)[1])),
            }
            let targets = scale.targets();
            assert_eq!(targets[0], oklab(dark)[0]);
            assert_eq!(targets[scale.cells - 1], oklab(light)[0]);
            let steps: Vec<f64> = targets.windows(2).map(|w| w[1] - w[0]).collect();
            assert!(steps.iter().all(|s| (s - steps[0]).abs() < 1e-12), "even");
            // Each step's colour has its lightness and, on a hue, keeps the hue.
            for target in &targets {
                let colour = scale.colour(*target);
                let lightness = oklab(colour)[0];
                assert!((lightness - target).abs() < 0.006, "{tone:?} {seed}");
                if tone == ScaleTone::Hue {
                    let off = (hsv(colour)[0] - scale.hue + 180.0).rem_euclid(360.0) - 180.0;
                    assert!(off.abs() < 6.0, "{seed}: {off} degrees off");
                }
            }
        }
        assert!(
            counts[5] > 0 && counts[6] > 0 && counts[7] > 0,
            "{counts:?}"
        );
    }
}

/// A copy with each cell filled with `values` (none: left bare), as pixels of the frame.
fn filled(scale: &ValueScale, values: &[Option<f64>]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let p = [x, y].map(|v| (f64::from(v) + 0.5) / f64::from(SIDE));
            let colour = (scale.cell_at(p))
                .and_then(|cell| values[cell])
                .map_or(PAPER, |v| scale.colour(v));
            pixels.extend_from_slice(&[colour[0], colour[1], colour[2], 255]);
        }
    }
    pixels
}

#[test]
fn an_even_copy_reads_even_and_mistakes_show_where_they_are() {
    let scale = ValueScale::generate(ScaleTone::Hue, 0x5EED_0045);
    let targets = scale.targets();
    let step = (targets[scale.cells - 1] - targets[0]) / (scale.cells - 1) as f64;
    let even: Vec<Option<f64>> = targets.iter().copied().map(Some).collect();
    let read = scale.read([SIDE, SIDE], &filled(&scale, &even), PAPER);
    for (got, want) in read.values.iter().zip(&targets) {
        let got = got.expect("filled");
        assert!((got - want).abs() < 0.006, "{got} {want}");
    }
    assert!(read.uneven.is_empty() && read.reversed.is_empty());
    // The second cell half a step too dark, the third left bare.
    let mut off = even.clone();
    off[1] = Some(targets[1] - step / 2.0);
    off[2] = None;
    let read = scale.read([SIDE, SIDE], &filled(&scale, &off), PAPER);
    assert_eq!(read.uneven, [1]);
    assert_eq!(read.values[2], None);
    assert!(read.reversed.is_empty());
    // Two cells swapped: both uneven, and the later no lighter than the one before it.
    let mut swapped = even.clone();
    swapped.swap(1, 2);
    let read = scale.read([SIDE, SIDE], &filled(&scale, &swapped), PAPER);
    assert_eq!(read.uneven, [1, 2]);
    assert_eq!(read.reversed, [2]);
    let metrics = read.metrics();
    assert_eq!(value(&metrics, Metric::ScaleUneven), Some(2.0));
    assert_eq!(value(&metrics, Metric::ScaleReversed), Some(1.0));
    let mean = value(&metrics, Metric::ScaleOff).expect("filled cells");
    let expected = 2.0 * step / (scale.cells - 2) as f64;
    assert!((mean - expected).abs() < 0.006, "{mean} {expected}");
    // A quarter of a step is where uneven begins; a little less is even.
    let mut near = even.clone();
    near[1] = Some(targets[1] + step * 0.2);
    let read = scale.read([SIDE, SIDE], &filled(&scale, &near), PAPER);
    assert!(read.uneven.is_empty(), "{:?}", read.values);
}

/// The wheel's value is no lightness: a blue scale's middle cell painted yellow at the very value
/// of its even step is far lighter to the eye, and reads so (too light, uneven); painted grey of
/// the step's lightness it reads even, as the eye sees it.
#[test]
fn a_cell_is_judged_by_its_lightness_not_the_wheel_s_value() {
    let scale = ValueScale {
        cells: 5,
        hue: 230.0,
        saturation: 0.5,
        dark: 0.2,
        light: 0.9,
    };
    let targets = scale.targets();
    let mut cells: Vec<Option<f64>> = targets.iter().copied().map(Some).collect();
    let middle = scale.colour(targets[2]);
    let yellow = from_hsv([60.0, 0.5, hsv(middle)[2]]);
    assert!(
        (hsv(yellow)[2] - hsv(middle)[2]).abs() < 0.005,
        "the same value"
    );
    let mut pixels = filled(&scale, &cells);
    paint_cell(&scale, &mut pixels, 2, yellow);
    let read = scale.read([SIDE, SIDE], &pixels, PAPER);
    assert_eq!(read.uneven, [2], "{:?}", read.values);
    assert!(
        read.values[2].expect("painted") > targets[2] + 0.1,
        "lighter"
    );
    // A grey of the step's lightness is even, though its value differs from the blue's.
    cells[2] = Some(targets[2]);
    let mut pixels = filled(&scale, &cells);
    let grey = (0..=255u8)
        .map(|g| [g; 3])
        .min_by(|a, b| {
            let off = |c: &Rgb8| (oklab(*c)[0] - targets[2]).abs();
            off(a).total_cmp(&off(b))
        })
        .expect("a grey");
    paint_cell(&scale, &mut pixels, 2, grey);
    let read = scale.read([SIDE, SIDE], &pixels, PAPER);
    assert!(read.uneven.is_empty(), "{:?}", read.values);
}

/// Cell `cell` of the frame's pixels painted `colour` all over.
fn paint_cell(scale: &ValueScale, pixels: &mut [u8], cell: usize, colour: Rgb8) {
    for y in 0..SIDE {
        for x in 0..SIDE {
            let p = [x, y].map(|v| (f64::from(v) + 0.5) / f64::from(SIDE));
            if scale.cell_at(p) == Some(cell) {
                let at = 4 * (y * SIDE + x) as usize;
                pixels[at..at + 3].copy_from_slice(&colour);
            }
        }
    }
}

#[test]
fn a_cell_reads_its_middle_whatever_strays_over_its_edges() {
    let scale = ValueScale::generate(ScaleTone::Grey, 7);
    let targets = scale.targets();
    let even: Vec<Option<f64>> = targets.iter().copied().map(Some).collect();
    let mut pixels = filled(&scale, &even);
    // A stroke of the next cell's value across the left fifth of every cell, and dark specks.
    for y in 0..SIDE {
        for x in 0..SIDE {
            let p = [x, y].map(|v| (f64::from(v) + 0.5) / f64::from(SIDE));
            let Some(cell) = scale.cell_at(p) else {
                continue;
            };
            let [[left, _], [right, _]] = scale.cell(cell);
            let at = 4 * (y * SIDE + x) as usize;
            if p[0] < left + (right - left) * 0.2 && cell + 1 < scale.cells {
                let next = scale.colour(targets[cell + 1]);
                pixels[at..at + 3].copy_from_slice(&next);
            } else if (x + y) % 7 == 0 {
                pixels[at..at + 3].copy_from_slice(&[0, 0, 0]);
            }
        }
    }
    let read = scale.read([SIDE, SIDE], &pixels, PAPER);
    assert!(
        read.uneven.is_empty() && read.reversed.is_empty(),
        "{:?}",
        read.values
    );
}
