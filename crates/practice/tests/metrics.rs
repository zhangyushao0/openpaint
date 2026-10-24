//! The colour numbers against known cases: Oklab and known colour pairs give known differences,
//! the difference parts as the eye sees them (equal colours, a pure lightness, chroma and hue
//! step, two dark colours an olive and a brown), and a match scores its first pick, its final
//! colour and its corrections.

use openpaint_practice::{
    ColourCopy, Difference, HUELESS, Metric, delta_e, from_oklch, oklab, oklch, score_colour, value,
};

/// `a` within `tolerance` of `b`.
fn near(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

#[test]
fn oklab_and_its_differences_are_the_reference_values() {
    let close = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 2e-4);
    assert!(close(oklab([255, 0, 0]), [0.627_955, 0.224_863, 0.125_846]));
    assert!(close(
        oklab([0, 255, 0]),
        [0.866_440, -0.233_888, 0.179_498]
    ));
    assert!(close(oklab([255, 255, 255]), [1.0, 0.0, 0.0]));
    assert!((delta_e([255, 255, 255], [0, 0, 0]) - 1.0).abs() < 2e-4);
    // The polar form: red's hue about 29 degrees, blue's 264; there and back.
    let [_, chroma, hue] = oklch([255, 0, 0]);
    assert!(
        near(chroma, 0.257_7, 2e-4) && near(hue, 29.23, 0.05),
        "{hue}"
    );
    assert!(near(oklch([0, 0, 255])[2], 264.05, 0.05));
    for colour in [
        [255, 0, 0],
        [42, 46, 37],
        [50, 47, 42],
        [128, 64, 64],
        [10, 200, 240],
    ] {
        assert_eq!(from_oklch(oklch(colour)), colour);
    }
}

#[test]
fn equal_colours_differ_in_nothing() {
    for colour in [
        [0, 0, 0],
        [128, 128, 128],
        [42, 46, 37],
        [255, 0, 0],
        [240, 230, 200],
    ] {
        let same = Difference::of(colour, colour);
        assert_eq!(
            [same.delta, same.lightness, same.chroma, same.hue],
            [0.0; 4]
        );
    }
}

#[test]
fn a_pure_step_shows_in_its_own_part_alone() {
    // Lightness: two greys.
    let lighter = Difference::of([100, 100, 100], [140, 140, 140]);
    assert!(lighter.lightness > 0.1, "{lighter:?}");
    assert!(lighter.chroma.abs() < 1e-6 && lighter.hue.abs() < 1e-6);
    assert!(near(lighter.delta, lighter.lightness, 1e-6));
    // Chroma: the same lightness and hue, twice as pure.
    let purer = Difference::of(
        from_oklch([0.6, 0.05, 200.0]),
        from_oklch([0.6, 0.1, 200.0]),
    );
    assert!(near(purer.chroma, 0.05, 0.003), "{purer:?}");
    assert!(purer.lightness.abs() < 0.003 && purer.hue.abs() < 0.003);
    // Hue: 30 degrees on from red's side towards yellow is positive; the chord at chroma 0.1.
    let (red, orange) = (from_oklch([0.65, 0.1, 30.0]), from_oklch([0.65, 0.1, 60.0]));
    let on = Difference::of(red, orange);
    let chord = 2.0 * 0.1 * 15f64.to_radians().sin();
    assert!(near(on.hue, chord, 0.003), "{on:?}");
    assert!(on.lightness.abs() < 0.003 && on.chroma.abs() < 0.003);
    assert!(near(Difference::of(orange, red).hue, -chord, 0.003));
    // Round the wheel the short way: 350 degrees to 10 is 20 on.
    let wrapped = Difference::of(from_oklch([0.6, 0.1, 350.0]), from_oklch([0.6, 0.1, 10.0]));
    assert!(near(
        wrapped.hue,
        2.0 * 0.1 * 10f64.to_radians().sin(),
        0.003
    ));
}

