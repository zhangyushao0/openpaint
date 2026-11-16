use std::f64::consts::PI;

use super::*;

/// A path bending every way: weaving side to side as it goes back, rising and falling.
const POINTS: [Vector; 5] = [
    [-1.0, -1.0, 3.0],
    [0.6, -0.8, 4.2],
    [-0.2, -0.3, 5.5],
    [0.9, 0.1, 6.1],
    [0.3, -0.6, 8.0],
];
const UP: Vector = [0.0, 1.0, 0.0];

fn path() -> SpacePath {
    SpacePath::through(&POINTS).expect("points")
}

/// The length along the curve from `from` to `to`, by short chords between its points.
fn measured(path: &SpacePath, from: f64, to: f64) -> f64 {
    let s = |k: usize| from + (to - from) * k as f64 / 200.0;
    (0..200)
        .map(|k| norm(sub(path.at(s(k + 1)), path.at(s(k)))))
        .sum()
}

fn finite(samples: &[Sample]) -> bool {
    samples.iter().all(|sample| {
        [sample.at, sample.tangent, sample.normal, sample.binormal]
            .iter()
            .flatten()
            .all(|v| v.is_finite())
    })
}

#[test]
fn the_path_passes_through_its_points() {
    for count in 3..=5 {
        let path = SpacePath::through(&POINTS[..count]).expect("points");
        for (index, point) in POINTS[..count].iter().enumerate() {
            let s = path.along[index * STEPS] / path.length();
            let off = norm(sub(path.at(s), *point));
            assert!(off < 1e-9, "{count} points, point {index}: {off}");
        }
    }
}

#[test]
fn samples_lie_at_even_lengths() {
    let path = path();
    // The length against short chords through the spans' own parameter.
    let chords: f64 = path
        .spans
        .iter()
        .map(|cubic| {
            let at = |k: usize| point(cubic, k as f64 / 4000.0);
            (0..4000).map(|k| norm(sub(at(k + 1), at(k)))).sum::<f64>()
        })
        .sum();
    assert!((path.length() - chords).abs() < 1e-7 * chords);
    let samples = path.samples(40, UP, &[]);
    assert_eq!(samples.len(), 40);
    let step = path.length() / 39.0;
    for (k, pair) in samples.windows(2).enumerate() {
        assert!((pair[0].s - k as f64 / 39.0).abs() < 1e-15);
        let gap = measured(&path, pair[0].s, pair[1].s) - step;
        assert!(gap.abs() < 1e-6 * path.length(), "{k}: {gap}");
    }
    assert!(norm(sub(samples[0].at, POINTS[0])) < 1e-9);
    assert!(norm(sub(samples[39].at, POINTS[4])) < 1e-9);
}

#[test]
fn tangents_are_unit_and_follow_the_curve() {
    let path = path();
    let h = 1e-7;
    for sample in path.samples(25, UP, &[]) {
        assert!((norm(sample.tangent) - 1.0).abs() < 1e-12);
        let (before, after) = ((sample.s - h).max(0.0), (sample.s + h).min(1.0));
        let difference = unit(sub(path.at(after), path.at(before)));
        let off = norm(sub(difference, sample.tangent));
        assert!(off < 1e-5, "{sample:?}: {off}");
    }
}

#[test]
fn the_frame_stays_orthonormal() {
    let turns = [
        Turn {
            at: 0.3,
            span: 0.2,
            by: PI,
        },
        Turn {
            at: 0.7,
            span: 0.1,
            by: -PI,
        },
    ];
    let path = path();
    for samples in [
        path.samples(60, UP, &[]),
        path.samples(7, [0.3, 0.2, -1.0], &turns),
    ] {
        for Sample {
            tangent,
            normal,
            binormal,
            ..
        } in samples
        {
            for v in [tangent, normal, binormal] {
                assert!((norm(v) - 1.0).abs() < 1e-9);
            }
            for (p, q) in [(tangent, normal), (normal, binormal), (binormal, tangent)] {
                assert!(dot(p, q).abs() < 1e-9);
            }
            assert!(norm(sub(cross(tangent, normal), binormal)) < 1e-9);
        }
    }
}

#[test]
fn untwisted_the_normal_never_turns_about_the_tangent() {
    let path = path();
    let samples = path.samples(2001, UP, &[]);
    let step = path.length() / 2000.0;
    // Where the spans meet the curvature jumps (C1), and a difference across it reads it.
    let joints: Vec<f64> = (1..POINTS.len())
        .map(|index| path.along[index * STEPS] / path.length())
        .collect();
    let mut moved = 0.0_f64;
    for five in samples.windows(5) {
        let here = five[2];
        if joints.iter().any(|&joint| (joint - here.s).abs() < 2e-3) {
            continue;
        }
        // The normal's derivative (central, fourth order) along the binormal: its turning about
        // the tangent.
        let near = times(sub(five[3].normal, five[1].normal), 8.0);
        let change = times(sub(near, sub(five[4].normal, five[0].normal)), 1.0 / 12.0);
        let turning = dot(change, here.binormal) / step;
        assert!(turning.abs() < 1e-6, "at {}: {turning}", here.s);
        moved = moved.max(norm(change) / step);
    }
    // The normal does move, with the tangent: the curve is no plane with `UP` square to it.
    assert!(moved > 0.1, "{moved}");
}

#[test]
fn a_half_turn_turns_the_normal_by_pi_over_its_span() {
    let path = path();
    let plain = path.samples(11, UP, &[]);
    for by in [PI, -PI] {
        let half = [Turn {
            at: 0.5,
            span: 0.2,
            by,
        }];
        let turned = path.samples(11, UP, &half);
        for (k, (plain, turned)) in plain.iter().zip(&turned).enumerate() {
            assert!(turned.at == plain.at && turned.tangent == plain.tangent);
            let angle = dot(turned.normal, plain.binormal).atan2(dot(turned.normal, plain.normal));
            // The span is 0.4 to 0.6: samples 4 to 6.
            let expected = match k {
                0..=4 => 0.0,
                5 => by / 2.0,
                _ => PI,
            };
            let off = if k > 5 {
                angle.abs() - expected
            } else {
                angle - expected
            };
            assert!(off.abs() < 1e-9, "{by}, sample {k}: {angle}");
        }
    }
}

#[test]
fn degenerate_paths_give_no_nan() {
    let p = [1.0, 2.0, 3.0];
    // Two equal points: a path of no length.
    let still = SpacePath::through(&[p, p]).expect("a point");
    assert!(still.length() == 0.0 && still.at(0.5) == p);
    let samples = still.samples(5, UP, &[]);
    assert!(finite(&samples) && samples.iter().all(|sample| sample.at == p));
    // A straight line, its middle point twice, `up` along it: the frame stays as it started.
    let line = [[0.0; 3], UP, UP, [0.0, 3.0, 0.0]];
    let line = SpacePath::through(&line).expect("points");
    assert!((line.length() - 3.0).abs() < 1e-12);
    let samples = line.samples(9, UP, &[]);
    assert!(finite(&samples));
    for sample in &samples {
        assert!(norm(sub(sample.at, [0.0, 3.0 * sample.s, 0.0])) < 1e-9);
        assert!(norm(sub(sample.tangent, UP)) < 1e-12);
        assert!(norm(sub(sample.normal, samples[0].normal)) < 1e-12);
    }
    // Nothing, no sample, an up of no length.
    assert!(SpacePath::through(&[]).is_none());
    assert!(path().samples(0, UP, &[]).is_empty());
    let one = path().samples(1, [0.0; 3], &[]);
    assert!(one.len() == 1 && finite(&one) && one[0].s == 0.0);
}
