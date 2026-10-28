//! Estimates on the flat (S90): each generated the same per seed and kept to its rules, its exact
//! answer on known cases, and the copy read as the last mark or line with its signed error.

use openpaint_practice::{Estimate, Flat, FlatAnswer, FlatCheck, Metric, Point, distance, value};

fn check(flat: &Flat, lines: &[&[Point]]) -> FlatCheck {
    FlatCheck::of(flat, lines)
}

fn inside(p: Point, margin: f64) -> bool {
    p.iter().all(|v| (margin..=1.0 - margin).contains(v))
}

fn heading([a, b]: [Point; 2]) -> f64 {
    (b[1] - a[1]).atan2(b[0] - a[0]).to_degrees()
}

#[test]
fn every_estimate_is_the_same_per_seed_and_keeps_its_rules() {
    for estimate in Estimate::ALL {
        assert_eq!(
            Flat::generate(estimate, 9),
            Flat::generate(estimate, 9),
            "{estimate:?}"
        );
        assert_ne!(Flat::generate(estimate, 9), Flat::generate(estimate, 10));
        assert_eq!(Flat::generate(estimate, 9).estimate(), estimate);
    }
    for seed in 0..500 {
        let Flat::Point { at } = Flat::generate(Estimate::Point, seed) else {
            panic!("a point")
        };
        assert!(inside(at, 0.1 - 1e-12), "{at:?}");
        let Flat::Proportion {
            original,
            share,
            copy,
        } = Flat::generate(Estimate::Proportion, seed)
        else {
            panic!("a proportion")
        };
        assert!(
            original
                .iter()
                .chain(&copy)
                .all(|p| inside(*p, 0.08 - 1e-9)),
            "{seed}: {original:?} {copy:?}"
        );
        assert!((0.15..=0.85).contains(&share));
        let ratio = distance(copy[0], copy[1]) / distance(original[0], original[1]);
        assert!(!(0.86..1.14).contains(&ratio), "another length: {ratio}");
        let turn = (heading(copy) - heading(original)).rem_euclid(360.0);
        let turn = turn.min(360.0 - turn);
        assert!((25.0 - 1e-9..=60.0 + 1e-9).contains(&turn), "turned {turn}");
        let Flat::Angle { original, through } = Flat::generate(Estimate::Angle, seed) else {
            panic!("an angle")
        };
        assert!(original.iter().all(|p| inside(*p, 0.08 - 1e-9)));
        assert!(inside(through, 0.3 - 1e-12));
        let level = heading(original).rem_euclid(90.0);
        assert!((6.0 - 1e-9..=84.0 + 1e-9).contains(&level), "{level}");
    }
}

#[test]
fn a_point_at_known_proportions_is_read_from_the_last_mark() {
    let flat = Flat::Point {
        at: [1.0 / 3.0, 0.5],
    };
    assert_eq!(flat.answer(), FlatAnswer::Point([1.0 / 3.0, 0.5]));
    // A dot at the half rather than the third, then (the last mark counting) a cross at the third.
    let dot: &[Point] = &[[0.5, 0.5], [0.502, 0.501]];
    let read = check(&flat, &[dot]);
    let across = value(&read.metrics, Metric::PointAcross).expect("measured");
    assert!((across - (0.501 - 1.0 / 3.0)).abs() < 1e-9, "{across}");
    let bar: &[Point] = &[[0.30, 0.5], [0.36, 0.5]];
    let upright: &[Point] = &[[1.0 / 3.0, 0.47], [1.0 / 3.0, 0.53]];
    let read = check(&flat, &[dot, bar, upright]);
    let mark = read.mark.expect("a cross");
    assert!(distance(mark, [1.0 / 3.0, 0.5]) < 1e-12, "{mark:?}");
    for metric in [Metric::PointAcross, Metric::PointDown] {
        assert!(value(&read.metrics, metric).is_some_and(|v| v.abs() < 1e-12));
    }
    // A long line drawn with Draw is no mark; nothing marked, nothing kept.
    let long: &[Point] = &[[0.1, 0.1], [0.9, 0.8]];
    let read = check(&flat, &[long]);
    assert_eq!((read.mark, read.metrics.len()), (None, 0));
}

#[test]
fn a_proportion_is_read_along_the_copy_s_segment() {
    let flat = Flat::Proportion {
        original: [[0.1, 0.2], [0.9, 0.2]],
        share: 0.25,
        copy: [[0.2, 0.6], [0.8, 0.6]],
    };
    let FlatAnswer::Point(answer) = flat.answer() else {
        panic!("a point")
    };
    assert!(distance(answer, [0.35, 0.6]) < 1e-12);
    // A tick across the segment a little towards its far end: off by 0.03 of the segment.
    let tick: &[Point] = &[[0.368, 0.57], [0.368, 0.63]];
    let off = value(&check(&flat, &[tick]).metrics, Metric::ProportionOff).expect("measured");
    assert!((off - 0.03).abs() < 1e-9, "{off}");
}

#[test]
fn an_angle_is_read_from_the_last_line_whichever_way_it_was_drawn() {
    let flat = Flat::Angle {
        original: [[0.2, 0.8], [0.8, 0.2]],
        through: [0.5, 0.5],
    };
    let FlatAnswer::Line([a, b]) = flat.answer() else {
        panic!("a line")
    };
    assert!((heading([a, b]) - heading([[0.2, 0.8], [0.8, 0.2]])).abs() < 1e-9);
    let middle = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    assert!(
        distance(middle, [0.5, 0.5]) < 1e-9,
        "through the point, as long"
    );
    assert!((distance(a, b) - distance([0.2, 0.8], [0.8, 0.2])).abs() < 1e-9);
    // Drawn from its top end down, turned 5 degrees clockwise; a tick after it is no line.
    let turned = (-45.0_f64 + 5.0).to_radians();
    let (sin, cos) = turned.sin_cos();
    let line: &[Point] = &[
        [0.5 + 0.3 * cos, 0.5 + 0.3 * sin],
        [0.5 - 0.3 * cos, 0.5 - 0.3 * sin],
    ];
    let tick: &[Point] = &[[0.3, 0.3], [0.31, 0.3]];
    let read = check(&flat, &[line, tick]);
    let off = value(&read.metrics, Metric::AngleOff).expect("measured");
    assert!((off - 5.0).abs() < 1e-6, "{off}");
    let exact: &[Point] = &[[0.3, 0.7], [0.7, 0.3]];
    let read = check(&flat, &[line, exact]);
    let off = value(&read.metrics, Metric::AngleOff).expect("measured");
    assert!(off.abs() < 1e-6, "the last line counts: {off}");
    // Near the frame's edge the answer stops at it.
    let near = Flat::Angle {
        original: [[0.2, 0.8], [0.8, 0.2]],
        through: [0.1, 0.5],
    };
    let FlatAnswer::Line(ends) = near.answer() else {
        panic!("a line")
    };
    assert!(ends.iter().all(|p| inside(*p, -1e-12)), "{ends:?}");
}