#[test]
fn a_dark_olive_against_a_dark_warm_brown_shows_their_hues_apart() {
    // The author's pair on the tablet: both under 0.06 of RGB chroma, which once called them
    // grey and said nothing of their hues.
    let (olive, brown) = ([42, 46, 37], [50, 47, 42]);
    let [l0, c0, h0] = oklch(olive);
    let [l1, c1, h1] = oklch(brown);
    assert!(near(l0, 0.294, 0.001) && near(c0, 0.016_8, 0.000_5) && near(h0, 127.0, 1.0));
    assert!(near(l1, 0.307, 0.001) && near(c1, 0.009_8, 0.000_5) && near(h1, 80.7, 1.0));
    assert!(c0 > HUELESS && c1 > HUELESS, "both have a hue to draw");
    let brown_against_olive = Difference::of(olive, brown);
    // A little lighter, a little greyer, and its hue back towards orange: each part about one
    // just-noticeable half step, ΔE 1.75.
    assert!(near(brown_against_olive.lightness, 0.012_5, 0.000_5));
    assert!(near(brown_against_olive.chroma, -0.007_1, 0.000_5));
    assert!(near(brown_against_olive.hue, -0.010_1, 0.000_5));
    assert!(near(brown_against_olive.delta, 0.017_5, 0.000_5));
    let olive_against_brown = Difference::of(brown, olive);
    assert!(
        near(olive_against_brown.hue, 0.010_1, 0.000_5),
        "towards green"
    );
}

#[test]
fn the_parts_make_up_the_distance() {
    let mut state = 0x5EED_u64;
    let mut byte = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (state >> 56) as u8
    };
    for _ in 0..500 {
        let (a, b) = ([byte(), byte(), byte()], [byte(), byte(), byte()]);
        let d = Difference::of(a, b);
        let parts = d.lightness.powi(2) + d.chroma.powi(2) + d.hue.powi(2);
        assert!(near(d.delta.powi(2), parts, 1e-9), "{a:?} {b:?}: {d:?}");
        assert!(d.largest_part() <= d.delta + 1e-12);
    }
    // Red against green: lighter, purer and on round the wheel towards green.
    let red_green = Difference::of([255, 0, 0], [0, 255, 0]);
    assert!((red_green.delta - 0.519_81).abs() < 5e-4, "{red_green:?}");
    assert!(red_green.lightness > 0.2 && red_green.chroma > 0.0 && red_green.hue > 0.2);
}

#[test]
fn a_match_scores_its_first_pick_its_last_and_its_corrections() {
    let original = [128, 64, 64];
    let copy = ColourCopy {
        picks: vec![
            [160, 120, 100],
            [160, 120, 100],
            [140, 80, 70],
            [130, 66, 64],
        ],
    };
    let score = score_colour(original, &copy).expect("score");
    assert_eq!(score.first, Difference::of(original, [160, 120, 100]));
    assert_eq!(score.last, Difference::of(original, [130, 66, 64]));
    assert_eq!(score.corrections, 2);
    assert!(score.last.delta < score.first.delta);
    let metrics = score.metrics();
    assert_eq!(value(&metrics, Metric::Corrections), Some(2.0));
    assert_eq!(value(&metrics, Metric::FirstDelta), Some(score.first.delta));
    assert_eq!(value(&metrics, Metric::FirstHue), Some(score.first.hue));
    assert_eq!(
        value(&metrics, Metric::FinalLightness),
        Some(score.last.lightness)
    );
    assert_eq!(
        value(&metrics, Metric::FinalChroma),
        Some(score.last.chroma)
    );
    assert_eq!(metrics.len(), 9, "every part stored, a grey's hue too");
    assert_eq!(score_colour(original, &ColourCopy::default()), None);
    let exact = score_colour(
        original,
        &ColourCopy {
            picks: vec![original],
        },
    )
    .expect("score");
    assert_eq!((exact.last.delta, exact.corrections), (0.0, 0));
}
