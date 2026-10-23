//! The colour numbers against known cases: Oklab and known colour pairs give known differences,
//! a match scores its first pick, its final colour and its corrections, and a hue's move is put
//! in V大's words.

use openpaint_practice::{
    ColourCopy, Difference, Metric, delta_e, hue_name, hue_towards, oklab, score_colour, value,
};

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
}

#[test]
fn a_known_colour_pair_gives_known_hue_saturation_value_and_delta() {
    // Red against green: a third of the way round, as pure and as light.
    let red_green = Difference::of([255, 0, 0], [0, 255, 0]);
    assert!((red_green.hue.expect("hue") - 120.0).abs() < 1e-9);
    assert!(red_green.saturation.abs() < 1e-9 && red_green.value.abs() < 1e-9);
    assert!((red_green.delta - 0.519_81).abs() < 5e-4, "{red_green:?}");
    // A muted red against a lighter, greyer orange-red.
    let muted = Difference::of([128, 64, 64], [160, 120, 100]);
    assert!((muted.hue.expect("hue") - 20.0).abs() < 1e-9, "{muted:?}");
    assert!((muted.saturation - (60.0 / 160.0 - 0.5)).abs() < 1e-9);
    assert!((muted.value - 32.0 / 255.0).abs() < 1e-9);
    // Round the wheel the short way: 350 degrees to 10 is 20 on.
    let wrapped = Difference::of([255, 0, 43], [255, 43, 0]);
    assert!(
        (wrapped.hue.expect("hue") - 20.24).abs() < 0.01,
        "{wrapped:?}"
    );
    // A grey has no hue to compare.
    assert_eq!(Difference::of([120, 120, 120], [255, 0, 0]).hue, None);
}

#[test]
fn a_hue_moved_is_redder_or_yellower_towards_the_next_of_six() {
    // Orange (30 degrees) moved on is yellower (towards 1, yellow), moved back redder (0, red).
    assert_eq!(hue_towards(30.0, 12.0), 1);
    assert_eq!(hue_towards(30.0, -12.0), 0);
    // From a hue of the six itself, towards the next one either way.
    assert_eq!(hue_towards(60.0, 5.0), 2);
    assert_eq!(hue_towards(60.0, -5.0), 0);
    assert_eq!(hue_towards(0.0, -5.0), 5);
    assert_eq!(hue_towards(350.0, 15.0), 0);
    assert_eq!(hue_towards(200.0, 10.0), 4);
    assert_eq!(hue_towards(200.0, -10.0), 3);
    // Twelve names 30 degrees apart: 15 degrees on is the next one.
    assert_eq!(
        [0.0, 14.0, 16.0, 200.0, 350.0].map(hue_name),
        [0, 0, 1, 7, 0]
    );
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
