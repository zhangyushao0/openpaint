//! Value scales: the same scale per seed, its even steps even in lightness (Oklab `L`) with the
//! hue held, and a copy's cells read from its pixels and checked against the even scale on known
//! cases: a cell of another hue at the same value of the wheel reads as lighter as the eye sees.
//! The order to paint (S97): halved and halved again; the even row cell under cell.

use openpaint_practice::{
    Before, CELLS, EVEN_ROW, FIRST_CELLS, Family, Kind, Metric, Pictures, Plan, ROW, Rgb8,
    ScaleTone, ValueScale, drill, from_hsv, halving, hsv, oklab, stage_two, value,
};

const PAPER: Rgb8 = [255, 255, 255];
const SIDE: u32 = 400;

#[test]
fn the_same_seed_gives_the_same_scale_between_a_far_dark_and_light() {
    for tone in ScaleTone::ALL {
        for (seed, cells) in (0..300).zip([FIRST_CELLS, CELLS].into_iter().cycle()) {
            let scale = ValueScale::generate(tone, cells, seed);
            assert_eq!(scale, ValueScale::generate(tone, cells, seed));
            assert_eq!(scale.cells, cells, "{tone:?} {seed}");
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
    }
}

/// S97: a value scale has nine cells, but the first two a beginner meets (in sessions and drills
/// alike) five; a plan's own scales count in order after those met before; stored as (6, tone +
/// 2 x cells).
#[test]
fn the_first_two_scales_met_have_five_cells_the_rest_nine() {
    let none = Pictures::default();
    let before = |met, scales| Before {
        met,
        pictures: &none,
        used: &[],
        recent: &[],
        scales,
    };
    let scales = |plan: Plan| -> Vec<Kind> {
        (plan.exercises.into_iter())
            .filter(|e| e.kind.family() == Family::Scale)
            .map(|e| e.kind)
            .collect()
    };
    let cells = |met: usize, number| scales(stage_two(number, 9, &before(&[], met)));
    assert_eq!(cells(0, 1), [Kind::Scale(ScaleTone::Grey, 5)]);
    assert_eq!(cells(1, 2), [Kind::Scale(ScaleTone::Hue, 5)]);
    assert_eq!(cells(2, 3), [Kind::Scale(ScaleTone::Grey, 9)]);
    // A drill of two after one met: the second of the beginner's, then a full one.
    let drill = drill(Family::Scale, 1, 4, &before(&[], 1)).expect("a drill");
    let full = [
        Kind::Scale(ScaleTone::Grey, 5),
        Kind::Scale(ScaleTone::Hue, 9),
    ];
    assert_eq!(scales(drill), full);
    // A warm-up drawing on a beginner's scale met gives it its full nine once two were met.
    let met = [Kind::Scale(ScaleTone::Hue, 5)];
    let warm = scales(stage_two(3, 5, &before(&met, 2)));
    assert!(
        warm.iter().all(|k| matches!(k, Kind::Scale(_, 9))),
        "{warm:?}"
    );
    for kind in full {
        let (code, variant) = kind.code();
        assert_eq!(Kind::of_code(code, variant), Some(kind));
    }
    assert_eq!(Kind::Scale(ScaleTone::Hue, 9).code(), (6, 19));
    assert_eq!(
        Kind::of_code(6, 1),
        None,
        "a tone alone, 1.14.0's form: no scale"
    );
}

#[test]
fn the_order_to_paint_halves_and_halves_again_and_the_even_row_lies_cell_under_cell() {
    let rounds = halving;
    assert_eq!(rounds(9), vec![vec![4], vec![2, 6], vec![1, 3, 5, 7]]);
    assert_eq!(rounds(5), vec![vec![2], vec![1, 3]]);
    // Every cell between the ends painted once, whatever the count (a 1.14.0 scale's 6 or 7).
    for cells in 5..=9 {
        let mut all: Vec<usize> = rounds(cells).into_iter().flatten().collect();
        all.sort_unstable();
        assert_eq!(all, (1..cells - 1).collect::<Vec<_>>(), "{cells}");
    }
    let scale = ValueScale::generate(ScaleTone::Hue, CELLS, 2);
    for cell in 0..CELLS {
        let [[left, top], [right, bottom]] = scale.cell(cell);
        let [[even_left, even_top], [even_right, even_bottom]] = scale.even_cell(cell);
        assert_eq!((left, right), (even_left, even_right), "cell under cell");
        assert_eq!([top, bottom], [ROW[1], ROW[3]]);
        assert_eq!([even_top, even_bottom], EVEN_ROW);
        assert!(
            even_top > bottom && even_bottom <= 1.0,
            "under the copy's row"
        );
        assert_eq!(
            scale.cell_at([(left + right) / 2.0, (even_top + even_bottom) / 2.0]),
            None
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
    let scale = ValueScale::generate(ScaleTone::Hue, 7, 0x5EED_0045);
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
    let scale = ValueScale::generate(ScaleTone::Grey, 9, 7);
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
