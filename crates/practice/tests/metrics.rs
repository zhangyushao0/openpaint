//! The metrics against known cases: an exact copy scores zero; a moved or evenly sized copy errs
//! in the frame but not in shape; a turned one errs in shape too; a shaky one wobbles; known
//! colour pairs give known differences.

use openpaint_practice::{
    ColourCopy, Difference, LineCopy, LineExercise, LineTier, Metric, Point, delta_e, length,
    oklab, score_colour, score_line, value,
};

/// A copy drawn along `points` in one stroke taking `ms`.
fn copy_of(points: Vec<Point>, ms: f64) -> LineCopy {
    LineCopy {
        length: length(&points),
        pieces: vec![points],
        strokes: 1,
        erased: 0,
        duration_ms: ms,
    }
}

fn line(tier: LineTier) -> LineExercise {
    LineExercise::generate(tier, 0x5EED_0045)
}

#[test]
fn an_exact_copy_scores_zero() {
    for tier in LineTier::ALL {
        let original = line(tier);
        let score = score_line(&original, &copy_of(original.points.clone(), 500.0)).expect("score");
        assert!(score.key_points < 1e-9, "{tier:?} {score:?}");
        assert!(score.frame < 1e-9, "{tier:?} {score:?}");
        assert!(score.shape < 1e-9, "{tier:?} {score:?}");
        assert!(score.wobble < 5e-4, "smooth: {tier:?} {score:?}");
        let speed = score.speed.expect("speed");
        assert!((speed - length(&original.points) / 0.5).abs() < 1e-9);
        assert_eq!((score.strokes, score.erased, score.seconds), (1, 0, 0.5));
    }
}

#[test]
fn a_moved_copy_errs_in_the_frame_but_not_in_shape() {
    let original = line(LineTier::Wave);
    let moved: Vec<Point> = original
        .points
        .iter()
        .map(|p| [p[0] + 0.05, p[1] - 0.03])
        .collect();
    let score = score_line(&original, &copy_of(moved, 400.0)).expect("score");
    assert!(score.frame > 0.02 && score.frame <= 0.06, "{score:?}");
    assert!(
        score.key_points > 0.02 && score.key_points <= 0.0584,
        "{score:?}"
    );
    assert!(score.shape < 1e-9, "{score:?}");
}

#[test]
fn an_evenly_sized_copy_errs_in_the_frame_but_not_in_shape() {
    let original = line(LineTier::Arc);
    let sized: Vec<Point> = original
        .points
        .iter()
        .map(|p| [0.5 + (p[0] - 0.5) * 0.8, 0.5 + (p[1] - 0.5) * 0.8])
        .collect();
    let score = score_line(&original, &copy_of(sized, 400.0)).expect("score");
    assert!(score.frame > 0.01, "{score:?}");
    assert!(score.shape < 1e-6, "{score:?}");
}

#[test]
fn a_turned_copy_errs_in_shape() {
    let original = line(LineTier::Hook);
    let (sin, cos) = 0.4_f64.sin_cos();
    let turned: Vec<Point> = original
        .points
        .iter()
        .map(|p| {
            let [x, y] = [p[0] - 0.5, p[1] - 0.5];
            [0.5 + x * cos - y * sin, 0.5 + x * sin + y * cos]
        })
        .collect();
    let score = score_line(&original, &copy_of(turned, 400.0)).expect("score");
    assert!(score.shape > 0.01, "{score:?}");
}

#[test]
fn a_shaky_copy_wobbles_and_a_smooth_wrong_one_does_not() {
    let original = line(LineTier::Straight);
    let points = &original.points;
    let mut along = 0.0;
    let shaky: Vec<Point> = (0..points.len())
        .map(|index| {
            if index > 0 {
                along += openpaint_practice::distance(points[index - 1], points[index]);
            }
            // A tremor across the line: 0.5 % of the frame to either side, a wave every 2 %.
            let side = 0.005 * (std::f64::consts::TAU * along / 0.02).sin();
            let [a, b] = [
                points[index.saturating_sub(1)],
                points[(index + 1).min(points.len() - 1)],
            ];
            let tangent = [b[0] - a[0], b[1] - a[1]];
            let norm = tangent[0].hypot(tangent[1]);
            let p = points[index];
            [
                p[0] - tangent[1] / norm * side,
                p[1] + tangent[0] / norm * side,
            ]
        })
        .collect();
    let shaky = score_line(&original, &copy_of(shaky, 400.0)).expect("score");
    // The tremor's own root mean square is 0.35 %.
    assert!(shaky.wobble > 0.0015, "{shaky:?}");
    let bowed: Vec<Point> = original
        .points
        .iter()
        .enumerate()
        .map(|(index, p)| {
            let t = index as f64 / (original.points.len() - 1) as f64;
            let bow = 0.06 * (std::f64::consts::PI * t).sin();
            [p[0] + bow, p[1] + bow]
        })
        .collect();
    let bowed = score_line(&original, &copy_of(bowed, 400.0)).expect("score");
    assert!(bowed.wobble < 5e-4, "{bowed:?}");
    assert!(bowed.frame > 0.01, "{bowed:?}");
    assert!(shaky.wobble > 10.0 * bowed.wobble);
}

#[test]
fn a_copy_in_pieces_counts_every_piece_and_nothing_scores_nothing() {
    let original = line(LineTier::Arc);
    let half = original.points.len() / 2;
    let copy = LineCopy {
        pieces: vec![
            original.points[..=half].to_vec(),
            original.points[half..].to_vec(),
        ],
        strokes: 2,
        erased: 1,
        length: length(&original.points),
        duration_ms: 0.0,
    };
    let score = score_line(&original, &copy).expect("score");
    assert!(score.frame < 1e-9 && score.key_points < 1e-9, "{score:?}");
    assert_eq!(score.speed, None, "no time, no speed");
    let metrics = score.metrics();
    assert_eq!(value(&metrics, Metric::Strokes), Some(2.0));
    assert_eq!(value(&metrics, Metric::Erased), Some(1.0));
    assert_eq!(value(&metrics, Metric::Speed), None);
    assert_eq!(score_line(&original, &LineCopy::default()), None);
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
