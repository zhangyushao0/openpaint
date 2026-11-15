//! Painted kinds judged against their answer, through Compare and the reading of their pixels:
//! a colour (its Oklab difference), a combination (its areas' mean difference and the weak
//! contrasts made strong), a value scale (its uneven and reversed cells), value masses (what is
//! hatched, the share matched, the share left bare) and a key shift (its areas marked).

use std::sync::Arc;

use openpaint_practice::{
    AWAY, Areas, Arrangement, Compare, Exercise, GeneratedAreas, Kind, Lean, Lightness, Masses,
    MassesShown, PictureFit, Point, Purity, Random, Rgb8, ScaleTone, SheetPicture, Tendency, Tier,
    Tone, Value, ValueMasses, ValueScale, Verdict, from_oklab, main_hue, oklab,
};

use crate::{Clean, Laid, Near, Redo, compare};

const PAPER: Rgb8 = [255, 255, 255];
const SIDE: u32 = 400;

/// `colour` darker by `by` in Oklab lightness.
fn darker(colour: Rgb8, by: f64) -> Rgb8 {
    let [l, a, b] = oklab(colour);
    from_oklab([l - by, a, b])
}

#[test]
fn a_colour_is_clean_within_a_noticeable_step_near_within_two_and_a_half_redo_past() {
    let tone = Tone {
        purity: Purity::Muted,
        lightness: Lightness::Middle,
    };
    let exercise = Exercise::generated(Kind::Colour(tone), 3);
    let swatch = exercise.original().swatch().expect("a swatch");
    let verdict = |picks: Vec<Rgb8>| {
        let laid = Laid {
            picks,
            ..Laid::default()
        };
        let Some(Compare::Made(shown)) = compare(&exercise, None, &laid) else {
            panic!("made")
        };
        shown.verdict()
    };
    assert_eq!(verdict(vec![swatch]), Verdict::App(Clean));
    // The first pick off, the last corrected: the colour at this Compare counts.
    assert_eq!(
        verdict(vec![darker(swatch, 0.1), swatch]),
        Verdict::App(Clean)
    );
    assert_eq!(verdict(vec![darker(swatch, 0.035)]), Verdict::App(Near));
    assert_eq!(verdict(vec![darker(swatch, 0.08)]), Verdict::App(Redo));
    // Nothing laid: nothing to compare.
    assert!(compare(&exercise, None, &Laid::default()).is_none());
}

/// A copy painted flat over `areas`: each pixel its area's colour in `colours`.
fn painted(areas: &Areas, colours: &[Rgb8]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let p = [x, y].map(|v| (f64::from(v) + 0.5) / f64::from(SIDE));
            let [r, g, b] = colours[areas.area_at(p)];
            pixels.extend_from_slice(&[r, g, b, 255]);
        }
    }
    pixels
}

/// `exercise`'s copy painted as `pixels` (the frame's, `SIDE` square) read, its verdict.
fn read(exercise: &Exercise, picture: Option<&SheetPicture>, pixels: &[u8]) -> Verdict {
    let held = Laid {
        held: true,
        ..Laid::default()
    };
    let Some(Compare::Read(reading)) = compare(exercise, picture, &held) else {
        panic!("read from its pixels")
    };
    assert_eq!(reading.unread().verdict(), Verdict::Incomplete, "unread");
    reading
        .read([SIDE, SIDE], pixels, PAPER)
        .comparison
        .verdict()
}

#[test]
fn a_combination_is_judged_by_its_areas_mean_difference_once_every_area_is_painted() {
    let exercise = Exercise::generated(Kind::Areas(Arrangement::Patches), 21);
    let Some(generated) = exercise.original().downcast::<GeneratedAreas>().cloned() else {
        panic!("a generated combination")
    };
    let areas = &generated.areas;
    let itself = generated.picture(SIDE);
    assert_eq!(read(&exercise, None, &itself), Verdict::App(Clean));
    // Darker all over keeps every contrast: judged by the mean alone.
    let all = |by: f64| -> Vec<Rgb8> { areas.colours.iter().map(|c| darker(*c, by)).collect() };
    assert_eq!(
        read(&exercise, None, &painted(areas, &all(0.07))),
        Verdict::App(Near)
    );
    assert_eq!(
        read(&exercise, None, &painted(areas, &all(0.15))),
        Verdict::App(Redo)
    );
    let mut bare = areas.colours.clone();
    bare[0] = PAPER;
    assert_eq!(
        read(&exercise, None, &painted(areas, &bare)),
        Verdict::Incomplete
    );
}

#[test]
fn a_value_scale_is_near_with_one_uneven_cell_of_five_two_of_nine_and_redo_reversed() {
    for (cells, near) in [(5, 1), (9, 2)] {
        let scale = ValueScale::generate(ScaleTone::Grey, cells, 4);
        let targets = scale.targets();
        let step = (targets[cells - 1] - targets[0]) / (cells - 1) as f64;
        // The cells 1, 2, ... `count` off by `by` steps.
        let filled = |count: usize, by: f64| -> Vec<Option<f64>> {
            (targets.iter().enumerate())
                .map(|(k, t)| {
                    Some(if (1..=count).contains(&k) {
                        t + by * step
                    } else {
                        *t
                    })
                })
                .collect()
        };
        let verdict = |values| scale.check(values).verdict();
        assert_eq!(verdict(filled(0, 0.0)), Verdict::App(Clean), "{cells}");
        assert_eq!(verdict(filled(near, 0.4)), Verdict::App(Near), "{cells}");
        assert_eq!(
            verdict(filled(near + 1, 0.4)),
            Verdict::App(Redo),
            "{cells}"
        );
        // One cell as dark as the one before it: out of order.
        assert_eq!(verdict(filled(1, -1.0)), Verdict::App(Redo), "{cells}");
        let mut half = filled(0, 0.0);
        half[cells / 2] = None;
        assert_eq!(verdict(half), Verdict::Incomplete, "{cells}");
    }
}

/// The frame's pixels (the picture's shape, its longer side `SIDE`) painted as `paint` gives each
/// share of the picture, bare paper for none.
fn painted_masses(fit: &PictureFit, paint: impl Fn(Point) -> Option<Value>) -> Vec<u8> {
    let unit = f64::from(SIDE);
    let size = fit.size.map(|side| (side * unit).round() as u32);
    (0..size[0] * size[1])
        .flat_map(|at| {
            let (x, y) = (f64::from(at % size[0]), f64::from(at / size[0]));
            let share = [
                (x + 0.5) / unit / fit.size[0],
                (y + 0.5) / unit / fit.size[1],
            ];
            let [r, g, b] = paint(share).map_or(PAPER, Value::grey);
            [r, g, b, 255]
        })
        .collect()
}

#[test]
fn value_masses_are_near_with_a_corner_off_redo_with_half_and_incomplete_half_bare() {
    // A square landscape, so the frame is the picture.
    let (size, pixels) = Masses::generate(3, 1.0).picture(SIDE);
    assert_eq!(size, [SIDE, SIDE]);
    let answer = Arc::new(ValueMasses::of(size, &pixels));
    let fit = PictureFit::of(size);
    let picture = SheetPicture {
        item: openpaint_practice::ItemRef::default(),
        size,
        fit,
        outline: None,
        credit: None,
        masses: Some(MassesShown {
            answer: Arc::clone(&answer),
            edges: Vec::new(),
        }),
        areas: None,
    };
    let exercise = Exercise::generated(Kind::ValueMasses(Tier::ALL[0]), 1);
    let other = |value: Value| match value {
        Value::Light => Value::Dark,
        _ => Value::Light,
    };
    let verdict = |paint: &dyn Fn(Point) -> Option<Value>| {
        read(&exercise, Some(&picture), &painted_masses(&fit, paint))
    };
    assert_eq!(verdict(&|p| Some(answer.at(p))), Verdict::App(Clean));
    let corner = |p: Point| {
        let value = answer.at(p);
        Some(if p[0] < 0.3 && p[1] < 0.3 {
            other(value)
        } else {
            value
        })
    };
    assert_eq!(verdict(&corner), Verdict::App(Near));
    let half = |p: Point| {
        let value = answer.at(p);
        Some(if p[0] < 0.5 { other(value) } else { value })
    };
    assert_eq!(verdict(&half), Verdict::App(Redo));
    let bare = |p: Point| (p[0] >= 0.5).then(|| answer.at(p));
    assert_eq!(verdict(&bare), Verdict::Incomplete);
}

#[test]
fn a_key_shift_is_near_with_one_area_marked_redo_with_two_incomplete_with_one_bare() {
    let areas =
        openpaint_practice::GeneratedAreas::generate(Arrangement::Patches, 0x5EED_0107).areas;
    let main = main_hue(&areas);
    let tendency = Tendency::chosen(Lean::CoolPure, main, AWAY, &mut Random::new(3));
    let picture = SheetPicture {
        item: openpaint_practice::ItemRef::default(),
        size: [SIDE, SIDE],
        fit: PictureFit::of([SIDE, SIDE]),
        outline: None,
        credit: None,
        masses: None,
        areas: Some(Arc::new(areas.clone())),
    };
    let exercise = Exercise {
        kind: Kind::KeyShift(tendency),
        seed: 1,
        picture: Some(picture.item.clone()),
    };
    let shifted = tendency.shifted(&areas);
    let verdict = |colours: &[Rgb8]| read(&exercise, Some(&picture), &painted(&areas, colours));
    assert_eq!(verdict(&shifted), Verdict::App(Clean));
    // Areas painted as they were: not toward the tendency.
    let left = |count: usize| -> Vec<Rgb8> {
        let mut copy = shifted.clone();
        let unmoved = (0..areas.count()).filter(|a| shifted[*a] != areas.colours[*a]);
        for area in unmoved.take(count) {
            copy[area] = areas.colours[area];
        }
        copy
    };
    assert_eq!(verdict(&left(1)), Verdict::App(Near));
    assert_eq!(verdict(&left(2)), Verdict::App(Redo));
    let mut bare = shifted.clone();
    bare[0] = PAPER;
    assert_eq!(verdict(&bare), Verdict::Incomplete);
}
